//! Engine state model - manages UCI engine lifecycle and analysis.
//!
//! This model handles spawning the engine process, sending commands,
//! and collecting output for display in the UI.
//!
//! Architecture:
//! - Engine I/O runs on OS threads (reader/writer)
//! - A GPUI background task polls the event channel and pushes updates to the UI
//! - This ensures the UI updates immediately when engine output arrives

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use gpui::{AsyncApp, Context, Task, WeakEntity};

use crate::domain::uci::{UciCommand, UciInfo, UciOutput, UciOutputKind};

/// Hardcoded engine path (will be configurable later)
const ENGINE_PATH: &str = "/opt/homebrew/bin/stockfish";

/// Maximum number of output lines to keep in history
const MAX_OUTPUT_LINES: usize = 100;

/// Number of principal variations to request from engine
const MULTI_PV: u32 = 3;

/// Messages sent from the engine reader thread to the model
#[derive(Debug)]
pub enum EngineEvent {
    /// A line of output from the engine
    Output(String),
    /// Engine process exited
    Exited,
    /// Error occurred
    Error(String),
}

/// The engine model - manages UCI engine state
pub struct EngineModel {
    /// Whether the engine is currently running
    running: bool,
    /// Whether the engine is currently analyzing
    analyzing: bool,
    /// Recent output lines from the engine (for display)
    output_lines: Vec<UciOutput>,
    /// Current analysis lines (keyed by multipv number, 1-indexed)
    analysis_lines: HashMap<u32, UciInfo>,
    /// Whether it's black's turn (for flipping eval display)
    black_to_move: bool,
    /// Current FEN being analyzed (if any)
    current_fen: Option<String>,
    /// Channel receiver for engine events (polled by background task)
    event_receiver: Option<Receiver<EngineEvent>>,
    /// Channel sender for commands to engine writer thread
    command_sender: Option<Sender<String>>,
    /// Handle to the engine process
    process: Option<Child>,
    /// Background polling task (kept alive while engine is running)
    _poll_task: Option<Task<()>>,
}

impl EngineModel {
    pub fn new() -> Self {
        Self {
            running: false,
            analyzing: false,
            output_lines: Vec::new(),
            analysis_lines: HashMap::new(),
            black_to_move: false,
            current_fen: None,
            event_receiver: None,
            command_sender: None,
            process: None,
            _poll_task: None,
        }
    }

    /// Check if the engine is currently running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Check if the engine is currently analyzing
    pub fn is_analyzing(&self) -> bool {
        self.analyzing
    }

    /// Get the output lines for display
    pub fn output_lines(&self) -> &[UciOutput] {
        &self.output_lines
    }

    /// Get all analysis lines sorted by multipv number
    pub fn analysis_lines(&self) -> Vec<&UciInfo> {
        let mut lines: Vec<_> = self.analysis_lines.values().collect();
        lines.sort_by_key(|info| info.multipv.unwrap_or(1));
        lines
    }

    /// Get the best (first) analysis line
    #[allow(dead_code)] // Reserved for future use
    pub fn best_analysis(&self) -> Option<&UciInfo> {
        self.analysis_lines.get(&1)
    }

    /// Whether it's black's turn in the current position
    pub fn is_black_to_move(&self) -> bool {
        self.black_to_move
    }

    /// Get the current FEN being analyzed (if any)
    pub fn current_fen(&self) -> Option<&str> {
        self.current_fen.as_deref()
    }

