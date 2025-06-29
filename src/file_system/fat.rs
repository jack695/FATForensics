//! FAT32 volume structure.
//!
//! This module implements the core functions to interact with a FAT volume.
//! Example: to write a file in the volume slack or to display its layout.

use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::bpb::BPB;
use super::dir_entry::DirEntry;
use super::fat_error::FATError;
use super::fat_type::FATType;
use crate::traits::{LayoutDisplay, SlackWriter};
use crate::utils::{read_sector, u32_at};

/// Structure for a FAT volume.
///
/// Essentially, it is a wrapper around the BPB.
pub struct FATVol {
    bpb: BPB,
    start: u32,
    end: u32,
    disk_path: String,
}

impl FATVol {
    /// Reads the BPB from a file at the specified sector and optionally validates the volume.
    ///
    /// # Parameters
    /// - `file`: The file containing the filesystem
    /// - `sector`: The sector number where the BPB is located
    /// - `validate`: Whether to perform validation checks on the BPB
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
        disk_path: &str,
        start: u32,
        sector_cnt: u32,
        validate: bool,
        sector_size: usize,
    ) -> Result<FATVol, FATError> {
        let mut file = File::open(disk_path).unwrap();
        let bpb = BPB::from_file(&mut file, start, validate, sector_size)?;

        Ok(Self {
            bpb: bpb,
            start: start,
            end: start + sector_cnt,
            disk_path: disk_path.to_string(),
        })
    }

    /// Find a file in the FAT volume and return its first cluster number.
    ///
    /// # Parameters
    /// - `file_path`: The path of the file to find
    ///
    /// # Returns
    /// - `u32`: The first cluster number of the file if found, otherwise `0`.
    pub fn find_file(&self, file_path: &Path) -> Result<u32, FATError> {
        if file_path.components().count() == 0 {
            return Err(FATError::FileNotFound);
        }

        let fat_type = self.bpb.fat_type();
        let root_dir_cluster = match fat_type {
            FATType::FAT12 => return Err(FATError::UnsupportedFATType(fat_type.to_string())),
            FATType::FAT16 => return Err(FATError::UnsupportedFATType(fat_type.to_string())),
            _ => self.bpb.root_clus,
        };

        self.find_file_rec(file_path, root_dir_cluster)
    }

    fn find_file_rec(&self, file_path: &Path, fst_cluster: u32) -> Result<u32, FATError> {
        let mut parts = file_path.components();
        let current_part = parts.next().unwrap();
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
                    return Ok(dir_entry.cluster_number());
                }
            }
        }

        Err(FATError::FileNotFound)
    }

    pub fn list_dir(&self, first_cluster: u32) -> io::Result<Vec<DirEntry>> {
        let clusters = self.list_clusters(first_cluster);
        let mut dir_entries = vec![];

        for cluster_nb in clusters {
            let buf = self.read_cluster(cluster_nb)?;

            for off in (0..buf.len()).step_by(32) {
                if u32_at(&buf, off) != 0 {
                    dir_entries.push(DirEntry::from_slice(&buf[off..]));
                }
            }
        }

        Ok(dir_entries)
    }

    fn read_cluster(&self, cluster_nb: u32) -> io::Result<Vec<u8>> {
        let mut file = File::open(self.disk_path.as_str()).unwrap();

        let cluster_size = self.bpb.sec_per_clus as u16 * self.bpb.bytes_per_sec;
        let mut buf: Vec<u8> = vec![0; cluster_size.into()];

        file.seek(SeekFrom::Start(
            (self.bpb.bytes_per_sec as u32 * self.clus_to_sector(cluster_nb)).into(),
        ))?;

        file.read_exact(&mut buf).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Failed to read cluster {}: {}", cluster_nb, err),
            )
        })?;

        Ok(buf)
    }

    fn list_clusters(&self, cluster: u32) -> Vec<u32> {
        let mut all_clusters = vec![];
        let mut cluster = cluster;

        while !self.is_eof(cluster) {
            all_clusters.push(cluster);
            cluster = self.get_next_cluster(cluster);
        }
        all_clusters
    }

    fn get_next_cluster(&self, cluster: u32) -> u32 {
        let mut file = File::open(self.disk_path.as_str()).unwrap();
        let mut buf = vec![];
        let sector = self.fat_start()
            + (cluster * self.fat_entry_bit_sz() / 8) / (self.bpb.bytes_per_sec as u32);

        read_sector(
            &mut file,
            sector.into(),
            (self.bpb.bytes_per_sec).into(),
            &mut buf,
        )
        .expect(format!("Couldn't read sector {}.", sector).as_str());

        u32_at(
            &buf,
            (cluster * self.fat_entry_bit_sz() / 8 % self.bpb.bytes_per_sec as u32) as usize,
        ) & 0x0FFFFFFF
    }

    fn is_eof(&self, cluster: u32) -> bool {
        match self.bpb.fat_type() {
            FATType::FAT12 => cluster >= 0x0FF8,
            FATType::FAT16 => cluster >= 0xFFF8,
            FATType::FAT32 => cluster >= 0x0FFFFFF8,
        }
    }

    fn is_bad_cluster(&self, cluster: u32) -> bool {
        match self.bpb.fat_type() {
            FATType::FAT12 => cluster == 0x0FF7,
            FATType::FAT16 => cluster == 0xFFF7,
            FATType::FAT32 => cluster == 0x0FFFFFF7,
        }
    }

    fn fat_entry_bit_sz(&self) -> u32 {
        match self.bpb.fat_type() {
            FATType::FAT12 => 12,
            FATType::FAT16 => 16,
            FATType::FAT32 => 32,
        }
    }

    /// Returns the starting sector of the root directory.
    ///
    /// # Returns
    /// - `u32`: The starting sector of the root directory.
    pub fn root_dir_sector(&self) -> u32 {
        match self.bpb.fat_type() {
            FATType::FAT32 => self.clus_to_sector(self.bpb.root_clus),
            _ => self.rsvd_start(),
        }
    }

    fn clus_to_sector(&self, cluster: u32) -> u32 {
        self.data_start() + (cluster - 2) * self.bpb.sec_per_clus as u32
    }

    pub fn start(&self) -> u32 {
        self.start
    }

    fn rsvd_start(&self) -> u32 {
        self.start()
    }

    fn fat_start(&self) -> u32 {
        self.rsvd_start() + u32::from(self.bpb.rsvd_sec_cnt)
    }

    fn root_start(&self) -> u32 {
        self.fat_start() + self.bpb.fat_sz() * self.bpb.num_fat as u32
    }

    fn data_start(&self) -> u32 {
        self.root_start()
            + (self.bpb.root_ent_cnt as u32 * 32).div_ceil(self.bpb.bytes_per_sec as u32)
    }

    fn data_end(&self) -> u32 {
        self.data_start() + self.bpb.cluster_count() * self.bpb.sec_per_clus as u32
    }
}

