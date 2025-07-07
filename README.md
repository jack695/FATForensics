# FATForensics

**FATForensics** is a Rust toolkit for digital forensics and educational CTF/lab preparation on FAT32 disk images. It provides both a reusable library and a command-line interface (CLI) for:

- **Basic forensic analysis** of FAT32 disk images (partition and filesystem structure, directory tree, etc.)
- **Hiding and recovering flags** in various locations of a FAT32 filesystem (e.g., after the MBR, in volume slack, in file slack, or in bad clusters)

## Features

- Parse and validate Master Boot Records (MBR) and FAT32 filesystems
- Print disk and partition layouts in a human-readable format
- Traverse and display the directory tree of a FAT32 volume
- Write arbitrary data (flags) into:
  - Unallocated space after the MBR
  - Volume slack space
  - File slack space
  - Bad clusters
- Modular Rust library for scripting or integration
- CLI tools for interactive analysis and lab preparation

## Usage

### CLI Forensics

The main CLI (`src/bin/main.rs`) allows you to:
- Open a FAT32 disk image
- Print the disk and partition layout
- Traverse the directory tree
- Select and inspect partitions
- Write files to specific sectors

Check our `src/bin/command.rs` for details on the CLI usage.

### Lab Preparation

The `prepare_lab` CLI (`src/bin/prepare_lab.rs`) is designed for instructors or CTF organizers to:
- Hide multiple flags in different locations of a FAT32 disk image
- Prepare disk images for student exercises or competitions

## Example

```sh
# Analyze a disk image interactively
cargo run

# Prepare a lab image with hidden flags
cargo run --bin prepare_lab data/base.img data/flags
```

## Limitations

- **Only FAT32 is supported.** FAT12/16 and other filesystems are not recognized.
- **No support for long file names (LFN).** Only 8.3 short names are handled.
- **No support for non-MBR partition tables.** Only classic MBR is parsed.
- **No file system repair or recovery features.** This tool is for analysis and CTF/lab prep, not forensics-grade recovery.
- **No Windows or non-UNIX support tested.** The tool is developed and tested on UNIX-like systems.

## Project Structure

- `src/` — Core library code (partitioning, filesystem, utilities)
- `src/bin/main.rs` — Interactive CLI for analysis
- `src/bin/prepare_lab.rs` — CLI for hiding flags in disk images
- `data/` — Example disk images and flags (not included in repo)
- `reference/` — Reference material and documentation

## License

MIT

## Tutorials

### Creation of a disk image file (Linux)

You can start by creating a disk image file with the following command:

> dd if=/dev/zero of=disk.img bs=1M count=72

Then, partition it with:

> parted disk.img --script mklabel msdos
> parted disk.img --script unit s mkpart primary fat32 2048 $((2048+70*1024*1024/512))

This allocates 70MiB + 1 sector for the FAT32 volume, hence creating some volume slack.

Finally, format the partition as FAT32.

> LOOPDEV=$(sudo losetup --find --show --partscan disk.img)
> sudo mkfs.vfat -F 32 -s 2 "${LOOPDEV}p1"

### Mounting the image on MacOS

Start with these two commands:
> hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount <PATH_TO_IMAGE>
> mount -t msdos /dev/<DISK> /Volumes/fatvol

Modify the volume as you wish, then run the 2 final commands:

> umount /Volumes/fatvol
> hdiutil detach /dev/disk4