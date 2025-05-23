//! This is the main library module for the FAT32 file system tool.
//!
//! It provides functionality for interacting with FAT32 disk images, including
//! parsing Master Boot Records (MBR), handling user commands, and printing disk layouts.
//!
//! The module re-exports key components such as `Command` and `MBR` for external use.

mod commands;
mod constants;
mod fat;
mod partitioning;
mod utils;

pub use commands::Command;
pub use fat::BPB;
pub use partitioning::MBR;
pub use partitioning::PTType;
use partitioning::mbr_error::MBRError;
use std::fs::File;

/// Prints the layout of the disk based on the provided Master Boot Record (MBR).
///
/// # Parameters
/// - `mbr`: A reference to an `MBR` instance representing the parsed Master Boot Record.
///
/// # Behavior
/// - Prints the MBR sector range.
/// - Iterates through the partition table entries and prints their sector ranges.
pub fn print_disk_layout(mbr: &MBR) {
    let mut last_end = 0;
    let disk_end = mbr.sector_cnt();

    println!("Disk size (in sectors): {}", disk_end);
    println!("[{:<8}, {:>8}[: MBR", 0, last_end);

    for (i, entry) in mbr.pt_entries().iter().enumerate() {
        let start = entry.lba_start();
        let end = start + entry.sector_cnt();

        if start > last_end {
            println!("[{:<8}, {:>8}[: Unallocated", last_end, start);
        }

        println!("[{:<8}, {:>8}[: Part #{}", start, end, i + 1);

        last_end = end;
    }

    let last_end = last_end as u64;
    if last_end < disk_end {
        println!("[{:<8}, {:>8}[: Unallocated", last_end, disk_end);
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
pub fn open_file(path: &str) -> Result<(File, MBR), MBRError> {
    let mut f = File::open(path)?;

    let mbr = MBR::from_file(&mut f)?;

    Ok((f, mbr))
}
