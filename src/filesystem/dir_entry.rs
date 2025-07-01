//! FAT directory entry structure and parsing.
//!
//! This module implements the FAT directory entry structure which contains metadata
//! about files and directories stored in the filesystem. Each directory entry is 32 bytes
//! and contains information such as filename, attributes, timestamps, and cluster allocation.

use binread::{BinRead, BinReaderExt};
use getset::Getters;
use std::fmt;
use std::io;
use std::io::{Error, ErrorKind};
use std::str::Utf8Error;

use crate::filesystem::fat_error::FATError;

/// FAT directory entry structure.
///
/// Each directory entry is exactly 32 bytes and contains metadata about a file or directory.
/// The structure follows Microsoft's FAT specification for directory entries.
///
/// # Fields
/// - `name`: 8.3 format filename (8 characters for name, 3 for extension)
/// - `attr`: File attributes (read-only, hidden, system, volume label, directory, archive)
/// - `fst_clus_hi`: High 16 bits of the first cluster number
/// - `fst_clus_lo`: Low 16 bits of the first cluster number
/// - `file_size`: Size of the file in bytes (0 for directories)
///
/// # Notes
/// - Timestamp fields are prefixed with underscore as they're not currently used
/// - The name field uses the legacy 8.3 format with space padding
#[derive(BinRead, Debug, Clone, Getters)]
#[br(little)]
pub struct DirEntry {
    /// Filename in 8.3 format (8 characters name + 3 characters extension)
    name: [u8; 11],
    /// File attributes byte
    attr: u8,
    /// NT reserved (unused)
    _n_t_res: u8,
    /// Creation time in 10ms units
    _ctr_time_tenth: u8,
    /// Creation time
    _crt_time: u16,
    /// Creation date
    _crt_date: u16,
    /// Last access date
    _lst_acc_date: u16,
    /// High 16 bits of first cluster number
    fst_clus_hi: u16,
    /// Last write time
    _wrt_time: u16,
    /// Last write date
    _wrt_date: u16,
    /// Low 16 bits of first cluster number
    fst_clus_lo: u16,
    /// File size in bytes (0 for directories)
    #[get = "pub(super)"]
    file_size: u32,
}

impl DirEntry {
    /// Creates a directory entry from a byte slice.
    ///
    /// # Parameters
    /// - `buf`: A byte slice containing exactly 32 bytes of directory entry data
    ///
    /// # Returns
    /// - `DirEntry`: The parsed directory entry structure
    ///
    /// # Panics
    /// - Panics if the byte slice is not exactly 32 bytes or if parsing fails
    pub fn from_slice(buf: &[u8]) -> Result<Self, FATError> {
        let mut reader = io::Cursor::new(buf);
        reader.read_le().map_err(FATError::from)
    }

    /// Checks if a given filename matches this directory entry's short name.
    ///
    /// # Parameters
    /// - `name`: The filename to compare (can include extension)
    ///
    /// # Returns
    /// - `true`: If the filename matches this directory entry
    /// - `false`: If the filename doesn't match or is invalid
    pub fn same_short_name(&self, name: &str) -> bool {
        let parts: Vec<&str> = name.split('.').collect();
        let shortname = match Self::to_8_3_name(parts.first().unwrap(), parts.get(1).copied()) {
            Ok(short_name) => short_name,
            Err(_) => return false,
        };

        shortname == self.name
    }

    fn to_8_3_name(name: &str, ext_opt: Option<&str>) -> Result<Vec<u8>, io::Error> {
        if name.len() > 8 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "A short filename is composed of max 8 characters and max 3 characters for the extension.".to_string(),
            ));
        }
        let name = match ext_opt {
            Some(ext) => format!(
                "{:<8}{:<3}",
                name.to_ascii_uppercase(),
                ext.to_ascii_uppercase()
            ),
            None => format!("{:<8}{:<3}", name.to_ascii_uppercase(), ""),
        };

        Ok(name.as_bytes().to_vec())
    }

    fn from_8_3_name(&self) -> Result<String, Utf8Error> {
        let raw_name = &self.name[0..8];
        let raw_ext = &self.name[8..11];

        // Convert &[u8] to &str assuming ASCII encoding
        let name = std::str::from_utf8(raw_name)?.trim_end();
        let ext = std::str::from_utf8(raw_ext)?.trim_end();

        if ext.is_empty() {
            Ok(name.to_ascii_uppercase())
        } else {
            Ok(format!(
                "{}.{}",
                name.to_ascii_uppercase(),
                ext.to_ascii_uppercase()
            ))
        }
    }

    /// Returns the complete first cluster number for this entry.
    ///
    /// # Returns
    /// - `u32`: The 32-bit cluster number combining high and low 16-bit values
    ///
    /// # Implementation Details
    /// Combines `fst_clus_hi` and `fst_clus_lo` to form the complete cluster number:
    /// `(fst_clus_hi << 16) | fst_clus_lo`
    pub fn cluster_number(&self) -> u32 {
        ((self.fst_clus_hi as u32) << 16) + self.fst_clus_lo as u32
    }

    /// Checks if this directory entry represents a directory.
    ///
    /// # Returns
    /// - `true`: If the entry represents a directory
    /// - `false`: If the entry represents a file
    ///
    /// # Implementation Details
    /// Checks if the directory attribute bit (0x10) is set in the attributes field
    pub fn is_dir(&self) -> bool {
        self.attr & 0x10 != 0
    }
}

impl fmt::Display for DirEntry {
    /// Formats the directory entry for display.
    ///
    /// # Returns
    /// - A string representation showing the filename and file size
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.from_8_3_name() {
            Ok(fmt_name) => {
                write!(f, "\"{}\" {}B", fmt_name, self.file_size)
            }
            _ => {
                write!(f, "\"{:?}\" {}B", self.name, self.file_size)
            }
        }
    }
}
