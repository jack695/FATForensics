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
    /// Select the partition to analyse (by index).
    Partition(u8),
    /// Skip the MBR validation.
    Skip,
    /// Write a file to a given sector: (file path, starting sector).
    Write((String, u64)),
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
    /// - The corresponding `Command` variant based on the input string.
    ///
    /// # Behavior
    /// - Recognizes commands: `quit`, `open <file>`, `print`, `part <idx>`, `skip`, `write <file> <sector>`
    /// - Returns `Command::Invalid` for missing or malformed arguments.
    /// - Returns `Command::Unknown` for unrecognized commands.
    /// - Returns `Command::Empty` for empty or whitespace-only input.
    pub fn from_string(s: &str) -> Self {
        let mut parts = s.split_whitespace();
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
                        "Arg parsing error: 'part' expects the partition number as an unsigned integer.",
                    )),
                },
                None => Command::Invalid(String::from(
                    "Missing arg: 'part' expects the partition number.",
                )),
            },
            Some("skip") => Command::Skip,
            Some("write") => {
                // Get the filepath
                let filepath = match parts.next() {
                    Some(arg) => arg,
                    None => {
                        return Command::Invalid(String::from(
                            "Missing arg: 'write' expects the file and the starting sector to write it.",
                        ));
                    }
                };

                match parts.next() {
                    Some(arg) => match arg.parse::<u64>() {
                        Ok(sector) => Command::Write((filepath.to_string(), sector)),
                        Err(_) => Command::Invalid(String::from(
                            "Arg parsing error: 'write' expects the starting sector as an unsigned integer.",
                        )),
                    },
                    None => Command::Invalid(String::from(
                        "Missing arg: 'write' expects the file and the starting sector to write it.",
                    )),
                }
            }
            Some(other) => Command::Unknown(other.to_string()),
            None => Command::Empty,
        }
    }
}
