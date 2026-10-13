use crate::{
    Host,
    engine::{Role, ToolSpec, ToolchainProvider, child_bins},
};
use std::path::PathBuf;
pub struct Python;
impl ToolchainProvider for Python {
    fn language(&self) -> &'static str {
        "Python"
    }
    fn specifications(&self) -> Vec<ToolSpec> {
        use Role::*;
        vec![
            ToolSpec::new("python", Runtime),
            ToolSpec::new("python3", Runtime),
            ToolSpec::new("pip", PackageManager),
            ToolSpec::new("pip3", PackageManager),
            ToolSpec::new("pipx", PackageManager),
            ToolSpec::new("uv", PackageManager),
            ToolSpec::new("poetry", PackageManager),
            ToolSpec::new("virtualenv", Runtime),
            ToolSpec::new("ruff", Formatter),
            ToolSpec::new("pyright", LanguageServer),
        ]
    }
    fn extra_directories(&self, host: &Host) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(h) = &host.home {
            paths.push(h.join(".local/bin"));
            paths.extend(child_bins(h.join(".pyenv/versions")));
            paths.extend(child_bins(h.join("miniconda3/envs")));
            paths.extend(child_bins(h.join("anaconda3/envs")));
        }
        for key in ["VIRTUAL_ENV", "CONDA_PREFIX"] {
            if let Some(p) = std::env::var_os(key) {
                paths.push(PathBuf::from(p).join(if cfg!(windows) { "Scripts" } else { "bin" }));
            }
        }
        paths
    }
    fn environments(&self, host: &Host) -> Vec<PathBuf> {
        let mut environments = Vec::new();
        for key in ["VIRTUAL_ENV", "CONDA_PREFIX"] {
            if let Some(p) = std::env::var_os(key)
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
            {
                environments.push(p);
            }
        }
        if let Some(h) = &host.home {
            for root in [
                h.join(".pyenv/versions"),
                h.join("miniconda3/envs"),
                h.join("anaconda3/envs"),
            ] {
                if let Ok(entries) = std::fs::read_dir(root) {
                    environments.extend(
                        entries
                            .filter_map(Result::ok)
                            .map(|e| e.path())
                            .filter(|p| p.is_dir())
                            .take(64),
                    );
                }
            }
        }
        environments.sort();
        environments.dedup();
        environments
    }
}
