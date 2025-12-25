//! Move list panel - displays the game's move history.

use gpui::{App, Div, Entity, div, prelude::*, px, rgb};

use crate::models::GameModel;
use crate::ui::theme::{
    BOARD_PADDING, BORDER_COLOR, MOVE_LIST_BG, PANEL_BG, TEXT_PRIMARY, TEXT_SECONDARY,
};

/// Render the move list panel for a given game model.
/// Returns a Div element that can be used as a child.
pub fn render_move_list_panel(model: &Entity<GameModel>, cx: &App) -> Div {
    let game = model.read(cx);
    let move_pairs = game.move_pairs();

    let move_list = div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(MOVE_LIST_BG))
        .border_1()
        .border_color(rgb(BORDER_COLOR))
        .rounded_md()
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
                .children(
                    move_pairs
                        .into_iter()
                        .map(|(move_num, white_move, black_move)| {
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .py_1()
                                .child(
                                    div()
                                        .text_color(rgb(TEXT_SECONDARY))
                                        .text_sm()
                                        .w(px(40.0))
                                        .child(format!("{}.", move_num)),
                                )
                                .child(
                                    div()
                                        .text_color(rgb(TEXT_PRIMARY))
                                        .text_sm()
                                        .flex_1()
                                        .child(white_move),
                                )
                                .when_some(black_move, |el, bm| {
                                    el.child(
                                        div()
                                            .text_color(rgb(TEXT_PRIMARY))
                                            .text_sm()
                                            .flex_1()
                                            .child(bm),
                                    )
                                })
                        }),
                ),
        );

    div()
        .size_full()
        .bg(rgb(PANEL_BG))
        .p(px(BOARD_PADDING))
        .child(move_list)
}
