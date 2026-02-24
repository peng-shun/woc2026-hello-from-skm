#!/bin/bash

set -euo pipefail

usage() {
	echo "Usage: $0 [-b /path/to/busybox] [-k /path/to/kernel] [-v]"
	echo ""
	echo "Options:"
	echo "  -b: Path to busybox directory (default: ./busybox)"
	echo "  -k: Path to kernel directory (default: ./linux)"
	echo "  -v: Enable KVM virtualization"
}

requires() {
	for i in "$@"; do
		if ! command -v "$i" &>/dev/null; then
			echo "Error: $i is required but not installed."
			exit 1
		fi
	done
}

# Dependencies
requires qemu-system-x86_64 cpio gzip

BUSYBOX=./busybox
KERNEL=./linux
USE_KVM=false

# ./scripts/run -b /path/to/busybox -k /path/to/kernel -v
while getopts "b:k:v" opt; do
	case $opt in
	b)
		BUSYBOX=$OPTARG
		;;
	k)
		KERNEL=$OPTARG
		;;
	v)
		USE_KVM=true
		;;
	\?)
		usage
		exit 1
		;;
	:)
		echo "Option -$OPTARG requires an argument." >&2
		exit 1
		;;
	esac
done

# Prepare QEMU arguments
QEMU_ARGS=(
	-kernel "$KERNEL/arch/x86_64/boot/bzImage"
	-initrd "$BUSYBOX/rootfs.img"
	-nographic
	-machine q35
	-device intel-iommu
	-m 4G
	-nic "user,model=virtio-net-pci,hostfwd=tcp::5555-:23,hostfwd=tcp::5556-:8080"
	-append "console=ttyS0,115200 loglevel=3 rdinit=/sbin/init"
)

if [ "$USE_KVM" = true ]; then
	QEMU_ARGS+=(-cpu host -enable-kvm)
fi

# Run the kernel in qemu
qemu-system-x86_64 "${QEMU_ARGS[@]}"
