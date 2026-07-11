use std::sync::{Arc, OnceLock};

use crate::runtime::AppHandle;

/// A lazily-populated handle, shared between the App builder and handlers.
///
/// Populated synchronously by App::build() before the message loop starts,
/// so it is always initialized by the time any handler is invoked.
#[derive(Clone)]
pub struct AppCell(Arc<OnceLock<AppHandle>>);

impl AppCell {
    pub fn new() -> (Self, AppCellResolver) {
        let inner = Arc::new(OnceLock::new());
        (AppCell(inner.clone()), AppCellResolver(inner))
    }

    /// Returns the handle. Panics if called before App::build().
    /// In practice this never happens because handlers only run after startup.
    pub fn get(&self) -> &AppHandle {
        self.0.get().expect("AppHandle not yet initialized")
    }
}

pub struct AppCellResolver(Arc<OnceLock<AppHandle>>);

impl AppCellResolver {
    pub(crate) fn resolve(self, handle: AppHandle) {
        self.0.set(handle).ok();
    }
}
