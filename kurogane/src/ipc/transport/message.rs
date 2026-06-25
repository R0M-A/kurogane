use std::sync::Arc;

use cef::*;

use crate::ipc::envelope::*;
use crate::ipc::binary_buffer::{ShmBinary};
use crate::ipc::binary_buffer::SharedBinary;

/// Minimum message size for shared-memory transport.
///
/// Smaller messages are sent inline.
pub const SHM_THRESHOLD: usize = 16 * 1024;

/// Message data received over the CEF transport.
///
/// The payload may be backed by inline storage or shared memory.
pub enum ReceivedMessage {
    /// Inline message data.
    Inline(Vec<u8>),
    /// Shared-memory-backed message data.
    Shm(SharedBinary),
}

impl ReceivedMessage {
    /// Returns the serialized message.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Inline(v) => v.as_slice(),
            Self::Shm(b) => b.data(),
        }
    }
}

/// Builds a ProcessMessage from an envelope and payload.
///
/// Uses shared-memory transport for large messages and falls back to
/// inline transport if shared memory is unavailable.
pub fn build_message(
    name: &str,
    envelope: &Envelope,
    payload: &[u8],
) -> Option<ProcessMessage> {
    build_message_parts(name, envelope, &[payload])
}

/// Builds a ProcessMessage from an envelope and payload segments.
///
/// The payload is assembled directly from the provided slices.
pub fn build_message_parts(
    name: &str,
    envelope: &Envelope,
    parts: &[&[u8]],
) -> Option<ProcessMessage> {
    let total_payload: usize = parts.iter().map(|p| p.len()).sum();
    let total_size = ENVELOPE_SIZE + total_payload;

    if total_size < SHM_THRESHOLD {
        build_inline_parts(name, envelope, parts)
    } else {
        build_shm_parts(name, envelope, parts)
            .or_else(|| build_inline_parts(name, envelope, parts))
    }
}

fn build_inline_parts(
    name: &str,
    envelope: &Envelope,
    parts: &[&[u8]],
) -> Option<ProcessMessage> {
    let msg = process_message_create(Some(&CefString::from(name)))?;
    let args = msg.argument_list()?;

    let total_payload: usize = parts.iter().map(|p| p.len()).sum();
    let total_size = ENVELOPE_SIZE + total_payload;
    let mut buf = Vec::with_capacity(total_size);
    buf.extend_from_slice(&encode_envelope_bytes(envelope));
    for part in parts {
        buf.extend_from_slice(part);
    }

    let mut binary = binary_value_create(Some(&buf))?;
    args.set_binary(0, Some(&mut binary));
    args.set_int(1, ENVELOPE_VERSION as i32);

    Some(msg)
}

fn build_shm_parts(
    name: &str,
    envelope: &Envelope,
    parts: &[&[u8]],
) -> Option<ProcessMessage> {
    let total_payload: usize = parts.iter().map(|p| p.len()).sum();
    let total_size = ENVELOPE_SIZE + total_payload;
    let builder = shared_process_message_builder_create(
        Some(&CefString::from(name)),
        total_size,
    )?;
    if builder.is_valid() == 0 {
        return None;
    }

    unsafe {
        let ptr = builder.memory() as *mut u8;
        let env_bytes = encode_envelope_bytes(envelope);
        std::ptr::copy_nonoverlapping(env_bytes.as_ptr(), ptr, ENVELOPE_SIZE);
        let mut offset = ENVELOPE_SIZE;
        for part in parts {
            std::ptr::copy_nonoverlapping(part.as_ptr(), ptr.add(offset), part.len());
            offset += part.len();
        }
    }

    builder.build()
}

/// Extracts a serialized message from a ProcessMessage.
///
/// Returns either shared-memory-backed or inline storage.
pub fn extract_message(message: &ProcessMessage) -> Option<ReceivedMessage> {
    // SHM path: zero-copy from shared memory
    if let Some(region) = message.shared_memory_region() {
        if region.is_valid() != 0 && region.size() >= ENVELOPE_SIZE {
            return Some(ReceivedMessage::Shm(Arc::new(ShmBinary::new(region, 0))));
        }
    }

    // Inline path: read BinaryValue from argument list
    let args = message.argument_list()?;
    let version = args.int(1);
    if version != ENVELOPE_VERSION as i32 {
        return None;
    }
    let binary = args.binary(0)?;
    let size = binary.size();
    let mut buf = vec![0u8; size];
    let written = binary.data(Some(&mut buf), 0);
    buf.truncate(written);

    Some(ReceivedMessage::Inline(buf))
}
