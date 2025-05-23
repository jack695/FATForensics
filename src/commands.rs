//! This module defines the `Command` enum and its associated methods for parsing
//! and handling user commands in the FAT32 file system tool.
//!
//! The `Command` enum represents various commands that the user can input,
//! such as quitting the program, opening a file, printing information, or handling
//! invalid or unknown commands.

/// Represents a user command in the FAT32 file system tool.
#[derive(Debug)]
pub enum Command {
    /// Command to quit the program.
    Quit,
    /// Command to open a disk image, encapsulating the file path as a `String`.
    Open(String),
    /// Command to print general disk information.
    Print,
    /// Select the partition to analyse.
    Partition(u8),
    /// Skip the MBP validation
    Skip,
    /// Command for an unknown input, encapsulating the raw input as a `String`.
    Unknown(String),
    /// Command for invalid input, encapsulating an error message as a `String`.
    Invalid(String),
    /// Command for an empty input.
    Empty,
}

impl Command {
    /// Parses a string into a `Command` instance.
    ///
    /// # Parameters
    /// - `s`: A string slice representing the user input.
    ///
    /// # Returns
    /// - `Command::Quit` if the input is "quit".
    /// - `Command::Open` with the file path if the input starts with "open" followed by a valid argument.
    /// - `Command::Print` if the input is "print".
    /// - `Command::Part` if the input is "part".
    /// - `Command::Skip` if the input is "skip".
    /// - `Command::Unknown` if the input does not match any known command.
    /// - `Command::Invalid` if the input is "open" but missing an argument.
    /// - `Command::Empty` if the input is empty or contains only whitespace.
    pub fn from_string(s: &str) -> Self {
        let mut parts = s.trim().split_whitespace();
        match parts.next() {
            Some("quit") => Command::Quit,
            Some("open") => match parts.next() {
                Some(arg) => Command::Open(arg.to_string()),
                None => Command::Invalid(String::from(
                    "Missing arg: 'open' expects the path to a '.img' file.",
                )),
            },
            Some("print") => Command::Print,
            Some("part") => match parts.next() {
                Some(arg) => match arg.parse::<u8>() {
                    Ok(nb) => Command::Partition(nb),
                    Err(_) => Command::Invalid(String::from(
                        "Arg parsing error: 'part' expects an unsigned integer.",
                    )),
                },
                None => Command::Invalid(String::from(
                    "Missing arg: 'part' expects the partition number.",
                )),
            },
            Some("skip") => Command::Skip,
            Some(other) => Command::Unknown(other.to_string()),
            None => Command::Empty,
        }
    }
}
