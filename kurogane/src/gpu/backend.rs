use cef::*;

use super::detection::RenderingEnvironment;

#[cfg(target_os = "linux")]
use super::linux as platform;

#[cfg(target_os = "windows")]
use super::windows as platform;

#[cfg(target_os = "macos")]
use super::macos as platform;

/// GPU backend selection strategy.
///
/// Pass to App::gpu_mode to control how Chromium selects its rendering backend (default = GpuMode::Auto).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GpuMode {
    /// Automatically select a backend
    #[default]
    Auto,

    /// Use hardware acceleration
    Hardware,

    /// Use SwiftShader software rendering
    Software,

    /// Disable GPU entirely. No canvas acceleration, no WebGL
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResolvedGpuMode {
    Hardware,
    Software,
    Disabled,
}

/// Apply Chromium command-line flags for the configured GPU mode
pub(crate) fn apply(
    cmd: &mut CommandLine,
    requested: GpuMode,
) {
    let env = RenderingEnvironment::detect();

    let mode = resolve(requested, &env);

    match mode {
        ResolvedGpuMode::Hardware => platform::apply_hardware(cmd),
        ResolvedGpuMode::Software => apply_software(cmd),
        ResolvedGpuMode::Disabled => apply_disabled(cmd),
    }
}

fn resolve(
    requested: GpuMode,
    env: &RenderingEnvironment,
) -> ResolvedGpuMode {
    match requested {
        GpuMode::Auto => resolve_auto(env),

        GpuMode::Hardware => ResolvedGpuMode::Hardware,

        GpuMode::Software => ResolvedGpuMode::Software,

        GpuMode::Disabled => ResolvedGpuMode::Disabled,
    }
}

fn resolve_auto(env: &RenderingEnvironment) -> ResolvedGpuMode {
    if env.is_virtual_gpu {
        ResolvedGpuMode::Software
    } else {
        ResolvedGpuMode::Hardware
    }
}

fn gpu_off(cmd: &mut CommandLine) {
    cmd.append_switch(Some(&CefString::from("disable-gpu")));
    cmd.append_switch(Some(&CefString::from("disable-gpu-compositing")));
}

fn apply_software(cmd: &mut CommandLine) {
    gpu_off(cmd);

    cmd.append_switch_with_value(
        Some(&CefString::from("use-gl")),
        Some(&CefString::from("swiftshader")),
    );
}

fn apply_disabled(cmd: &mut CommandLine) {
    gpu_off(cmd);

    cmd.append_switch(Some(&CefString::from("disable-software-rasterizer")));
}
