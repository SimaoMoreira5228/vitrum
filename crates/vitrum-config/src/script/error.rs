use std::fmt;

use crate::script::eval::EvalError;
use crate::script::lexer::LexError;
use crate::script::parser::ParseError;

pub enum ScriptError {
	Lex(LexError),
	Parse(ParseError),
	Eval(EvalError),
	Io(std::io::Error),
}

impl From<LexError> for ScriptError {
	fn from(e: LexError) -> Self {
		ScriptError::Lex(e)
	}
}

impl From<ParseError> for ScriptError {
	fn from(e: ParseError) -> Self {
		ScriptError::Parse(e)
	}
}

impl From<EvalError> for ScriptError {
	fn from(e: EvalError) -> Self {
		ScriptError::Eval(e)
	}
}

impl From<std::io::Error> for ScriptError {
	fn from(e: std::io::Error) -> Self {
		ScriptError::Io(e)
	}
}

impl fmt::Display for ScriptError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ScriptError::Lex(e) => write!(f, "Lexer error: {}", e),
			ScriptError::Parse(e) => write!(f, "Parse error: {}", e),
			ScriptError::Eval(e) => write!(f, "Eval error: {}", e),
			ScriptError::Io(e) => write!(f, "IO error: {}", e),
		}
	}
}

impl fmt::Debug for ScriptError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self)
	}
}

impl std::error::Error for ScriptError {}

pub fn render_error(err: &ScriptError, filename: &str, source: &str) -> String {
	let (line, col, msg) = match err {
		ScriptError::Lex(e) => (e.line, e.col, &e.msg),
		ScriptError::Parse(e) => (e.line, e.col, &e.msg),
		ScriptError::Eval(e) => (e.line, e.col, &e.msg),
		ScriptError::Io(e) => return format!("IO error in {}: {}", filename, e),
	};

	let lines: Vec<&str> = source.lines().collect();
	let source_line = lines.get(line - 1).unwrap_or(&"");

	let mut result = format!("Error in {}, line {}, column {}:\n\n", filename, line, col);
	result.push_str(&format!("  {}\n", source_line));
	result.push_str(&format!("  {:>width$}^\n", "", width = col.saturating_sub(1)));
	result.push_str(&format!("  {}", msg));
	result
}
