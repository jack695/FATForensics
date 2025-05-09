//! This is the main entry point for the FAT32 file system tool.
//!
//! The program provides an interactive command-line interface for analyzing FAT32 disk images.
//! Users can open disk images, print their layout, and quit the program using commands.

use fat_forensics;
use fat_forensics::{Command, MBR};
use std::{fs::File, io};

/// Represents the runtime state of the program.
///
/// This struct keeps track of the currently opened file and its associated Master Boot Record (MBR).
struct RunState {
    /// The currently opened disk image file, if any.
    file: Option<File>,
    /// The parsed Master Boot Record (MBR) of the opened file, if any.
    mbr: Option<MBR>,
}

fn main() {
    let mut run_state = RunState {
        file: None,
        mbr: None,
    };

    loop {
        let mut s = String::new();
        io::stdin()
            .read_line(&mut s)
            .expect("Failed to read command");
        let cmd = Command::from_string(&s);

        match cmd {
            Command::Open(path) => match fat_forensics::open_file(&path) {
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
                Some(ref mbr) => fat_forensics::print_disk_layout(mbr),
                None => eprintln!("Open disk image first"),
            },
            Command::Unknown(s) => eprintln!("Unknown command: {:?}", s),
            Command::Invalid(s) => eprintln!("{s}"),
            Command::Empty => {}
        }
    }
}
