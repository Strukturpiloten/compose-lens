//! Source-aware parsing, processing, validation, editing, and rendering of Docker Compose projects.

pub mod diagnostic;
pub mod interpolation;
pub mod loader;
pub mod merge;
pub mod model;
pub mod profiles;
pub mod project;
pub mod render;
pub mod resolution;
pub mod source;
pub mod syntax;
pub mod validation;
