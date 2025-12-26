//! Theme constants and colors for the chess UI.

use gpui::{Rgba, rgb};

// Layout constants
pub const BOARD_PADDING: f32 = 20.0;
pub const PIECE_SCALE: f32 = 0.98; // piece size relative to square
pub const GHOST_OPACITY: f32 = 0.4;

// Initial panel sizes
pub const INITIAL_LEFT_PANEL: f32 = 450.0;

// Board colors
pub const LIGHT_SQUARE: u32 = 0xEFD9B5;
pub const DARK_SQUARE: u32 = 0xB48764;

// Panel colors
pub const PANEL_BG: u32 = 0x2a2a2a;
pub const MOVE_LIST_BG: u32 = 0x1e1e1e;
pub const BORDER_COLOR: u32 = 0x4a4a4a;
pub const TEXT_PRIMARY: u32 = 0xffffff;
pub const TEXT_SECONDARY: u32 = 0x888888;

/// Get the color for a board square based on its position
pub fn square_color(row: usize, col: usize) -> Rgba {
    if (row + col) % 2 == 0 {
        rgb(LIGHT_SQUARE)
    } else {
        rgb(DARK_SQUARE)
    }
}
