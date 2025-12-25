//! Square rendering component.

use crate::domain::Piece;
use crate::ui::components::render_piece;
use crate::ui::theme::{GHOST_OPACITY, square_color};
use gpui::{div, img, prelude::*, px};

/// Render a single board square with optional piece
pub fn render_square(
    row: usize,
    col: usize,
    piece: Option<Piece>,
    is_being_dragged: bool,
    square_size: f32,
    piece_size: f32,
) -> impl IntoElement {
    div()
        .flex_shrink_0() // never shrink - maintain aspect ratio
        .size(px(square_size))
        .bg(square_color(row, col))
        .flex()
        .items_center()
        .justify_center()
        .when_some(piece, |el, p| {
            if is_being_dragged {
                // ghost piece on original square
                el.child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(GHOST_OPACITY)
                        .child(img(p.svg_path()).size(px(piece_size))),
                )
            } else {
                el.child(render_piece(p, piece_size))
            }
        })
}
