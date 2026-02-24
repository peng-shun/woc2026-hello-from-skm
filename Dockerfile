# =============================================================================
# Stage: runtime
#
# This is a *runtime-only* image. It does NOT compile anything.
# The build artifacts (bzImage, rootfs.img) are produced by `make build`
# in CI and then COPY-ed in at the docker build step.
#
# Usage (after `make build`):
#   docker build -t woc2026-hello-from-skm .
#   docker run --rm -it -p 5555:5555 woc2026-hello-from-skm
#
# Then in another terminal:
#   telnet localhost 5555   # connects to the guest's /dev/pts/0 via init
# =============================================================================

FROM ubuntu:24.04

# --------------------------------------------------------------------------
# KEY POINT 1: Install only what the container needs at *runtime*.
# We do NOT install compilers (gcc, clang, cargo, etc.) here — those
# are only needed during the build phase which runs on the CI runner.
#
# qemu-system-x86: the QEMU binary for x86_64 emulation
# libglib2.0-0 / libpixman-1-0: runtime libraries that QEMU links against
#   (Ubuntu 24.04's package already pulls these in as dependencies, but
#    listing them makes the intent explicit)
# --------------------------------------------------------------------------
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        qemu-system-x86 \
    && rm -rf /var/lib/apt/lists/*
# ^^^^ always clean apt cache at the end of a RUN —
# this keeps the image layer small by not baking in the package index.

# --------------------------------------------------------------------------
# KEY POINT 2: Copy build artifacts into a fixed, well-known location.
# The COPY instruction runs at `docker build` time, pulling files from
# your build context (the repo root where you run `docker build .`).
# --------------------------------------------------------------------------
RUN mkdir -p /artifacts

COPY linux/arch/x86_64/boot/bzImage  /artifacts/bzImage
COPY busybox/rootfs.img               /artifacts/rootfs.img

# --------------------------------------------------------------------------
# KEY POINT 3: Copy + make executable the entrypoint script.
# Using COPY then RUN chmod (vs ADD) is best practice: explicit & auditable.
# --------------------------------------------------------------------------
COPY scripts/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# --------------------------------------------------------------------------
# KEY POINT 4: EXPOSE documents which ports the container listens on.
# It does NOT actually open ports — that's done with `docker run -p`.
# 5555 → guest telnet (port 23 inside the VM)
# 5556 → guest HTTP (port 8080 inside the VM)
# --------------------------------------------------------------------------
EXPOSE 5555 5556

# --------------------------------------------------------------------------
# KEY POINT 5: ENTRYPOINT vs CMD
# ENTRYPOINT is the fixed executable that always runs.
# CMD provides default arguments that users can override.
# Using ["exec-form"] (JSON array) is preferred over shell form because it
# makes the process PID 1 directly — important for correct signal handling
# (e.g. Ctrl-C / SIGINT reaching QEMU properly).
# --------------------------------------------------------------------------
ENTRYPOINT ["/entrypoint.sh"]
