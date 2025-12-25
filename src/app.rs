//! Application setup and window creation.

use gpui::{App, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_component::Root;

use crate::models::GameModel;
use crate::ui::views::ChessBoardView;

/// Initialize and run the chess application
pub fn run(cx: &mut App) {
    gpui_component::init(cx);

    // Create the game model
    let model = cx.new(|_| GameModel::new());

    let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| ChessBoardView::new(model, cx));
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .unwrap();
}
