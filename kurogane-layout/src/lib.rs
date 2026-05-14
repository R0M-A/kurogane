mod discover;
mod layout;
mod platform;
mod profile;
mod validate;
mod bundle;

pub use discover::{
    detect_cef_root,
    DetectError,
    DetectedCef,
    DiscoveryMode,
};

pub use layout::{
    bundled_cef_root,
    installed_cef_root,
    cef_install_dir,
    install_root,
};

pub use profile::{
    cache_root,
    profile_dir,
};

pub use validate::{
    validate_cef_root,
    CefValidationError,
};

pub use bundle::BundleLayout;
