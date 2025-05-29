//! FAT32 filesystem structures and parsing.
//!
//! This module implements the core structures for FAT32 filesystem analysis, including:
//! - BIOS Parameter Block (BPB) parsing and validation
//! - FAT type detection (FAT12/16/32)
//! - Filesystem structure validation according to Microsoft's FAT specification

use binread::{BinRead, BinReaderExt};
use std::fmt;
use std::fmt::Write;
use std::fs::File;
use std::io::Cursor;
use std::vec;

use super::bpb_error::BPBError;
use crate::traits::LayoutDisplay;
use crate::utils;

/// Represents the different types of FAT filesystems.
///
/// # Values
/// - `FAT12`: 12-bit File Allocation Table entries
/// - `FAT16`: 16-bit File Allocation Table entries
/// - `FAT32`: 32-bit File Allocation Table entries (most common on large volumes)
///
/// Note: Currently only FAT32 is fully supported for analysis.
#[derive(PartialEq)]
pub enum FATType {
    FAT12,
    FAT16,
    FAT32,
}

impl fmt::Display for FATType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FATType::FAT12 => "FAT12",
            FATType::FAT16 => "FAT16",
            FATType::FAT32 => "FAT32",
        };
        write!(f, "{}", s)
    }
}

/// BIOS Parameter Block structure for FAT filesystems.
///
/// The BPB contains essential information about the filesystem layout and properties.
/// This implementation follows Microsoft's FAT32 specification.
#[derive(BinRead, Debug)]
#[br(little)]
pub struct BPB {
    /// Jump instruction to boot code (must be 0xEB ?? 0x90 or 0xE9 ?? ??)
    jmp: [u8; 3],
    /// OEM identifier (e.g., "MSWIN4.1")
    oem_name: [u8; 8],
    /// Number of bytes per sector (512, 1024, 2048, or 4096)
    bytes_per_sec: u16,
    /// Number of sectors per cluster (power of 2: 1, 2, 4, 8, 16, 32, 64, or 128)
    sec_per_clus: u8,
    /// Number of reserved sectors from start of volume
    rsvd_sec_cnt: u16,
    /// Number of FAT copies (typically 2 for redundancy)
    num_fat: u8,
    /// Maximum number of root directory entries (0 for FAT32)
    root_ent_cnt: u16,
    /// Total sectors for volumes < 32MB (0 for FAT32)
    tot_sec_16: u16,
    /// Media descriptor (0xF8 for fixed disk)
    media: u8,
    /// Sectors per FAT for FAT12/FAT16 (0 for FAT32)
    fat_sz_16: u16,
    /// Sectors per track
    sec_per_trl: u16,
    /// Number of heads
    num_heds: u16,
    /// Number of hidden sectors preceding the partition
    hidd_sec: u32,
    /// Total sectors for volumes >= 32MB
    tot_sec_32: u32,

    // FAT32-specific fields
    /// Sectors per FAT
    fat_sz_32: u32,
    /// FAT flags (mirroring, active FAT)
    ext_flags: u16,
    /// Filesystem version (should be 0:0)
    fs_ver: u16,
    /// First cluster of root directory (typically 2)
    root_clus: u32,
    /// Sector number of FSINFO structure
    fs_info: u16,
    /// Sector number of backup boot sector
    bk_boot_sec: u16,
    /// Reserved for future expansion
    reserved: [u8; 12],
    /// Drive number (0x80 for hard disk)
    drv_num: u8,
    /// Reserved (used by Windows NT)
    reserved_1: u8,
    /// Extended boot signature (0x29)
    boot_sig: u8,
    /// Volume serial number
    vol_id: u32,
    /// Volume label (11 bytes)
    vol_lab: [u8; 11],
    /// Filesystem type label ("FAT32   ")
    fil_sys_type: [u8; 8],

    /// Boot code (not part of BPB specification)
    #[br(count = 420)]
    boot_code: Vec<u8>,
    /// Boot sector signature (0x55 0xAA)
    sig: [u8; 2],
}

impl BPB {
    /// Reads and optionally validates a BPB from a file at the specified sector.
    ///
    /// # Parameters
    /// - `file`: The file containing the filesystem
    /// - `sector`: The sector number where the BPB is located
    /// - `validate`: Whether to perform validation checks on the BPB
    /// - `sector_size`: The size of each sector in bytes
    ///
    /// # Returns
    /// - `Ok(BPB)`: The parsed and optionally validated BPB structure
    /// - `Err(BPBError)`: If reading fails or validation fails
    ///
    /// # Errors
    /// - Returns `BPBError::IOError` if reading from the file fails
    /// - Returns various `BPBError` variants if validation fails and `validate` is true
    pub fn from_file(
        file: &mut File,
        sector: u32,
        validate: bool,
        sector_size: usize,
    ) -> Result<BPB, BPBError> {
        let mut buf = vec![0; sector_size];
        utils::read_sector(file, sector.into(), sector_size, &mut buf)?;

        let mut reader = Cursor::new(buf);
        let bpb: BPB = reader.read_be().unwrap();

        if validate { bpb.validate() } else { Ok(bpb) }
    }

