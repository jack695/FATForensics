//! FAT volume structure and operations.
//!
//! This module implements the core functions to interact with a FAT volume, including:
//! - Reading and validating the BPB
//! - Listing directory entries
//! - Finding files and clusters
//! - Writing to slack space
//! - Displaying the volume layout

use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::{io, result};

use super::bpb::Bpb;
use super::dir_entry::DirEntry;
use super::fat_error::FATError;
use super::fat_type::FATType;
use crate::filesystem::dir_entry;
use crate::traits::{LayoutDisplay, SlackWriter, TraitError, TreeDisplay};
use crate::utils::{read_sector, u32_at, write_at};

/// Structure for a FAT volume.
///
/// Essentially, it is a wrapper around the Bpb.
pub struct FATVol {
    bpb: Bpb,
    start: u32,
    end: u32,
    disk_path: PathBuf,
}

impl FATVol {
    /// Reads the Bpb from a file at the specified sector and optionally validates the volume.
    ///
    /// # Parameters
    /// - `file`: The file containing the filesystem
    /// - `sector`: The sector number where the Bpb is located
    /// - `validate`: Whether to perform validation checks on the Bpb
    /// - `sector_size`: The size of each sector in bytes
    ///
    /// # Returns
    /// - `Ok(FATVol)`: The FAT volume
    /// - `Err(FATError)`: If reading fails or validation fails
    ///
    /// # Errors
    /// - Returns `FATError::IOError` if reading from the file fails
    /// - Returns various `FATError` variants if validation fails and `validate` is true
    pub fn from_file(
        disk_path: &Path,
        start: u32,
        sector_cnt: u32,
        validate: bool,
        sector_size: usize,
    ) -> Result<FATVol, FATError> {
        let mut file = File::open(disk_path)?;
        let bpb = Bpb::from(&mut file, start, validate, sector_size)?;

        Ok(Self {
            bpb,
            start,
            end: start + sector_cnt,
            disk_path: disk_path.to_path_buf(),
        })
    }

    /// Find a file in the FAT volume and return its first cluster number.
    ///
    /// # Parameters
    /// - `file_path`: The path of the file to find
    ///
    /// # Returns
    /// - `u32`: The first cluster number of the file if found, otherwise `0`.
    pub fn find_file(&self, file_path: &Path) -> Result<DirEntry, FATError> {
        if file_path.components().count() == 0 {
            return Err(FATError::FileNotFound);
        }

        let fat_type = self.bpb.fat_type();
        let root_dir_cluster = match fat_type {
            FATType::FAT12 => return Err(FATError::UnsupportedFATType(fat_type.to_string())),
            FATType::FAT16 => return Err(FATError::UnsupportedFATType(fat_type.to_string())),
            _ => *self.bpb.root_clus(),
        };

        self.find_file_rec(file_path, root_dir_cluster)
    }

    fn find_file_rec(
        &self,
        file_path: &Path,
        fst_cluster: u32,
    ) -> Result<dir_entry::DirEntry, FATError> {
        let mut parts = file_path.components();
        let current_part = match parts.next() {
            Some(part) => part,
            None => return Err(FATError::FileNotFound),
        };
        let remaining: PathBuf = parts.clone().collect();

        let dir_entries: Vec<DirEntry> = if parts.count() > 0 {
            self.list_dir(fst_cluster)?
                .iter()
                .filter(|entry| entry.is_dir())
                .cloned()
                .collect()
        } else {
            self.list_dir(fst_cluster)?
                .iter()
                .filter(|entry| !entry.is_dir())
                .cloned()
                .collect()
        };

        for dir_entry in dir_entries.iter() {
            if dir_entry.same_short_name(current_part.as_os_str().to_str().unwrap()) {
                if dir_entry.is_dir() {
                    return self.find_file_rec(remaining.as_path(), dir_entry.cluster_number());
                } else {
                    return Ok(dir_entry.clone());
                }
            }
        }

        Err(FATError::FileNotFound)
    }

