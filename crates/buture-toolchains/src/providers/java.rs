use crate::{
    Host,
    engine::{Role, ToolSpec, ToolchainProvider, child_bins},
};
use std::path::PathBuf;
pub struct Java;
impl ToolchainProvider for Java {
    fn language(&self) -> &'static str {
        "Java"
    }
    fn specifications(&self) -> Vec<ToolSpec> {
        use Role::*;
        vec![
            ToolSpec {
                name: "java",
                role: Runtime,
                arguments: &["-version"],
            },
            ToolSpec::new("javac", Compiler),
            ToolSpec::new("mvn", BuildTool),
            ToolSpec::new("gradle", BuildTool),
            ToolSpec {
                name: "jdb",
                role: Debugger,
                arguments: &["-version"],
            },
            ToolSpec::new("jdtls", LanguageServer),
        ]
    }
    fn extra_directories(&self, host: &Host) -> Vec<PathBuf> {
        let mut paths = child_bins(PathBuf::from("/usr/lib/jvm"));
        if let Some(h) = std::env::var_os("JAVA_HOME") {
            paths.push(PathBuf::from(h).join("bin"));
        }
        if let Some(h) = &host.home {
            paths.extend(child_bins(h.join(".sdkman/candidates/java")));
            paths.extend(child_bins(h.join(".sdkman/candidates/maven")));
            paths.extend(child_bins(h.join(".sdkman/candidates/gradle")));
        }
        if let Ok(entries) = std::fs::read_dir("/Library/Java/JavaVirtualMachines") {
            paths.extend(
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.path().join("Contents/Home/bin"))
                    .filter(|p| p.is_dir())
                    .take(64),
            );
        }
        if let Some(root) = std::env::var_os("ProgramFiles") {
            paths.extend(child_bins(PathBuf::from(root).join("Java")));
        }
        paths
    }
    fn environments(&self, host: &Host) -> Vec<PathBuf> {
        self.extra_directories(host)
            .into_iter()
            .filter_map(|p| p.parent().map(PathBuf::from))
            .collect()
    }
}
