//! Move list panel - displays the game's move history with variations.
//!
//! Uses a hybrid display: main line inline, variations as expandable sections.

use std::mem;

use gpui::{AnyElement, App, Div, Entity, Hsla, SharedString, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::ContextMenuExt;
use gpui_component::{ActiveTheme, Disableable};

use super::board_view::MoveListState;
use super::{
    DeleteMove, MoveBack, MoveForward, MoveToEnd, MoveToStart, PromoteToMainLine, PromoteVariation,
};
use crate::domain::MoveNodeId;
use crate::models::GameModel;
use crate::ui::display::{get_sibling_sub_variations, get_sibling_variations, main_line_display};
use crate::ui::icons::ChessIcon;
use crate::ui::theme::BOARD_PADDING;
use crate::ui::view_models::{MainLineMoveDisplay, VariationDisplay};

/// Theme colors for the move list, extracted from gpui-component theme
#[derive(Clone, Copy)]
struct MoveListColors {
    bg: Hsla,
    secondary_bg: Hsla,
    border: Hsla,
    fg: Hsla,
    muted_fg: Hsla,
    primary: Hsla,
    primary_fg: Hsla,
    accent: Hsla,
    muted: Hsla,
}

/// Render the move list panel for a given game model (standalone, with full container).
/// Returns a Div element that can be used as a child.
pub fn render_move_list_panel(
    model: &Entity<GameModel>,
    move_list_state: &Entity<MoveListState>,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let colors = extract_colors(cx);

    let move_list = div()
        .flex_1()
        .flex()
        .flex_col()
        .bg(colors.secondary_bg)
        .border_1()
        .border_color(colors.border)
        .rounded_md()
        // Header (fixed)
        .child(
            div()
                .p_4()
                .pb_2()
                .text_color(colors.fg)
                .border_b_1()
                .border_color(colors.border)
                .child("Move History"),
        )
        // Content (scrollable moves + nav buttons)
        .child(render_move_list_content(model, move_list_state, cx));

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .p(px(BOARD_PADDING))
        .child(move_list)
}

/// Render just the move list content (scrollable moves + nav buttons).
/// Used by the tabbed PGN panel.
pub fn render_move_list_content(
    model: &Entity<GameModel>,
    move_list_state: &Entity<MoveListState>,
    cx: &App,
) -> Div {
    let game = model.read(cx);
    let main_line = main_line_display(game);
    let is_at_root = game.is_at_root();
    let is_at_leaf = game.is_at_leaf();
    let current_node_id = game.current_node_id();

    let collapsed_variations = &move_list_state.read(cx).collapsed_variations;
    let colors = extract_colors(cx);

    // Build the move content
    let moves_content = if main_line.is_empty() {
        div().text_color(colors.muted_fg).child("No moves yet")
    } else {
        render_main_line_with_variations(
            model,
            move_list_state,
            &main_line,
            current_node_id,
            game,
            collapsed_variations,
            colors,
        )
    };

    div()
        .flex_1()
        .flex()
        .flex_col()
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
                .border_color(colors.border)
                // Start button
                .child(render_nav_button(
                    ChessIcon::CaretDoubleLeft,
                    !is_at_root,
                    |window, cx| window.dispatch_action(Box::new(MoveToStart), cx),
                ))
                // Back button
                .child(render_nav_button(
                    ChessIcon::CaretLeft,
                    !is_at_root,
                    |window, cx| window.dispatch_action(Box::new(MoveBack), cx),
                ))
                // Forward button
                .child(render_nav_button(
                    ChessIcon::CaretRight,
                    !is_at_leaf,
                    |window, cx| window.dispatch_action(Box::new(MoveForward), cx),
                ))
                // End button
                .child(render_nav_button(
                    ChessIcon::CaretDoubleRight,
                    !is_at_leaf,
                    |window, cx| window.dispatch_action(Box::new(MoveToEnd), cx),
                )),
        )
}

/// Extract theme colors for move list rendering
fn extract_colors(cx: &App) -> MoveListColors {
    let theme = cx.theme();
    MoveListColors {
        bg: theme.background,
        secondary_bg: theme.secondary,
        border: theme.border,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        primary: theme.primary,
        primary_fg: theme.primary_foreground,
        accent: theme.accent,
        muted: theme.muted,
    }
}

