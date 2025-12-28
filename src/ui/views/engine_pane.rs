//! Engine analysis pane - displays UCI engine output with start/stop control.

use gpui::{App, Corner, Entity, Hsla, SharedString, Window, div, prelude::*, px};

use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::{ActiveTheme, IconName, Sizable};

use crate::domain::uci::{Score, UciInfo};
use crate::models::EngineModel;
use crate::ui::theme::BOARD_PADDING;

/// Available thread count options
const THREAD_OPTIONS: &[u32] = &[1, 2, 4, 6, 8];

/// Render the engine analysis pane.
/// Shows parsed analysis (eval, depth, PV) and raw output below.
pub fn render_engine_pane(
    engine_model: &Entity<EngineModel>,
    _window: &Window,
    cx: &App,
) -> impl IntoElement {
    let engine = engine_model.read(cx);
    let is_running = engine.is_running();
    let analysis_lines = engine.analysis_lines();
    let black_to_move = engine.is_black_to_move();
    let output_lines = engine.output_lines();
    let multi_pv = engine.multi_pv();
    let threads = engine.threads();
    let show_uci_output = engine.show_uci_output();

    // Theme colors
    let theme = cx.theme();
    let bg = theme.background;
    let secondary_bg = theme.secondary;
    let border = theme.border;
    let fg = theme.foreground;
    let muted_fg = theme.muted_foreground;

    // Start/Stop button
    let engine_model_clone = engine_model.clone();
    let toggle_button = if is_running {
        Button::new("stop-engine")
            .label("Stop")
            .danger()
            .compact()
            .on_click(move |_, _, cx| {
                engine_model_clone.update(cx, |engine, cx| {
                    engine.stop();
                    cx.notify();
                });
            })
    } else {
        Button::new("start-engine")
            .label("Start")
            .primary()
            .compact()
            .on_click(move |_, _, cx| {
                engine_model_clone.update(cx, |engine, cx| {
                    if let Err(e) = engine.start(cx) {
                        eprintln!("Failed to start engine: {}", e);
                    }
                    cx.notify();
                });
            })
    };

    // MultiPV dropdown
    let lines_control = render_lines_control(engine_model, multi_pv, muted_fg);

    // Threads dropdown
    let threads_control = render_threads_control(engine_model, threads, muted_fg);

    // Get depth and nps from first analysis line for header display
    let header_depth = analysis_lines.first().and_then(|info| info.depth);
    let header_nps = analysis_lines.first().and_then(|info| info.nps);

    // Build the analysis display section
    let analysis_section = render_analysis_section(&analysis_lines, black_to_move, is_running, cx);

    // Raw output toggle button
    let engine_model_raw = engine_model.clone();
    let uci_output_toggle = Button::new("toggle-raw-output")
        .label("UCI")
        .ghost()
        .xsmall()
        .on_click(move |_, _, cx| {
            engine_model_raw.update(cx, |engine, cx| {
                engine.toggle_uci_output();
                cx.notify();
            });
        });

    // Build the raw output section (only if visible)
    let uci_output_section = if show_uci_output {
        Some(render_uci_output_section(output_lines, muted_fg, border))
    } else {
        None
    };

    let engine_pane = div()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(secondary_bg)
        .border_1()
        .border_color(border)
        .rounded_md()
        .overflow_hidden()
        // Header with title and controls
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_color(fg)
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("Stockfish"),
                        )
                        .when_some(header_depth, |el, depth| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child(format!("depth {}", depth)),
                            )
                        })
                        .when_some(header_nps, |el, nps| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child(format!("{}", format_nps(nps))),
                            )
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(lines_control)
                        .child(threads_control)
                        .child(toggle_button)
                        .child(div().flex_1().flex().justify_end().child(uci_output_toggle)),
                ),
        )
        // Analysis section (shows all PV lines)
        .child(analysis_section)
        .when_some(uci_output_section, |el, section| el.child(section));

    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(bg)
        .p(px(BOARD_PADDING))
        .child(engine_pane)
}

/// Render the lines (MultiPV) control dropdown
fn render_lines_control(
    engine_model: &Entity<EngineModel>,
    current_lines: u32,
    muted_fg: Hsla,
) -> impl IntoElement {
    let engine_model = engine_model.clone();

    div()
        .flex()
        .items_center()
        .gap_1()
        .child(div().text_xs().text_color(muted_fg).child("Lines"))
        .child(
            Button::new("lines-dropdown")
                .ghost()
                .compact()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(format!("{}", current_lines))
                        .child(IconName::ChevronDown),
                )
                .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _window, _cx| {
                    let mut m = menu;
                    for line_count in 1..=5u32 {
                        let engine_model = engine_model.clone();
                        let is_selected = line_count == current_lines;
                        m = m.item(
                            PopupMenuItem::new(format!(
                                "{} {}",
                                line_count,
                                if line_count == 1 { "line" } else { "lines" }
                            ))
                            .checked(is_selected)
                            .on_click(move |_ev, _window, cx| {
                                engine_model.update(cx, |engine, cx| {
                                    engine.set_multi_pv(line_count);
                                    cx.notify();
                                });
                            }),
                        );
                    }
                    m
                }),
        )
}

