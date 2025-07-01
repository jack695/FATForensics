//! This module provides functionality for parsing and handling partition tables
//! and Master Boot Records (MBR) in the FAT32 file system.
//!
//! It defines structures and methods to interpret partition table entries,
//! validate partition tables, and extract relevant metadata from disk images.
use getset::Getters;
use std::fs::File;
use std::vec;

use super::disk_error::DiskError;
use crate::traits::LayoutDisplay;
use crate::utils;
use std::fmt::Write;
use std::fmt::{self, Display};

/// The number of primary partitions supported by MBR.
pub const PART_CNT: usize = 4;

/// Represents the type of a partition table entry.
#[derive(Debug)]
pub enum PTType {
    /// Logical Block Addressing (LBA) FAT32 partition type.
    LBAFat32,
    /// Unsupported partition type, encapsulating the raw type byte.
    Unsupported(u8),
}

impl Display for PTType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PTType::LBAFat32 => write!(f, "LBA FAT32"),
            PTType::Unsupported(b) => write!(f, "Unsupported: 0x{:02X}", b),
        }
    }
}

impl PTType {
    /// Creates a `PTType` instance from a raw byte.
    ///
    /// # Parameters
    /// - `byte`: A single byte representing the partition type.
    ///
    /// # Returns
    /// - `PTType::LBAFat32` if the byte matches the FAT32 LBA type (0x0C).
    /// - `PTType::Unsupported(byte)` for any other value.
    fn from_byte(byte: u8) -> Self {
        match byte {
            0x0C => PTType::LBAFat32,
            _ => PTType::Unsupported(byte),
        }
    }
}

/// Represents a single partition table entry.
#[derive(Debug, Getters)]
pub struct PTEntry {
    /// The type of the partition.
    #[get = "pub(super)"]
    pt_type: PTType,
    /// The starting Logical Block Address (LBA) of the partition.
    #[get = "pub(super)"]
    lba_start: u32,
    /// The number of sectors in the partition.
    #[get = "pub(super)"]
    sector_cnt: u32,
}

/// Represents the boot signature of a Master Boot Record (MBR).
#[derive(Debug)]
enum BootSignature {
    /// Standard MBR boot signature (0x55AA).
    Mbr(u16),
    /// Unsupported boot signature, encapsulating the raw value.
    Unsupported(u16),
}

impl BootSignature {
    /// Creates a `BootSignature` instance from a `u16` value.
    ///
    /// # Parameters
    /// - `sig`: A 16-bit unsigned integer representing the boot signature.
    ///
    /// # Returns
    /// - `BootSignature::Mbr` if the signature matches `0x55AA`.
    /// - `BootSignature::Unsupported(other)` for any other value.
    pub fn from_u16(sig: u16) -> BootSignature {
        match sig {
            // The signature 0x55AA is stored on disk in little-endian byte order.
            0xAA55 => BootSignature::Mbr(0xAA55),
            other => BootSignature::Unsupported(other),
        }
    }
}

/// Implements the trait Display for BootSignature by displaying its hex value.
impl fmt::Display for BootSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootSignature::Mbr(sig) => write!(f, "0x{:04X}", sig),
            BootSignature::Unsupported(sig) => write!(f, "0x{:04X}", sig),
        }
    }
}

/// Represents a Master Boot Record (MBR), including partition table entries
/// and the boot signature.
#[derive(Debug)]
pub struct Mbr {
    /// The partition table entries in the MBR.
    pt_entries: [PTEntry; PART_CNT],
    /// The boot signature of the MBR.
    boot_signature: BootSignature,
    sector_cnt: u64,
}

impl Mbr {
    /// Reads and parses an MBR from a file.
    ///
    /// # Parameters
    /// - `file`: A mutable reference to a `File` object representing the disk image.
    ///
    /// # Returns
    /// - `Ok(MBR)` if the MBR is successfully parsed.
    /// - `Err(std::io::Error)` if an error occurs during reading or parsing.
    pub fn from_file(file: &mut File, sector_size: usize) -> Result<Mbr, DiskError> {
        let mut buffer = vec![0; sector_size];
        utils::read_sector(file, 0, sector_size, &mut buffer)?;

        let pt_entries: [PTEntry; PART_CNT] = core::array::from_fn(|i| {
            let offset = 446 + i * 16;
            PTEntry {
                pt_type: PTType::from_byte(utils::u8_at(&buffer, offset + 0x04)),
                lba_start: utils::u32_at(&buffer, offset + 0x08),
                sector_cnt: utils::u32_at(&buffer, offset + 0x0C),
            }
        });

        let mbr = Mbr {
            pt_entries,
            boot_signature: BootSignature::from_u16(utils::u16_at(&buffer, 510)),
            sector_cnt: file.metadata()?.len() / sector_size as u64,
        };

        mbr.validate()
    }

