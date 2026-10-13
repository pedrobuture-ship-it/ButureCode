use crate::{
    Host,
    engine::{Role, ToolSpec, ToolchainProvider},
};
use std::path::PathBuf;
pub struct Cpp;
impl ToolchainProvider for Cpp {
    fn language(&self) -> &'static str {
        "C/C++"
    }
    fn specifications(&self) -> Vec<ToolSpec> {
        use Role::*;
        let mut tools = vec![
            ToolSpec::new("gcc", Compiler),
            ToolSpec::new("g++", Compiler),
            ToolSpec::new("clang", Compiler),
            ToolSpec::new("clang++", Compiler),
            ToolSpec::new("cmake", BuildTool),
            ToolSpec::new("make", BuildTool),
            ToolSpec::new("ninja", BuildTool),
            ToolSpec::new("gdb", Debugger),
            ToolSpec::new("lldb", Debugger),
            ToolSpec::new("clang-format", Formatter),
            ToolSpec::new("clangd", LanguageServer),
        ];
        if cfg!(windows) {
            tools.push(ToolSpec {
                name: "cl",
                role: Compiler,
                arguments: &[],
            });
        }
        tools
    }
    fn extra_directories(&self, host: &Host) -> Vec<PathBuf> {
        let mut paths = vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/opt/llvm/bin"),
            PathBuf::from("/Library/Developer/CommandLineTools/usr/bin"),
        ];
        if let Some(p) = std::env::var_os("VCToolsInstallDir") {
            paths.push(PathBuf::from(p).join(
                if host.architecture == crate::Architecture::Aarch64 {
                    "bin/Hostarm64/arm64"
                } else {
                    "bin/Hostx64/x64"
                },
            ));
        }
        if let Some(p) = std::env::var_os("ProgramFiles") {
            let root = PathBuf::from(p);
            paths.push(root.join("LLVM/bin"));
            paths.push(root.join("CMake/bin"));
            if let Ok(years) = std::fs::read_dir(root.join("Microsoft Visual Studio")) {
                for year in years.filter_map(Result::ok).take(8) {
                    if let Ok(editions) = std::fs::read_dir(year.path()) {
                        for edition in editions.filter_map(Result::ok).take(8) {
                            if let Ok(versions) =
                                std::fs::read_dir(edition.path().join("VC/Tools/MSVC"))
                            {
                                for version in versions.filter_map(Result::ok).take(16) {
                                    paths.push(version.path().join("bin/Hostx64/x64"));
                                }
                            }
                        }
                    }
                }
            }
        }
        paths
    }
}
