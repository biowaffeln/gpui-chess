//! Move list panel - displays the game's move history with variations.
//!
//! Uses a hybrid display: main line inline, variations as expandable sections.

use gpui::{AnyElement, App, Div, Entity, SharedString, div, prelude::*, px, rgb};

use crate::domain::MoveNodeId;
use crate::models::{GameModel, MainLineMoveDisplay, VariationDisplay};
use crate::ui::theme::{
    BOARD_PADDING, BORDER_COLOR, MOVE_LIST_BG, PANEL_BG, TEXT_PRIMARY, TEXT_SECONDARY,
};

// Colors for move highlighting
const MOVE_HOVER_BG: u32 = 0x3a3a3a;
const MOVE_SELECTED_BG: u32 = 0x4a6da7;
const NAV_BUTTON_BG: u32 = 0x3a3a3a;
const NAV_BUTTON_HOVER_BG: u32 = 0x4a4a4a;
const NAV_BUTTON_DISABLED: u32 = 0x555555;
const VARIATION_BG: u32 = 0x252525;
const VARIATION_BORDER: u32 = 0x3a3a3a;

/// Render the move list panel for a given game model.
/// Returns a Div element that can be used as a child.
pub fn render_move_list_panel(model: &Entity<GameModel>, cx: &App) -> Div {
    let game = model.read(cx);
    let main_line = game.main_line_display();
    let is_at_root = game.is_at_root();
    let is_at_leaf = game.is_at_leaf();
    let current_node_id = game.current_node_id();

    // Clone model for navigation closures
    let model_start = model.clone();
    let model_back = model.clone();
    let model_forward = model.clone();
    let model_end = model.clone();

    // Build the move content
    let moves_content = if main_line.is_empty() {
        div().text_color(rgb(TEXT_SECONDARY)).child("No moves yet")
    } else {
        render_main_line_with_variations(model, &main_line, current_node_id, game)
    };

    let move_list = div()
        .flex_1()
        .flex()
        .flex_col()
        .bg(rgb(MOVE_LIST_BG))
        .border_1()
        .border_color(rgb(BORDER_COLOR))
        .rounded_md()
        .overflow_hidden()
        // Header (fixed)
        .child(
            div()
                .p_4()
                .pb_2()
                .text_color(rgb(TEXT_PRIMARY))
                .border_b_1()
                .border_color(rgb(BORDER_COLOR))
                .child("Move History"),
        )
        // Scrollable moves content
        .child(
            div()
                .id("move-list-scroll")
                .flex_1()
                .overflow_y_scroll()
                .p_4()
                .pt_2()
                .child(moves_content),
        )
        // Navigation buttons at bottom
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .p_3()
                .border_t_1()
                .border_color(rgb(BORDER_COLOR))
                // Start button
                .child(render_nav_button("⟨⟨", !is_at_root, move |cx| {
                    model_start.update(cx, |game, cx| {
                        game.go_to_start();
                        cx.notify();
                    });
                }))
                // Back button
                .child(render_nav_button("⟨", !is_at_root, move |cx| {
                    model_back.update(cx, |game, cx| {
                        game.go_back();
                        cx.notify();
                    });
                }))
                // Forward button
                .child(render_nav_button("⟩", !is_at_leaf, move |cx| {
                    model_forward.update(cx, |game, cx| {
                        game.go_forward();
                        cx.notify();
                    });
                }))
                // End button
                .child(render_nav_button("⟩⟩", !is_at_leaf, move |cx| {
                    model_end.update(cx, |game, cx| {
                        game.go_to_end();
                        cx.notify();
                    });
                })),
        );

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(PANEL_BG))
        .p(px(BOARD_PADDING))
        .child(move_list)
}

/// Render the main line with inline variations
/// Uses a column layout where main line moves flow inline and variations are block-level
fn render_main_line_with_variations(
    model: &Entity<GameModel>,
    main_line: &[MainLineMoveDisplay],
    current_node_id: MoveNodeId,
    game: &GameModel,
) -> Div {
    // Build segments: each segment is either inline moves or a variation block
    let mut segments: Vec<AnyElement> = Vec::new();
    let mut current_inline_moves: Vec<AnyElement> = Vec::new();

    for mv in main_line {
        let model_move = model.clone();
        let node_id = mv.node_id;
        let is_selected = node_id == current_node_id;

        // Add move number for white moves
        if !mv.is_black {
            current_inline_moves.push(
                div()
                    .text_color(rgb(TEXT_SECONDARY))
                    .child(format!("{}.", mv.move_num))
                    .into_any_element(),
            );
        }

        // The move itself
        current_inline_moves.push(
            render_clickable_move_node(
                node_id,
                mv.san.clone(),
                is_selected,
                mv.is_check,
                mv.is_checkmate,
                model_move,
            )
            .into_any_element(),
        );

        // If this move has sibling variations, add collapse button and conditionally render variations
        if mv.has_sibling_variations {
            let model_collapse = model.clone();
            let is_collapsed = game.is_variation_collapsed(node_id);

            // Add collapse button after the move
            current_inline_moves.push(
                render_collapse_button(node_id, is_collapsed, model_collapse).into_any_element(),
            );

            // Only flush and render variation block if expanded
            if !is_collapsed {
                // Flush current inline moves as a row
                if !current_inline_moves.is_empty() {
                    segments.push(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .children(current_inline_moves.drain(..).collect::<Vec<_>>())
                            .into_any_element(),
                    );
                }

                let variations = game.get_sibling_variations(node_id);
                if !variations.is_empty() {
                    segments.push(
                        render_variations_block(model, &variations, current_node_id, game)
                            .into_any_element(),
                    );
                }
            }
            // When collapsed, just keep adding to current_inline_moves (stays inline)
        }
    }

    // Flush any remaining inline moves
    if !current_inline_moves.is_empty() {
        segments.push(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .children(current_inline_moves)
                .into_any_element(),
        );
    }

    div().flex().flex_col().gap_1().children(segments)
}

