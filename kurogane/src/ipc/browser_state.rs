//! Browser-process IPC dispatch and transaction state.
//!
//! Defines the immutable command dispatcher used by the browser process
//! and the runtime state required for active IPC transactions.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::ipc::protocol::IpcId;
use crate::ipc::transport::shm::SharedBuffer;
use crate::browser_registry::BrowserId;

// Handler types

pub type IpcResult = Result<String, String>;
pub type IpcHandler = Box<dyn Fn(&str) -> IpcResult + Send + Sync>;
pub type BinaryHandler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>;

/// Immutable IPC command router.
/// Resolves incoming IPC command names to their registered JSON
/// and binary handlers during browser-process message dispatch.
pub struct IpcDispatcher {
    handlers: HashMap<String, IpcHandler>,
    binary_handlers: HashMap<String, BinaryHandler>,
}

/// Contextual information for an IPC dispatch call.
/// Carries browser identity without modifying handler signatures.
pub struct IpcContext {
    pub browser_id: Option<BrowserId>,
    pub frame_id: Option<i64>,
}

impl IpcDispatcher {
    pub fn new(
        handlers: HashMap<String, IpcHandler>,
        binary_handlers: HashMap<String, BinaryHandler>,
    ) -> Self {
        Self { handlers, binary_handlers }
    }

    pub fn dispatch(&self, command: &str, payload: &str) -> IpcResult {
        match self.handlers.get(command) {
            Some(h) => h(payload),
            None => Err(format!("[IPC] unknown command '{command}'")),
        }
    }

    pub fn dispatch_binary(&self, command: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
        match self.binary_handlers.get(command) {
            Some(h) => h(payload),
            None => Err(format!("[IPC] unknown binary command '{command}'")),
        }
    }

    /// Dispatch a JSON command with browser context.
    /// Currently delegates to dispatch - handlers do not yet receive IpcContext.
    pub fn dispatch_with_context(&self, command: &str, payload: &str, _ctx: IpcContext) -> IpcResult {
        self.dispatch(command, payload)
    }

    /// Dispatch a binary command with browser context.
    /// Currently delegates to dispatch_binary - handlers do not yet receive IpcContext.
    pub fn dispatch_binary_with_context(&self, command: &str, payload: &[u8], _ctx: IpcContext) -> Result<Vec<u8>, String> {
        self.dispatch_binary(command, payload)
    }
}

/// Keep SHM alive until the renderer signals it has finished reading (msg_type 5)
static RESPONSE_SHM_STORE: OnceLock<Mutex<HashMap<IpcId, SharedBuffer>>> = OnceLock::new();

//
// State accessors
//

pub fn response_shm_store() -> &'static Mutex<HashMap<IpcId, SharedBuffer>> {
    RESPONSE_SHM_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}
