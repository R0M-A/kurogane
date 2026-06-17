//! CEF IPC message browser process entrypoint
//!
//! Boundary between CEF's message system and the IPC infrastructure.

use std::sync::Arc;
use cef::*;
use crate::debug;
use crate::ipc::protocol::IpcMsgKind;
use crate::ipc::transport::cef_shm;
use crate::ipc::browser_state::{IpcDispatcher, IpcContext};
use crate::ipc::binary;
use crate::browser_registry::BrowserId;

pub fn handle_ipc_message(
    _browser: &mut Browser,
    frame: &mut Frame,
    message: &mut ProcessMessage,
    dispatcher: &Arc<IpcDispatcher>,
    browser_id: Option<BrowserId>,
) -> bool {
    let name: CefString = (&message.name()).into();
    if name.to_string() != "ipc" {
        return false;
    }

    // SHM-backed messages have no ListValue; dispatch by SHM header
    if let Some(region) = message.shared_memory_region() {
        if let Some((kind, id)) = cef_shm::read_header(&region) {
            match kind {
                k if k == IpcMsgKind::BinaryInvoke as i32 => {
                    if let Some(payload) = cef_shm::as_slice(&region) {
                        if payload.len() < 2 {
                            debug!("[IPC Browser] SHM BinaryInvoke payload too small");
                            return true;
                        }
                        let cmd_len = u16::from_le_bytes([payload[0], payload[1]]) as usize;
                        if 2 + cmd_len > payload.len() {
                            debug!("[IPC Browser] SHM BinaryInvoke invalid cmd_len");
                            return true;
                        }
                        let cmd = String::from_utf8_lossy(&payload[2..2 + cmd_len]).to_string();
                        let data = &payload[2 + cmd_len..];

                        debug!(
                            "[IPC Browser] binary invoke (SHM): '{}' (id={}, {} bytes)",
                            cmd, id, data.len()
                        );

                        let ctx = IpcContext {
                            browser_id,
                            frame_id: None,
                        };
                        binary::handle_invoke(frame, id, cmd, data, dispatcher, ctx);
                    }
                    return true;
                }
                _ => {
                    debug!("[IPC Browser] unexpected SHM message kind {}", kind);
                }
            }
        }
        return true;
    }

    // Inline message via ListValue
    let Some(args) = message.argument_list() else {
        debug!("[IPC Browser] missing argument list");
        return false;
    };

    let ctx = IpcContext {
        browser_id,
        frame_id: None,
    };
    crate::ipc::router::route_browser(frame, &args, dispatcher, ctx)
}
