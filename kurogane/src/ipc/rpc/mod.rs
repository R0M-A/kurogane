//! Request/response IPC subsystem.
//!
//! Supports synchronous and asynchronous command handlers, pending
//! request tracking, cancellation and promise resolution.
//! Handles both JSON string and arbitrary binary payloads.

use crate::ipc::browser_state::{IpcContext, IpcResult};
use crate::ipc::responder::Responder;

pub type SyncRpcHandler = Box<dyn Fn(&str, IpcContext) -> IpcResult + Send + Sync>;
pub type AsyncRpcHandler = Box<dyn Fn(serde_json::Value, IpcResponder, IpcContext) + Send + Sync>;
pub type IpcResponder = Responder<String>;

pub type SyncBinaryHandler = Box<dyn Fn(&[u8], IpcContext) -> Result<Vec<u8>, String> + Send + Sync>;
pub type AsyncBinaryHandler = Box<dyn Fn(&[u8], BinaryResponder, IpcContext) + Send + Sync>;
pub type BinaryResponder = Responder<Vec<u8>>;

pub mod renderer;
