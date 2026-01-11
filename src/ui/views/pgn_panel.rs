//! PGN panel - tabbed view with Move History and PGN Library.

use gpui::{App, Context, Div, Entity, PathPromptOptions, Window, div, prelude::*, px};
use gpui_component::button::Button;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::table::{Column, Table, TableDelegate, TableState};
use gpui_component::{ActiveTheme, IconName, Sizable};

use crate::models::{GameModel, PgnLibraryModel};
use crate::ui::theme::BOARD_PADDING;

use super::board_view::MoveListState;
use super::move_list::render_move_list_content;

/// Which tab is currently active
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PgnPanelTab {
    #[default]
    Moves,
    Library,
}

/// State for the PGN panel (tabs + library)
pub struct PgnPanelState {
    pub active_tab: PgnPanelTab,
}

impl PgnPanelState {
    pub fn new() -> Self {
        Self {
            active_tab: PgnPanelTab::Moves,
        }
    }
}

impl Default for PgnPanelState {
    fn default() -> Self {
        Self::new()
    }
}

/// Table delegate for the PGN library
pub struct PgnTableDelegate {
    library: Entity<PgnLibraryModel>,
    game_model: Entity<GameModel>,
    columns: Vec<Column>,
}

impl PgnTableDelegate {
    pub fn new(library: Entity<PgnLibraryModel>, game_model: Entity<GameModel>) -> Self {
        let columns = vec![
            Column::new("white", "White").width(px(90.)),
            Column::new("white_elo", "Elo").width(px(40.)),
            Column::new("black", "Black").width(px(90.)),
            Column::new("black_elo", "Elo").width(px(40.)),
            Column::new("date", "Date").width(px(80.)),
            Column::new("event", "Event").width(px(100.)),
            Column::new("result", "Result").width(px(45.)),
            Column::new("moves", "#").width(px(30.)),
        ];
        Self {
            library,
            game_model,
            columns,
        }
    }
}

impl TableDelegate for PgnTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, cx: &App) -> usize {
        self.library.read(cx).games_count()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let summaries = self.library.read(cx).summaries();
        let summary = summaries.get(row_ix);

        let text = match (summary, col_ix) {
            (Some(s), 0) => s.white.clone(),
            (Some(s), 1) => s.white_elo.map(|e| e.to_string()).unwrap_or_default(),
            (Some(s), 2) => s.black.clone(),
            (Some(s), 3) => s.black_elo.map(|e| e.to_string()).unwrap_or_default(),
            (Some(s), 4) => s.date.clone(),
            (Some(s), 5) => s.event.clone(),
            (Some(s), 6) => s.result.clone(),
            (Some(s), 7) => s.moves_count.to_string(),
            _ => String::new(),
        };

        div().px_2().py_1().child(text)
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> gpui::Stateful<Div> {
        let library = self.library.clone();
        let game_model = self.game_model.clone();
        let selected = self.library.read(cx).selected_index() == Some(row_ix);
        let theme = cx.theme();

        div()
            .id(("pgn-row", row_ix))
            .cursor_pointer()
            .when(selected, |el| el.bg(theme.accent))
            .on_click(move |_ev, _window, cx| {
                // Select this game and load it
                library.update(cx, |lib, _| {
                    lib.set_selected(Some(row_ix));
                });

                // Load the game into the game model
                let game = library.read(cx).get_game(row_ix).cloned();
                if let Some(pgn_game) = game {
                    game_model.update(cx, |model, cx| {
                        model.load_pgn_game(&pgn_game);
                        cx.notify();
                    });
                }
            })
    }
}

/// Render the PGN panel with tabs
pub fn render_pgn_panel(
    model: &Entity<GameModel>,
    move_list_state: &Entity<MoveListState>,
    pgn_panel_state: &Entity<PgnPanelState>,
    pgn_library: &Entity<PgnLibraryModel>,
    table_state: &Entity<TableState<PgnTableDelegate>>,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let active_tab = pgn_panel_state.read(cx).active_tab;
    let selected_index = if active_tab == PgnPanelTab::Moves { 0 } else { 1 };

    let pgn_panel_state_clone = pgn_panel_state.clone();

    // Tab bar
    let tab_bar = TabBar::new("pgn-tabs")
        .selected_index(selected_index)
        .child(Tab::new().label("Moves"))
        .child(Tab::new().label("Library"))
        .on_click(move |ix, _window, cx| {
            pgn_panel_state_clone.update(cx, |state, cx| {
                state.active_tab = if *ix == 0 {
                    PgnPanelTab::Moves
                } else {
                    PgnPanelTab::Library
                };
                cx.notify();
            });
        });

    // Content based on active tab
    let content = match active_tab {
        PgnPanelTab::Moves => render_moves_tab(model, move_list_state, cx),
        PgnPanelTab::Library => render_library_tab(pgn_library, table_state, window, cx),
    };

    let theme = cx.theme();

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .p(px(BOARD_PADDING))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .bg(theme.secondary)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .overflow_hidden()
                // Tab bar header
                .child(
                    div()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(tab_bar),
                )
                // Tab content
                .child(content),
        )
}

/// Render the Moves tab content
fn render_moves_tab(
    model: &Entity<GameModel>,
    move_list_state: &Entity<MoveListState>,
    cx: &App,
) -> Div {
    render_move_list_content(model, move_list_state, cx)
}

/// Render the Library tab content
fn render_library_tab(
    pgn_library: &Entity<PgnLibraryModel>,
    table_state: &Entity<TableState<PgnTableDelegate>>,
    _window: &mut Window,
    cx: &mut App,
) -> Div {
    let theme = cx.theme();
    let games_count = pgn_library.read(cx).games_count();

    let library_for_picker = pgn_library.clone();

    // Load PGN button - opens system file picker
    let load_button = Button::new("load-pgn")
        .label("Load PGN")
        .icon(IconName::FolderOpen)
        .small()
        .on_click(move |_ev, _window, cx| {
            let library = library_for_picker.clone();
            let receiver = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Select PGN File".into()),
            });

            cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    if let Some(path) = paths.first() {
                        library
                            .update(cx, |lib, cx| {
                                match lib.load_from_file(path) {
                                    Ok(count) => {
                                        eprintln!("Loaded {} games from {:?}", count, path);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to load PGN: {}", e);
                                    }
                                }
                                cx.notify();
                            })
                            .ok();
                    }
                }
            })
            .detach();
        });

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .p_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!("{} games", games_count)),
        )
        .child(load_button);

    let table_content = if games_count == 0 {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.muted_foreground)
            .child("No games loaded. Click 'Load PGN' to load sample games.")
    } else {
        div()
            .flex_1()
            .overflow_hidden()
            .child(Table::new(table_state).stripe(true).small())
    };

    div()
        .flex_1()
        .flex()
        .flex_col()
        .child(header)
        .child(table_content)
}
