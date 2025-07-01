//! This is the main entry point for the FAT32 file system tool.
//!
//! The program provides an interactive command-line interface for analyzing FAT32 disk images.
//! Users can open disk images, print their layout, and quit the program using commands.

use fat_forensics::Disk;
use fat_forensics::commands::Command;
use fat_forensics::utils::write_file_at;
use log::error;
use std::{
    fs::File,
    io::{self, Write},
};

/// Represents the runtime state of the program.
///
/// This struct keeps track of the currently opened file and its associated Master Boot Record (MBR).
struct RunState {
    /// The currently opened disk image.
    disk: Option<Disk>,
    /// Volume in inspection mode
    vol_nb: Option<u8>,
    /// Enable the validation of the bpb
    bpb_validation: bool,
    /// The size of a sector
    sector_size: usize,
}

fn main() {
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
                match Disk::from_file(&path, run_state.sector_size, run_state.bpb_validation) {
                    Ok(disk) => {
                        run_state.disk = Some(disk);
                    }
                    Err(err) => {
                        error!("{}", err);
                    }
                }
            }
            Command::Quit => break,
            Command::Print => match &run_state.disk {
                Some(disk) => disk.print_layout(3),
                None => error!("Open disk image first"),
            },
            Command::Partition(vol_nb) => {
                let part_index: isize = vol_nb as isize - 1;

                if let Some(disk) = &run_state.disk {
                    if part_index < 0 || part_index >= disk.vol_count() as isize {
                        error!(
                            "Invalid volume number. There are {} valid volumes on disk.",
                            disk.vol_count()
                        );
                    }

                    run_state.vol_nb = Some(vol_nb);
                } else {
                    error!("Open disk image first");
                }
            }
            Command::Skip => run_state.bpb_validation = false,
            Command::Write((file_path, sector)) => match &mut run_state.disk {
                Some(disk) => {
                    let mut disk_file = File::options()
                        .read(true)
                        .write(true)
                        .open(disk.file_path())
                        .expect("Failed to open disk image file.");

                    match write_file_at(
                        &mut disk_file,
                        sector * run_state.sector_size as u64,
                        file_path.as_str(),
                        run_state.sector_size,
                        0,
                    ) {
                        Ok(()) => println!("Write succeeded!"),
                        Err(err) => error!("Write failed: {}", err),
                    }
                }
                None => {
                    error!("Open disk image first");
                }
            },
            Command::Unknown(s) => error!("Unknown command: {:?}", s),
            Command::Invalid(s) => error!("{s}"),
            Command::Empty => {}
        }
    }
}
