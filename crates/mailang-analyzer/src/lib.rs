#![allow(clippy::collapsible_match)]
mod analyzer;
mod error;
mod span;

pub use analyzer::Analyzer;
pub use error::AnalyzerError;
pub use span::{diagnose, locate_identifier, Diagnostic, Severity};
