//! Browser-process IPC dispatch and transaction state.
//!
//! Defines the immutable command dispatcher used by the browser process
//! and the runtime state required for active IPC transactions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::Mutex;

use crate::browser_registry::BrowserId;
use crate::browser_info_map::{BrowserInfoMap, BrowserInfoMapVisitor, BrowserInfoMapVisitorResult};
use crate::debug;

// Handler types

pub type IpcResult = Result<String, String>;
pub type IpcHandler = Box<dyn Fn(&str, IpcContext) -> IpcResult + Send + Sync>;
pub type BinaryHandler = Box<dyn Fn(&[u8], IpcContext) -> Result<Vec<u8>, String> + Send + Sync>;

/// Responder for async JSON IPC handlers.
/// Holds a single-use callback that sends the response to the renderer.
/// Thread-safe: can be sent between threads and called exactly once.
pub struct IpcResponder {
    callback: Mutex<Option<Box<dyn FnOnce(IpcResult) + Send>>>,
}

impl IpcResponder {
    pub fn new(callback: Box<dyn FnOnce(IpcResult) + Send>) -> Self {
        Self {
            callback: Mutex::new(Some(callback)),
        }
    }

    /// Send a response. No-op if already called.
    pub fn resolve(&self, result: IpcResult) {
        let cb = self.callback.lock().unwrap().take();
        if let Some(cb) = cb {
            cb(result);
        }
    }
}

/// Responder for async binary IPC handlers.
pub struct BinaryResponder {
    callback: Mutex<Option<Box<dyn FnOnce(Result<Vec<u8>, String>, i32) + Send>>>,
}

impl BinaryResponder {
    pub fn new(callback: Box<dyn FnOnce(Result<Vec<u8>, String>, i32) + Send>) -> Self {
        Self {
            callback: Mutex::new(Some(callback)),
        }
    }

    /// Send a binary response with an error code (0 = success, -1 = panic, etc.).
    pub fn resolve(&self, result: Result<Vec<u8>, String>, error_code: i32) {
        let cb = self.callback.lock().unwrap().take();
        if let Some(cb) = cb {
            cb(result, error_code);
        }
    }
}

pub type AsyncIpcHandler = Box<dyn Fn(serde_json::Value, IpcResponder, IpcContext) + Send + Sync>;
pub type AsyncBinaryHandler = Box<dyn Fn(Vec<u8>, BinaryResponder, IpcContext) + Send + Sync>;

/// A pending async handler entry that can be cancelled.
#[derive(Clone)]
pub struct PendingEntry {
    pub aborted: Arc<AtomicBool>,
}

/// Immutable IPC command router.
/// Resolves incoming IPC command names to their registered JSON
/// and binary handlers during browser-process message dispatch.
pub struct IpcDispatcher {
    handlers: HashMap<String, IpcHandler>,
    binary_handlers: HashMap<String, BinaryHandler>,
    async_handlers: HashMap<String, AsyncIpcHandler>,
    async_binary_handlers: HashMap<String, AsyncBinaryHandler>,
    pending: Mutex<BrowserInfoMap<i32, PendingEntry>>,
}

/// Contextual information for an IPC dispatch call.
/// Carries browser identity without modifying handler signatures.
pub struct IpcContext {
    pub browser_id: Option<BrowserId>,
    pub frame_id: Option<i64>,
}

struct CancelAllVisitor {
    count: AtomicUsize,
}

impl BrowserInfoMapVisitor<i32, PendingEntry> for CancelAllVisitor {
    fn on_next_info(
        &self,
        _browser_id: BrowserId,
        _key: i32,
        value: &PendingEntry,
    ) -> ControlFlow<BrowserInfoMapVisitorResult, BrowserInfoMapVisitorResult> {
        value.aborted.store(true, Ordering::SeqCst);
        self.count.fetch_add(1, Ordering::Relaxed);

        ControlFlow::Continue(BrowserInfoMapVisitorResult::RemoveEntry)
    }
}

