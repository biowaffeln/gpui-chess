//! Filesystem-based asset source for loading piece SVGs and other assets.

use gpui::{AssetSource, SharedString};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use crate::domain::{Piece, PieceColor, PieceKind};

/// Get the SVG asset path for a chess piece
pub fn piece_svg_path(piece: &Piece) -> &'static str {
    match (piece.kind, piece.color) {
        (PieceKind::Pawn, PieceColor::White) => "assets/pawn-white.svg",
        (PieceKind::Pawn, PieceColor::Black) => "assets/pawn-black.svg",
        (PieceKind::Rook, PieceColor::White) => "assets/rook-white.svg",
        (PieceKind::Rook, PieceColor::Black) => "assets/rook-black.svg",
        (PieceKind::Knight, PieceColor::White) => "assets/knight-white.svg",
        (PieceKind::Knight, PieceColor::Black) => "assets/knight-black.svg",
        (PieceKind::Bishop, PieceColor::White) => "assets/bishop-white.svg",
        (PieceKind::Bishop, PieceColor::Black) => "assets/bishop-black.svg",
        (PieceKind::Queen, PieceColor::White) => "assets/queen-white.svg",
        (PieceKind::Queen, PieceColor::Black) => "assets/queen-black.svg",
        (PieceKind::King, PieceColor::White) => "assets/king-white.svg",
        (PieceKind::King, PieceColor::Black) => "assets/king-black.svg",
    }
}

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
