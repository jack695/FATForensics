//! FAT32 volume structure.
//!
//! This module implements the core functions to interact with a FAT volume.
//! Example: to write a file in the volume slack or to display its layout.

use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io;

use super::bpb::BPB;
use super::fat_error::FATError;
use crate::traits::{LayoutDisplay, SlackWriter};

/// Structure for a FAT volume.
///
/// Essentially, it is a wrapper around the BPB.
pub struct FATVol {
    bpb: BPB,
    start: u32,
    end: u32,
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
        file: &mut File,
        start: u32,
        sector_cnt: u32,
        validate: bool,
        sector_size: usize,
    ) -> Result<FATVol, FATError> {
        let bpb = BPB::from_file(file, start, validate, sector_size)?;

        Ok(Self {
            bpb: bpb,
            start: start,
            end: start + sector_cnt,
        })
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

    fn data_start(&self) -> u32 {
        self.fat_start() + self.bpb.fat_sz() * self.bpb.num_fat as u32
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
