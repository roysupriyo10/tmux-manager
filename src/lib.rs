pub mod cli;
pub mod completions;
pub mod config;
pub mod matcher;
pub mod migrate;

pub use migrate::normalize_config_paths;
pub mod model;
pub mod paths;
pub mod schema_gen;
pub mod tmux;
