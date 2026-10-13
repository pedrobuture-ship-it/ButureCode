//! Shared identity without GUI or platform dependencies.

pub const APP_NAME: &str = "Buture Code";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_AUTHOR: &str = "Pedro Buture";
pub const TAGLINE: &str = "Engineering without friction.";

pub mod commands;