    /// Returns a vector of references to non-empty partition table entries.
    ///
    /// This method filters the partition table entries to exclude any entries
    /// with a sector count of zero, as these entries are considered empty.
    ///
    /// # Returns
    /// - A `Vec` containing references to `PTEntry` instances that have a non-zero sector count.
    pub fn pt_entries(&self) -> Vec<&PTEntry> {
        self.pt_entries
            .iter()
            .filter(|entry| entry.sector_cnt != 0)
            .collect()
    }

    /// Validates the MBR by checking the partition table and boot signature.
    ///
    /// # Returns
    /// - `Ok(Self)` if the MBR is valid.
    /// - `Err(DiskError)` if any validation step fails.
    fn validate(self) -> Result<Self, DiskError> {
        self.check_partition_table_sorted()?
            .check_partitions_non_overlapping()?
            .check_signature()
    }

    /// Checks if the boot signature is valid.
    ///
    /// # Returns
    /// - `Ok(Self)` if the boot signature is valid.
    /// - `Err(DiskError::InvalidSignature)` if the boot signature is unsupported.
    fn check_signature(self) -> Result<Self, DiskError> {
        match self.boot_signature {
            BootSignature::Unsupported(sig) => Err(DiskError::InvalidSignature(sig)),
            _ => Ok(self),
        }
    }

    /// Checks if the partition table entries are sorted by their starting LBA.
    ///
    /// # Returns
    /// - `Ok(Self)` if the entries are sorted.
    /// - `Err(DiskError::PartitionTableNotSorted)` if the entries are not sorted.
    fn check_partition_table_sorted(self) -> Result<Self, DiskError> {
        match self
            .pt_entries()
            .windows(2)
            .all(|pair| pair[0].lba_start <= pair[1].lba_start)
        {
            true => Ok(self),
            false => Err(DiskError::PartitionTableNotSorted),
        }
    }

    /// Checks if the partition table entries are non-overlapping.
    ///
    /// # Returns
    /// - `Ok(Self)` if the entries do not overlap.
    /// - `Err(DiskError::OverlappingPartitions)` if any entries overlap.
    fn check_partitions_non_overlapping(self) -> Result<Self, DiskError> {
        match self
            .pt_entries()
            .windows(2)
            .any(|pair| pair[0].lba_start + pair[0].sector_cnt > pair[1].lba_start)
        {
            true => Err(DiskError::OverlappingPartitions),
            false => Ok(self),
        }
    }
}

/// Prints the layout of the disk based on the provided Master Boot Record (MBR).
///
/// # Parameters
/// - `mbr`: A reference to an `MBR` instance representing the parsed Master Boot Record.
///
/// # Behavior
/// - Prints the MBR sector range.
/// - Iterates through the partition table entries and prints their sector ranges.
impl LayoutDisplay for Mbr {
    fn display_layout(&self, indent: u8) -> String {
        let mut out = String::from("");
        let indent = " ".repeat(indent.into());

        let mut last_end = 0;
        let disk_end = self.sector_cnt;

        writeln!(out, "{}┌{:─^55}┐", indent, " Master Boot Record Layout ").unwrap();
        writeln!(out, "{}├{:<45}{:>10}┤", indent, "Disk Size", disk_end,).unwrap();
        writeln!(
            out,
            "{}├{:<45}{:>10}┤",
            indent,
            "Boot Signature",
            format!("{:>10}", self.boot_signature)
        )
        .unwrap();
        writeln!(out, "{}├{:─^55}┤", indent, "").unwrap();

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

        for (i, entry) in self.pt_entries().iter().enumerate() {
            let start = u64::from(*entry.lba_start());
            let end = start + u64::from(*entry.sector_cnt());

            if start > last_end {
                writeln!(
                    out,
                    "{}│{:^12}│{:>12}│{:>12}│{:^16}│",
                    indent, "", last_end, start, "Unallocated"
                )
                .unwrap();
            }

            writeln!(
                out,
                "{}│{:^12}│{:>12}│{:>12}│{:^16}│",
                indent,
                format!("Part #{}", i + 1),
                start,
                end,
                format!("{:}", entry.pt_type())
            )
            .unwrap();

            last_end = end;
        }

        if last_end < disk_end {
            writeln!(
                out,
                "{}│{:^12}│{:>12}│{:>12}│{:^16}│",
                indent, "", last_end, disk_end, "Unallocated"
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
