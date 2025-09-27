// Task state and errors for Agent module (Phase 2)
// Introduce submodules; currently they re-export legacy implementations to keep API stable.

pub mod context;
pub mod error;
pub mod manager;

pub use context::*;
pub use error::*;
pub use manager::*;
