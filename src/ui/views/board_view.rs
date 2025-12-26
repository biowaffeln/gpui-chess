//! Chess board view - the main board with drag-and-drop piece movement.

use gpui::{
    Context, Entity, FocusHandle, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Subscription, Window, actions, canvas, div, img, prelude::*, px, rgb,
};
use gpui_component::resizable::{h_resizable, resizable_panel};
use std::collections::HashSet;

use crate::domain::MoveNodeId;
use crate::models::GameModel;
use crate::ui::BoardLayout;
use crate::ui::assets::piece_svg_path;
use crate::ui::theme::{
    BOARD_CORNER_RADIUS, BOARD_PADDING, GHOST_OPACITY, INITIAL_LEFT_PANEL, PANEL_BG,
};
use crate::ui::view_models::DragState;
use crate::ui::views::render_move_list_panel;

// Define navigation actions
actions!(chess, [MoveBack, MoveForward, MoveToStart, MoveToEnd]);

/// UI state for the board view (not part of game model)
pub struct BoardViewState {
    pub drag_state: Option<DragState>,
}

impl BoardViewState {
    pub fn new() -> Self {
        Self { drag_state: None }
    }
}

/// Board layout state (entity so canvas can update it)
pub struct BoardLayoutState {
    pub layout: BoardLayout,
}

impl BoardLayoutState {
    pub fn new() -> Self {
        Self {
            layout: BoardLayout::default(),
        }
    }
}

/// UI state model for move list (entity so it can be shared and updated)
pub struct MoveListState {
    pub collapsed_variations: HashSet<MoveNodeId>,
}

impl MoveListState {
    pub fn new() -> Self {
        Self {
            collapsed_variations: HashSet::new(),
        }
    }

    pub fn toggle_variation(&mut self, node_id: MoveNodeId) {
        if self.collapsed_variations.contains(&node_id) {
            self.collapsed_variations.remove(&node_id);
        } else {
            self.collapsed_variations.insert(node_id);
        }
    }
}

/// The main chess board view that observes a GameModel
pub struct ChessBoardView {
    model: Entity<GameModel>,
    pub view_state: BoardViewState,
    layout_state: Entity<BoardLayoutState>,
    move_list_state: Entity<MoveListState>,
    focus_handle: FocusHandle,
    _subscription: Subscription,
    _layout_subscription: Subscription,
    _move_list_subscription: Subscription,
}

impl ChessBoardView {
    pub fn new(model: Entity<GameModel>, cx: &mut Context<Self>) -> Self {
        let _subscription = cx.observe(&model, |_, _, cx| cx.notify());
        let layout_state = cx.new(|_| BoardLayoutState::new());
        let _layout_subscription = cx.observe(&layout_state, |_, _, cx| cx.notify());
        let move_list_state = cx.new(|_| MoveListState::new());
        let _move_list_subscription = cx.observe(&move_list_state, |_, _, cx| cx.notify());
        Self {
            model,
            view_state: BoardViewState::new(),
            layout_state,
            move_list_state,
            focus_handle: cx.focus_handle(),
            _subscription,
            _layout_subscription,
            _move_list_subscription,
        }
    }
}

