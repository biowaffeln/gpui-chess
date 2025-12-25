//! Piece rendering component.

use crate::domain::Piece;
use gpui::{div, img, prelude::*, px};

/// Render a chess piece centered in its container
pub fn render_piece(piece: Piece, piece_size: f32) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(img(piece.svg_path()).size(px(piece_size)))
}
