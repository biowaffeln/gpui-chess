//! Board layout calculations - handles sizing and coordinate transformations.

use crate::ui::theme::{BOARD_PADDING, PIECE_SCALE};
use gpui::{Pixels, Size, px};

/// Handles all layout calculations for the chess board
#[derive(Clone, Copy, Debug)]
pub struct BoardLayout {
    pub panel_size: Size<Pixels>,
}

impl BoardLayout {
    pub fn new(panel_size: Size<Pixels>) -> Self {
        Self { panel_size }
    }

    /// Calculate square size from measured panel dimensions
    pub fn square_size(&self) -> f32 {
        let panel_width: f32 = self.panel_size.width.into();
        let panel_height: f32 = self.panel_size.height.into();
        let available_width = panel_width - BOARD_PADDING * 2.0;
        let available_height = panel_height - BOARD_PADDING * 2.0;
        (available_width.min(available_height) / 8.0).max(30.0)
    }

    /// Calculate piece size based on square size
    pub fn piece_size(&self) -> f32 {
        self.square_size() * PIECE_SCALE
    }

    /// Convert position relative to board panel to board row/col (if within board)
    pub fn pos_to_square(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let board_x = x - BOARD_PADDING;
        let board_y = y - BOARD_PADDING;

        if board_x < 0.0 || board_y < 0.0 {
            return None;
        }

        let square_size = self.square_size();
        let col = (board_x / square_size) as usize;
        let row = (board_y / square_size) as usize;

        if row < 8 && col < 8 {
            Some((row, col))
        } else {
            None
        }
    }

    /// Get the total size of the board (8 squares)
    pub fn board_total_size(&self) -> f32 {
        self.square_size() * 8.0
    }

    /// Get the offset for centering pieces within squares
    pub fn piece_offset(&self) -> f32 {
        (self.square_size() - self.piece_size()) / 2.0
    }
}

impl Default for BoardLayout {
    fn default() -> Self {
        Self::new(Size {
            width: px(450.0),
            height: px(600.0),
        })
    }
}