impl Render for ChessBoardView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.clone();

        let game = self.model.read(cx);
        let drag_state = self.view_state.drag_state;
        let dragging_from = drag_state.map(|d| (d.from_row, d.from_col));

        // Sizing based on measured panel dimensions
        let layout = self.layout_state.read(cx).layout;
        let square_size = layout.square_size();
        let piece_size = layout.piece_size();

        // Floating piece follows cursor during drag
        let floating_piece = drag_state.map(|d| {
            div()
                .absolute()
                .left(px(d.mouse_x - piece_size / 2.0))
                .top(px(d.mouse_y - piece_size / 2.0))
                .size(px(piece_size))
                .child(img(piece_svg_path(&d.piece)).size(px(piece_size)))
        });

        // Board element with fixed size - always maintains 1:1 aspect ratio
        let board_total_size = layout.board_total_size();

        // Collect only pieces that exist with their positions
        let pieces: Vec<_> = (0..8)
            .flat_map(|row| {
                (0..8).filter_map(move |col| {
                    game.piece_at(row, col).map(|piece| {
                        let is_being_dragged = dragging_from == Some((row, col));
                        (row, col, piece, is_being_dragged)
                    })
                })
            })
            .collect();

        let radius = px(BOARD_CORNER_RADIUS);

        // Board background image
        let board_bg = img("assets/maple.jpg")
            .absolute()
            .top_0()
            .left_0()
            .size(px(board_total_size))
            .rounded(radius);

        // Pieces absolutely positioned on the board
        let piece_offset = layout.piece_offset();
        let piece_elements: Vec<_> = pieces
            .into_iter()
            .map(|(row, col, piece, is_being_dragged)| {
                let x = col as f32 * square_size + piece_offset;
                let y = row as f32 * square_size + piece_offset;
                img(piece_svg_path(&piece))
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .size(px(piece_size))
                    .when(is_being_dragged, |el| el.opacity(GHOST_OPACITY))
            })
            .collect();

        // Combined board with background + pieces
        let board = div()
            .relative()
            .flex_shrink_0()
            .w(px(board_total_size))
            .h(px(board_total_size))
            .child(board_bg)
            .children(piece_elements);

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
                cx.listener(|view, ev: &MouseDownEvent, _window, cx| {
                    let pos = ev.position;
                    let game = view.model.read(cx);
                    let layout = view.layout_state.read(cx).layout;

                    if let Some((row, col)) = layout.pos_to_square(pos.x.into(), pos.y.into()) {
                        if let Some(piece) = game.piece_at(row, col) {
                            if piece.color == game.current_turn() {
                                view.view_state.drag_state = Some(DragState {
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
                }),
            )
            // Mouse move: update drag position
            .on_mouse_move(cx.listener(|view, ev: &MouseMoveEvent, _window, cx| {
                if let Some(ref mut drag) = view.view_state.drag_state {
                    drag.mouse_x = ev.position.x.into();
                    drag.mouse_y = ev.position.y.into();
                    cx.notify();
                }
            }))
            // Mouse up: complete the move
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, ev: &MouseUpEvent, _window, cx| {
                    let pos = ev.position;

                    if let Some(drag) = view.view_state.drag_state.take() {
                        let layout = view.layout_state.read(cx).layout;
                        if let Some((to_row, to_col)) =
                            layout.pos_to_square(pos.x.into(), pos.y.into())
                        {
                            view.model.update(cx, |game, _cx| {
                                game.try_move((drag.from_row, drag.from_col), (to_row, to_col));
                            });
                        }
                        cx.notify();
                    }
                }),
            );

        // Canvas to measure actual panel size
        let layout_state = self.layout_state.clone();
        let measure_canvas = canvas(
            move |bounds, _window, cx| {
                layout_state.update(cx, |state, cx| {
                    if state.layout.panel_size != bounds.size {
                        state.layout = BoardLayout::new(bounds.size);
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
        let move_list_panel_content = render_move_list_panel(&model, &self.move_list_state, cx);

        // Clone model for each action handler
        let model_back = model.clone();
        let model_forward = model.clone();
        let model_start = model.clone();
        let model_end = model.clone();

        // Main resizable layout
        div()
            .size_full()
            .font_family("Berkeley Mono")
            .track_focus(&self.focus_handle)
            .on_action(move |_: &MoveBack, _window, cx| {
                model_back.update(cx, |game, cx| {
                    game.go_back();
                    cx.notify();
                });
            })
            .on_action(move |_: &MoveForward, _window, cx| {
                model_forward.update(cx, |game, cx| {
                    game.go_forward();
                    cx.notify();
                });
            })
            .on_action(move |_: &MoveToStart, _window, cx| {
                model_start.update(cx, |game, cx| {
                    game.go_to_start();
                    cx.notify();
                });
            })
            .on_action(move |_: &MoveToEnd, _window, cx| {
                model_end.update(cx, |game, cx| {
                    game.go_to_end();
                    cx.notify();
                });
            })
            .child(
                h_resizable("chess-layout")
                    .child(
                        resizable_panel()
                            .size(px(INITIAL_LEFT_PANEL))
                            .size_range(px(320.)..px(1200.))
                            .child(board_panel_with_measure),
                    )
                    .child(
                        resizable_panel()
                            .size_range(px(150.)..Pixels::MAX)
                            .child(move_list_panel_content),
                    ),
            )
    }
}
