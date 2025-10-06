// Task state and errors for Agent module (Phase 2)
// Introduce submodules; currently they re-export legacy implementations to keep API stable.

pub mod error;
pub mod iteration;
pub mod manager;
pub mod session;

pub use error::*;
pub use iteration::*;
pub use manager::*;
pub use session::*;