/// Implements the LayoutDisplay trait for BPB
impl LayoutDisplay for FATVol {
    fn display_layout(&self, indent: u8) -> String {
        let mut out = String::from("");
        let indent = " ".repeat(indent.into());

        writeln!(out, "{}┌{:─^55}┐", indent, " FAT32 Partition Layout ").unwrap();
        writeln!(
            out,
            "{}├{:^12}┬{:^12}┬{:^12}┬{:^16}┤",
            indent, "Region", "Start", "End", "Description"
        )
        .unwrap();
        writeln!(
            out,
            "{}├{:─<12}┼{:─<12}┼{:─<12}┼{:─<16}┤",
            indent, "", "", "", ""
        )
        .unwrap();

        writeln!(
            out,
            "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
            indent,
            "Reserved",
            self.rsvd_start(),
            self.fat_start(),
            "Boot + Reserved"
        )
        .unwrap();
        for i in 0..self.bpb.num_fat {
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
            )
            .unwrap();
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
            )
            .unwrap();
        }
        writeln!(
            out,
            "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
            indent,
            "Data",
            self.data_start(),
            self.data_end(),
            "Cluster Data"
        )
        .unwrap();
        if self.data_end() < self.end {
            writeln!(
                out,
                "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
                indent,
                "",
                self.data_end(),
                self.end,
                "Volume Slack"
            )
            .unwrap();
        }

        writeln!(
            out,
            "{}└{:─<12}┴{:─<12}┴{:─<12}┴{:─<16}┘",
            indent, "", "", "", ""
        )
        .unwrap();

        out
    }
}

impl SlackWriter for FATVol {
    fn write_to_volume_slack<T: io::Write + io::Seek>(
        &self,
        writer: &mut T,
        data: &[u8],
    ) -> io::Result<()> {
        let slack_sector_cnt = self.end - self.data_end();
        if (slack_sector_cnt * self.bpb.bytes_per_sec as u32) < data.len() as u32 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Volume slack space ({} sector(s)) isn't large enough to write {} bytes.",
                    slack_sector_cnt,
                    data.len()
                ),
            ));
        }

        writer.seek(std::io::SeekFrom::Start(
            (self.data_end() * self.bpb.bytes_per_sec as u32).into(),
        ))?;
        writer.write_all(data)?;
        Ok(())
    }
}
