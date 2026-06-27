use cef::*;

/// Create a V8 ArrayBuffer by copying bytes into a new backing store.
pub fn create_array_buffer_from_bytes(payload: &[u8]) -> Option<V8Value> {
    let mut store = v8_backing_store_create(payload.len())?;

    if store.is_valid() == 0 {
        return None;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            payload.as_ptr(),
            store.data() as *mut u8,
            payload.len(),
        );
    }

    v8_value_create_array_buffer_from_backing_store(Some(&mut store))
}
