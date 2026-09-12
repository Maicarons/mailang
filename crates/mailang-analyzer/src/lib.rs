mod error;
mod analyzer;
mod span;

pub use error::AnalyzerError;
pub use analyzer::Analyzer;
pub use span::{diagnose, locate_identifier, Diagnostic, Severity};
