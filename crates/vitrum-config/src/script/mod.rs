pub mod arena;
mod ast;
mod builtins;
pub mod de;
mod error;
pub mod eval;
mod lexer;
mod parser;
pub mod scope;

pub use arena::Arena;
pub use ast::Span;
pub use de::{AccumulatorDeserializer, ConfigAccumulator, ValueDeserializer};
pub use error::{ScriptError, render_error};
pub use eval::{Evaluator, Value};
pub use scope::Scope;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn eval(source: &str) -> Result<ConfigAccumulator, ScriptError> {
	eval_with_base(source, None, &mut HashSet::new())
}

pub fn eval_file(path: &Path) -> Result<ConfigAccumulator, ScriptError> {
	let source = std::fs::read_to_string(path)?;
	eval_with_base(&source, Some(&path.to_path_buf()), &mut HashSet::new())
}

fn eval_with_base(
	source: &str,
	base_path: Option<&PathBuf>,
	in_progress: &mut HashSet<PathBuf>,
) -> Result<ConfigAccumulator, ScriptError> {
	let arena = Arena::new(64 * 1024);

	let mut lexer = lexer::Lexer::new(source);
	let tokens = lexer.tokenize().map_err(ScriptError::Lex)?;

	let mut parser = parser::Parser::new(&tokens, &arena);
	let program = parser.parse().map_err(ScriptError::Parse)?;

	let mut acc = ConfigAccumulator::new();

	if let Some(base) = base_path {
		let base_dir = base.parent().unwrap_or(Path::new("."));
		for stmt in program.stmts {
			if let ast::Stmt::Import { path: import_path, .. } = stmt {
				let import_full = base_dir.join(import_path);
				let canonical = import_full.canonicalize().map_err(|e| {
					ScriptError::Io(std::io::Error::new(
						std::io::ErrorKind::NotFound,
						format!("cannot resolve import '{}': {}", import_path, e),
					))
				})?;
				if !in_progress.contains(&canonical) {
					in_progress.insert(canonical.clone());
					let imported_source = std::fs::read_to_string(&canonical)?;
					let imported_acc = eval_with_base(&imported_source, Some(&canonical), in_progress)?;

					acc.sections.extend(imported_acc.sections);
					for (tag, values) in imported_acc.tagged {
						acc.tagged.entry(tag).or_default().extend(values);
					}

					in_progress.remove(&canonical);
				}
			}
		}
	}

	let evaluator = Evaluator::new(&arena);
	let file_acc = evaluator.eval_program(&program).map_err(ScriptError::Eval)?;

	acc.sections.extend(file_acc.sections);
	for (tag, values) in file_acc.tagged {
		acc.tagged.entry(tag).or_default().extend(values);
	}

	Ok(acc)
}

pub fn eval_string<'a>(source: &'a str, evaluator: &Evaluator<'a>) -> Result<ConfigAccumulator, eval::EvalError> {
	let tokens_vec = lexer::Lexer::new(source).tokenize().map_err(|e| eval::EvalError {
		line: e.line,
		col: e.col,
		msg: e.msg,
	})?;
	let tokens = evaluator.arena.alloc_slice(&tokens_vec);
	let program = parser::Parser::new(tokens, evaluator.arena)
		.parse()
		.map_err(|e| eval::EvalError {
			line: e.line,
			col: e.col,
			msg: e.msg,
		})?;
	evaluator.eval_program(&program)
}

pub fn eval_string_with_scope<'a>(
	source: &'a str,
	evaluator: &Evaluator<'a>,
	mut scope: scope::Scope<'a>,
) -> Result<ConfigAccumulator, eval::EvalError> {
	let tokens_vec = lexer::Lexer::new(source).tokenize().map_err(|e| eval::EvalError {
		line: e.line,
		col: e.col,
		msg: e.msg,
	})?;
	let tokens = evaluator.arena.alloc_slice(&tokens_vec);
	let program = parser::Parser::new(tokens, evaluator.arena)
		.parse()
		.map_err(|e| eval::EvalError {
			line: e.line,
			col: e.col,
			msg: e.msg,
		})?;

	let mut acc = ConfigAccumulator::new();
	for stmt in program.stmts {
		evaluator.eval_stmt(stmt, &mut scope, &mut acc)?;
	}
	Ok(acc)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_basic_eval() {
		let src = concat!(
			"[theme]\n",
			"accent = \"#FF0000\"\n",
			"background = \"#000000\"\n",
			"dpi = 96\n",
		);
		let result = eval(src).unwrap();

		match result.sections.get("theme") {
			Some(Value::Map(m)) => {
				assert_eq!(m.get("accent").and_then(|v| v.as_string()), Some("#FF0000"));
				assert_eq!(m.get("dpi").and_then(|v| v.as_int()), Some(96));
			}
			_ => panic!("expected theme map"),
		}
	}

	#[test]
	fn test_let_binding() {
		let src = "let x = 42\nlet name = \"hello\"\n";
		let result = eval(src).unwrap();
		assert!(result.sections.is_empty());
	}

	#[test]
	fn test_conditional() {
		let src = concat!("let is_hidpi = true\n", "[theme]\n", "dpi = if is_hidpi then 192 else 96\n",);
		let result = eval(src).unwrap();

		match result.sections.get("theme") {
			Some(Value::Map(m)) => {
				assert_eq!(m.get("dpi").and_then(|v| v.as_int()), Some(192));
			}
			_ => panic!("expected theme map"),
		}
	}

	#[test]
	fn test_arithmetic() {
		let src = "[layout]\ngaps = 4 + 4\ntotal = 8 * 2\n";
		let result = eval(src).unwrap();

		match result.sections.get("layout") {
			Some(Value::Map(m)) => {
				assert_eq!(m.get("gaps").and_then(|v| v.as_int()), Some(8));
				assert_eq!(m.get("total").and_then(|v| v.as_int()), Some(16));
			}
			_ => panic!("expected layout map"),
		}
	}

	#[test]
	fn test_hostname_builtin() {
		let result = eval("let name = hostname()\n").unwrap();
		assert!(result.sections.is_empty());
	}

	#[test]
	fn test_comments() {
		let src = concat!(
			"# This is a comment\n",
			"[theme]\n",
			"# Another comment\n",
			"accent = \"#1A6B8A\"\n",
		);
		let result = eval(src).unwrap();

		match result.sections.get("theme") {
			Some(Value::Map(m)) => {
				assert_eq!(m.get("accent").and_then(|v| v.as_string()), Some("#1A6B8A"));
			}
			_ => panic!("expected theme map"),
		}
	}

	#[test]
	fn test_range_literal() {
		let src = "let nums = 1..=3\n";
		let result = eval(src).unwrap();
		assert!(result.sections.is_empty());
	}

	#[test]
	fn test_keybind_accumulation() {
		let src = concat!(
			"keybind([\"super\"], \"return\", spawn(\"foot\"))\n",
			"keybind([\"super\", \"shift\"], \"q\", kill_focused())\n",
		);
		let result = eval(src).unwrap();
		assert_eq!(result.tagged.get("keybind").map(|l| l.len()).unwrap_or(0), 2);
	}

	#[test]
	fn test_keybind_loop() {
		let src = concat!(
			"for n in 1..=9 {\n",
			"    keybind([\"super\"], str(n), workspace(n))\n",
			"    keybind([\"super\", \"shift\"], str(n), move_to_workspace(n))\n",
			"}\n",
		);
		let result = eval(src).unwrap();
		assert_eq!(result.tagged.get("keybind").map(|l| l.len()).unwrap_or(0), 18);
	}
}
