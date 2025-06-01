//! Disk image parsing and analysis.
//!
//! This module provides functionality for:
//! - Opening and parsing disk images
//! - Handling different partition table types (currently MBR)
//! - Managing volume analysis (currently FAT32)
//! - Displaying disk layout information

use std::fs::File;

use super::disk_error::DiskError;
use super::mbr::Mbr;
use super::mbr::PTType;
use crate::traits::LayoutDisplay;
use crate::volume::BPB;

/// Represents different types of partition tables that can be found on a disk.
/// Currently only MBR is supported.
enum PartTable {
    /// Master Boot Record partition table
    Mbr(Mbr),
}

/// Represents different types of volumes that can be found in partitions.
/// Currently only FAT32 is supported.
enum Volume {
    /// FAT32 volume with its BIOS Parameter Block
    FAT32(BPB),
}

/// Represents a disk image with its partition table and volumes.
pub struct Disk {
    /// The partition table found on the disk
    part_table: PartTable,
    /// List of volumes found on the disk, with their starting sector offsets
    volumes: Vec<(u32, Volume)>,
}

impl Disk {
    /// Opens a disk image file and analyzes its structure.
    ///
    /// # Parameters
    /// - `path`: Path to the disk image file
    /// - `sector_size`: Size of each sector in bytes
    /// - `validation`: Whether to validate volume structures (like BPB)
    ///
    /// # Returns
    /// - `Ok(Disk)`: Successfully parsed disk with its partition table and volumes
    /// - `Err(DiskError)`: If any error occurs during parsing
    ///
    /// # Errors
    /// - Returns `DiskError::Io` if the file cannot be opened or read
    /// - Returns `DiskError::Mbr` if the MBR cannot be parsed
    /// - Individual volume parsing errors are logged but don't cause function failure
    pub fn from_file(path: &str, sector_size: usize, validation: bool) -> Result<Self, DiskError> {
        let mut f = File::open(path)?;

        let mbr = Mbr::from_file(&mut f, sector_size)?;

        let mut vol = vec![];
        for (part_idx, pt_entry) in mbr.pt_entries().iter().enumerate() {
            if let PTType::LBAFat32 = pt_entry.pt_type() {
                {
                    match BPB::from_file(&mut f, pt_entry.lba_start(), validation, sector_size) {
                        Ok(bpb) => {
                            vol.push((pt_entry.lba_start(), Volume::FAT32(bpb)));
                        }
                        Err(error) => {
                            eprintln!("Error while reading partition #{}: {}", part_idx, error)
                        }
                    }
                }
            }
        }

        let disk = Disk {
            part_table: PartTable::Mbr(mbr),
            volumes: vol,
        };

        Ok(disk)
    }

    /// Prints a hierarchical layout of the disk structure.
    ///
    /// # Parameters
    /// - `indent`: Number of spaces to indent the layout
    ///
    /// The layout includes:
    /// - Partition table information
    /// - Volume information for each partition
    pub fn print_layout(&self, indent: u8) {
        match &self.part_table {
            PartTable::Mbr(mbr) => print!("{}", mbr.display_layout(0, indent)),
        }

        for (offset, vol) in self.volumes.iter() {
            match vol {
                Volume::FAT32(bpb) => {
                    print!("\n{}", bpb.display_layout((*offset).into(), indent + 3))
                }
            }
        }
    }

    /// Returns the number of valid volumes found on the disk.
    ///
    /// # Returns
    /// - The count of successfully parsed volumes
    pub fn vol_count(&self) -> usize {
        self.volumes.len()
    }
}
