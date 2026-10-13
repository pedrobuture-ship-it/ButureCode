use crate::{
    Host,
    engine::{Role, ToolSpec, ToolchainProvider, child_bins},
};
use std::path::PathBuf;
pub struct Node;
impl ToolchainProvider for Node {
    fn language(&self) -> &'static str {
        "Node.js"
    }
    fn specifications(&self) -> Vec<ToolSpec> {
        use Role::*;
        vec![
            ToolSpec::new("node", Runtime),
            ToolSpec::new("npm", PackageManager),
            ToolSpec::new("pnpm", PackageManager),
            ToolSpec::new("yarn", PackageManager),
            ToolSpec::new("bun", Runtime),
            ToolSpec::new("fnm", VersionManager),
            ToolSpec::new("volta", VersionManager),
            ToolSpec::new("mise", VersionManager),
            ToolSpec::new("prettier", Formatter),
            ToolSpec::new("typescript-language-server", LanguageServer),
        ]
    }
    fn extra_directories(&self, host: &Host) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(h) = &host.home {
            paths.push(h.join(".bun/bin"));
            paths.push(h.join(".volta/bin"));
            paths.push(h.join(".local/share/pnpm"));
            paths.extend(child_bins(h.join(".nvm/versions/node")));
            paths.extend(child_bins(h.join(".local/share/mise/installs/node")));
        }
        for key in ["NVM_BIN", "PNPM_HOME", "NVM_SYMLINK"] {
            if let Some(p) = std::env::var_os(key) {
                paths.push(PathBuf::from(p));
            }
        }
        paths
    }
    fn environments(&self, host: &Host) -> Vec<PathBuf> {
        let mut managers = Vec::new();
        if let Some(h) = &host.home {
            for suffix in [".nvm", ".volta", ".local/share/fnm", ".local/share/mise"] {
                let p = h.join(suffix);
                if p.is_dir() {
                    managers.push(p);
                }
            }
        }
        for key in ["NVM_HOME", "FNM_DIR", "VOLTA_HOME"] {
            if let Some(p) = std::env::var_os(key)
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
            {
                managers.push(p);
            }
        }
        managers.sort();
        managers.dedup();
        managers
    }
}
