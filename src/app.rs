//! Application setup and window creation.

use gpui::{App, Bounds, KeyBinding, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_component::{Root, Theme};

use crate::models::GameModel;
use crate::ui::views::{ChessBoardView, MoveBack, MoveForward, MoveToEnd, MoveToStart};

/// Initialize and run the chess application
pub fn run(cx: &mut App) {
    gpui_component::init(cx);

    Theme::global_mut(cx).border = gpui::hsla(0.0, 0.0, 0.24, 1.0);

    // Bind keyboard shortcuts
    cx.bind_keys([
        KeyBinding::new("left", MoveBack, None),
        KeyBinding::new("right", MoveForward, None),
        KeyBinding::new("home", MoveToStart, None),
        KeyBinding::new("end", MoveToEnd, None),
    ]);

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
