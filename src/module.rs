// SPDX-License-Identifier: GPL-2.0

//! SKM is a simple linux kernel module written in rust.

use kernel::prelude::*;
mod debugfs;
mod tetris;

module! {
    type: SASTKernelModule,
    name: "woc2026_hello_from_skm",
    authors: ["fermata"],
    description: "SKM is a simple linux kernel module written in rust",
    license: "GPL",
}

struct SASTKernelModule {
    _dev:
        Pin<kernel::alloc::KBox<kernel::miscdevice::MiscDeviceRegistration<tetris::TetrisDevice>>>,
    _debugfs: Option<debugfs::TetrisDebugfs>,
}

#[allow(unreachable_code)]
impl kernel::Module for SASTKernelModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Welcome to SAST WoC 2026!\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        pr_info!("Tetris kernel module loaded!\n");
        pr_info!("Device: /dev/tetris\n");
        pr_info!("Controls: a=left, d=right, s=down, w=rotate, space=drop, r=reset\n");

        // panic!("Try fix me!");
        let _dev = tetris::register_tetris_device()?;

        let _debugfs = match tetris::GLOBAL_DEVICE.lock().as_ref() {
            Some(dev) => debugfs::TetrisDebugfs::register(dev.clone()).ok(),
            None => None,
        };

        Ok(Self { 
            _dev,
            _debugfs,
        })
    }
}

impl Drop for SASTKernelModule {
    fn drop(&mut self) {
        pr_info!("Tetris module unloading, cleaning up global game state\n");

        let mut global_device = tetris::GLOBAL_DEVICE.lock();
        *global_device = None;
        drop(global_device);

        pr_info!("bye bye\n");
    }
}
