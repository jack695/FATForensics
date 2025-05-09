use std::io::{Read, Seek, SeekFrom};
use std::{fs, io};

use crate::constants::SECTOR_SIZE;

/// Reads a specific sector from a file into a buffer.
///
/// # Arguments
///
/// * `file` - A mutable reference to the file to read from.
/// * `sector` - The sector number to read.
/// * `buffer` - A mutable reference to a vector where the sector data will be stored.
///
/// The buffer will be resized to match the sector size defined by `SECTOR_SIZE`.
///
/// # Errors
///
/// Returns an `io::Error` if the sector cannot be read.
pub fn read_sector(file: &mut fs::File, sector: u64, buffer: &mut Vec<u8>) -> io::Result<()> {
    buffer.resize(SECTOR_SIZE, 0);

    file.seek(SeekFrom::Start(SECTOR_SIZE as u64 * sector))?;

    file.read_exact(buffer).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("Failed to read sector {}: {}", sector, err),
        )
    })?;

    Ok(())
}

/// Extracts a 32-bit unsigned integer from a buffer at a given offset.
///
/// # Arguments
///
/// * `buffer` - A slice of bytes from which the value will be extracted.
/// * `offset` - The offset within the buffer where the 32-bit value starts.
///
/// # Panics
///
/// Panics if the slice does not contain enough bytes starting from the offset.
pub fn u32_at(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        buffer[offset..offset + 4]
            .try_into()
            .expect("invalid slice"),
    )
}

/// Extracts a 16-bit unsigned integer from a buffer at a given offset.
///
/// # Arguments
///
/// * `buffer` - A slice of bytes from which the value will be extracted.
/// * `offset` - The offset within the buffer where the 16-bit value starts.
///
/// # Panics
///
/// Panics if the slice does not contain enough bytes starting from the offset.
pub fn u16_at(buffer: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        buffer[offset..offset + 2]
            .try_into()
            .expect("invalid slice"),
    )
}

/// Extracts a 8-bit unsigned integer from a buffer at a given offset.
///
/// # Arguments
///
/// * `buffer` - A slice of bytes from which the value will be extracted.
/// * `offset` - The offset within the buffer where the 8-bit value starts.
///
/// # Panics
///
/// Panics if the slice does not contain enough bytes starting from the offset.
pub fn u8_at(buffer: &[u8], offset: usize) -> u8 {
    u8::from_le_bytes(
        buffer[offset..offset + 1]
            .try_into()
            .expect("invalid slice"),
    )
}
