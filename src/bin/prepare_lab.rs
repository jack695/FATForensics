//! This script prepares the disk image for the lab that will be given to students.
//!
//! The program hides files and flags all over a MBR partitioned disk image and FAT32 fs.

use fat_forensics::Disk;
use fat_forensics::FATVol;
use fat_forensics::Volume;
use fat_forensics::traits::SlackWriter;
use fat_forensics::utils::write_file_at;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;

const SECTOR_SIZE: usize = 512;

fn main() {
    // Parse the args: the path to the disk image and the path to the directory containing all flag files.
    let args: Vec<String> = env::args().collect();
    let (disk_path, flag_dir_path) = match args.len() {
        3 => (args[1].as_str(), args[2].as_str()),
        _ => {
            eprintln!(
                "Please provide the path to the disk image file and the path to the directory containing all flag files."
            );
            return;
        }
    };

    // Open the disk
    let disk = Disk::from_file(&disk_path, SECTOR_SIZE, false).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    // Check the disk contains exactly one FAT32 volume
    assert_eq!(
        disk.vol_count(),
        1,
        "The number of volumes should be exactly one."
    );
    let vol = match &disk.volumes().get(0) {
        Some(Volume::FAT32(fat_vol)) => fat_vol,
        _ => {
            eprintln!("The disk should contain one FAT32 volume.");
            std::process::exit(1);
        }
    };

    let mut entries: Vec<_> = fs::read_dir(flag_dir_path)
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let full_path = fs::canonicalize(&path).unwrap(); // gives absolute path

        hide_flag(i, full_path.to_str().unwrap(), &disk, vol);
    }
}

fn hide_flag(flag_idx: usize, flag_file_path: &str, disk: &Disk, fat_vol: &FATVol) {
    let mut disk_file = File::options()
        .read(true)
        .write(true)
        .open(disk.file_path())
        .expect("Failed to open disk image file.");

    match flag_idx {
        0 => hide_flag_after_mbr(flag_file_path, &mut disk_file, fat_vol, &disk),
        1 => hide_flag_in_volume_slack(flag_file_path, &mut disk_file, fat_vol),
        2 => hide_flag_in_file_slack(flag_file_path, &mut disk_file, fat_vol),
        3 => hide_file_in_bad_clusters(flag_file_path, &mut disk_file, fat_vol),
        _ => {
            println!("Unsupported flag count to hide: {}", flag_idx);
            std::process::exit(1);
        }
    }
}

fn hide_flag_after_mbr(flag_file_path: &str, disk_file: &mut File, fat_vol: &FATVol, disk: &Disk) {
    write_file_at(
        disk_file,
        SECTOR_SIZE as u64 * 1,
        flag_file_path,
        SECTOR_SIZE,
        (fat_vol.start() * disk.sector_size() as u32).into(),
    )
    .expect("Failed to hide the flag after the MBR.");
}

fn hide_flag_in_volume_slack(flag_file_path: &str, disk: &mut File, fat_vol: &FATVol) {
    let data: Vec<u8> = fs::read(flag_file_path).expect("Failed to read flag file.");

    fat_vol
        .write_to_volume_slack(disk, &data)
        .unwrap_or_else(|e| {
            eprintln!("Failed to write to volume slack: {}", e);
            std::process::exit(1);
        });
}

fn hide_flag_in_file_slack(flag_file_path: &str, disk: &mut File, fat_vol: &FATVol) {
    let data: Vec<u8> = fs::read(flag_file_path).expect("Failed to read flag file.");

    fat_vol
        .write_to_file_slack(disk, Path::new("1/t.txt"), &data)
        .unwrap_or_else(|e| {
            eprintln!("Failed to write to volume slack: {}", e);
            std::process::exit(1);
        });
}

fn hide_file_in_bad_clusters(flag_file_path: &str, disk: &mut File, fat_vol: &FATVol) {
    let data: Vec<u8> = fs::read(flag_file_path).expect("Failed to read flag file.");

    let cluster_cnt = (data.len() as u32).div_ceil(fat_vol.cluster_size());
    let chain_start = fat_vol.mark_as_bad(cluster_cnt).unwrap_or_else(|e| {
        eprintln!("Failed to mark the file's clusters as bad: {}", e);
        std::process::exit(1);
    });

    let offset = fat_vol.clus_to_sector(chain_start) as u64 * SECTOR_SIZE as u64;
    let limit = offset + cluster_cnt as u64 * fat_vol.cluster_size() as u64;
    write_file_at(disk, offset, flag_file_path, SECTOR_SIZE, limit).unwrap()
}
