#![deny(missing_docs)]

//! This is the main library module for the FAT32 file system tool.
//!
//! It provides functionality for interacting with FAT32 disk images, including
//! parsing Master Boot Records (MBR), handling user commands, and printing disk layouts.
//!
//! The module re-exports key components such as `Command` and `MBR` for external use.

mod commands;
mod constants;
mod partition;
mod utils;

pub use commands::Command;
pub use partition::MBR;
use std::{fs::File, io};

/// Prints the layout of the disk based on the provided Master Boot Record (MBR).
///
/// # Parameters
/// - `mbr`: A reference to an `MBR` instance representing the parsed Master Boot Record.
///
/// # Behavior
/// - Prints the MBR sector range.
/// - Iterates through the partition table entries and prints their sector ranges.
pub fn print_disk_layout(mbr: &MBR) {
    let (s, e) = (0, 1);
    print!("MBR\n___\n\tSectors: {s} -> {e}\n\n");

    for (part, pt_entry) in mbr.get_pt_entries().iter().enumerate() {
        print!(
            "Part #{}\n______\n\tSectors: {} -> {}\n\n",
            part + 1,
            pt_entry.get_lba_start(),
            pt_entry.get_lba_start() + pt_entry.get_sector_cnt()
        );
    }
}

/// Opens a disk image file and parses its Master Boot Record (MBR).
///
/// # Parameters
/// - `path`: A string slice representing the path to the disk image file.
///
/// # Returns
/// - `Ok((File, MBR))` if the file is successfully opened and the MBR is parsed.
/// - `Err(io::Error)` if an error occurs while opening the file or parsing the MBR.
///
/// # Errors
/// - Returns an error if the file cannot be opened.
/// - Returns an error if the MBR cannot be parsed from the file.
pub fn open_file(path: &str) -> io::Result<(File, MBR)> {
    let mut f = File::open(path)?;

    let mbr = MBR::from_file(&mut f).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("Failed to parse MBR from file '{}': {}", &path, err),
        )
    })?;

    Ok((f, mbr))
}
