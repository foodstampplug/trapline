//! Built-in Watch: continuous attack-surface change detection, ported in-process
//! from the standalone `trapline-watch` crate. Engine logic is unchanged; the
//! config/alert/findings edges are rewired to the app.

pub mod config;
pub mod engine;
pub mod fetch;
pub mod normalize;
pub mod parse;
pub mod score;
pub mod sink;
pub mod sourcemap;
pub mod store;
