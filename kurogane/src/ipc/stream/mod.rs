//! Stream IPC subsystem.
//!
//! Provides streaming IPC from the renderer to the browser. Streams are
//! identified by a correlation ID and consist of open, data, end and error messages.

use std::collections::HashMap;

use crate::browser_registry::BrowserId;
use crate::ipc::browser_state::IpcContext;

/// Application handler for an incoming stream.
///
/// Invoked for each data chunk and once more when the stream ends.
pub type StreamHandler =
    Box<dyn Fn(u32, &[u8], bool, &str, IpcContext) -> Result<(), String> + Send + Sync>;

pub mod browser;

/// Browser-side stream manager.
pub struct StreamSubsystem {
    pub handlers: HashMap<String, StreamHandler>,
    // Active streams keyed by stream ID
    pub streams: std::sync::Mutex<HashMap<u32, (String, BrowserId)>>,
    pub pending: crate::ipc::rpc::PendingMap,
}

impl StreamSubsystem {
    pub fn new(handlers: HashMap<String, StreamHandler>) -> Self {
        Self {
            handlers,
            streams: std::sync::Mutex::new(HashMap::new()),
            pending: crate::ipc::rpc::PendingMap::new(),
        }
    }

    /// Remove all streams for a given browser.
    pub fn clear_for_browser(&self, browser_id: BrowserId) -> usize {
        let mut streams = self.streams.lock().unwrap();
        let before = streams.len();
        streams.retain(|_, (_, bid)| *bid != browser_id);
        before - streams.len()
    }
}
