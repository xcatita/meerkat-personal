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
        .parse(input, interner, lex_stream)
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

    let parser = meerkat::ProgParser::new();
    match parser.parse(input, interner, lex_stream) {
        Ok(stmts) => match stmts.first() {
            Some(_) => ReplParseResult::Complete(stmts),
            None => ReplParseResult::Incomplete,
        },
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
    /// Verify that parsing an assertion captures the correct
    /// raw string
    #[test]
    fn test_parse_assert_captures_string() {
        use crate::ast::{ActionStmt, Stmt};

        let mut interner = Interner::new();
        let input = "assert (x == 5);";
        let res = parse_string(input, &mut interner);
        assert!(res.is_ok());
        let ast = res.unwrap();
        assert_eq!(ast.len(), 1);
        match &ast[0] {
            Stmt::ActionStmt(ActionStmt::Assert(_, text)) => {
                assert_eq!(text, "x == 5");
            }
            _ => panic!("Expected ActionStmt::Assert"),
        }
    }

    /// Verify that parsing an assertion exceeding length
    /// limit fails
    #[test]
    fn test_parse_oversized_assert() {
        let mut interner = Interner::new();
        let limit = MAX_STRING_LITERAL_LENGTH;
        let long_expr = "1+".repeat((limit / 2) + 1) + "1";
        let input = format!("assert ({});", long_expr);
        let res = parse_string(&input, &mut interner);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("Assertion text exceeds maximum length"));
    }

    /// Verify that parsing an `assert` exceeding the length limit
    /// returns a `ParseError::User` error variant
    #[test]
    fn test_parse_oversized_assert_error_type() {
        use lalrpop_util::ParseError;
        let mut interner = Interner::new();
        let limit = MAX_STRING_LITERAL_LENGTH;
        let half_limit = limit / 2;
        let repeat_count = half_limit + 1;
        let repeated = "1+".repeat(repeat_count);
        let long_expr = format!("{}1", repeated);
        let input = format!("assert ({});", long_expr);
        let mut lex_stream = Vec::new();
        for (t, span) in lex::Token::lexer(&input).spanned() {
            lex_stream.push((span.start, t, span.end));
        }
        let parser = meerkat::ProgParser::new();
        let res = parser.parse(&input, &mut interner, lex_stream);
        assert!(matches!(
            res,
            Err(ParseError::User { ref error }) if error.contains("Assertion text exceeds maximum length")
        ));
    }
}
