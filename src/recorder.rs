use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Records request/response pairs to disk as JSON files.
///
/// Enabled by setting `FASTERMAIL_RECORD_DIR` to a directory path.
/// Each interaction is saved as a timestamped JSON file for later use as test data.
pub struct Recorder {
    dir: PathBuf,
}

impl Recorder {
    /// Create a recorder if `FASTERMAIL_RECORD_DIR` is set.
    pub fn from_env() -> Option<Self> {
        let dir = std::env::var("FASTERMAIL_RECORD_DIR").ok()?;
        if dir.is_empty() {
            return None;
        }

        let dir = PathBuf::from(dir);
        if let Err(e) = fs::create_dir_all(&dir) {
            log_warn!("recorder", "failed to create directory {}: {e}", dir.display());
            return None;
        }

        log_info!("recorder", "recording to {}", dir.display());
        Some(Self { dir })
    }

    /// Record a JMAP HTTP exchange.
    pub fn record_jmap(
        &self,
        method_name: &str,
        request: &serde_json::Value,
        response: &serde_json::Value,
    ) {
        let entry = serde_json::json!({
            "type": "jmap",
            "timestamp": timestamp(),
            "method": method_name,
            "request": request,
            "response": response,
        });

        self.write_file("jmap", method_name, &entry);
    }

    /// Record an incoming MCP message (from client).
    pub fn record_mcp_request(&self, method: &str, raw: &str) {
        let parsed: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();

        let entry = serde_json::json!({
            "type": "mcp_request",
            "timestamp": timestamp(),
            "method": method,
            "message": parsed,
        });

        self.write_file("mcp_req", method, &entry);
    }

    /// Record an outgoing MCP response (to client).
    pub fn record_mcp_response(&self, method: &str, raw: &str) {
        let parsed: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();

        let entry = serde_json::json!({
            "type": "mcp_response",
            "timestamp": timestamp(),
            "method": method,
            "message": parsed,
        });

        self.write_file("mcp_resp", method, &entry);
    }

    fn write_file(&self, prefix: &str, method: &str, entry: &serde_json::Value) {
        let ts = timestamp_filename();
        let safe_method = method.replace('/', "_");
        let filename = format!("{ts}_{prefix}_{safe_method}.json");
        let path = self.dir.join(filename);

        let json = match serde_json::to_string_pretty(entry) {
            Ok(j) => j,
            Err(e) => {
                log_warn!("recorder", "serialize error: {e}");
                return;
            }
        };

        if let Err(e) = fs::write(&path, json) {
            log_warn!("recorder", "write error {}: {e}", path.display());
        }
    }
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("{}.{:03}", d.as_secs(), d.subsec_millis()))
        .unwrap_or_else(|_| "0".to_string())
}

fn timestamp_filename() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("{}_{:06}", d.as_secs(), d.subsec_micros()))
        .unwrap_or_else(|_| "0_000000".to_string())
}
