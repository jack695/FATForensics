//! This is the main entry point for the FAT32 file system tool.
//!
//! The program provides an interactive command-line interface for analyzing FAT32 disk images.
//! Users can open disk images, print their layout, and quit the program using commands.

use fat_forensics::commands::Command;
use fat_forensics::traits::TreeDisplay;
use fat_forensics::utils::write_file_at;
use fat_forensics::{Disk, traits::LayoutDisplay};
use log::{error, warn};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

/// Represents the runtime state of the program.
///
/// This struct keeps track of the currently opened file and its associated Master Boot Record (MBR).
struct RunState<T: LayoutDisplay + TreeDisplay, U: LayoutDisplay> {
    /// The currently opened disk image.
    disk: Option<Disk<T, U>>,
    /// Volume in inspection mode
    vol_nb: Option<u8>,
    /// Enable the validation of the bpb
    bpb_validation: bool,
    /// The size of a sector
    sector_size: usize,
}

fn main() {
    stderrlog::new().module(module_path!()).init().unwrap();

    let mut run_state = RunState {
        disk: None,
        vol_nb: None,
        bpb_validation: true,
        sector_size: 512,
    };

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut s = String::new();
        io::stdin()
            .read_line(&mut s)
            .expect("Failed to read command");
        let cmd = Command::from_string(&s);

        match cmd {
            Command::Open(path) => {
                match Disk::from_file(
                    Path::new(&path),
                    run_state.sector_size,
                    run_state.bpb_validation,
                ) {
                    Ok(disk) => {
                        run_state.disk = Some(disk);
                    }
                    Err(err) => {
                        error!("{err}");
                    }
                }
            }
            Command::Quit => break,
            Command::Print => match &run_state.disk {
                Some(disk) => {
                    if let Err(e) = disk.print_layout(3) {
                        error!("Print layout error: {e}");
                    }
                }
                None => error!("Open disk image first"),
            },
            Command::Partition(vol_nb) => {
                let part_index: isize = vol_nb as isize - 1;

                if let Some(disk) = &run_state.disk {
                    if part_index < 0 || part_index >= disk.volumes().len() as isize {
                        error!(
                            "Invalid volume number. There are {} valid volumes on disk.",
                            disk.volumes().len()
                        );
                    }

                    run_state.vol_nb = Some(vol_nb);
                } else {
                    warn!("Open disk image first");
                }
            }
            Command::Skip => run_state.bpb_validation = false,
            Command::Write((file_path, sector)) => {
                write_file_to_disk(&mut run_state, Path::new(&file_path), sector)
            }
            Command::Tree => {
                if let Some(disk) = run_state.disk.as_ref() {
                    if let Err(err) = disk.print_tree() {
                        error!("Tree printing failed: {err}");
                    }
                } else {
                    warn!("Open disk image first")
                }
            }
            Command::Unknown(s) => error!("Unknown command: {s:?}"),
            Command::Invalid(s) => error!("{s}"),
            Command::Empty => {}
        }
    }
}

fn write_file_to_disk<T: LayoutDisplay + TreeDisplay, U: LayoutDisplay>(
    run_state: &mut RunState<T, U>,
    file_path: &Path,
    sector: u64,
) {
    let disk = match &mut run_state.disk {
        Some(disk) => disk,
        None => {
            warn!("Open disk image first");
            return;
        }
    };

    // Open the disk image
    let mut disk_file = match File::options()
        .read(true)
        .write(true)
        .open(disk.file_path())
    {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open disk image file: {e}");
            return;
        }
    };

    // Open the file to copy on disk
    let mut f = match File::open(file_path) {
        Err(e) => {
            error!(
                "Can't open {}: {}",
                file_path.to_str().unwrap_or("invalid_file_name"),
                e
            );
            return;
        }
        Ok(file) => file,
    };
    let f_len = match f.metadata() {
        Ok(metadata) => metadata.len(),
        Err(e) => {
            error!(
                "Can't read meatadata of {}: {}",
                file_path.to_str().unwrap_or("invalid_file_name"),
                e
            );
            return;
        }
    };

    match write_file_at(
        &mut disk_file,
        sector * run_state.sector_size as u64,
        &mut f,
        f_len,
        run_state.sector_size,
        0,
    ) {
        Ok(()) => println!("Write succeeded!"),
        Err(err) => error!("Write failed: {err}"),
    }
}