    /// Start the engine process
    /// 
    /// Must be called from a Context<EngineModel> to spawn the background polling task.
    pub fn start(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        if self.running {
            return Ok(());
        }

        // Spawn the engine process
        let mut child = Command::new(ENGINE_PATH)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start engine: {}", e))?;

        // Take ownership of stdin/stdout
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;

        // Create channels for communication
        let (event_tx, event_rx) = mpsc::channel::<EngineEvent>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>();

        // Spawn reader thread (OS thread for blocking I/O)
        let event_tx_clone = event_tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        if event_tx_clone.send(EngineEvent::Output(text)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = event_tx_clone.send(EngineEvent::Error(e.to_string()));
                        break;
                    }
                }
            }
            let _ = event_tx_clone.send(EngineEvent::Exited);
        });

        // Spawn writer thread (OS thread for blocking I/O)
        thread::spawn(move || {
            let mut writer = stdin;
            while let Ok(cmd) = cmd_rx.recv() {
                if writeln!(writer, "{}", cmd).is_err() {
                    break;
                }
                if writer.flush().is_err() {
                    break;
                }
            }
        });

        self.process = Some(child);
        self.event_receiver = Some(event_rx);
        self.command_sender = Some(cmd_tx);
        self.running = true;

        // Spawn background polling task that pushes events to the UI
        let poll_task = cx.spawn(async move |weak_entity: WeakEntity<EngineModel>, cx: &mut AsyncApp| {
            Self::run_event_loop(weak_entity, cx).await;
        });
        self._poll_task = Some(poll_task);

        // Initialize UCI
        self.send_command(UciCommand::Uci);
        self.send_command(UciCommand::IsReady);
        
        // Set MultiPV option
        self.send_command(UciCommand::SetOption {
            name: "MultiPV".to_string(),
            value: MULTI_PV.to_string(),
        });

        self.add_output("[Engine started]".to_string());

        Ok(())
    }
    
    /// Background event loop that polls the channel and updates the model
    async fn run_event_loop(weak_entity: WeakEntity<EngineModel>, cx: &mut AsyncApp) {
        const POLL_INTERVAL: Duration = Duration::from_millis(16); // ~60fps
        
        loop {
            // Small delay to avoid busy-waiting
            cx.background_executor().timer(POLL_INTERVAL).await;
            
            // Try to update the entity - if it's gone, exit the loop
            let should_continue = weak_entity.update(cx, |engine, cx| {
                if !engine.running {
                    return false;
                }
                
                // Drain all available events from the channel
                let had_events = engine.process_pending_events();
                if had_events {
                    cx.notify(); // Trigger UI re-render
                }
                
                true
            });
            
            match should_continue {
                Ok(true) => continue,
                _ => break, // Engine stopped or entity dropped
            }
        }
    }
    
    /// Process all pending events from the channel
    /// Returns true if any events were processed
    fn process_pending_events(&mut self) -> bool {
        let events: Vec<EngineEvent> = match &self.event_receiver {
            Some(rx) => {
                let mut collected = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    collected.push(event);
                }
                collected
            }
            None => return false,
        };

        if events.is_empty() {
            return false;
        }
        
        for event in events {
            match event {
                EngineEvent::Output(line) => {
                    self.add_output(line);
                }
                EngineEvent::Exited => {
                    self.running = false;
                    self.analyzing = false;
                    self.add_output("[Engine exited]".to_string());
                }
                EngineEvent::Error(e) => {
                    self.add_output(format!("[Error: {}]", e));
                }
            }
        }

        true
    }

    /// Stop the engine process
    pub fn stop(&mut self) {
        if !self.running {
            return;
        }

        // Stop any ongoing analysis
        if self.analyzing {
            self.stop_analysis();
        }

        // Send quit command
        self.send_command(UciCommand::Quit);

        // Clean up channels (this will cause the polling loop to exit)
        self.command_sender = None;
        self.event_receiver = None;
        
        // Drop the poll task (it will exit on next iteration when it sees running=false)
        self._poll_task = None;

        // Kill the process if it's still running
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        self.running = false;
        self.analyzing = false;
        self.add_output("[Engine stopped]".to_string());
    }

    /// Start analyzing the given FEN position
    pub fn start_analysis(&mut self, fen: &str) {
        if !self.running {
            return;
        }

        // Stop previous analysis if any
        if self.analyzing {
            self.send_command(UciCommand::Stop);
        }

        self.current_fen = Some(fen.to_string());
        self.analysis_lines.clear(); // Clear previous analysis
        
        // Parse side to move from FEN (second field)
        // FEN format: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        self.black_to_move = fen.split_whitespace()
            .nth(1)
            .map(|s| s == "b")
            .unwrap_or(false);

        // Send position and start analysis
        self.send_command(UciCommand::Position {
            fen: Some(fen.to_string()),
            moves: vec![],
        });
        self.send_command(UciCommand::GoInfinite);

        self.analyzing = true;
    }

    /// Stop the current analysis
    pub fn stop_analysis(&mut self) {
        if !self.analyzing {
            return;
        }

        self.send_command(UciCommand::Stop);
        self.analyzing = false;
    }

    /// Send a UCI command to the engine
    fn send_command(&self, cmd: UciCommand) {
        let cmd_str = cmd.to_uci_string();
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(cmd_str);
        }
    }

    /// Add an output line (with truncation) and parse info if applicable
    fn add_output(&mut self, line: String) {
        let output = UciOutput::new(line);

        // If this is an info line, try to parse it and update analysis
        if let UciOutputKind::Info(info_str) = &output.kind {
            let info = UciInfo::parse(info_str);
            // Only update if this has meaningful analysis (depth + score + pv)
            if info.has_analysis() {
                let pv_num = info.multipv.unwrap_or(1);
                self.analysis_lines.insert(pv_num, info);
            }
        }

        self.output_lines.push(output);

        // Keep only the last MAX_OUTPUT_LINES
        if self.output_lines.len() > MAX_OUTPUT_LINES {
            let excess = self.output_lines.len() - MAX_OUTPUT_LINES;
            self.output_lines.drain(0..excess);
        }
    }

    /// Clear all output lines
    #[allow(dead_code)] // Reserved for future use
    pub fn clear_output(&mut self) {
        self.output_lines.clear();
    }
}

impl Default for EngineModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EngineModel {
    fn drop(&mut self) {
        self.stop();
    }
}