    pub fn list_dir(&self, first_cluster: u32) -> Result<Vec<DirEntry>, FATError> {
        match first_cluster {
            0 => return Err(FATError::InvalidClusterError(0)),
            1 => return Err(FATError::InvalidClusterError(1)),
            _ => {}
        }

        let clusters = self.list_clusters(first_cluster)?;
        let mut dir_entries = vec![];

        for cluster_nb in clusters {
            let buf = self.read_cluster(cluster_nb)?;

            for off in (0..buf.len()).step_by(32) {
                if u32_at(&buf, off) != 0 {
                    dir_entries.push(DirEntry::from_slice(&buf[off..])?);
                }
            }
        }

        Ok(dir_entries)
    }

    fn read_cluster(&self, cluster_nb: u32) -> io::Result<Vec<u8>> {
        let mut file = File::open(&self.disk_path).unwrap();

        let cluster_size = *self.bpb.sec_per_clus() as u16 * *self.bpb.bytes_per_sec();
        let mut buf: Vec<u8> = vec![0; cluster_size.into()];

        file.seek(SeekFrom::Start(
            (*self.bpb.bytes_per_sec() as u32 * self.clus_to_sector(cluster_nb)).into(),
        ))?;

        file.read_exact(&mut buf).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Failed to read cluster {cluster_nb}: {err}"),
            )
        })?;

        Ok(buf)
    }

    fn list_clusters(&self, cluster: u32) -> Result<Vec<u32>, FATError> {
        match cluster {
            0 => return Err(FATError::InvalidClusterError(0)),
            1 => return Err(FATError::InvalidClusterError(1)),
            _ => {}
        }

        let mut all_clusters = vec![];
        let mut cluster = cluster;

        while !DirEntry::is_eof(cluster, self.bpb.fat_type()) {
            all_clusters.push(cluster);
            cluster = self.get_next_cluster(cluster);
        }
        Ok(all_clusters)
    }

    fn get_next_cluster(&self, cluster: u32) -> u32 {
        let mut file = File::open(&self.disk_path).unwrap();
        let mut buf = vec![];
        let sector = self.fat_start()
            + (cluster * self.fat_entry_bit_sz() / 8) / (*self.bpb.bytes_per_sec() as u32);

        let err_msg = format!("Couldn't read sector {sector}").to_string();
        read_sector(
            &mut file,
            sector.into(),
            (*self.bpb.bytes_per_sec()).into(),
            &mut buf,
        )
        .expect(&err_msg);

        u32_at(
            &buf,
            (cluster * self.fat_entry_bit_sz() / 8 % *self.bpb.bytes_per_sec() as u32) as usize,
        ) & 0x0FFFFFFF
    }

    pub fn mark_as_bad(&self, cluster_cnt: u32) -> Result<u32, FATError> {
        let mut start = 2;
        let mut i = 0;

        while start + i < self.bpb.cluster_count() + 2 {
            if self.get_next_cluster(start + i) != 0 || !self.is_zero_cluster(start + i)? {
                start = start + i + 1;
                i = 0;
            } else {
                i += 1;
            }

            if i == cluster_cnt {
                // Found a list of `cluster_cnt` free clusters
                for cluster in start..start + cluster_cnt {
                    self.update_fat_entry(
                        cluster,
                        DirEntry::bad_cluster_marker(self.bpb.fat_type()),
                    )?;
                }

                return Ok(start);
            }
        }

        Err(FATError::NoFreeClusterChain(cluster_cnt))
    }

    fn is_zero_cluster(&self, cluster: u32) -> io::Result<bool> {
        let mut buffer = Vec::new();
        let mut disk_file = File::open(&self.disk_path).unwrap();

        for i in 0..*self.bpb.sec_per_clus() {
            read_sector(
                &mut disk_file,
                self.clus_to_sector(cluster) as u64 + i as u64,
                *self.bpb.bytes_per_sec() as usize,
                &mut buffer,
            )?;

            for byte in &buffer {
                if *byte != 0 {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    pub fn cluster_size(&self) -> u32 {
        *self.bpb.bytes_per_sec() as u32 * *self.bpb.sec_per_clus() as u32
    }

    fn update_fat_entry(&self, cluster_nb: u32, value: u32) -> io::Result<()> {
        // Prepare the data to write
        let mut data: Vec<u8> = Vec::new();
        let mut mask = 0xff000000;
        for i in 1..5 {
            data.push(((value & mask) >> (32 - i * 8)) as u8);
            mask >>= 8;
        }

        // Update the entry for every fat structure
        for i in 0..*self.bpb.num_fat() {
            let off = (self.fat_start() as u64 + i as u64 * self.bpb.fat_sz() as u64)
                * *self.bpb.bytes_per_sec() as u64
                + (cluster_nb as u64 * self.fat_entry_bit_sz() as u64 / 8);

            let mut disk_file = File::options()
                .write(true)
                .read(true)
                .open(&self.disk_path)?;

            write_at(&mut disk_file, off, &data)?
        }

        Ok(())
    }

    fn fat_entry_bit_sz(&self) -> u32 {
        match self.bpb.fat_type() {
            FATType::FAT12 => 12,
            FATType::FAT16 => 16,
            FATType::FAT32 => 32,
        }
    }

    /// Recursively prints the directory tree starting from the given cluster.
    ///
    /// # Parameters
    /// - `cluster`: The starting cluster number for the directory.
    /// - `indent`: The indentation level for pretty-printing.
    ///
    /// # Returns
    /// - `Ok(())` if the directory tree is printed successfully.
    /// - `Err(FATError)` if an error occurs while listing directories.
    fn print_dir_rec(&self, cluster: u32, indent: usize) -> Result<(), FATError> {
        let dir_entries = self.list_dir(cluster)?;

        for entry in dir_entries {
            println!("{} {}", " ".repeat(indent), entry);
            if entry.is_regular_dir() {
                self.print_dir_rec(entry.cluster_number(), indent + 3)?;
            }
        }

        Ok(())
    }

    /// Converts a cluster number to its corresponding sector number.
    ///
    /// # Parameters
    /// - `cluster`: The cluster number to convert.
    ///
    /// # Returns
    /// - The sector number corresponding to the given cluster.
    pub fn clus_to_sector(&self, cluster: u32) -> u32 {
        self.data_start() + (cluster - 2) * *self.bpb.sec_per_clus() as u32
    }

    /// Returns the starting cluster of the volume.
    pub fn start(&self) -> u32 {
        self.start
    }

    /// Returns the starting sector of the reserved region.
    fn rsvd_start(&self) -> u32 {
        self.start()
    }

    /// Returns the starting sector of the first FAT.
    fn fat_start(&self) -> u32 {
        self.rsvd_start() + u32::from(*self.bpb.rsvd_sec_cnt())
    }

    /// Returns the starting sector of the root directory.
    fn root_start(&self) -> u32 {
        self.fat_start() + self.bpb.fat_sz() * *self.bpb.num_fat() as u32
    }

    /// Returns the starting sector of the data region.
    pub fn data_start(&self) -> u32 {
        self.root_start()
            + (*self.bpb.root_ent_cnt() as u32 * 32).div_ceil(*self.bpb.bytes_per_sec() as u32)
    }

    /// Returns the ending sector of the data region.
    fn data_end(&self) -> u32 {
        self.data_start() + self.bpb.cluster_count() * *self.bpb.sec_per_clus() as u32
    }
}

/// Implements the LayoutDisplay trait for Bpb
impl LayoutDisplay for FATVol {
    fn display_layout(&self, indent: u8) -> Result<String, std::fmt::Error> {
        let mut out = String::from("");
        let indent = " ".repeat(indent.into());

        writeln!(out, "{}┌{:─^55}┐", indent, " FAT32 Partition Layout ")?;
        writeln!(
            out,
            "{}├{:^12}┬{:^12}┬{:^12}┬{:^16}┤",
            indent, "Region", "Start", "End", "Description"
        )?;
        writeln!(
            out,
            "{}├{:─<12}┼{:─<12}┼{:─<12}┼{:─<16}┤",
            indent, "", "", "", ""
        )?;

        writeln!(
            out,
            "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
            indent,
            "Reserved",
            self.rsvd_start(),
            self.fat_start(),
            "Boot + Reserved"
        )?;
        for i in 0..*self.bpb.num_fat() {
            let fat_i_start = self.fat_start() + i as u32 * self.bpb.fat_sz();
            let fat_i_end = fat_i_start + self.bpb.fat_sz();
            writeln!(
                out,
                "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
                indent,
                format!("FAT #{}", i),
                fat_i_start,
                fat_i_end,
                "FAT Tables"
            )?;
        }
        if self.bpb.fat_type() != FATType::FAT32 {
            writeln!(
                out,
                "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
                indent,
                "Root Dir",
                self.root_start(),
                self.data_start(),
                "Root Directory"
            )?;
        }
        writeln!(
            out,
            "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
            indent,
            "Data",
            self.data_start(),
            self.data_end(),
            "Cluster Data"
        )?;
        if self.data_end() < self.end {
            writeln!(
                out,
                "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
                indent,
                "",
                self.data_end(),
                self.end,
                "Volume Slack"
            )?;
        }

        writeln!(
            out,
            "{}└{:─<12}┴{:─<12}┴{:─<12}┴{:─<16}┘",
            indent, "", "", "", ""
        )?;

        Ok(out)
    }
}

impl TreeDisplay for FATVol {
    fn display_tree(&self) -> Result<(), TraitError> {
        match self.bpb.fat_type() {
            FATType::FAT32 => self.print_dir_rec(*self.bpb.root_clus(), 0)?,
            fat_type => {
                return Err(TraitError::FATError(FATError::UnsupportedFATType(format!(
                    "Displaying the directory tree for {fat_type} is currently not supported."
                ))));
            }
        }

        Ok(())
    }
}

impl SlackWriter for FATVol {
    fn write_to_volume_slack<T: io::Write + io::Seek>(
        &self,
        writer: &mut T,
        data: &[u8],
    ) -> result::Result<(), FATError> {
        let slack_sector_cnt = self.end - self.data_end();
        if (slack_sector_cnt * *self.bpb.bytes_per_sec() as u32) < data.len() as u32 {
            return Err(FATError::InsufficientSlackSpace {
                free: slack_sector_cnt * *self.bpb.bytes_per_sec() as u32,
                needed: data.len() as u32,
            });
        }

        writer.seek(std::io::SeekFrom::Start(
            (self.data_end() * *self.bpb.bytes_per_sec() as u32).into(),
        ))?;
        writer.write_all(data)?;
        Ok(())
    }

    fn write_to_file_slack<T: io::Write + io::Seek>(
        &self,
        disk_file: &mut T,
        file_path: &Path,
        data: &[u8],
    ) -> result::Result<(), FATError> {
        // Checks the file isn't empty and has at least one allocated cluster
        let entry = self.find_file(file_path)?;
        if *entry.file_size() == 0 && entry.cluster_number() == 0 {
            return Err(FATError::InsufficientSlackSpace {
                free: 0,
                needed: data.len() as u32,
            });
        }

        let clusters = self.list_clusters(entry.cluster_number())?;
        let slack_byte_size =
            clusters.len() * *self.bpb.sec_per_clus() as usize * *self.bpb.bytes_per_sec() as usize
                - *entry.file_size() as usize;
        let cluster_size = *self.bpb.sec_per_clus() as u32 * *self.bpb.bytes_per_sec() as u32;

        if data.len() > slack_byte_size {
            return Err(FATError::InsufficientSlackSpace {
                free: slack_byte_size as u32,
                needed: data.len() as u32,
            });
        }

        // Note: Technically, we could allocate extra clusters for a file to extend the slack space.
        // However, this is not supported for now.
        if data.len().div_ceil(cluster_size as usize) > 1 {
            return Err(FATError::UnsupportedFeature(
                "Writing data to a file slack which spans over more than one cluster is not currently supported.".to_string(),
            ));
        }

        match clusters.last() {
            Some(last_cluster) => {
                let offset = (self.clus_to_sector(*last_cluster) as u64)
                    * *self.bpb.bytes_per_sec() as u64
                    + (*entry.file_size() as u64) % (cluster_size as u64);
                write_at(disk_file, offset, data)?;
            }
            _ => {
                return Err(FATError::InsufficientSlackSpace {
                    free: 0,
                    needed: data.len() as u32,
                });
            }
        };

        Ok(())
    }
}
