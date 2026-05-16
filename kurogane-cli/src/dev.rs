use anyhow::Result;
use std::ffi::OsString;
use std::process::Command;
use std::{env, path::PathBuf};

use crate::tui;

pub fn run(cargo_args: Vec<OsString>) -> Result<()> {
    tui::section("Kurogane Dev");

    // TODO: XLR- leaving this to you, below current workaround for ENV.
    // let version = env!("KUROGANE_CEF_VERSION");
    // let cef = cef_install_dir(version);

    let version = env::var("KUROGANE_CEF_VERSION")?;
    let cef: PathBuf = env::var("CEF_PATH")?.into();

    tui::step("Checking Chromium engine");

    if !cef.exists() {
        tui::warn("Chromium engine not found");
        tui::info("Initiating install process");
        crate::install::run()?;
    } else {
        tui::success("Chromium engine ready");
        tui::field("path", tui::format_path(&cef));
    }

    // Resolve the actual library root (where libcef.so lives)
    let cef_lib_dir = if cef.join("libcef.so").exists() {
        cef.clone()
    } else {
        cef.join(&version).join("cef_linux_x86_64")
    };

    // Pass env to build step
    let mut cmd = Command::new("cargo");

    cmd.arg("run");

    for arg in cargo_args {
        cmd.arg(arg);
    }

    // Set CEF_PATH to the actual library root for the application
    cmd.env("CEF_PATH", &cef_lib_dir);
    cmd.env("KUROGANE_CEF_VERSION", &version);

    //
    // OS-specific runtime linking
    //
    #[cfg(target_os = "linux")]
    {
        let mut ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        ld = format!("{}:{}", cef_lib_dir.display(), ld);
        cmd.env("LD_LIBRARY_PATH", ld);
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = std::env::var("PATH").unwrap_or_default();
        path = format!("{};{}", cef.display(), path);
        cmd.env("PATH", path);
    }

    #[cfg(target_os = "macos")]
    {
        let mut dyld = std::env::var("DYLD_FALLBACK_LIBRARY_PATH").unwrap_or_default();
        dyld = format!("{}:{}", cef.display(), dyld);
        cmd.env("DYLD_FALLBACK_LIBRARY_PATH", dyld);
    }

    println!();
    tui::step("Launching application");

    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Application failed");
    }

    println!();
    tui::success("Application exited");

    Ok(())
}
