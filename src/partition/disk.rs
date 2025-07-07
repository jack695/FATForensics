//! Disk image parsing and analysis.
//!
//! This module provides functionality for:
//! - Opening and parsing disk images
//! - Handling different partition table types (currently only MBR)
//! - Managing volume analysis (currently only FAT32 filesystems)
//! - Displaying disk layout information

use getset::Getters;
use std::fs::File;
use std::path::{Path, PathBuf};

use super::disk_error::DiskError;
use super::mbr::Mbr;
use super::mbr::PTType;
use crate::filesystem::fat::FATVol;
use crate::traits::TreeDisplay;
use crate::traits::{LayoutDisplay, TraitError};

/// Represents a disk image with its partition table and volumes.
#[derive(Getters)]
pub struct Disk<T: TreeDisplay + LayoutDisplay, U: LayoutDisplay> {
    /// The open disk image file path.
    #[get = "pub"]
    file_path: PathBuf,
    /// The partition table found on the disk
    #[get = "pub"]
    part_table: U,
    /// List of volumes found on the disk
    #[get = "pub"]
    volumes: Vec<T>,
    /// The size in bytes of a sector
    #[get = "pub"]
    sector_size: usize,
}

impl Disk<FATVol, Mbr> {
    /// Opens a disk image file and analyzes its structure.
    ///
    /// # Parameters
    /// - `path`: Path to the disk image file
    /// - `sector_size`: Size of each sector in bytes
    /// - `validation`: Whether to validate volume structures (like Bpb)
    ///
    /// # Returns
    /// - `Ok(Disk)`: Successfully parsed disk with its partition table and volumes
    /// - `Err(DiskError)`: If any error occurs during parsing
    ///
    /// # Errors
    /// - Returns `DiskError::Io` if the file cannot be opened or read
    /// - Returns `DiskError::ParsingError` if the MBR or a volume cannot be parsed
    pub fn from_file(path: &Path, sector_size: usize, validation: bool) -> Result<Self, DiskError> {
        let mut f = File::options().read(true).write(true).open(path)?;
        let f_len = f.metadata()?.len();

        let mbr = Mbr::from(&mut f, f_len, sector_size)?;

        let mut vol = vec![];
        for (part_idx, pt_entry) in mbr.pt_entries().iter().enumerate() {
            if let PTType::LBAFat32 = *pt_entry.pt_type() {
                match FATVol::from_file(
                    path,
                    *pt_entry.lba_start(),
                    *pt_entry.sector_cnt(),
                    validation,
                    sector_size,
                ) {
                    Ok(fat_vol) => {
                        vol.push(fat_vol);
                    }
                    Err(error) => {
                        return Err(DiskError::ParsingError(format!(
                            "Error while reading partition #{part_idx}: {error}"
                        )));
                    }
                }
            }
        }

        let disk = Disk {
            file_path: path.to_path_buf(),
            part_table: mbr,
            volumes: vol,
            sector_size,
        };

        Ok(disk)
    }

    /// Prints a hierarchical layout of the disk structure.
    ///
    /// # Parameters
    /// - `indent`: Number of spaces to indent the layout
    ///
    /// # Returns
    /// - `Ok(())` if the layout is printed successfully
    /// - `Err(std::fmt::Error)` if formatting fails
    ///
    /// The layout includes:
    /// - Partition table information
    /// - Volume information for each partition
    pub fn print_layout(&self, indent: u8) -> Result<(), std::fmt::Error> {
        print!("{}", self.part_table.display_layout(indent)?);

        for vol in self.volumes.iter() {
            print!("\n{}", vol.display_layout(indent + 3)?);
        }

        Ok(())
    }

    pub fn print_tree(&self) -> Result<(), TraitError> {
        for vol in self.volumes.iter() {
            vol.display_tree()?;
        }

        Ok(())
    }
}
