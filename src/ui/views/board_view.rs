//! Chess board view - the main board with drag-and-drop piece movement.

use gpui::{
    Context, Entity, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Subscription, Window, canvas, div, img, prelude::*, px, rgb,
};
use gpui_component::resizable::{h_resizable, resizable_panel};

use crate::models::{DragState, GameModel};
use crate::ui::components::render_square;
use crate::ui::theme::{BOARD_PADDING, INITIAL_LEFT_PANEL, INITIAL_RIGHT_PANEL, PANEL_BG};
use crate::ui::views::render_move_list_panel;

/// The main chess board view that observes a GameModel
pub struct ChessBoardView {
    model: Entity<GameModel>,
    _subscription: Subscription,
}

impl ChessBoardView {
    pub fn new(model: Entity<GameModel>, cx: &mut Context<Self>) -> Self {
        let _subscription = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            _subscription,
        }
    }
}

impl Render for ChessBoardView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.clone();
        let model_down = model.clone();
        let model_move = model.clone();
        let model_up = model.clone();
        let model_measure = model.clone();

        let game = self.model.read(cx);
        let drag_state = game.drag_state;
        let dragging_from = drag_state.map(|d| (d.from_row, d.from_col));

        // Sizing based on measured panel dimensions
        let square_size = game.square_size();
        let piece_size = game.piece_size();

        // Floating piece follows cursor during drag
        let floating_piece = drag_state.map(|d| {
            div()
                .absolute()
                .left(px(d.mouse_x - piece_size / 2.0))
                .top(px(d.mouse_y - piece_size / 2.0))
                .size(px(piece_size))
                .child(img(d.piece.svg_path()).size(px(piece_size)))
        });

        // Board element with fixed size - always maintains 1:1 aspect ratio
        let board_total_size = square_size * 8.0;

        // Collect pieces for rendering (can't borrow game in closure)
        let pieces: Vec<_> = (0..8)
            .flat_map(|row| {
                (0..8).map(move |col| {
                    let piece = game.piece_at(row, col);
                    let is_being_dragged = dragging_from == Some((row, col));
                    (row, col, piece, is_being_dragged)
                })
            })
            .collect();

        let board = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .w(px(board_total_size))
            .h(px(board_total_size))
            .overflow_hidden()
            .rounded_md()
            .children((0..8).map(|row| {
                div().flex().flex_shrink_0().children((0..8).map(|col| {
                    let idx = row * 8 + col;
                    let (_, _, piece, is_being_dragged) = pieces[idx];
                    render_square(row, col, piece, is_being_dragged, square_size, piece_size)
                }))
            }));

        let board_panel_content = div()
            .id("board-panel")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(PANEL_BG))
            .p(px(BOARD_PADDING))
            .child(board)
            .when_some(floating_piece, |el, fp| el.child(fp))
            // Mouse down: start drag if clicking on a piece
            .on_mouse_down(
                MouseButton::Left,
                move |ev: &MouseDownEvent, _window, cx| {
                    model_down.update(cx, |game, cx| {
                        let pos = ev.position;
                        if let Some((row, col)) = game.pos_to_square(pos.x.into(), pos.y.into()) {
                            if let Some(piece) = game.piece_at(row, col) {
                                if piece.color == game.current_turn() {
                                    game.drag_state = Some(DragState {
                                        piece,
                                        from_row: row,
                                        from_col: col,
                                        mouse_x: pos.x.into(),
                                        mouse_y: pos.y.into(),
                                    });
                                    cx.notify();
                                }
                            }
                        }
                    });
                },
            )
            // Mouse move: update drag position
            .on_mouse_move(move |ev: &MouseMoveEvent, _, cx| {
                model_move.update(cx, |game, cx| {
                    if let Some(ref mut drag) = game.drag_state {
                        drag.mouse_x = ev.position.x.into();
                        drag.mouse_y = ev.position.y.into();
                        cx.notify();
                    }
                });
            })
            // Mouse up: complete the move
            .on_mouse_up(MouseButton::Left, move |ev: &MouseUpEvent, _window, cx| {
                model_up.update(cx, |game, cx| {
                    if let Some(drag) = game.drag_state.take() {
                        let pos = ev.position;
                        if let Some((to_row, to_col)) =
                            game.pos_to_square(pos.x.into(), pos.y.into())
                        {
                            game.try_move((drag.from_row, drag.from_col), (to_row, to_col));
                        }
                        cx.notify();
                    }
                });
            });

        // Canvas to measure actual panel size
        let measure_canvas = canvas(
            move |bounds, _window, cx| {
                model_measure.update(cx, |game, cx| {
                    if game.panel_size != bounds.size {
                        game.panel_size = bounds.size;
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        // Wrap board panel content with measuring canvas
        let board_panel_with_measure = div()
            .relative()
            .size_full()
            .child(measure_canvas)
            .child(board_panel_content);

        // Move list panel
        let move_list_panel_content = render_move_list_panel(&model, cx);

        // Main resizable layout
        div().size_full().font_family("Berkeley Mono").child(
            h_resizable("chess-layout")
                .child(
                    resizable_panel()
                        .size(px(INITIAL_LEFT_PANEL))
                        .size_range(px(320.)..px(1200.))
                        .child(board_panel_with_measure),
                )
                .child(
                    resizable_panel()
                        .size(px(INITIAL_RIGHT_PANEL))
                        .size_range(px(150.)..Pixels::MAX)
                        .child(move_list_panel_content),
                ),
        )
    }
}