    /// Determines the number of clusters in the data section.
    ///
    /// # Returns
    /// - The number of data clusters.
    fn cluster_count(&self) -> u32 {
        let root_dir_sectors =
            (((self.root_ent_cnt * 32) + (self.bytes_per_sec - 1)) / self.bytes_per_sec) as u32;

        let fat_sz = if self.fat_sz_16 > 0 {
            self.fat_sz_16 as u32
        } else {
            self.fat_sz_32
        };

        let tot_sec = if self.tot_sec_16 != 0 {
            self.tot_sec_16 as u32
        } else {
            self.tot_sec_32
        };

        let data_sec = tot_sec
            - (self.rsvd_sec_cnt as u32 + (self.num_fat as u32 * fat_sz) + root_dir_sectors);
        let clus_cnt = data_sec / self.sec_per_clus as u32;

        clus_cnt
    }

    fn fat_sz(&self) -> u32 {
        match self.fat_type() {
            FATType::FAT32 => self.fat_sz_32,
            _ => self.fat_sz_16.into(),
        }
    }

    fn tot_sec(&self) -> u32 {
        match self.fat_type() {
            FATType::FAT32 => self.tot_sec_32,
            _ => {
                if self.tot_sec_16 == 0 {
                    self.tot_sec_32
                } else {
                    self.tot_sec_16.into()
                }
            }
        }
    }

    /// Determines the FAT type based on the number of clusters in the filesystem.
    ///
    /// # Returns
    /// - `FATType`: The detected filesystem type based on cluster count:
    ///   - `FAT12` if cluster count < 4085
    ///   - `FAT16` if cluster count < 65525
    ///   - `FAT32` if cluster count >= 65525
    fn fat_type(&self) -> FATType {
        let clus_cnt = self.cluster_count();

        if clus_cnt < 4085 {
            return FATType::FAT12;
        } else if clus_cnt < 65525 {
            return FATType::FAT16;
        } else {
            return FATType::FAT32;
        }
    }

    /// Validates the BPB structure according to FAT32 specification requirements.
    ///
    /// # Returns
    /// - `Ok(Self)`: If all validation checks pass
    /// - `Err(BPBError)`: If any validation check fails
    ///
    /// # Errors
    /// - `BPBError::InvalidJmp`: If the jump instruction is invalid
    /// - `BPBError::InvalidBytesPerSec`: If bytes per sector is not a valid value
    /// - `BPBError::InvalidSecPerClus`: If sectors per cluster is not a valid value
    /// - `BPBError::InvalidClusSz`: If cluster size exceeds 32 KiB
    /// - `BPBError::InvalidSignature`: If boot sector signature is not 0x55AA
    /// - `BPBError::UnsupportedFATType`: If filesystem is not FAT32
    fn validate(self) -> Result<Self, BPBError> {
        // General verification
        if !((self.jmp[0] == 0xEB && self.jmp[2] == 0x90) || self.jmp[0] == 0xE9) {
            return Err(BPBError::InvalidJmp(format!(
                "0x{:02X}{:02X}{:02X}",
                self.jmp[0], self.jmp[1], self.jmp[2],
            )));
        }

        const VALID_BYTES_PER_SEC: [u16; 4] = [512, 1024, 2048, 4096];
        if !VALID_BYTES_PER_SEC.contains(&self.bytes_per_sec) {
            return Err(BPBError::InvalidBytesPerSec(self.bytes_per_sec));
        }

        const VALID_SEC_PER_CLUS: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
        if !VALID_SEC_PER_CLUS.contains(&self.sec_per_clus) {
            return Err(BPBError::InvalidSecPerClus(self.sec_per_clus));
        }

        if self.bytes_per_sec as u32 * self.sec_per_clus as u32 > 32 * 1024 {
            return Err(BPBError::InvalidClusSz(
                self.bytes_per_sec as u32 * self.sec_per_clus as u32,
            ));
        }

        const SIG: [u8; 2] = [0x55, 0xAA];
        if !self.sig.eq(&SIG) {
            return Err(BPBError::InvalidSignature(format!(
                "0x{:02X}{:02X}",
                self.sig[0], self.sig[1]
            )));
        }

        // Specific verification depending on the type of FAT
        let fat_type = self.fat_type();
        if fat_type == FATType::FAT32 {
            self.validate_fat32()
        } else {
            return Err(BPBError::UnsupportedFATType(fat_type.to_string()));
        }
    }

