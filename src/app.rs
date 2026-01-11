//! Application setup and window creation.

use gpui::{App, Bounds, KeyBinding, WindowBounds, WindowOptions, actions, prelude::*, px, size};
use gpui_component::{Root, Theme, ThemeMode};

use crate::models::GameModel;
use crate::ui::views::{ChessBoardView, MoveBack, MoveForward, MoveToEnd, MoveToStart};

actions!(app, [ToggleTheme]);

/// Initialize and run the chess application
pub fn run(cx: &mut App) {
    gpui_component::init(cx);

    // Set dark theme for the application
    Theme::change(ThemeMode::Dark, None, cx);

    // Bind keyboard shortcuts
    cx.bind_keys([
        KeyBinding::new("left", MoveBack, None),
        KeyBinding::new("right", MoveForward, None),
        KeyBinding::new("home", MoveToStart, None),
        KeyBinding::new("end", MoveToEnd, None),
        KeyBinding::new("cmd-t", ToggleTheme, None),
    ]);

    // Register global action for theme toggling
    cx.on_action(|_: &ToggleTheme, cx| {
        let current_mode = Theme::global(cx).mode;
        let new_mode = match current_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        Theme::change(new_mode, None, cx);
        cx.refresh_windows();
    });

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
