//! Chess application entry point.

use gpui::Application;

mod app;
mod domain;
mod models;
mod ui;

fn main() {
    Application::new()
        .with_assets(ui::FileAssets::new())
        .run(app::run);
}
