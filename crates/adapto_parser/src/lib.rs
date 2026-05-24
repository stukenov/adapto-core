pub mod ast;
pub mod error;
pub mod parser;

pub use ast::*;
pub use error::{ParseError, ParseResult};

use std::path::Path;

/// Parse an `.adapto` source string into an `AdaptoFile` AST.
pub fn parse(input: &str) -> ParseResult<AdaptoFile> {
    parser::parse(input)
}

/// Parse an `.adapto` file from disk into an `AdaptoFile` AST.
pub fn parse_file(path: &Path) -> ParseResult<AdaptoFile> {
    let content = std::fs::read_to_string(path)?;
    parse(&content)
}
