//! Theme constants for the chess UI.
//!
//! Colors are handled by gpui-component's theming system.
//! Access colors via `cx.theme()` using the `ActiveTheme` trait.

// Layout constants
pub const BOARD_PADDING: f32 = 20.0;
pub const PIECE_SCALE: f32 = 0.98; // piece size relative to square
pub const GHOST_OPACITY: f32 = 0.4;
pub const BOARD_CORNER_RADIUS: f32 = 6.0;

// Initial panel sizes
pub const INITIAL_LEFT_PANEL: f32 = 450.0;