impl IpcDispatcher {
    pub fn new(
        handlers: HashMap<String, IpcHandler>,
        binary_handlers: HashMap<String, BinaryHandler>,
    ) -> Self {
        Self {
            handlers,
            binary_handlers,
            async_handlers: HashMap::new(),
            async_binary_handlers: HashMap::new(),
            pending: Mutex::new(BrowserInfoMap::default()),
        }
    }

    pub fn with_async(
        handlers: HashMap<String, IpcHandler>,
        binary_handlers: HashMap<String, BinaryHandler>,
        async_handlers: HashMap<String, AsyncIpcHandler>,
        async_binary_handlers: HashMap<String, AsyncBinaryHandler>,
    ) -> Self {
        Self {
            handlers,
            binary_handlers,
            async_handlers,
            async_binary_handlers,
            pending: Mutex::new(BrowserInfoMap::default()),
        }
    }

    /// Dispatch a JSON command with browser context.
    pub fn dispatch(&self, command: &str, payload: &str, ctx: IpcContext) -> IpcResult {
        match self.handlers.get(command) {
            Some(h) => h(payload, ctx),
            None => Err(format!("[IPC] unknown command '{command}'")),
        }
    }

    /// Dispatch a binary command with browser context.
    pub fn dispatch_binary(&self, command: &str, payload: &[u8], ctx: IpcContext) -> Result<Vec<u8>, String> {
        match self.binary_handlers.get(command) {
            Some(h) => h(payload, ctx),
            None => Err(format!("[IPC] unknown binary command '{command}'")),
        }
    }

    /// Check if a command is registered as an async handler.
    pub fn is_async(&self, command: &str) -> bool {
        self.async_handlers.contains_key(command)
    }

    /// Check if a binary command is registered as an async handler.
    pub fn is_async_binary(&self, command: &str) -> bool {
        self.async_binary_handlers.contains_key(command)
    }

    /// Dispatch an async JSON command with browser context. Returns true if handled.
    pub fn dispatch_async(&self, command: &str, payload: &str, responder: IpcResponder, ctx: IpcContext) -> bool {
        if let Some(handler) = self.async_handlers.get(command) {
            let value: serde_json::Value = if payload.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_str(payload) {
                    Ok(v) => v,
                    Err(e) => {
                        responder.resolve(Err(format!("invalid JSON payload: {e}")));
                        return true;
                    }
                }
            };
            handler(value, responder, ctx);
            true
        } else {
            false
        }
    }

    /// Dispatch an async binary command with browser context. Returns true if handled.
    pub fn dispatch_async_binary(&self, command: &str, payload: &[u8], responder: BinaryResponder, ctx: IpcContext) -> bool {
        if let Some(handler) = self.async_binary_handlers.get(command) {
            handler(payload.to_vec(), responder, ctx);
            true
        } else {
            false
        }
    }

    // Pending handler registry

    pub fn insert_pending(&self, browser_id: Option<BrowserId>, id: i32, entry: PendingEntry) {
        if let Some(bid) = browser_id {
            self.pending.lock().unwrap().insert(bid, id, entry);
        }
    }

    pub fn remove_pending(&self, browser_id: Option<BrowserId>, id: i32) {
        if let Some(bid) = browser_id {
            self.pending.lock().unwrap().remove(bid, id);
        }
    }

    /// Cancel a pending handler by browser and id. Returns true if found.
    pub fn cancel_pending(&self, browser_id: Option<BrowserId>, id: i32) -> bool {
        if let Some(bid) = browser_id {
            if let Some(entry) = self.pending.lock().unwrap().remove(bid, id) {
                entry.aborted.store(true, Ordering::SeqCst);
                debug!("[IPC Browser] canceled pending id={}", id);
                return true;
            }
        }
        false
    }

    /// Cancel all pending handlers for a given browser.
    pub fn cancel_all_for_browser(&self, browser_id: BrowserId) -> usize {
        let visitor = CancelAllVisitor {
            count: AtomicUsize::new(0),
        };
        self.pending.lock().unwrap().find_browser_all(browser_id, &visitor);
        let count = visitor.count.load(Ordering::Relaxed);
        if count > 0 {
            debug!("[IPC Browser] canceled {} pending handlers for browser", count);
        }
        count
    }
}
