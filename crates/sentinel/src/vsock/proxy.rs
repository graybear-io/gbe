use crate::error::SentinelError;

/// Tool call proxy (phase 3) and CONNECT proxy (phase 2).
///
/// Phase 2: Forward HTTP CONNECT requests from operative through sentinel
/// to allowlisted endpoints only.
///
/// Phase 3: Operative sends structured tool-call requests over vsock.
/// Sentinel validates against task-scoped policy, executes on behalf of
/// operative, returns result. VM has no network capability.
pub struct ToolProxy;

impl ToolProxy {
    /// # Errors
    ///
    /// Returns `SentinelError::Vm` on policy violation or execution failure.
    pub async fn handle_tool_call(
        &self,
        _tool: &str,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, SentinelError> {
        // TODO: validate against tool policy, execute call, return result
        Err(SentinelError::Vm("tool proxy not implemented".into()))
    }
}
