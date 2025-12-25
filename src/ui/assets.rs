//! Filesystem-based asset source for loading piece SVGs and other assets.

use gpui::{AssetSource, SharedString};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

/// Filesystem-based asset source that looks for assets in multiple locations
pub struct FileAssets {
    base_path: PathBuf,
}

impl FileAssets {
    pub fn new() -> Self {
        let base_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        Self { base_path }
    }
}

impl Default for FileAssets {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetSource for FileAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let paths_to_try = [
            self.base_path.join(path),
            PathBuf::from(path),
            std::env::current_dir().unwrap().join(path),
        ];

        for p in &paths_to_try {
            if let Ok(data) = fs::read(p) {
                return Ok(Some(Cow::Owned(data)));
            }
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let dir_path = self.base_path.join(path);
        let mut results = Vec::new();

        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    results.push(SharedString::from(name.to_string()));
                }
            }
        }
        Ok(results)
    }
}
