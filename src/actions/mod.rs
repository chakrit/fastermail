pub mod email;
pub mod identity;
pub mod mailbox;
pub mod masked_email;
pub mod vacation;

use crate::error::Result;
use crate::jmap::client::JmapClient;
use crate::mcp::types::Tool;
use crate::recorder::Recorder;

/// Context passed to all actions.
pub struct Context {
    pub jmap: JmapClient,
    pub account_id: String,
    pub recorder: Option<Recorder>,
}

/// Every MCP tool implements this trait.
pub trait Action {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value>;
}

/// Return the list of all registered tool definitions.
pub fn tool_definitions() -> Vec<Tool> {
    let mut tools = Vec::new();

    tools.extend(mailbox::tools());
    tools.extend(email::tools());
    tools.extend(vacation::tools());
    tools.extend(identity::tools());
    tools.extend(masked_email::tools());

    tools
}
