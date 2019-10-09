#![allow(dead_code)]
pub mod parse;
pub mod token;

use parse::Parser;
use std::error::Error;
use token::Tokenizer;

/// inline embeds all the referenced resources to create a standalone html
/// document.
/// Input must be valid html, and can be unformatted.
pub fn inline(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(Parser::new(Tokenizer::new(input.chars()).merged())?
        .parse()?
        .to_string())
}
