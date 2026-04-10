//! Report module
//!
//! This module contains the report domain model, sorting logic,
//! text formatting utilities, and plain-text renderer.
//!
//! The plain-text renderer (`plain::render_report_rows`) is the stable
//! test-facing contract for report rendering.

pub mod adapter;
pub mod builder;
pub mod model;
pub mod plain;
pub mod sort;
pub mod text;

// Re-export commonly used types
pub use builder::build_rows;
pub use model::{ModelRow, ReportInput, SortMode, UsageLabel, UsageSource};
pub use plain::render_report_rows;
pub use text::{format_cost, ljust, rjust, split_model_id, strip_ansi, wrap_usage};
