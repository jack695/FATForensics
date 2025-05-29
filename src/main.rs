//! This is the main entry point for the FAT32 file system tool.
//!
//! The program provides an interactive command-line interface for analyzing FAT32 disk images.
//! Users can open disk images, print their layout, and quit the program using commands.

use fat_forensics::commands::Command;
use fat_forensics::disk::{MBR, PTType};
use fat_forensics::traits::LayoutDisplay;
use fat_forensics::volume::BPB;
use std::{fs::File, io};

/// Represents the runtime state of the program.
///
/// This struct keeps track of the currently opened file and its associated Master Boot Record (MBR).
struct RunState {
    /// The currently opened disk image file, if any.
    file: Option<File>,
    /// The parsed Master Boot Record (MBR) of the opened file, if any.
    mbr: Option<MBR>,
    /// The index of the partition to analyse.
    bpb: Option<BPB>,
    /// Enable the validation of the bpb
    bpb_validation: bool,
    /// The size of a sector
    sector_size: usize,
}

fn main() {
    let mut run_state = RunState {
        file: None,
        mbr: None,
        bpb: None,
        bpb_validation: true,
        sector_size: 512,
    };

    loop {
        let mut s = String::new();
        io::stdin()
            .read_line(&mut s)
            .expect("Failed to read command");
        let cmd = Command::from_string(&s);

        match cmd {
            Command::Open(path) => match fat_forensics::open_file(&path, run_state.sector_size) {
                Ok((file, mbr)) => {
                    run_state.file = Some(file);
                    run_state.mbr = Some(mbr);
                }
                Err(err) => {
                    eprintln!("{}", err);
                }
            },
            Command::Quit => break,
            Command::Print => match run_state.mbr {
                Some(ref mbr) => println!("{}", mbr.display_layout(0, 0)),
                None => eprintln!("Open disk image first"),
            },
            Command::Partition(part_nb) => {
                let mbr = run_state.mbr.as_ref().expect("Open disk image first");

                // Check whether a partition exists for that index
                if part_nb < 1 || part_nb as usize > mbr.pt_entries().len() {
                    eprintln!(
                        "Partition number for this disk should be between 1 and {}.",
                        mbr.pt_entries().len()
                    );
                    continue;
                }
                let part_index: usize = part_nb as usize - 1;

                // Read the MBR
                let pt_entry = mbr.pt_entries()[part_index];
                match pt_entry.pt_type() {
                    PTType::LBAFat32 => {
                        match BPB::from_file(
                            run_state.file.as_mut().unwrap(),
                            pt_entry.lba_start(),
                            run_state.bpb_validation,
                            run_state.sector_size,
                        ) {
                            Ok(bpb) => {
                                run_state.bpb = Some(bpb);
                                print!(
                                    "{}",
                                    run_state
                                        .bpb
                                        .as_ref()
                                        .unwrap()
                                        .display_layout(pt_entry.lba_start() as u64, 3)
                                );
                            }
                            Err(error) => eprintln!("{}", error),
                        }
                    }
                    PTType::Unsupported(pt_type) => {
                        eprintln!("Unsupported partition type: {:x}", pt_type)
                    }
                }
            }
            Command::Skip => run_state.bpb_validation = false,
            Command::Unknown(s) => eprintln!("Unknown command: {:?}", s),
            Command::Invalid(s) => eprintln!("{s}"),
            Command::Empty => {}
        }
    }
}
