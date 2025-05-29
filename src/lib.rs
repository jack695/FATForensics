//! This is the main library module for the FAT32 file system tool.
//!
//! It provides functionality for interacting with FAT32 disk images, including
//! parsing Master Boot Records (MBR), handling user commands, and printing disk layouts.
//!
//! The module re-exports key components such as `Command` and `MBR` for external use.

pub mod commands;
pub mod disk;
pub mod traits;
mod utils;
pub mod volume;

use disk::MBR;
use disk::MBRError;
use std::fs::File;

/// Opens a disk image file and parses its Master Boot Record (MBR).
///
/// # Parameters
/// - `path`: A string slice representing the path to the disk image file.
/// - `sector_size`: The size in bytes of a sector.
///
/// # Returns
/// - `Ok((File, MBR))` if the file is successfully opened and the MBR is parsed.
/// - `Err(io::Error)` if an error occurs while opening the file or parsing the MBR.
///
/// # Errors
/// - Returns an error if the file cannot be opened.
/// - Returns an error if the MBR cannot be parsed from the file.
pub fn open_file(path: &str, sector_size: usize) -> Result<(File, MBR), MBRError> {
    let mut f = File::open(path)?;

    let mbr = MBR::from_file(&mut f, sector_size)?;

    Ok((f, mbr))
}
