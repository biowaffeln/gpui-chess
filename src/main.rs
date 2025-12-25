use gpui::{
    App, Application, AssetSource, Bounds, Context, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Rgba, SharedString, Window, WindowBounds, WindowOptions, div, img, prelude::*,
    px, rgb, size,
};
use shakmaty::{Chess, Color as SColor, File, Move, Position, Rank, Role, Square};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

const SQUARE_SIZE: f32 = 60.0;
const BOARD_SIZE: f32 = SQUARE_SIZE * 8.0;
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

struct ChessBoard {
    position: Chess,
    drag_state: Option<DragState>,
}

impl ChessBoard {
    fn new() -> Self {
        Self {
            position: Chess::default(),
            drag_state: None,
        }
    }

    // convert window position to board row/col (if within board)
    fn pos_to_square(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        // board starts at (BOARD_PADDING, BOARD_PADDING)
        let board_x = x - BOARD_PADDING;
        let board_y = y - BOARD_PADDING;

        if board_x < 0.0 || board_y < 0.0 {
            return None;
        }

        let col = (board_x / SQUARE_SIZE) as usize;
        let row = (board_y / SQUARE_SIZE) as usize;

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

    fn is_checkmate(&self) -> bool {
        self.position.is_checkmate()
    }

    fn is_stalemate(&self) -> bool {
        self.position.is_stalemate()
    }
}

fn square_color(row: usize, col: usize) -> Rgba {
    if (row + col) % 2 == 0 {
        rgb(0xEFD9B5)
    } else {
        rgb(0xB48764)
    }
}

fn piece_size() -> f32 {
    SQUARE_SIZE * PIECE_SCALE
}

fn render_piece(piece: Piece) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(img(piece.svg_path()).size(px(piece_size())))
}

fn render_square(
    row: usize,
    col: usize,
    piece: Option<Piece>,
    is_being_dragged: bool,
) -> impl IntoElement {
    div()
        .size(px(SQUARE_SIZE))
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
                        .child(img(p.svg_path()).size(px(piece_size()))),
                )
            } else {
                el.child(render_piece(p))
            }
        })
}

impl Render for ChessBoard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();
        let entity_down = entity.clone();
        let entity_move = entity.clone();
        let entity_up = entity;
        let drag_state = self.drag_state;
        let dragging_from = drag_state.map(|d| (d.from_row, d.from_col));

        // floating piece follows cursor during drag
        let floating_piece = drag_state.map(|d| {
            let size = piece_size();
            div()
                .absolute()
                .left(px(d.mouse_x - size / 2.0))
                .top(px(d.mouse_y - size / 2.0))
                .size(px(size))
                .child(img(d.piece.svg_path()).size(px(size)))
        });

        div()
            .id("chess-window")
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(0x2a2a2a))
            .w_full()
            .h_full()
            .min_w(px(BOARD_SIZE + 44.0))
            .p(px(BOARD_PADDING))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .shadow_xl()
                    .border_2()
                    .border_color(rgb(0x4a4a4a))
                    .children((0..8).map(|row| {
                        div().flex().children((0..8).map(|col| {
                            let piece = self.piece_at(row, col);
                            let is_being_dragged = dragging_from == Some((row, col));
                            render_square(row, col, piece, is_being_dragged)
                        }))
                    })),
            )
            .when_some(floating_piece, |el, fp| el.child(fp))
            // mouse down: start drag if clicking on a piece
            .on_mouse_down(MouseButton::Left, move |ev: &MouseDownEvent, _, cx| {
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
            })
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
            .on_mouse_up(MouseButton::Left, move |ev: &MouseUpEvent, _, cx| {
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
            })
    }
}

fn main() {
    Application::new()
        .with_assets(FileAssets::new())
        .run(|cx: &mut App| {
            let window_size = BOARD_SIZE + 40.0;
            let bounds = Bounds::centered(None, size(px(window_size), px(window_size)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| ChessBoard::new()),
            )
            .unwrap();
        });
}
