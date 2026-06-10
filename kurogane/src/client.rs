//! Browser client implementation.

use cef::*;
use crate::debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::ipc::IpcDispatcher;
use crate::ShutdownSignal;

//
// LifeSpanHandler
//
wrap_life_span_handler! {
    pub struct KuroganeLifeSpanHandler {
        shutdown_signal: ShutdownSignal,
        browser_ref_count: Arc<AtomicUsize>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, _browser: Option<&mut Browser>) {
            self.browser_ref_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            if self.browser_ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
                self.shutdown_signal.request_shutdown();
                debug!("Browser destroyed");
            }
        }
    }
}

//
// LOAD HANDLER
//
wrap_load_handler! {
    pub struct KuroganeLoadHandler;

    impl LoadHandler {
        fn on_load_start(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _transition_type: TransitionType,
        ) {
            if let Some(f) = frame {
                let u: CefString = (&f.url()).into();
                debug!("[LoadHandler] START {}", u.to_string());
            }
        }

        fn on_load_end(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            http_status_code: i32,
        ) {
            if let Some(f) = frame {
                let u: CefString = (&f.url()).into();
                debug!("[LoadHandler] END {} status={}", u.to_string(), http_status_code);
            }
        }

        fn on_load_error(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            error_code: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            let err = error_text.map(|s| s.to_string()).unwrap_or_default();
            let url = failed_url.map(|s| s.to_string()).unwrap_or_default();
            debug!("[LoadHandler] ERROR {:?} '{}' {}", error_code, err, url);
        }
    }
}

//
// CLIENT
//
wrap_client! {
    pub struct KuroganeClient {
        dispatcher: Arc<IpcDispatcher>,
        shutdown_signal: ShutdownSignal,
        browser_ref_count: Arc<AtomicUsize>,
    }

    impl Client {
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(KuroganeLoadHandler::new())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(KuroganeLifeSpanHandler::new(self.shutdown_signal.clone(), self.browser_ref_count.clone()))
        }

        fn on_process_message_received(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> i32 {
            // Only handle messages from renderer
            if source_process != ProcessId::RENDERER {
                return 0;
            }

            let browser = browser.unwrap();
            let frame = frame.unwrap();
            let msg = message.unwrap();

            // Delegate to IPC dispatcher
            if crate::ipc::handle_ipc_message(browser, frame, msg, &self.dispatcher) {
                return 1;
            }

            0
        }
    }
}
