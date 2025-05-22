//! This module provides functionality for parsing and handling partition tables
//! and Master Boot Records (MBR) in the FAT32 file system.
//!
//! It defines structures and methods to interpret partition table entries,
//! validate partition tables, and extract relevant metadata from disk images.
pub mod mbr_error;

use std::fs::File;
use std::vec;

use crate::constants;
use crate::partitioning::mbr_error::MBRError;
use crate::utils;

/// Represents the type of a partition table entry.
#[derive(Debug)]
pub enum PTType {
    /// Logical Block Addressing (LBA) FAT32 partition type.
    LBAFat32,
    /// Unsupported partition type, encapsulating the raw type byte.
    Unsupported(u8),
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
#[derive(Debug)]
pub struct PTEntry {
    /// The type of the partition.
    pt_type: PTType,
    /// The starting Logical Block Address (LBA) of the partition.
    lba_start: u32,
    /// The number of sectors in the partition.
    sector_cnt: u32,
}

impl PTEntry {
    /// Returns the starting Logical Block Address (LBA) of the partition.
    pub fn lba_start(&self) -> u32 {
        self.lba_start
    }

    /// Returns the number of sectors in the partition.
    pub fn sector_cnt(&self) -> u32 {
        self.sector_cnt
    }

    /// Return the partition type
    pub fn pt_type(&self) -> &PTType {
        &self.pt_type
    }
}

/// Represents the boot signature of a Master Boot Record (MBR).
#[derive(Debug)]
enum BootSignature {
    /// Standard MBR boot signature (0x55AA).
    MBR,
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
    /// - `BootSignature::MBR` if the signature matches `0x55AA`.
    /// - `BootSignature::Unsupported(other)` for any other value.
    pub fn from_u16(sig: u16) -> BootSignature {
        match sig {
            // The signature 0x55AA is stored on disk in little-endian byte order.
            0xAA55 => BootSignature::MBR,
            other => BootSignature::Unsupported(other),
        }
    }
}

/// Represents a Master Boot Record (MBR), including partition table entries
/// and the boot signature.
#[derive(Debug)]
pub struct MBR {
    /// The partition table entries in the MBR.
    pt_entries: [PTEntry; constants::PART_CNT],
    /// The boot signature of the MBR.
    boot_signature: BootSignature,
    sector_cnt: u64,
}

impl MBR {
    /// Reads and parses an MBR from a file.
    ///
    /// # Parameters
    /// - `file`: A mutable reference to a `File` object representing the disk image.
    ///
    /// # Returns
    /// - `Ok(MBR)` if the MBR is successfully parsed.
    /// - `Err(std::io::Error)` if an error occurs during reading or parsing.
    pub fn from_file(file: &mut File) -> Result<MBR, MBRError> {
        let mut buffer = vec![0; constants::SECTOR_SIZE];
        utils::read_sector(file, 0, &mut buffer)?;

        let pt_entries: [PTEntry; constants::PART_CNT] = core::array::from_fn(|i| {
            let offset = 446 + i * 16;
            PTEntry {
                pt_type: PTType::from_byte(utils::u8_at(&buffer, offset + 0x04)),
                lba_start: utils::u32_at(&buffer, offset + 0x08),
                sector_cnt: utils::u32_at(&buffer, offset + 0x0C),
            }
        });

        let mbr = MBR {
            pt_entries,
            boot_signature: BootSignature::from_u16(utils::u16_at(&buffer, 510)),
            sector_cnt: file.metadata()?.len() / constants::SECTOR_SIZE as u64,
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

    /// Returns the size of the disk in sectors.
    pub fn sector_cnt(&self) -> u64 {
        self.sector_cnt
    }

    /// Validates the MBR by checking the partition table and boot signature.
    ///
    /// # Returns
    /// - `Ok(Self)` if the MBR is valid.
    /// - `Err(MBRError)` if any validation step fails.
    fn validate(self) -> Result<Self, MBRError> {
        self.check_partition_table_sorted()?
            .check_partitions_non_overlapping()?
            .check_signature()
    }

    /// Checks if the boot signature is valid.
    ///
    /// # Returns
    /// - `Ok(Self)` if the boot signature is valid.
    /// - `Err(MBRError::InvalidSignature)` if the boot signature is unsupported.

    fn check_signature(self) -> Result<Self, MBRError> {
        match self.boot_signature {
            BootSignature::Unsupported(sig) => Err(MBRError::InvalidSignature(sig)),
            _ => Ok(self),
        }
    }

    /// Checks if the partition table entries are sorted by their starting LBA.
    ///
    /// # Returns
    /// - `Ok(Self)` if the entries are sorted.
    /// - `Err(MBRError::PartitionTableNotSorted)` if the entries are not sorted.
    fn check_partition_table_sorted(self) -> Result<Self, MBRError> {
        match self
            .pt_entries()
            .windows(2)
            .all(|pair| pair[0].lba_start <= pair[1].lba_start)
        {
            true => Ok(self),
            false => Err(MBRError::PartitionTableNotSorted),
        }
    }

    /// Checks if the partition table entries are non-overlapping.
    ///
    /// # Returns
    /// - `Ok(Self)` if the entries do not overlap.
    /// - `Err(MBRError::OverlappingPartitions)` if any entries overlap.
    fn check_partitions_non_overlapping(self) -> Result<Self, MBRError> {
        match self
            .pt_entries()
            .windows(2)
            .any(|pair| pair[0].lba_start + pair[0].sector_cnt > pair[1].lba_start)
        {
            true => Err(MBRError::OverlappingPartitions),
            false => Ok(self),
        }
    }
}