/// Render a block of variations
fn render_variations_block(
    model: &Entity<GameModel>,
    variations: &[VariationDisplay],
    current_node_id: MoveNodeId,
    game: &GameModel,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .mt_1()
        .mb_1()
        .children(
            variations
                .iter()
                .map(|var| render_variation_line(model, var, current_node_id, game)),
        )
}

/// Render a single variation line
fn render_variation_line(
    model: &Entity<GameModel>,
    variation: &VariationDisplay,
    current_node_id: MoveNodeId,
    game: &GameModel,
) -> Div {
    // Build the content with proper segmentation for sub-variations
    let mut segments: Vec<AnyElement> = Vec::new();
    let mut current_inline: Vec<AnyElement> = Vec::new();

    for (i, mv) in variation.moves.iter().enumerate() {
        let model_move = model.clone();
        let node_id = mv.node_id;
        let is_selected = node_id == current_node_id;

        // Show move number for first move or white moves
        if i == 0 || !mv.is_black {
            let num_display = if mv.is_black {
                format!("{}...", mv.move_num)
            } else {
                format!("{}.", mv.move_num)
            };
            current_inline.push(
                div()
                    .text_color(rgb(TEXT_SECONDARY))
                    .child(num_display)
                    .into_any_element(),
            );
        }

        current_inline.push(
            render_clickable_move_node(
                node_id,
                mv.san.clone(),
                is_selected,
                mv.is_check,
                mv.is_checkmate,
                model_move,
            )
            .into_any_element(),
        );

        // Check for sibling sub-variations (alternatives to this move)
        if mv.has_sibling_sub_variations {
            let model_collapse = model.clone();
            let is_collapsed = game.is_variation_collapsed(node_id);

            // Add collapse button after the move
            current_inline.push(
                render_collapse_button(node_id, is_collapsed, model_collapse).into_any_element(),
            );

            // Only flush and render sub-variation block if expanded
            if !is_collapsed {
                // Flush inline moves
                if !current_inline.is_empty() {
                    segments.push(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .children(current_inline.drain(..).collect::<Vec<_>>())
                            .into_any_element(),
                    );
                }

                let sub_vars = game.get_sibling_sub_variations(node_id);
                if !sub_vars.is_empty() {
                    segments.push(
                        render_variations_block(model, &sub_vars, current_node_id, game)
                            .into_any_element(),
                    );
                }
            }
            // When collapsed, just keep adding to current_inline (stays inline)
        }
    }

    // Flush remaining
    if !current_inline.is_empty() {
        segments.push(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .children(current_inline)
                .into_any_element(),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_2()
        .py_1()
        .bg(rgb(VARIATION_BG))
        .border_l_2()
        .border_color(rgb(VARIATION_BORDER))
        .rounded_sm()
        .children(segments)
}

/// Render a clickable move that navigates to a specific node
fn render_clickable_move_node(
    node_id: MoveNodeId,
    san: String,
    is_selected: bool,
    is_check: bool,
    is_checkmate: bool,
    model: Entity<GameModel>,
) -> impl IntoElement {
    // Build the display text with check/checkmate symbols
    let mut display_text = san;
    if is_checkmate {
        display_text.push_str("#");
    } else if is_check {
        display_text.push_str("+");
    }

    div()
        .id(SharedString::from(format!("move-node-{}", node_id)))
        .px_1()
        .rounded(px(3.0))
        .cursor_pointer()
        .text_color(rgb(TEXT_PRIMARY))
        .when(is_selected, |el| el.bg(rgb(MOVE_SELECTED_BG)))
        .when(!is_selected, |el| el.hover(|s| s.bg(rgb(MOVE_HOVER_BG))))
        .on_click(move |_ev, _window, cx| {
            model.update(cx, |game, cx| {
                game.go_to_node(node_id);
                cx.notify();
            });
        })
        .child(display_text)
}

/// Render a collapse/expand button for variations
fn render_collapse_button(
    node_id: MoveNodeId,
    is_collapsed: bool,
    model: Entity<GameModel>,
) -> impl IntoElement {
    let symbol = if is_collapsed { "+" } else { "-" };
    div()
        .id(SharedString::from(format!("collapse-{}", node_id)))
        .px_1()
        .rounded(px(3.0))
        .cursor_pointer()
        .text_color(rgb(TEXT_SECONDARY))
        .hover(|s| s.bg(rgb(MOVE_HOVER_BG)))
        .on_click(move |_ev, _window, cx| {
            model.update(cx, |game, cx| {
                game.toggle_variation_collapse(node_id);
                cx.notify();
            });
        })
        .child(symbol)
}

/// Render a navigation button (back/forward)
fn render_nav_button(
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("nav-{}", label)))
        .px_4()
        .py_2()
        .rounded(px(4.0))
        .text_color(if enabled {
            rgb(TEXT_PRIMARY)
        } else {
            rgb(NAV_BUTTON_DISABLED)
        })
        .font_weight(gpui::FontWeight::BOLD)
        .when(enabled, |el| {
            el.bg(rgb(NAV_BUTTON_BG))
                .cursor_pointer()
                .hover(|s| s.bg(rgb(NAV_BUTTON_HOVER_BG)))
                .on_click(move |_ev, _window, cx| {
                    on_click(cx);
                })
        })
        .when(!enabled, |el| el.bg(rgb(0x2a2a2a)))
        .child(label)
}