/// Render the threads control dropdown
fn render_threads_control(
    engine_model: &Entity<EngineModel>,
    current_threads: u32,
    muted_fg: Hsla,
) -> impl IntoElement {
    let engine_model = engine_model.clone();

    div()
        .flex()
        .items_center()
        .gap_1()
        .child(div().text_xs().text_color(muted_fg).child("Cores"))
        .child(
            Button::new("threads-dropdown")
                .ghost()
                .compact()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(format!("{}", current_threads))
                        .child(IconName::ChevronDown),
                )
                .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _window, _cx| {
                    let mut m = menu;
                    for &thread_count in THREAD_OPTIONS {
                        let engine_model = engine_model.clone();
                        let is_selected = thread_count == current_threads;
                        m = m.item(
                            PopupMenuItem::new(format!(
                                "{} {}",
                                thread_count,
                                if thread_count == 1 { "core" } else { "cores" }
                            ))
                            .checked(is_selected)
                            .on_click(move |_ev, _window, cx| {
                                engine_model.update(cx, |engine, cx| {
                                    engine.set_threads(thread_count);
                                    cx.notify();
                                });
                            }),
                        );
                    }
                    m
                }),
        )
}

/// Render the main analysis display (all PV lines)
fn render_analysis_section(
    analysis_lines: &[&UciInfo],
    black_to_move: bool,
    is_running: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let muted_fg = theme.muted_foreground;
    let fg = theme.foreground;
    let border = theme.border;
    let green = theme.green;
    let red = theme.red;

    let content = if !analysis_lines.is_empty() {
        // Show all analysis lines
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(analysis_lines.iter().enumerate().map(|(i, info)| {
                render_pv_line(
                    info,
                    i == 0,
                    black_to_move,
                    fg,
                    muted_fg,
                    border,
                    green,
                    red,
                )
            }))
    } else if is_running {
        div()
            .text_color(muted_fg)
            .text_sm()
            .child("Waiting for analysis...")
    } else {
        div()
            .text_color(muted_fg)
            .text_sm()
            .child("Start engine to analyze position")
    };

    div().flex_shrink_0().px_4().py_3().child(content)
}

/// Render a single PV line
#[allow(clippy::too_many_arguments)]
fn render_pv_line(
    info: &UciInfo,
    is_best: bool,
    black_to_move: bool,
    fg: Hsla,
    muted_fg: Hsla,
    border: Hsla,
    green: Hsla,
    red: Hsla,
) -> gpui::Div {
    let (eval_text, eval_color) =
        format_evaluation(info.score, black_to_move, muted_fg, green, red);
    let pv_text = if info.pv_san.is_empty() {
        "...".to_string()
    } else {
        info.pv_san.clone()
    };

    if is_best {
        // First line gets uniform display with eval
        div()
            .flex()
            .items_center()
            .gap_2()
            // Eval
            .child(
                div()
                    .w(px(60.))
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(eval_color)
                    .child(eval_text),
            )
            // PV
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(fg)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(pv_text),
            )
    } else {
        // Secondary lines get uniform display with eval
        div()
            .flex()
            .items_center()
            .gap_2()
            .pt_1()
            .border_t_1()
            .border_color(border)
            // Eval
            .child(
                div()
                    .w(px(60.))
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(eval_color)
                    .child(eval_text),
            )
            // PV
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(fg)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(pv_text),
            )
    }
}

/// Render the raw output section
fn render_uci_output_section(
    output_lines: &[crate::domain::uci::UciOutput],
    muted_fg: Hsla,
    border: Hsla,
) -> impl IntoElement {
    let content = if output_lines.is_empty() {
        div()
            .text_color(muted_fg)
            .text_xs()
            .child("No output yet...")
    } else {
        // Show last N output lines (most recent first for relevance)
        let lines_to_show: Vec<_> = output_lines.iter().rev().take(50).collect();
        div()
            .flex()
            .flex_col()
            .gap_px()
            .children(lines_to_show.iter().enumerate().map(|(i, line)| {
                div()
                    .id(SharedString::from(format!("engine-line-{}", i)))
                    .text_xs()
                    .text_color(muted_fg)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(line.raw.clone())
            }))
    };

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_hidden()
        .border_t_1()
        .border_color(border)
        .child(
            div()
                .id("engine-raw-output-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px_4()
                .py_2()
                .child(content),
        )
}

/// Format the evaluation score for display (always from white's perspective)
fn format_evaluation(
    score: Option<Score>,
    black_to_move: bool,
    neutral: Hsla,
    positive: Hsla,
    negative: Hsla,
) -> (String, Hsla) {
    match score {
        Some(Score::Centipawns(cp)) => {
            // Flip sign if it's black's turn (engine gives score from side-to-move perspective)
            let white_cp = if black_to_move { -cp } else { cp };
            let pawns = white_cp as f64 / 100.0;
            let text = if pawns >= 0.0 {
                format!("+{:.2}", pawns)
            } else {
                format!("{:.2}", pawns)
            };
            let color = if white_cp > 50 {
                positive
            } else if white_cp < -50 {
                negative
            } else {
                neutral
            };
            (text, color)
        }
        Some(Score::Mate(moves)) => {
            // Flip sign if it's black's turn
            let white_mate = if black_to_move { -moves } else { moves };
            let text = if white_mate > 0 {
                format!("M{}", white_mate)
            } else {
                format!("-M{}", white_mate.abs())
            };
            // Color based on who's winning
            let color = if white_mate > 0 { positive } else { negative };
            (text, color)
        }
        None => ("--".to_string(), neutral),
    }
}

/// Format nodes per second with K/M/B suffixes
fn format_nps(nps: u64) -> String {
    if nps >= 1_000_000 {
        format!("{:.1}M nps", nps as f64 / 1_000_000.0)
    } else if nps >= 1_000 {
        format!("{:.1}K nps", nps as f64 / 1_000.0)
    } else {
        format!("{} nps", nps)
    }
}
