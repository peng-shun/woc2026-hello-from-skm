// src/debugfs.rs
// SPDX-License-Identifier: GPL-2.0

//! Debugfs interface for the Tetris game module.

use core::pin::Pin;
use kernel::{
    c_str,
    debugfs::{Dir, File},
    prelude::*,
    sync::Arc,
};

use crate::tetris::TetrisDevice;

pub(crate) struct TetrisDebugfs {
    _dir: Dir,
    _state: Pin<kernel::alloc::KBox<File<Arc<TetrisDevice>>>>,
    _board: Pin<kernel::alloc::KBox<File<Arc<TetrisDevice>>>>,
    _stats: Pin<kernel::alloc::KBox<File<Arc<TetrisDevice>>>>,
    _bag: Pin<kernel::alloc::KBox<File<Arc<TetrisDevice>>>>,
    _control: Pin<kernel::alloc::KBox<File<Arc<TetrisDevice>>>>,
}

impl TetrisDebugfs {
    pub(crate) fn register(device: Arc<TetrisDevice>) -> Result<Self> {
        let dir = Dir::new(c_str!("tetris_debugfs"));

        let state_file = kernel::alloc::KBox::pin_init(
            dir.read_callback_file(
                c_str!("state"),
                device.clone(),
                &|dev: &Arc<TetrisDevice>, f: &mut core::fmt::Formatter<'_>| {
                    let game = dev.inner_game_lock();
                    let (ptype, x, y, rot) = if let Some(p) = game.current_piece() {
                        (p.piece_type_str(), p.x(), p.y(), p.rotation())
                    } else {
                        ("None", 0, 0, 0)
                    };

                    core::writeln!(
                        f,
                        "game_over:    {}\nscore:        {}\ncurrent_type: {}\ncurrent_x:    {}\ncurrent_y:    {}\ncurrent_rot:  {}\nnext_type:    {}\nbag_idx:      {}",
                        game.is_game_over(),
                        game.score(),
                        ptype,
                        x,
                        y,
                        rot,
                        game.next_piece_type_str(),
                        game.bag_idx()
                    )
                },
            ),
            GFP_KERNEL,
        )?;

        let board_file = kernel::alloc::KBox::pin_init(
            dir.read_callback_file(
                c_str!("board"),
                device.clone(),
                &|dev: &Arc<TetrisDevice>, f: &mut core::fmt::Formatter<'_>| {
                    let game = dev.inner_game_lock();
                    let mut buffer = kernel::alloc::KVec::new();
                    // Just allocate enough for an ASCII board, matching RENDER_BUFFER_SIZE in character device
                    if buffer.resize(4096, 0, GFP_KERNEL).is_ok() {
                        let len = game.render_ascii_to_buffer(&mut buffer);
                        if let Ok(s) = core::str::from_utf8(&buffer[..len]) {
                            return core::write!(f, "{}", s);
                        }
                    }
                    core::writeln!(f, "Error rendering board")
                },
            ),
            GFP_KERNEL,
        )?;

        let stats_file = kernel::alloc::KBox::pin_init(
            dir.read_callback_file(
                c_str!("stats"),
                device.clone(),
                &|dev: &Arc<TetrisDevice>, f: &mut core::fmt::Formatter<'_>| {
                    let game = dev.inner_game_lock();
                    let lines = game.lines_by_type();
                    let pieces = game.pieces_by_type();
                    core::writeln!(
                        f,
                        "lines_total:    {}\nlines_single:   {}\nlines_double:   {}\nlines_triple:   {}\nlines_tetris:   {}\n\npieces_total:   {}\npieces_I:       {}\npieces_O:       {}\npieces_T:       {}\npieces_S:       {}\npieces_Z:       {}\npieces_J:       {}\npieces_L:       {}",
                        game.lines_cleared_total(),
                        lines[0], lines[1], lines[2], lines[3],
                        game.pieces_spawned(),
                        pieces[0], pieces[1], pieces[2], pieces[3], pieces[4], pieces[5], pieces[6]
                    )
                },
            ),
            GFP_KERNEL,
        )?;

        let bag_file = kernel::alloc::KBox::pin_init(
            dir.read_callback_file(
                c_str!("bag"),
                device.clone(),
                &|dev: &Arc<TetrisDevice>, f: &mut core::fmt::Formatter<'_>| {
                    let game = dev.inner_game_lock();

                    core::write!(f, "bag_idx:    {}\nremaining:  ", game.bag_idx())?;
                    for p in game.bag_remaining() {
                        core::write!(f, "{} ", p.as_str())?;
                    }
                    core::write!(f, "\nused:       ")?;
                    for p in game.bag_used() {
                        core::write!(f, "{} ", p.as_str())?;
                    }
                    core::writeln!(f, "\nprng_state: {:#018x}", game.prng_state())
                },
            ),
            GFP_KERNEL,
        )?;

        let control_file = kernel::alloc::KBox::pin_init(
            dir.read_write_callback_file(
                c_str!("control"),
                device.clone(),
                &|_dev: &Arc<TetrisDevice>, f: &mut core::fmt::Formatter<'_>| {
                    core::writeln!(f, "Write-only control file. Use 'echo cmd > control'")
                },
                &|dev: &Arc<TetrisDevice>, reader: &mut kernel::uaccess::UserSliceReader| {
                    let mut buf = [0u8; 32];
                    let len = core::cmp::min(reader.len(), buf.len());
                    reader.read_slice(&mut buf[..len])?;

                    if let Ok(cmd_str) = core::str::from_utf8(&buf[..len]) {
                        let cmd = cmd_str.trim();
                        let mut game = dev.inner_game_lock();

                        match cmd {
                            "left" => {
                                game.move_left();
                            }
                            "right" => {
                                game.move_right();
                            }
                            "down" => {
                                game.move_down();
                            }
                            "drop" => {
                                game.hard_drop();
                            }
                            "rotate" => {
                                game.rotate();
                            }
                            "reset" => {
                                game.reset();
                            }
                            "tick" => {
                                game.ticks += 1;
                                game.move_down();
                            }
                            _ => {
                                // For `spawn <type>`
                                if cmd.starts_with("spawn ") {
                                    let p_str = &cmd[6..];
                                    use crate::tetris::TetrominoType;
                                    let new_type = match p_str {
                                        "I" | "i" => Some(TetrominoType::I),
                                        "O" | "o" => Some(TetrominoType::O),
                                        "T" | "t" => Some(TetrominoType::T),
                                        "S" | "s" => Some(TetrominoType::S),
                                        "Z" | "z" => Some(TetrominoType::Z),
                                        "J" | "j" => Some(TetrominoType::J),
                                        "L" | "l" => Some(TetrominoType::L),
                                        _ => None,
                                    };

                                    if let Some(t) = new_type {
                                        game.next_piece_type = t;
                                        game.spawn_piece();
                                    }
                                }
                            }
                        }
                    }

                    Ok(())
                },
            ),
            GFP_KERNEL,
        )?;

        Ok(Self {
            _dir: dir,
            _state: state_file,
            _board: board_file,
            _stats: stats_file,
            _bag: bag_file,
            _control: control_file,
        })
    }
}
