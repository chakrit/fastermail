use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

static GUARD_ACTIVE: AtomicBool = AtomicBool::new(false);

/// RAII guard that restores terminal state on drop (SIGINT, panic, normal exit).
/// Clears active spinners, re-shows cursor, resets colors.
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn new() -> Self {
        GUARD_ACTIVE.store(true, Ordering::SeqCst);
        Self
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if GUARD_ACTIVE.swap(false, Ordering::SeqCst) {
            // Best-effort terminal cleanup
            let term = console::Term::stderr();
            let _ = term.show_cursor();
            let _ = term.clear_last_lines(0);
        }
    }
}

/// How the CLI renders output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Colored tables, spinners, status indicators (TTY default).
    Human,
    /// Simplified JSON matching MCP tool responses (non-TTY default or `--json`).
    Json,
    /// Full JMAP response for debugging (`--raw`).
    Raw,
}

impl OutputMode {
    /// Detect output mode from flags and terminal state.
    ///
    /// Precedence: `--raw` > `--json` > auto-detect (TTY → Human, pipe → Json).
    pub fn detect(json: bool, raw: bool) -> Self {
        if raw {
            return Self::Raw;
        }
        if json {
            return Self::Json;
        }
        if io::stdout().is_terminal() {
            Self::Human
        } else {
            Self::Json
        }
    }
}

/// Centralized output for CLI commands.
///
/// All user-facing output goes through `Io`. Commands never write directly to
/// stdout/stderr. In JSON/Raw mode, status messages are suppressed; only `data`
/// and `error` produce output.
pub struct Io {
    mode: OutputMode,
}

impl Io {
    pub fn new(mode: OutputMode) -> Self {
        Self { mode }
    }

    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Show a progress spinner (Human mode only). Returns a handle to finish later.
    pub fn progress(&self, msg: &str) -> Option<ProgressBar> {
        if self.mode != OutputMode::Human {
            return None;
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .expect("valid template"),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    }

    /// Finish a progress spinner.
    pub fn finish_progress(spinner: Option<ProgressBar>) {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
    }

    /// Print success message: ✓ green (Human mode only).
    pub fn done(&self, msg: &str) {
        if self.mode == OutputMode::Human {
            eprintln!("{} {}", style("✓").green(), msg);
        }
    }

    /// Print warning: ⚠ yellow (Human mode only).
    pub fn warn(&self, msg: &str) {
        if self.mode == OutputMode::Human {
            eprintln!("{} {}", style("⚠").yellow(), msg);
        }
    }

    /// Print error: ✗ red (always shown).
    pub fn error(&self, msg: &str) {
        eprintln!("{} {}", style("✗").red(), msg);
    }

    /// Print hint: → dim (Human mode only).
    pub fn hint(&self, msg: &str) {
        if self.mode == OutputMode::Human {
            eprintln!("{} {}", style("→").dim(), style(msg).dim());
        }
    }

    /// Print data to stdout (tables in Human mode, JSON in JSON/Raw mode).
    pub fn data(&self, msg: &str) {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{msg}");
    }

    /// Print a visual separator (Human mode only).
    pub fn separator(&self) {
        if self.mode == OutputMode::Human {
            eprintln!();
        }
    }
}
