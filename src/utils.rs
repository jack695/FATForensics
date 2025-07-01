use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::{fs, io};

/// Reads a specific sector from a file into a buffer.
///
/// # Arguments
///
/// - `file`: A mutable reference to the file to read from.
/// - `sector`: The sector number to read.
/// - `sector_size`: The size in bytes of a sector.
/// - `buffer`: A mutable reference to a vector where the sector data will be stored.
///
/// The buffer will be resized to match the sector size.
///
/// # Errors
///
/// Returns an `io::Error` if the sector cannot be read.
pub fn read_sector(
    file: &mut fs::File,
    sector: u64,
    sector_size: usize,
    buffer: &mut Vec<u8>,
) -> io::Result<()> {
    buffer.resize(sector_size, 0);

    file.seek(SeekFrom::Start(sector_size as u64 * sector))?;

    file.read_exact(buffer).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("Failed to read sector {}: {}", sector, err),
        )
    })?;

    Ok(())
}

/// Writes data to a file at a specific offset.
///
/// # Arguments
///
/// - `disk`: A mutable reference to the file to write to.
/// - `offset`: The offset in bytes where the data will be written.
/// - `data`: A reference to a vector containing the data to be written.
pub fn write_at<T: io::Write + io::Seek>(disk: &mut T, offset: u64, data: &[u8]) -> io::Result<()> {
    disk.seek(SeekFrom::Start(offset))?;
    disk.write_all(data)
}

/// Writes the contents of a file to a specific offset in a disk.
///
/// # Arguments
///
/// - `disk`: A mutable reference to the file to write to.
/// - `offset`: The offset in bytes where the data will be written.
/// - `path`: The path to the file to write into the disk.
/// - `sector_size`: The size in bytes of a sector.
/// - `limit`: The byte offset after which writing should be forbidden.
pub fn write_file_at<T: io::Write + io::Seek>(
    disk: &mut T,
    offset: u64,
    path: &str,
    sector_size: usize,
    limit: u64,
) -> io::Result<()> {
    let mut f = File::open(path)?;
    let f_len = f.metadata().unwrap().len();

    // Check the file wouldn't cross the limit
    if limit > 0 && offset + f_len > limit {
        return Err(std::io::Error::other(format!(
            "Cannot write the {}-byte long file starting from {} without crossing the limit {}.",
            f_len, offset, limit
        )));
    }

    for s in (0..f_len).step_by(sector_size) {
        let mut v: Vec<u8> = vec![0; sector_size];
        let bytes_read = f.read(&mut v)?;
        v.resize(bytes_read, 0);
        write_at(disk, offset + s, &v)?;
    }

    Ok(())
}

/// Extracts a 32-bit unsigned integer from a buffer at a given offset.
///
/// # Arguments
///
/// - `buffer`: A slice of bytes from which the value will be extracted.
/// - `offset`: The offset within the buffer where the 32-bit value starts.
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
/// - `buffer`: A slice of bytes from which the value will be extracted.
/// - `offset`: The offset within the buffer where the 16-bit value starts.
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
/// - `buffer`: A slice of bytes from which the value will be extracted.
/// - `offset`: The offset within the buffer where the 8-bit value starts.
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
