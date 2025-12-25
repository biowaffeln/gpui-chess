//! Move list panel - displays the game's move history.

use gpui::{App, Div, Entity, SharedString, div, prelude::*, px, rgb};

use crate::models::GameModel;
use crate::ui::theme::{
    BOARD_PADDING, BORDER_COLOR, MOVE_LIST_BG, PANEL_BG, TEXT_PRIMARY, TEXT_SECONDARY,
};

// Colors for move highlighting
const MOVE_HOVER_BG: u32 = 0x3a3a3a;
const MOVE_SELECTED_BG: u32 = 0x4a6da7;
const NAV_BUTTON_BG: u32 = 0x3a3a3a;
const NAV_BUTTON_HOVER_BG: u32 = 0x4a4a4a;
const NAV_BUTTON_DISABLED: u32 = 0x555555;

/// Render the move list panel for a given game model.
/// Returns a Div element that can be used as a child.
pub fn render_move_list_panel(model: &Entity<GameModel>, cx: &App) -> Div {
    let game = model.read(cx);
    let move_pairs = game.move_pairs();
    let is_at_start = game.is_at_start();
    let is_at_latest = game.is_at_latest();

    // Clone model for each closure
    let model_start = model.clone();
    let model_back = model.clone();
    let model_forward = model.clone();
    let model_end = model.clone();

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
                .text_sm()
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
                .flex()
                .flex_col()
                .gap_1()
                .when(move_pairs.is_empty(), |el| {
                    el.child(
                        div()
                            .text_color(rgb(TEXT_SECONDARY))
                            .text_sm()
                            .child("No moves yet"),
                    )
                })
                .children(move_pairs.into_iter().map({
                    let model = model.clone();
                    move |pair| {
                        let model_white = model.clone();
                        let model_black = model.clone();
                        let white_idx = pair.white_move_idx;
                        let white_selected = game.is_move_selected(white_idx);

                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .py_1()
                            // Move number
                            .child(
                                div()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .text_sm()
                                    .w(px(28.0))
                                    .flex_shrink_0()
                                    .child(format!("{}.", pair.move_num)),
                            )
                            // White move (clickable)
                            .child(render_clickable_move(
                                pair.white_san.clone(),
                                white_selected,
                                move |cx| {
                                    model_white.update(cx, |game, cx| {
                                        game.go_to_move(Some(white_idx));
                                        cx.notify();
                                    });
                                },
                            ))
                            // Black move (clickable, if exists)
                            .when_some(pair.black_move, move |el, (black_idx, black_san)| {
                                let black_selected = game.is_move_selected(black_idx);
                                el.child(render_clickable_move(
                                    black_san,
                                    black_selected,
                                    move |cx| {
                                        model_black.update(cx, |game, cx| {
                                            game.go_to_move(Some(black_idx));
                                            cx.notify();
                                        });
                                    },
                                ))
                            })
                    }
                })),
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
                .child(render_nav_button(
                    "⟨⟨",
                    !is_at_start,
                    move |cx| {
                        model_start.update(cx, |game, cx| {
                            game.go_to_start();
                            cx.notify();
                        });
                    },
                ))
                // Back button
                .child(render_nav_button(
                    "⟨",
                    !is_at_start,
                    move |cx| {
                        model_back.update(cx, |game, cx| {
                            game.go_back();
                            cx.notify();
                        });
                    },
                ))
                // Forward button
                .child(render_nav_button(
                    "⟩",
                    !is_at_latest,
                    move |cx| {
                        model_forward.update(cx, |game, cx| {
                            game.go_forward();
                            cx.notify();
                        });
                    },
                ))
                // End button
                .child(render_nav_button(
                    "⟩⟩",
                    !is_at_latest,
                    move |cx| {
                        model_end.update(cx, |game, cx| {
                            game.go_to_end();
                            cx.notify();
                        });
                    },
                )),
        );

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(PANEL_BG))
        .p(px(BOARD_PADDING))
        .child(move_list)
}

/// Render a clickable move button
fn render_clickable_move(
    san: String,
    is_selected: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("move-{}", san)))
        .px_2()
        .py_1()
        .rounded(px(4.0))
        .cursor_pointer()
        .text_color(rgb(TEXT_PRIMARY))
        .text_sm()
        .flex_1()
        .when(is_selected, |el| el.bg(rgb(MOVE_SELECTED_BG)))
        .when(!is_selected, |el| el.hover(|s| s.bg(rgb(MOVE_HOVER_BG))))
        .on_click(move |_ev, _window, cx| {
            on_click(cx);
        })
        .child(san)
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
        .text_sm()
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
