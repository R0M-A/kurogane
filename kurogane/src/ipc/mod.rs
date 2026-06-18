pub mod protocol;
pub mod transport {
    pub mod cef_shm;
}
pub mod browser_state;
pub mod renderer_state;
pub mod rpc;
pub mod binary_buffer;
pub mod binary;
pub mod router;
pub mod browser;
pub mod renderer;

// Public exports for the rest of the application
pub use browser::handle_ipc_message;
pub use renderer::IpcRenderProcessHandler;
pub use browser_state::{IpcDispatcher, IpcResult, IpcContext, IpcResponder, BinaryResponder, AsyncIpcHandler, AsyncBinaryHandler};
