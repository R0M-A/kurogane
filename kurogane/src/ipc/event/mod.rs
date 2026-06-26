//! Event IPC subsystem.
//!
//! Provides publish/subscribe event delivery between the browser and renderer processes.
//! Renderers subscribe to named events and the browser emits events to subscribed frames.

use std::collections::HashMap;
use std::sync::Mutex;

use cef::Frame;

use crate::browser_registry::BrowserId;

/// Browser-side event subscription.
pub struct EventSubscription {
    pub frame: Frame,
    pub browser_id: BrowserId,
    pub persistent: bool,
}

pub mod browser;
pub mod renderer;

/// Browser-side event dispatcher.
pub struct EventSubsystem {
    pub subscriptions: Mutex<HashMap<String, Vec<EventSubscription>>>,
}

impl EventSubsystem {
    pub fn new() -> Self {
        Self {
            subscriptions: Mutex::new(HashMap::new()),
        }
    }

    /// Removes all subscriptions associated with a browser.
    ///
    /// Returns the number of subscriptions removed.
    pub fn clear_for_browser(&self, browser_id: BrowserId) -> usize {
        let mut subs = self.subscriptions.lock().unwrap();
        let mut total = 0;
        subs.retain(|_, v| {
            let before = v.len();
            v.retain(|s| s.browser_id != browser_id);
            let removed = before - v.len();
            total += removed;
            !v.is_empty()
        });
        total
    }
}
