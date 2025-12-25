use gpui::{
    App, Application, AssetSource, Bounds, Context, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Rgba, SharedString, Size, Window, WindowBounds, WindowOptions, canvas,
    div, img, prelude::*, px, rgb, size,
};
use gpui_component::Root;
use gpui_component::resizable::{h_resizable, resizable_panel};
use shakmaty::san::San;
use shakmaty::{Chess, Color as SColor, File, Move, Position, Rank, Role, Square};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

const BOARD_PADDING: f32 = 20.0;
const PIECE_SCALE: f32 = 0.98; // piece size relative to square
const GHOST_OPACITY: f32 = 0.4;

// filesystem-based asset source
struct FileAssets {
    base_path: PathBuf,
}

impl FileAssets {
    fn new() -> Self {
        let base_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        Self { base_path }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PieceKind {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PieceColor {
    White,
    Black,
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    kind: PieceKind,
    color: PieceColor,
}

impl Piece {
    fn svg_path(&self) -> &'static str {
        match (self.kind, self.color) {
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
}

// state for a piece being dragged
#[derive(Clone, Copy, Debug)]
struct DragState {
    piece: Piece,
    from_row: usize,
    from_col: usize,
    // mouse position relative to window
    mouse_x: f32,
    mouse_y: f32,
}

// convert our row/col (0-indexed, row 0 = rank 8) to shakmaty Square
fn to_square(row: usize, col: usize) -> Square {
    let file = File::new(col as u32);
    let rank = Rank::new(7 - row as u32); // row 0 = rank 8, row 7 = rank 1
    Square::from_coords(file, rank)
}

// convert shakmaty piece to our Piece
fn shakmaty_to_piece(piece: shakmaty::Piece) -> Piece {
    let kind = match piece.role {
        Role::Pawn => PieceKind::Pawn,
        Role::Knight => PieceKind::Knight,
        Role::Bishop => PieceKind::Bishop,
        Role::Rook => PieceKind::Rook,
        Role::Queen => PieceKind::Queen,
        Role::King => PieceKind::King,
    };
    let color = match piece.color {
        SColor::White => PieceColor::White,
        SColor::Black => PieceColor::Black,
    };
    Piece { kind, color }
}

// initial panel sizes
const INITIAL_LEFT_PANEL: f32 = 540.0;
const INITIAL_RIGHT_PANEL: f32 = 280.0;

struct ChessBoard {
    position: Chess,
    drag_state: Option<DragState>,
    move_history: Vec<String>,
    panel_size: Size<Pixels>, // measured from canvas
}

impl ChessBoard {
    fn new() -> Self {
        Self {
            position: Chess::default(),
            drag_state: None,
            move_history: Vec::new(),
            panel_size: Size {
                width: px(INITIAL_LEFT_PANEL),
                height: px(600.0),
            },
        }
    }

    /// Calculate square size from measured panel dimensions
    fn square_size(&self) -> f32 {
        let panel_width: f32 = self.panel_size.width.into();
        let panel_height: f32 = self.panel_size.height.into();
        let available_width = panel_width - BOARD_PADDING * 2.0;
        let available_height = panel_height - BOARD_PADDING * 2.0;
        (available_width.min(available_height) / 8.0).max(30.0)
    }

    fn piece_size(&self) -> f32 {
        self.square_size() * PIECE_SCALE
    }

    /// Convert position relative to board panel to board row/col (if within board)
    fn pos_to_square(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        // board starts at (BOARD_PADDING, BOARD_PADDING) within the panel
        let board_x = x - BOARD_PADDING;
        let board_y = y - BOARD_PADDING;

        if board_x < 0.0 || board_y < 0.0 {
            return None;
        }

        let square_size = self.square_size();
        let col = (board_x / square_size) as usize;
        let row = (board_y / square_size) as usize;

        if row < 8 && col < 8 {
            Some((row, col))
        } else {
            None
        }
    }

    // get piece at row/col from shakmaty position
    fn piece_at(&self, row: usize, col: usize) -> Option<Piece> {
        let sq = to_square(row, col);
        self.position.board().piece_at(sq).map(shakmaty_to_piece)
    }

    /// Try to make a move from one square to another. Returns true if legal.
    fn try_move(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        let from_sq = to_square(from.0, from.1);
        let to_sq = to_square(to.0, to.1);

        for m in &self.position.legal_moves() {
            let (move_from, move_to) = match m {
                Move::Normal { from, to, .. } => (*from, *to),
                Move::EnPassant { from, to, .. } => (*from, *to),
                Move::Castle { king, rook, .. } => {
                    // for castling, user drags king to destination (g1/g8 or c1/c8)
                    let king_dest = if rook.file() == File::H {
                        Square::from_coords(File::G, rook.rank())
                    } else {
                        Square::from_coords(File::C, rook.rank())
                    };
                    (*king, king_dest)
                }
                Move::Put { .. } => continue,
            };

            if move_from == from_sq && move_to == to_sq {
                // for pawn promotion, auto-promote to queen
                let move_to_play = match m {
                    Move::Normal {
                        role: Role::Pawn,
                        from,
                        to,
                        capture,
                        promotion: None,
                    } if to.rank() == Rank::Eighth || to.rank() == Rank::First => Move::Normal {
                        role: Role::Pawn,
                        from: *from,
                        to: *to,
                        capture: *capture,
                        promotion: Some(Role::Queen),
                    },
                    _ => m.clone(),
                };

                // record move in standard notation
                let san = San::from_move(&self.position, move_to_play.clone());
                self.move_history.push(san.to_string());

                self.position = self.position.clone().play(move_to_play).unwrap();
                return true;
            }
        }
        false
    }

    fn current_turn(&self) -> PieceColor {
        match self.position.turn() {
            SColor::White => PieceColor::White,
            SColor::Black => PieceColor::Black,
        }
    }
}

fn square_color(row: usize, col: usize) -> Rgba {
    if (row + col) % 2 == 0 {
        rgb(0xEFD9B5)
    } else {
        rgb(0xB48764)
    }
}

fn render_piece(piece: Piece, piece_size: f32) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(img(piece.svg_path()).size(px(piece_size)))
}

fn render_square(
    row: usize,
    col: usize,
    piece: Option<Piece>,
    is_being_dragged: bool,
    square_size: f32,
    piece_size: f32,
) -> impl IntoElement {
    div()
        .flex_shrink_0() // never shrink - maintain aspect ratio
        .size(px(square_size))
        .bg(square_color(row, col))
        .flex()
        .items_center()
        .justify_center()
        .when_some(piece, |el, p| {
            if is_being_dragged {
                // ghost piece on original square
                el.child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(GHOST_OPACITY)
                        .child(img(p.svg_path()).size(px(piece_size))),
                )
            } else {
                el.child(render_piece(p, piece_size))
            }
        })
}

impl Render for ChessBoard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();
        let entity_down = entity.clone();
        let entity_move = entity.clone();
        let entity_up = entity.clone();
        let entity_measure = entity;
        let drag_state = self.drag_state;
        let dragging_from = drag_state.map(|d| (d.from_row, d.from_col));

        // sizing based on measured panel dimensions
        let square_size = self.square_size();
        let piece_size = self.piece_size();

        // prepare move pairs for display
        let move_pairs: Vec<(usize, String, Option<String>)> = self
            .move_history
            .chunks(2)
            .enumerate()
            .map(|(i, chunk)| {
                let move_num = i + 1;
                let white_move = chunk.get(0).cloned().unwrap_or_default();
                let black_move = chunk.get(1).cloned();
                (move_num, white_move, black_move)
            })
            .collect();

        // floating piece follows cursor during drag
        let floating_piece = drag_state.map(|d| {
            div()
                .absolute()
                .left(px(d.mouse_x - piece_size / 2.0))
                .top(px(d.mouse_y - piece_size / 2.0))
                .size(px(piece_size))
                .child(img(d.piece.svg_path()).size(px(piece_size)))
        });

        // board element with fixed size - always maintains 1:1 aspect ratio
        // board_size = 8 squares, never shrinks
        let board_total_size = square_size * 8.0;
        let board = div()
            .flex_shrink_0() // never shrink
            .flex()
            .flex_col()
            .w(px(board_total_size))
            .h(px(board_total_size))
            .overflow_hidden()
            .rounded_md()
            .children((0..8).map(|row| {
                div().flex().flex_shrink_0().children((0..8).map(|col| {
                    let piece = self.piece_at(row, col);
                    let is_being_dragged = dragging_from == Some((row, col));
                    render_square(row, col, piece, is_being_dragged, square_size, piece_size)
                }))
            }));

        let board_panel_content = div()
            .id("board-panel")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x2a2a2a))
            .p(px(BOARD_PADDING))
            .child(board)
            .when_some(floating_piece, |el, fp| el.child(fp))
            // mouse down: start drag if clicking on a piece
            .on_mouse_down(
                MouseButton::Left,
                move |ev: &MouseDownEvent, _window, cx| {
                    entity_down.update(cx, |board, cx| {
                        let pos = ev.position;
                        if let Some((row, col)) = board.pos_to_square(pos.x.into(), pos.y.into()) {
                            if let Some(piece) = board.piece_at(row, col) {
                                if piece.color == board.current_turn() {
                                    board.drag_state = Some(DragState {
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
            // mouse move: update drag position
            .on_mouse_move(move |ev: &MouseMoveEvent, _, cx| {
                entity_move.update(cx, |board, cx| {
                    if let Some(ref mut drag) = board.drag_state {
                        drag.mouse_x = ev.position.x.into();
                        drag.mouse_y = ev.position.y.into();
                        cx.notify();
                    }
                });
            })
            // mouse up: complete the move
            .on_mouse_up(MouseButton::Left, move |ev: &MouseUpEvent, _window, cx| {
                entity_up.update(cx, |board, cx| {
                    if let Some(drag) = board.drag_state.take() {
                        let pos = ev.position;
                        if let Some((to_row, to_col)) =
                            board.pos_to_square(pos.x.into(), pos.y.into())
                        {
                            board.try_move((drag.from_row, drag.from_col), (to_row, to_col));
                        }
                        cx.notify();
                    }
                });
            });

        // move list panel content
        let move_list =
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(0x1e1e1e))
                .border_1()
                .border_color(rgb(0x4a4a4a))
                .rounded_md()
                // header (fixed)
                .child(
                    div()
                        .p_4()
                        .pb_2()
                        .text_color(rgb(0xffffff))
                        .text_sm()
                        .border_b_1()
                        .border_color(rgb(0x4a4a4a))
                        .child("Move History"),
                )
                // scrollable moves content
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
                                    .text_color(rgb(0x888888))
                                    .text_sm()
                                    .child("No moves yet"),
                            )
                        })
                        .children(move_pairs.into_iter().map(
                            |(move_num, white_move, black_move)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .py_1()
                                    .child(
                                        div()
                                            .text_color(rgb(0x888888))
                                            .text_sm()
                                            .w(px(40.0))
                                            .child(format!("{}.", move_num)),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0xffffff))
                                            .text_sm()
                                            .flex_1()
                                            .child(white_move),
                                    )
                                    .when_some(black_move, |el, bm| {
                                        el.child(
                                            div()
                                                .text_color(rgb(0xffffff))
                                                .text_sm()
                                                .flex_1()
                                                .child(bm),
                                        )
                                    })
                            },
                        )),
                );

        let move_list_panel_content = div()
            .size_full()
            .bg(rgb(0x2a2a2a))
            .p(px(BOARD_PADDING))
            .child(move_list);

        // canvas to measure actual panel size - absolutely positioned so it doesn't affect layout
        let measure_canvas = canvas(
            move |bounds, _window, cx| {
                entity_measure.update(cx, |board, cx| {
                    if board.panel_size != bounds.size {
                        board.panel_size = bounds.size;
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

        // wrap board panel content with measuring canvas (canvas is absolute, doesn't affect layout)
        let board_panel_with_measure = div()
            .relative()
            .size_full()
            .child(measure_canvas)
            .child(board_panel_content);

        // main resizable layout
        div().size_full().child(
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

fn main() {
    Application::new()
        .with_assets(FileAssets::new())
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|_| ChessBoard::new());
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();
        });
}
