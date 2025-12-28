//! Engine analysis pane - displays UCI engine output with start/stop control.

use gpui::{App, Entity, SharedString, div, prelude::*, px, rgb};
use gpui_component::button::{Button, ButtonVariants};

use crate::domain::uci::{Score, UciInfo};
use crate::models::EngineModel;
use crate::ui::theme::{
    BOARD_PADDING, BORDER_COLOR, MOVE_LIST_BG, PANEL_BG, TEXT_PRIMARY, TEXT_SECONDARY,
};

// Colors for evaluation display
const EVAL_POSITIVE: u32 = 0x4ade80; // green - white advantage
const EVAL_NEGATIVE: u32 = 0xf87171; // red - black advantage  
const EVAL_NEUTRAL: u32 = 0xa1a1aa; // gray - equal
#[allow(dead_code)] // Reserved for mate display
const EVAL_MATE: u32 = 0xfbbf24; // yellow/gold - mate

/// Render the engine analysis pane.
/// Shows parsed analysis (eval, depth, PV) and raw output below.
pub fn render_engine_pane(engine_model: &Entity<EngineModel>, cx: &App) -> impl IntoElement {
    let engine = engine_model.read(cx);
    let is_running = engine.is_running();
    let is_analyzing = engine.is_analyzing();
    let analysis_lines = engine.analysis_lines();
    let black_to_move = engine.is_black_to_move();
    let output_lines = engine.output_lines();

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

    // Status indicator
    let status_text = if is_running {
        if is_analyzing {
            "Analyzing..."
        } else {
            "Ready"
        }
    } else {
        "Stopped"
    };

    let status_color = if is_running { EVAL_POSITIVE } else { 0xf87171 };

    // Build the analysis display section
    let analysis_section = render_analysis_section(&analysis_lines, black_to_move, is_running);

    // Build the raw output section
    let raw_output_section = render_raw_output_section(output_lines);

    let engine_pane = div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(rgb(MOVE_LIST_BG))
        .border_1()
        .border_color(rgb(BORDER_COLOR))
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
                .border_color(rgb(BORDER_COLOR))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_color(rgb(TEXT_PRIMARY))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("Engine"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(status_color))
                                .child(status_text),
                        ),
                )
                .child(toggle_button),
        )
        // Analysis section (shows all PV lines)
        .child(analysis_section)
        // Raw output section (scrollable, takes remaining space)
        .child(raw_output_section);

    div()
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(rgb(PANEL_BG))
        .p(px(BOARD_PADDING))
        .child(engine_pane)
}

/// Render the main analysis display (all PV lines)
fn render_analysis_section(
    analysis_lines: &[&UciInfo],
    black_to_move: bool,
    is_running: bool,
) -> impl IntoElement {
    let content = if !analysis_lines.is_empty() {
        // Show all analysis lines
        div().flex().flex_col().gap_2().children(
            analysis_lines
                .iter()
                .enumerate()
                .map(|(i, info)| render_pv_line(info, i == 0, black_to_move)),
        )
    } else if is_running {
        div()
            .text_color(rgb(TEXT_SECONDARY))
            .text_sm()
            .child("Waiting for analysis...")
    } else {
        div()
            .text_color(rgb(TEXT_SECONDARY))
            .text_sm()
            .child("Start engine to analyze position")
    };

    div()
        .flex_shrink_0()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(rgb(BORDER_COLOR))
        .child(content)
}

/// Render a single PV line
fn render_pv_line(info: &UciInfo, is_best: bool, black_to_move: bool) -> gpui::Div {
    let (eval_text, eval_color) = format_evaluation(info.score, black_to_move);
    let pv_text = format_pv(&info.pv);

    if is_best {
        // Best line gets prominent display
        let depth_text = format_depth(info.depth, info.seldepth);
        let stats_text = format_stats(info);

        div()
            .flex()
            .flex_col()
            .gap_1()
            // Top row: Evaluation + Depth + Stats
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    // Large evaluation display
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(eval_color))
                            .child(eval_text),
                    )
                    // Depth display
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child("Depth"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT_PRIMARY))
                                    .child(depth_text),
                            ),
                    )
                    // Stats (nodes/nps)
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_SECONDARY))
                            .child(stats_text),
                    ),
            )
            // Principal variation
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(TEXT_PRIMARY))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(pv_text),
            )
    } else {
        // Secondary lines get compact display
        div()
            .flex()
            .items_center()
            .gap_2()
            .pt_1()
            .border_t_1()
            .border_color(rgb(BORDER_COLOR))
            // Eval (smaller)
            .child(
                div()
                    .w(px(60.))
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(eval_color))
                    .child(eval_text),
            )
            // PV
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .text_color(rgb(TEXT_SECONDARY))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(pv_text),
            )
    }
}

/// Render the raw output section
fn render_raw_output_section(output_lines: &[crate::domain::uci::UciOutput]) -> impl IntoElement {
    let content = if output_lines.is_empty() {
        div()
            .text_color(rgb(TEXT_SECONDARY))
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
                    .text_color(rgb(TEXT_SECONDARY))
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
        // Section header
        .child(
            div()
                .flex_shrink_0()
                .px_4()
                .py_1()
                .text_xs()
                .text_color(rgb(TEXT_SECONDARY))
                .border_b_1()
                .border_color(rgb(BORDER_COLOR))
                .child("Raw Output"),
        )
        // Scrollable content
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
fn format_evaluation(score: Option<Score>, black_to_move: bool) -> (String, u32) {
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
                EVAL_POSITIVE
            } else if white_cp < -50 {
                EVAL_NEGATIVE
            } else {
                EVAL_NEUTRAL
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
            let color = if white_mate > 0 {
                EVAL_POSITIVE
            } else {
                EVAL_NEGATIVE
            };
            (text, color)
        }
        None => ("--".to_string(), EVAL_NEUTRAL),
    }
}

/// Format depth for display
fn format_depth(depth: Option<u32>, seldepth: Option<u32>) -> String {
    match (depth, seldepth) {
        (Some(d), Some(sd)) => format!("{}/{}", d, sd),
        (Some(d), None) => format!("{}", d),
        _ => "--".to_string(),
    }
}

/// Format the principal variation for display
fn format_pv(pv: &[String]) -> String {
    if pv.is_empty() {
        return "...".to_string();
    }

    // Show first several moves, join with spaces
    let display_moves: Vec<&str> = pv.iter().take(8).map(|s| s.as_str()).collect();
    let mut result = display_moves.join(" ");

    if pv.len() > 8 {
        result.push_str(" ...");
    }

    result
}

/// Format search statistics
fn format_stats(info: &UciInfo) -> String {
    let mut parts = Vec::new();

    if let Some(nodes) = info.nodes {
        parts.push(format_nodes(nodes));
    }

    if let Some(nps) = info.nps {
        parts.push(format!("{}/s", format_nodes(nps)));
    }

    if parts.is_empty() {
        return String::new();
    }

    parts.join(" | ")
}

/// Format large numbers with K/M/B suffixes
fn format_nodes(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}
