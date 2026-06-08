use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;

use cef::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageLoopMode {
    #[default]
    Blocking,
    Pump,
}

#[derive(Clone)]
pub struct ShutdownSignal {
    inner: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn request_shutdown(&self) {
        self.inner.store(true, Ordering::Release);
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.inner.load(Ordering::Acquire)
    }
}

const PUMP_SLEEP: Duration = Duration::from_millis(1);

pub fn run(mode: MessageLoopMode, shutdown: &ShutdownSignal) {
    match mode {
        MessageLoopMode::Blocking => {
            run_message_loop();
        }

        MessageLoopMode::Pump => {
            while !shutdown.is_shutdown_requested() {
                do_message_loop_work();
                std::thread::sleep(PUMP_SLEEP);
            }
        }
    }
}
