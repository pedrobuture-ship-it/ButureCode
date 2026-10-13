use crate::{
    Host,
    engine::{Role, ToolSpec, ToolchainProvider, child_bins},
};
use std::path::PathBuf;
pub struct Rust;
impl ToolchainProvider for Rust {
    fn language(&self) -> &'static str {
        "Rust"
    }
    fn specifications(&self) -> Vec<ToolSpec> {
        use Role::*;
        vec![
            ToolSpec::new("rustc", Compiler),
            ToolSpec::new("cargo", PackageManager),
            ToolSpec::new("rustup", VersionManager),
            ToolSpec::new("rust-analyzer", LanguageServer),
            ToolSpec::new("rustfmt", Formatter),
            ToolSpec::new("cargo-clippy", Formatter),
            ToolSpec::new("gdb", Debugger),
            ToolSpec::new("lldb", Debugger),
        ]
    }
    fn extra_directories(&self, host: &Host) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(h) = &host.home {
            paths.push(h.join(".cargo/bin"));
            paths.extend(child_bins(h.join(".rustup/toolchains")));
        }
        if let Some(h) = std::env::var_os("RUSTUP_HOME") {
            paths.extend(child_bins(PathBuf::from(h).join("toolchains")));
        }
        if let Some(h) = std::env::var_os("CARGO_HOME") {
            paths.push(PathBuf::from(h).join("bin"));
        }
        paths
    }
}
