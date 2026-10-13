pub mod cpp;
pub mod rust;
// Independent providers share bounded discovery machinery.
use crate::engine::{ToolSpec, ToolchainProvider};
pub struct Provider {
    pub language: &'static str,
    pub specs: fn() -> Vec<ToolSpec>,
}
impl ToolchainProvider for Provider {
    fn language(&self) -> &'static str {
        self.language
    }
    fn specifications(&self) -> Vec<ToolSpec> {
        (self.specs)()
    }
}
pub fn registry() -> Vec<Box<dyn ToolchainProvider>> {
    use crate::engine::Role::*;
    vec![
        Box::new(rust::Rust),
        Box::new(cpp::Cpp),
        Box::new(Provider {
            language: "Python",
            specs: || vec![ToolSpec::new("python3", Runtime)],
        }),
        Box::new(Provider {
            language: "Java",
            specs: || {
                vec![
                    ToolSpec {
                        name: "java",
                        role: Runtime,
                        arguments: &["-version"],
                    },
                    ToolSpec::new("javac", Compiler),
                ]
            },
        }),
        Box::new(Provider {
            language: "Node.js",
            specs: || vec![ToolSpec::new("node", Runtime)],
        }),
        Box::new(Provider {
            language: "Go",
            specs: || {
                vec![
                    ToolSpec {
                        name: "go",
                        role: Compiler,
                        arguments: &["version"],
                    },
                    ToolSpec {
                        name: "gofmt",
                        role: Formatter,
                        arguments: &["-h"],
                    },
                    ToolSpec::new("gopls", LanguageServer),
                    ToolSpec {
                        name: "dlv",
                        role: Debugger,
                        arguments: &["version"],
                    },
                ]
            },
        }),
    ]
}
