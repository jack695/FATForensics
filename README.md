# FATForensics

## Disk image file

You can start by creating a disk image file with the following command:

> dd if=/dev/zero of=disk.img bs=1M count=72

Then, partition it with:

> parted disk.img --script mklabel msdos
> parted disk.img --script unit s mkpart primary fat32 2048 $((2048+70*1024*1024/512))

This allocates 70MiB + 1 sector for the FAT32 volume, hence creating some volume slack.

Finally, format the partition as FAT32.

> LOOPDEV=$(sudo losetup --find --show --partscan disk.img)
> sudo mkfs.vfat -F 32 -s 2 "${LOOPDEV}p1"
