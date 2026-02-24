#!/bin/bash
# Entrypoint for the OCI/Docker container.
# Launches QEMU with the pre-built kernel and rootfs artifacts.
#
# Artifact paths inside the container (set at docker build time):
#   /artifacts/bzImage   - Linux kernel image
#   /artifacts/rootfs.img - BusyBox initramfs
#
# Users can override paths via environment variables:
#   BZIMAGE  - path to bzImage  (default: /artifacts/bzImage)
#   ROOTFS   - path to rootfs.img (default: /artifacts/rootfs.img)

set -euo pipefail

BZIMAGE="${BZIMAGE:-/artifacts/bzImage}"
ROOTFS="${ROOTFS:-/artifacts/rootfs.img}"

# Sanity check: make sure the artifacts actually exist.
# This guards against someone running the image without copying in the files.
if [ ! -f "$BZIMAGE" ]; then
    echo "ERROR: Kernel image not found at $BZIMAGE" >&2
    exit 1
fi
if [ ! -f "$ROOTFS" ]; then
    echo "ERROR: Rootfs image not found at $ROOTFS" >&2
    exit 1
fi

echo "==> Starting QEMU..."
echo "    kernel : $BZIMAGE"
echo "    initrd : $ROOTFS"
echo "    serial output follows (Ctrl-A X to quit QEMU)"
echo "------------------------------------------------------------"

# Mirror the flags from scripts/run.sh, minus KVM (not available in containers).
# Key flags explained:
#   -nographic        : no GUI window; serial port goes to stdout (crucial for docker)
#   -machine q35      : modern PCIe chipset
#   -device intel-iommu : IOMMU emulation (required by the kernel config)
#   -m 4G             : 4 GiB RAM for the guest
#   -nic user,...     : user-mode networking, forward host:5555 -> guest:23 (telnet)
#                       and host:5556 -> guest:8080
#   -append           : kernel command line; rdinit=/sbin/init boots into BusyBox
exec qemu-system-x86_64 \
    -kernel  "$BZIMAGE" \
    -initrd  "$ROOTFS" \
    -nographic \
    -machine q35 \
    -device  intel-iommu \
    -m 4G \
    -nic user,model=virtio-net-pci,hostfwd=tcp::5555-:23,hostfwd=tcp::5556-:8080 \
    -append  "console=ttyS0,115200 loglevel=3 rdinit=/sbin/init"
