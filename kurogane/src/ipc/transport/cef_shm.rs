use cef::*;

pub const HEADER_SIZE: usize = 8;

pub fn create(name: &str, kind: i32, id: i32, payload: &[u8]) -> Option<ProcessMessage> {
    let total_size = HEADER_SIZE + payload.len();
    let builder = shared_process_message_builder_create(
        Some(&CefString::from(name)),
        total_size,
    )?;
    if builder.is_valid() == 0 {
        return None;
    }
    unsafe {
        let ptr = builder.memory() as *mut u8;
        *(ptr as *mut i32) = kind;
        *(ptr.add(4) as *mut i32) = id;
        std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.add(8), payload.len());
    }
    builder.build()
}

pub fn read_header(region: &SharedMemoryRegion) -> Option<(i32, i32)> {
    if region.is_valid() == 0 || region.size() < HEADER_SIZE {
        return None;
    }
    unsafe {
        let ptr = region.memory() as *const u8;
        Some((*(ptr as *const i32), *(ptr.add(4) as *const i32)))
    }
}

pub fn as_slice(region: &SharedMemoryRegion) -> Option<&[u8]> {
    if region.is_valid() == 0 || region.size() < HEADER_SIZE {
        return None;
    }
    unsafe {
        let ptr = region.memory() as *const u8;
        Some(std::slice::from_raw_parts(ptr.add(8), region.size() - HEADER_SIZE))
    }
}
