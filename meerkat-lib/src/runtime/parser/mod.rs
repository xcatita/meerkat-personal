pub mod lex;

// LALRPOP-generated parser
#[allow(clippy::all)]
#[allow(unused)]
pub mod meerkat {
    include!(concat!(env!("OUT_DIR"), "/runtime/parser/meerkat.rs"));
}

/// Result of attempting to parse a `REPL` input buffer using `ReplParseResult`
pub enum ReplParseResult {
    /// Input parsed successfully into one or more statements
    Complete(Vec<crate::ast::Stmt>),
    /// Input is syntactically incomplete (e.g., an open brace with no matching close)
    ///
    /// The `REPL` should prompt for more input and append it to the buffer
    Incomplete,
    /// Input has a real syntax error that won't be resolved by adding more text
    Error(String),
}

use crate::ast::Stmt;
use crate::runtime::interner::Interner;
use crate::runtime::limits::{MAX_IDENTIFIER_LENGTH, MAX_STRING_LITERAL_LENGTH};
use logos::Logos;

/// Parse a string input into a vector of statements
///
/// Args:
///     `input` (`&str`): The raw string input to parse
///     `interner` (`&mut Interner`): The string interner instance
///
/// Returns:
///     `Result<Vec<Stmt>, String>`: The parsed statements, or an error string
pub fn parse_string(input: &str, interner: &mut Interner) -> Result<Vec<Stmt>, String> {
    let mut lex_stream = Vec::new();
    for (t, span) in lex::Token::lexer(input).spanned() {
        match t {
            lex::Token::Ident(name) => {
                if name.len() > MAX_IDENTIFIER_LENGTH {
                    return Err(format!(
                        "Parse error: identifier exceeds maximum length of {} characters",
                        MAX_IDENTIFIER_LENGTH
                    ));
                }
            }
            lex::Token::StrLit(val) if val.len() > MAX_STRING_LITERAL_LENGTH => {
                return Err(format!(
                    "Parse error: string literal exceeds maximum length of {} characters",
                    MAX_STRING_LITERAL_LENGTH
                ));
            }
            _ => {}
        }
        lex_stream.push((span.start, t, span.end));
    }

    meerkat::ProgParser::new()
        .parse(interner, lex_stream)
        .map_err(|e| format!("Parse error: {:?}", e))
}

/// Parse a file path into a vector of statements
///
/// Args:
///     `filename` (`&str`): The path of the file to parse
///     `interner` (`&mut Interner`): The string interner instance
///
/// Returns:
///     `Result<Vec<Stmt>, String>`: The parsed statements, or an error string
pub fn parse_file(filename: &str, interner: &mut Interner) -> Result<Vec<Stmt>, String> {
    let content =
        std::fs::read_to_string(filename).map_err(|e| format!("Failed to read file: {}", e))?;
    parse_string(&content, interner)
}

/// Try to parse accumulated `REPL` input, distinguishing incomplete input from real errors
///
/// Returns `Incomplete` when the grammar signals `UnrecognizedEof`, meaning the user
/// is mid-statement and the `REPL` should collect more lines before evaluating
///
/// Args:
///     `input` (`&str`): The accumulated REPL input buffer
///     `interner` (`&mut Interner`): The string interner instance
///
/// Returns:
///     `ReplParseResult`: The parsed result status
pub fn parse_repl(input: &str, interner: &mut Interner) -> ReplParseResult {
    use lalrpop_util::ParseError;

    if input.trim().is_empty() {
        return ReplParseResult::Incomplete;
    }

    let mut lex_stream = Vec::new();
    for (t, span) in lex::Token::lexer(input).spanned() {
        match t {
            lex::Token::Ident(name) => {
                if name.len() > MAX_IDENTIFIER_LENGTH {
                    return ReplParseResult::Error(format!(
                        "Parse error: identifier exceeds maximum length of {} characters",
                        MAX_IDENTIFIER_LENGTH
                    ));
                }
            }
            lex::Token::StrLit(val) if val.len() > MAX_STRING_LITERAL_LENGTH => {
                return ReplParseResult::Error(format!(
                    "Parse error: string literal exceeds maximum length of {} characters",
                    MAX_STRING_LITERAL_LENGTH
                ));
            }
            _ => {}
        }
        lex_stream.push((span.start, t, span.end));
    }

    match meerkat::ProgParser::new().parse(interner, lex_stream) {
        Ok(stmts) if !stmts.is_empty() => ReplParseResult::Complete(stmts),
        Ok(_) => ReplParseResult::Incomplete,
        Err(ParseError::UnrecognizedEof { .. }) => ReplParseResult::Incomplete,
        Err(e) => ReplParseResult::Error(format!("{:?}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::interner::Interner;

    /// Verify that parsing an identifier exceeding the limit
    /// returns an error
    #[test]
    fn test_parse_oversized_identifier() {
        let mut interner = Interner::new();
        let long_ident = "a".repeat(MAX_IDENTIFIER_LENGTH + 1);
        let input = format!("let {} = 42;", long_ident);
        let res = parse_string(&input, &mut interner);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("identifier exceeds maximum length"));
    }

    /// Verify that parsing a string literal exceeding the limit
    /// returns an error
    #[test]
    fn test_parse_oversized_string_literal() {
        let mut interner = Interner::new();
        let long_str = "a".repeat(MAX_STRING_LITERAL_LENGTH + 1);
        let input = format!("let x = \"{}\";", long_str);
        let res = parse_string(&input, &mut interner);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("string literal exceeds maximum length"));
    }
}
