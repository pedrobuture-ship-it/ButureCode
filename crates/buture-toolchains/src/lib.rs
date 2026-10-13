//! Read-only host and development environment discovery.
use std::{env, path::PathBuf};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    Linux,
    MacOS,
    Other,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Other,
}
#[derive(Debug, Clone)]
pub struct Host {
    pub os: OperatingSystem,
    pub architecture: Architecture,
    pub shell: Option<PathBuf>,
    pub path: Vec<PathBuf>,
    pub home: Option<PathBuf>,
    pub sdk_directories: Vec<PathBuf>,
}
impl Host {
    pub fn detect() -> Self {
        let os = match env::consts::OS {
            "windows" => OperatingSystem::Windows,
            "linux" => OperatingSystem::Linux,
            "macos" => OperatingSystem::MacOS,
            _ => OperatingSystem::Other,
        };
        let architecture = match env::consts::ARCH {
            "x86_64" => Architecture::X86_64,
            "aarch64" => Architecture::Aarch64,
            _ => Architecture::Other,
        };
        let home = env::var_os(if os == OperatingSystem::Windows {
            "USERPROFILE"
        } else {
            "HOME"
        })
        .map(PathBuf::from);
        let shell = env::var_os(if os == OperatingSystem::Windows {
            "COMSPEC"
        } else {
            "SHELL"
        })
        .map(PathBuf::from);
        let path = env::var_os("PATH")
            .map(|p| env::split_paths(&p).filter(|p| p.is_absolute()).collect())
            .unwrap_or_default();
        let mut sdk_directories = Vec::new();
        if let Some(h) = &home {
            for suffix in [
                ".cargo",
                ".rustup/toolchains",
                ".sdkman/candidates",
                "Android/Sdk",
                "Library/Android/sdk",
                ".local/share/mise/installs",
                ".nvm",
                ".pyenv/versions",
            ] {
                let p = h.join(suffix);
                if p.is_dir() {
                    sdk_directories.push(p);
                }
            }
        }
        for key in [
            "JAVA_HOME",
            "ANDROID_HOME",
            "ANDROID_SDK_ROOT",
            "GOPATH",
            "VIRTUAL_ENV",
            "CONDA_PREFIX",
            "ProgramFiles",
        ] {
            if let Some(p) = env::var_os(key).map(PathBuf::from).filter(|p| p.is_dir()) {
                sdk_directories.push(p);
            }
        }
        for p in [
            "/opt",
            "/usr/lib/jvm",
            "/Library/Developer/CommandLineTools",
            "/Applications/Xcode.app/Contents/Developer",
        ] {
            let p = PathBuf::from(p);
            if p.is_dir() {
                sdk_directories.push(p);
            }
        }
        sdk_directories.sort();
        sdk_directories.dedup();
        Self {
            os,
            architecture,
            shell,
            path,
            home,
            sdk_directories,
        }
    }
}
pub mod engine;
pub mod providers;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_matches_target() {
        let h = Host::detect();
        assert_eq!(h.os == OperatingSystem::Linux, cfg!(target_os = "linux"));
        assert!(h.path.iter().all(|p| p.is_absolute()));
    }
}
