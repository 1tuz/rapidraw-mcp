//! RapidRAW MCP server library — HTTP client + tool wiring.

pub mod api_client;
pub mod config;
pub mod error;
pub mod tools;

pub use api_client::RapidRawClient;
pub use config::Config;
pub use tools::RapidRawMcp;
