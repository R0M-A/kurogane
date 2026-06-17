//! Binary transfer subsystem with automatic SHM threshold
//!
//! Handles binary request/response with automatic switching between
//! inline and shared memory based on payload size.

use cef::*;
use std::sync::Arc;
use std::panic::{catch_unwind, AssertUnwindSafe};
use crate::ipc::protocol::IpcMsgKind;
use crate::ipc::transport::cef_shm;
use crate::ipc::binary_buffer::SharedBinary;
use crate::ipc::renderer_state::registry;
use crate::ipc::browser_state::IpcContext;
use crate::ipc::IpcDispatcher;
use crate::debug;

/// Inline/SHM threshold: payloads >= this size use CEF shared memory.
pub const SHM_THRESHOLD: usize = 16 * 1024; // 16KB

// BROWSER SIDE

pub fn handle_invoke(
    frame: &mut Frame,
    id: i32,
    command: String,
    data: &[u8],
    dispatcher: &Arc<IpcDispatcher>,
    ctx: IpcContext,
) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        dispatcher.dispatch_binary_with_context(&command, data, ctx)
    }))
    .unwrap_or_else(|_| Err("Binary handler panicked".to_string()));

    send_response(frame, id, result);
}

pub fn send_error(frame: &Frame, id: i32, err: String) {
    if frame.is_valid() == 0 {
        return;
    }

    let mut msg = process_message_create(Some(&CefString::from("ipc"))).unwrap();
    let mut args = msg.argument_list().unwrap();

    crate::ipc::protocol::set_kind(&mut args, IpcMsgKind::Reject);
    args.set_int(1, id);
    args.set_string(2, Some(&CefString::from(err.as_str())));

    frame.send_process_message(ProcessId::RENDERER, Some(&mut msg));
}

pub fn send_response(
    frame: &Frame,
    id: i32,
    result: Result<Vec<u8>, String>,
) {
    // Guard against destroyed frames
    if frame.is_valid() == 0 {
        debug!(
            "[IPC Browser] frame destroyed before binary response id={}",
            id
        );
        return;
    }

    match result {
        Ok(data) => {
            if data.len() < SHM_THRESHOLD {
                debug!("[IPC Browser] inline binary response: {} bytes", data.len());

                let mut msg = process_message_create(Some(&CefString::from("ipc"))).unwrap();
                let mut args = msg.argument_list().unwrap();
                crate::ipc::protocol::set_kind(&mut args, IpcMsgKind::BinaryResponse);
                args.set_int(1, id);

                let mut binary = binary_value_create(Some(data.as_slice())).unwrap();
                args.set_binary(2, Some(&mut binary));

                frame.send_process_message(ProcessId::RENDERER, Some(&mut msg));
            } else {
                debug!("[IPC Browser] SHM binary response: {} bytes", data.len());

                let mut msg = match cef_shm::create(
                    "ipc",
                    IpcMsgKind::BinaryResponse as i32,
                    id,
                    &data,
                ) {
                    Some(m) => m,
                    None => return send_error(frame, id, "CEF SHM creation failed".into()),
                };

                frame.send_process_message(ProcessId::RENDERER, Some(&mut msg));
            }
        }

        Err(err) => {
            let mut msg = process_message_create(Some(&CefString::from("ipc"))).unwrap();
            let mut args = msg.argument_list().unwrap();
            crate::ipc::protocol::set_kind(&mut args, IpcMsgKind::Reject);
            args.set_int(1, id);
            args.set_string(2, Some(&CefString::from(err.as_str())));
            frame.send_process_message(ProcessId::RENDERER, Some(&mut msg));
        }
    }
}

// RENDERER SIDE

/// Handle inline binary response (small payload, sent via ListValue BinaryValue).
pub fn handle_response(_frame: &mut Frame, id: i32, args: &ListValue) {
    if let Some(binary) = args.binary(2) {
        let size = binary.size();
        let mut buf = vec![0u8; size];
        let written = binary.data(Some(&mut buf), 0);
        buf.truncate(written);

        debug!("[IPC Renderer] inline binary response: {} bytes", written);
        resolve_binary(id, &buf);
    }
}

/// Handle SHM-backed binary response (large payload, zero-copy to V8).
pub fn handle_response_shm(_frame: &mut Frame, region: &SharedMemoryRegion, id: i32) {
    let buffer: SharedBinary = Arc::new(
        crate::ipc::binary_buffer::ShmBinary::new(region.clone(), cef_shm::HEADER_SIZE)
    );

    debug!(
        "[IPC Renderer] SHM binary response: {} bytes",
        buffer.data().len()
    );

    resolve_binary_shm(id, buffer);
}

fn resolve_binary(id: i32, payload: &[u8]) {
    let entry = registry().lock().unwrap().take(id);

    if let Some((context, promise)) = entry {
        if context.enter() == 0 {
            eprintln!("[IPC] Failed to enter V8 context for binary promise id={}", id);
            return;
        }

        let mut buf = v8_value_create_array_buffer_with_copy(
            payload.as_ptr() as *mut u8,
            payload.len(),
        ).unwrap();

        promise.resolve_promise(Some(&mut buf));
        context.exit(); // safe; lock not held
    }
}

wrap_v8_array_buffer_release_callback! {
    struct BinaryBufRelease {
        holder: SharedBinary,
    }
    impl V8ArrayBufferReleaseCallback {
        fn release_buffer(&self, _buffer: *mut u8) {}
    }
}

fn resolve_binary_shm(id: i32, buffer: SharedBinary) {
    let entry = registry().lock().unwrap().take(id);

    if let Some((context, promise)) = entry {
        if context.enter() == 0 {
            eprintln!("[IPC] Failed to enter V8 context for binary promise id={}", id);
            return;
        }

        let ptr = buffer.data().as_ptr() as *mut u8;
        let len = buffer.data().len();

        let mut release = BinaryBufRelease::new(buffer);

        let mut arr = v8_value_create_array_buffer(
            ptr,
            len,
            Some(&mut release),
        ).unwrap();

        promise.resolve_promise(Some(&mut arr));
        context.exit();
    }
}
