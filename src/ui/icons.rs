//! Custom icon definitions using Phosphor Icons.
//!
//! This module defines our own IconName enum that maps to phosphor SVG icons
//! in the assets/icons/ directory.

use gpui::{App, SharedString, Window, prelude::*};
use gpui_component::{Icon, IconNamed};

/// Custom icons for the chess application using Phosphor Icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChessIcon {
    /// Navigation: skip to start
    CaretDoubleLeft,
    /// Navigation: go back one move
    CaretLeft,
    /// Navigation: go forward one move
    CaretRight,
    /// Navigation: skip to end
    CaretDoubleRight,
    /// Collapse/expand indicator
    CaretDown,
    /// Play button for engine
    Play,
    /// Stop button for engine
    Stop,
    /// Plus for expand
    Plus,
    /// Minus for collapse
    Minus,
}

impl IconNamed for ChessIcon {
    fn path(self) -> SharedString {
        match self {
            Self::CaretDoubleLeft => "assets/icons/caret-double-left.svg",
            Self::CaretLeft => "assets/icons/caret-left.svg",
            Self::CaretRight => "assets/icons/caret-right.svg",
            Self::CaretDoubleRight => "assets/icons/caret-double-right.svg",
            Self::CaretDown => "assets/icons/caret-down.svg",
            Self::Play => "assets/icons/play.svg",
            Self::Stop => "assets/icons/stop.svg",
            Self::Plus => "assets/icons/plus.svg",
            Self::Minus => "assets/icons/minus.svg",
        }
        .into()
    }
}

impl From<ChessIcon> for gpui::AnyElement {
    fn from(val: ChessIcon) -> Self {
        Icon::new(val).into_any_element()
    }
}

impl RenderOnce for ChessIcon {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}
