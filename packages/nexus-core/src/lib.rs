//! NexusIDE core: storage, tools, sandbox, sync, and indexing.

pub mod config;
pub mod error_present;
pub mod context;
pub mod engine;
pub mod error;
pub mod mcp;
pub mod models;
pub mod plugins;
pub mod providers;
pub mod project;
pub mod search;
pub mod secrets;
pub mod storage;
pub mod sync;
pub mod tools;

pub use error::{NexusError, Result};