    /// Performs FAT32-specific validation checks.
    ///
    /// # Returns
    /// - `Ok(Self)`: If all FAT32-specific validation checks pass
    /// - `Err(BPBError)`: If any validation check fails
    ///
    /// # Errors
    /// - `BPBError::InvalidRsvdSecCnt`: If reserved sector count is 0
    /// - `BPBError::InvalidNumFat`: If number of FATs is 0
    /// - `BPBError::InvalidRootEntCnt`: If root directory entries is not 0
    /// - `BPBError::InvalidTotSec`: If total sector fields are invalid for FAT32
    /// - `BPBError::InvalidFatSz`: If FAT size fields are invalid for FAT32
    /// - `BPBError::InvalidRootClus`: If root directory cluster is less than 2
    fn validate_fat32(self) -> Result<Self, BPBError> {
        assert!(self.fat_type() == FATType::FAT32);

        if self.rsvd_sec_cnt == 0 {
            return Err(BPBError::InvalidRsvdSecCnt(self.rsvd_sec_cnt));
        }

        if self.num_fat == 0 {
            return Err(BPBError::InvalidNumFat(self.num_fat));
        }

        if self.root_ent_cnt != 0 {
            return Err(BPBError::InvalidRootEntCnt(self.root_ent_cnt));
        }

        // Check for the count of sectors
        if self.tot_sec_16 != 0 {
            return Err(BPBError::InvalidTotSec(String::from(
                "BPB_TotSec16 should be 0 for a FAT32 volume.",
            )));
        }
        if self.tot_sec() == 0 {
            return Err(BPBError::InvalidTotSec(String::from(
                "BPB_TotSec32 should be greater than 0 for a FAT32 volume.",
            )));
        }

        // Check the FAT size
        if self.fat_sz_16 != 0 {
            return Err(BPBError::InvalidFatSz(String::from(
                "BPB_FATSz32 should be 0 for a FAT32 volume.",
            )));
        }
        if self.fat_sz_32 == 0 {
            return Err(BPBError::InvalidFatSz(String::from(
                "BPB_FATSz32 should be greater than 0 for a FAT32 volume.",
            )));
        }

        if self.root_clus < 2 {
            return Err(BPBError::InvalidRootClus(self.root_clus));
        }

        Ok(self)
    }
}

/// Implements the LayoutDisplay trait for BPB
impl LayoutDisplay for BPB {
    fn display_layout(&self, sector_offset: u64, indent: u8) -> String {
        let mut out = String::from("");
        let indent = " ".repeat(indent.into());

        let rsvd_start = sector_offset;
        let fat_start: u64 = sector_offset + u64::from(self.rsvd_sec_cnt);
        let data_start = fat_start + u64::from(self.fat_sz()) * u64::from(self.num_fat);
        let data_end = data_start + u64::from(self.cluster_count()) * u64::from(self.sec_per_clus);

        writeln!(out, "{}┌{:─^55}┐", indent, " FAT32 Partition Layout ").unwrap();
        writeln!(
            out,
            "{}├{:^12}┬{:^12}┬{:^12}┬{:^16}┤",
            indent, "Region", "Start", "End", "Description"
        )
        .unwrap();
        writeln!(
            out,
            "{}├{:─<12}┼{:─<12}┼{:─<12}┼{:─<16}┤",
            indent, "", "", "", ""
        )
        .unwrap();

        writeln!(
            out,
            "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
            indent, "Reserved", rsvd_start, fat_start, "Boot + Reserved"
        )
        .unwrap();
        for i in 0..self.num_fat {
            let fat_i_start = fat_start + u64::from(i) * u64::from(self.fat_sz());
            let fat_i_end = fat_i_start + u64::from(self.fat_sz());
            writeln!(
                out,
                "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
                indent,
                format!("FAT #{}", i),
                fat_i_start,
                fat_i_end,
                "FAT Tables"
            )
            .unwrap();
        }
        writeln!(
            out,
            "{}│{:<12}│{:<12}│{:<12}│{:<16}│",
            indent, "Data", data_start, data_end, "Cluster Data"
        )
        .unwrap();

        writeln!(
            out,
            "{}└{:─<12}┴{:─<12}┴{:─<12}┴{:─<16}┘",
            indent, "", "", "", ""
        )
        .unwrap();

        out
    }
}