/// Render the main line with inline variations
/// Uses a column layout where main line moves flow inline and variations are block-level
fn render_main_line_with_variations(
    model: &Entity<GameModel>,
    move_list_state: &Entity<MoveListState>,
    main_line: &[MainLineMoveDisplay],
    current_node_id: MoveNodeId,
    game: &GameModel,
    collapsed_variations: &std::collections::HashSet<MoveNodeId>,
    colors: MoveListColors,
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
                    .text_color(colors.muted_fg)
                    .child(format!("{}.", mv.move_num))
                    .into_any_element(),
            );
        }

        // The move itself (main line = variation_depth 0)
        current_inline_moves.push(
            render_clickable_move_node(
                node_id,
                mv.san.clone(),
                is_selected,
                mv.is_check,
                mv.is_checkmate,
                model_move,
                0, // main line
                colors,
            )
            .into_any_element(),
        );

        // If this move has sibling variations, add collapse button and conditionally render variations
        if mv.has_sibling_variations {
            let is_collapsed = collapsed_variations.contains(&node_id);

            // Add collapse button after the move
            current_inline_moves.push(
                render_collapse_button(node_id, is_collapsed, move_list_state.clone(), colors)
                    .into_any_element(),
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
                            .children(mem::take(&mut current_inline_moves))
                            .into_any_element(),
                    );
                }

                let variations = get_sibling_variations(game, node_id);
                if !variations.is_empty() {
                    segments.push(
                        render_variations_block(
                            model,
                            move_list_state,
                            &variations,
                            current_node_id,
                            game,
                            collapsed_variations,
                            1, // first level of variation
                            colors,
                        )
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
    move_list_state: &Entity<MoveListState>,
    variations: &[VariationDisplay],
    current_node_id: MoveNodeId,
    game: &GameModel,
    collapsed_variations: &std::collections::HashSet<MoveNodeId>,
    variation_depth: usize,
    colors: MoveListColors,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .mt_1()
        .mb_1()
        .children(variations.iter().map(|var| {
            render_variation_line(
                model,
                move_list_state,
                var,
                current_node_id,
                game,
                collapsed_variations,
                variation_depth,
                colors,
            )
        }))
}

/// Render a single variation line
#[allow(clippy::too_many_arguments)]
fn render_variation_line(
    model: &Entity<GameModel>,
    move_list_state: &Entity<MoveListState>,
    variation: &VariationDisplay,
    current_node_id: MoveNodeId,
    game: &GameModel,
    collapsed_variations: &std::collections::HashSet<MoveNodeId>,
    variation_depth: usize,
    colors: MoveListColors,
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
                    .text_color(colors.muted_fg)
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
                variation_depth,
                colors,
            )
            .into_any_element(),
        );

        // Check for sibling sub-variations (alternatives to this move)
        if mv.has_sibling_sub_variations {
            let is_collapsed = collapsed_variations.contains(&node_id);

            // Add collapse button after the move
            current_inline.push(
                render_collapse_button(node_id, is_collapsed, move_list_state.clone(), colors)
                    .into_any_element(),
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
                            .children(mem::take(&mut current_inline))
                            .into_any_element(),
                    );
                }

                let sub_vars = get_sibling_sub_variations(game, node_id);
                if !sub_vars.is_empty() {
                    segments.push(
                        render_variations_block(
                            model,
                            move_list_state,
                            &sub_vars,
                            current_node_id,
                            game,
                            collapsed_variations,
                            variation_depth + 1, // nested deeper
                            colors,
                        )
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
        .px_3()
        .py_1()
        .bg(colors.muted)
        .border_l_2()
        .border_color(colors.border)
        .rounded_r_sm()
        .children(segments)
}

/// Render a clickable move that navigates to a specific node
/// variation_depth: 0 = main line, 1+ = inside a variation
#[allow(clippy::too_many_arguments)]
fn render_clickable_move_node(
    node_id: MoveNodeId,
    san: String,
    is_selected: bool,
    is_check: bool,
    is_checkmate: bool,
    model: Entity<GameModel>,
    variation_depth: usize,
    colors: MoveListColors,
) -> impl IntoElement {
    // SAN already includes +/# suffix (from SanPlus), just use as-is
    let display_text = san;
    let _ = (is_check, is_checkmate); // Silence unused warnings

    div()
        .id(SharedString::from(format!("move-node-{node_id}")))
        .px_1()
        .rounded(px(3.0))
        .cursor_pointer()
        .overflow_hidden()
        .when(is_selected, |el| {
            el.bg(colors.primary).text_color(colors.primary_fg)
        })
        .when(!is_selected, |el| {
            el.text_color(colors.fg).hover(|s| s.bg(colors.accent))
        })
        .on_click({
            let model = model.clone();
            move |_ev, _window, cx| {
                model.update(cx, |game, cx| {
                    game.go_to_node(node_id);
                    cx.notify();
                });
            }
        })
        .context_menu(move |menu, _window, _cx| {
            let mut menu = menu.menu("Delete Move", Box::new(DeleteMove { node_id }));

            if variation_depth >= 1 {
                menu = menu
                    .separator()
                    .menu("Promote Variation", Box::new(PromoteVariation { node_id }));
            }

            if variation_depth >= 2 {
                menu = menu.menu(
                    "Promote to Main Line",
                    Box::new(PromoteToMainLine { node_id }),
                );
            }

            menu
        })
        .child(display_text)
}

/// Render a collapse/expand button for variations
fn render_collapse_button(
    node_id: MoveNodeId,
    is_collapsed: bool,
    move_list_state: Entity<MoveListState>,
    colors: MoveListColors,
) -> impl IntoElement {
    let symbol = if is_collapsed { "+" } else { "-" };
    div()
        .id(SharedString::from(format!("collapse-{node_id}")))
        .px_1()
        .rounded(px(3.0))
        .cursor_pointer()
        .text_color(colors.muted_fg)
        .hover(|s| s.bg(colors.accent))
        .on_click(move |_ev, _window, cx| {
            move_list_state.update(cx, |state, cx| {
                state.toggle_variation(node_id);
                cx.notify();
            });
        })
        .child(symbol)
}

/// Render a navigation button (back/forward) using gpui-component Button
fn render_nav_button(
    icon: ChessIcon,
    enabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    if enabled {
        Button::new(SharedString::from(format!("nav-{:?}", icon)))
            .icon(icon)
            .ghost()
            .on_click(move |_ev, window, cx| {
                on_click(window, cx);
            })
    } else {
        Button::new(SharedString::from(format!("nav-{:?}", icon)))
            .icon(icon)
            .ghost()
            .disabled(true)
    }
}
