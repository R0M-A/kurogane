//! Native window delegate.
//!
//! Controls how the native window behaves and embeds the
//! browser view into the platform window.

use cef::*;
use std::sync::{Arc, Mutex};

use crate::debug;
use crate::browser_registry::{BrowserRegistry, BrowserType};

wrap_window_delegate! {
    pub struct KuroganeWindowDelegate {
        browser_view: BrowserView,
        window_ref: Arc<Mutex<Option<Window>>>,
    }

    impl ViewDelegate {
        fn on_child_view_changed(
            &self,
            _view: Option<&mut View>,
            _added: ::std::os::raw::c_int,
            _child: Option<&mut View>,
        ) {
            // Intentionally unused
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            if let Some(window) = window {
                let view = self.browser_view.clone();
                window.add_child_view(Some(&mut (&view).into()));
                window.show();
                debug!("Window shown");

                // store live window reference so the browser process keeps
                // an owning handle and we can clear it on destroy
                *self.window_ref.lock().unwrap() = Some(window.clone());
            }
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            debug!("Window destroyed");
            // clear stored window reference to avoid use-after-destroy
            *self.window_ref.lock().unwrap() = None;
            quit_message_loop();
        }

        fn with_standard_window_buttons(
            &self,
            _window: Option<&mut Window>,
        ) -> ::std::os::raw::c_int {
            1
        }

        fn can_resize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_maximize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_minimize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_close(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }
    }
}

wrap_browser_view_delegate! {
    pub struct KuroganeBrowserViewDelegate {
        registry: Arc<Mutex<BrowserRegistry>>,
    }

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn on_popup_browser_view_created(
            &self,
            _browser_view: Option<&mut BrowserView>,
            popup_browser_view: Option<&mut BrowserView>,
            _is_devtools: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int {
            debug!("[BrowserViewDelegate] popup browser view created");

            if let Some(pbv) = popup_browser_view {
                // Register the popup browser before it hits on_after_created
                if let Some(browser) = pbv.browser() {
                    let mut reg = self.registry.lock().unwrap();
                    reg.register(browser.clone(), BrowserType::Popup, None);
                    debug!("[BrowserViewDelegate] registered popup browser");
                }

                // Create the popup window with a minimal delegate
                let bv_clone = pbv.clone();
                let mut delegate = KuroganePopupDelegate::new(bv_clone);
                if let Some(window) = window_create_top_level(Some(&mut delegate)) {
                    window.show();
                    debug!("[BrowserViewDelegate] popup window created and shown");
                    return 1;
                }
            }

            0
        }
    }
}

wrap_window_delegate! {
    pub struct KuroganePopupDelegate {
        browser_view: BrowserView,
    }

    impl ViewDelegate {}

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            if let Some(window) = window {
                let view = self.browser_view.clone();
                window.add_child_view(Some(&mut (&view).into()));
                window.show();
                debug!("Popup window shown");
            }
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            debug!("Popup window destroyed");
        }

        fn with_standard_window_buttons(
            &self,
            _window: Option<&mut Window>,
        ) -> ::std::os::raw::c_int {
            1
        }

        fn can_resize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_maximize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_minimize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_close(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }
    }
}
