use std::fmt;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Trace => write!(f, "TRACE"),
            Level::Debug => write!(f, "DEBUG"),
            Level::Info => write!(f, "INFO"),
            Level::Warn => write!(f, "WARN"),
            Level::Error => write!(f, "ERROR"),
        }
    }
}

static MIN_LEVEL: OnceLock<Level> = OnceLock::new();

pub fn init() {
    let level = std::env::var("FASTERMAIL_LOG")
        .ok()
        .and_then(|s| parse_level(&s))
        .unwrap_or(Level::Info);

    MIN_LEVEL.get_or_init(|| level);
}

fn parse_level(s: &str) -> Option<Level> {
    match s.to_lowercase().as_str() {
        "trace" => Some(Level::Trace),
        "debug" => Some(Level::Debug),
        "info" => Some(Level::Info),
        "warn" => Some(Level::Warn),
        "error" => Some(Level::Error),
        _ => None,
    }
}

pub fn logf(level: Level, target: &str, args: fmt::Arguments<'_>) {
    let min = MIN_LEVEL.get().copied().unwrap_or(Level::Info);
    if level < min {
        return;
    }

    eprintln!("[fastermail] [{level}] [{target}] {args}");
}

// Convenience macros

#[macro_export]
macro_rules! log_error {
    ($target:expr, $($arg:tt)*) => {
        $crate::logging::logf($crate::logging::Level::Error, $target, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:expr, $($arg:tt)*) => {
        $crate::logging::logf($crate::logging::Level::Warn, $target, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($target:expr, $($arg:tt)*) => {
        $crate::logging::logf($crate::logging::Level::Info, $target, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $($arg:tt)*) => {
        $crate::logging::logf($crate::logging::Level::Debug, $target, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($target:expr, $($arg:tt)*) => {
        $crate::logging::logf($crate::logging::Level::Trace, $target, format_args!($($arg)*))
    };
}
