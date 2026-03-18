use std::collections::HashMap;

use crate::script::ast::Span;
use crate::script::eval::{EvalError, Value};

pub fn call_builtin(name: &str, args: &[Value], span: &Span) -> Result<Value, EvalError> {
	match name {
		"env" => builtin_env(args, span),
		"hostname" => builtin_hostname(span),
		"str" => builtin_str(args, span),
		"int" => builtin_int(args, span),
		"float" => builtin_float(args, span),
		"len" => builtin_len(args, span),
		"push" => builtin_push(args, span),
		"contains" => builtin_contains(args, span),
		"starts_with" => builtin_starts_with(args, span),
		"upper" => builtin_upper(args, span),
		"lower" => builtin_lower(args, span),
		"keybind" => builtin_keybind(args, span),
		"rule" => builtin_rule(args, span),
		"spawn" => builtin_spawn(args, span),
		"kill_focused" => builtin_kill_focused(args, span),
		"workspace" => builtin_workspace(args, span),
		"move_to_workspace" => builtin_move_to_workspace(args, span),
		"focus_direction" => builtin_focus_direction(args, span),
		"move_direction" => builtin_move_direction(args, span),
		"dispatch" => builtin_dispatch(args, span),
		"on_workspace" => builtin_on_workspace(args, span),
		"floating" => builtin_floating(args, span),
		"pinned" => builtin_pinned(args, span),
		"match_class" => builtin_match_class(args, span),
		"match_title" => builtin_match_title(args, span),
		"output" => builtin_output(args, span),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("unknown function: {}", name),
		}),
	}
}

fn builtin_env(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	match args.len() {
		1 => {
			let key = args[0]
				.as_string()
				.ok_or_else(|| type_error(span, "env() expects string key"))?;
			Ok(match std::env::var(key) {
				Ok(v) => Value::String(v),
				Err(_) => Value::Null,
			})
		}
		2 => {
			let key = args[0]
				.as_string()
				.ok_or_else(|| type_error(span, "env() expects string key"))?;
			let default = args[1].to_string();
			Ok(match std::env::var(key) {
				Ok(v) => Value::String(v),
				Err(_) => Value::String(default),
			})
		}
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("env() takes 1 or 2 arguments, got {}", args.len()),
		}),
	}
}

fn builtin_hostname(_span: &Span) -> Result<Value, EvalError> {
	let hostname = std::env::var("HOSTNAME")
		.or_else(|_| std::env::var("HOST"))
		.unwrap_or_else(|_| {
			std::process::Command::new("hostname")
				.output()
				.ok()
				.and_then(|o| String::from_utf8(o.stdout).ok())
				.map(|s| s.trim().to_string())
				.unwrap_or_else(|| "unknown".to_string())
		});
	Ok(Value::String(hostname))
}

fn builtin_str(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("str() takes 1 argument, got {}", args.len()),
		});
	}
	Ok(Value::String(args[0].to_string()))
}

fn builtin_int(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("int() takes 1 argument, got {}", args.len()),
		});
	}
	match &args[0] {
		Value::Int(n) => Ok(Value::Int(*n)),
		Value::Float(f) => Ok(Value::Int(*f as i64)),
		Value::String(s) => s.parse::<i64>().map(Value::Int).map_err(|_| EvalError {
			line: span.line,
			col: span.col,
			msg: format!("cannot convert '{}' to int", s),
		}),
		Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("cannot convert {} to int", args[0].type_name()),
		}),
	}
}

fn builtin_float(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("float() takes 1 argument, got {}", args.len()),
		});
	}
	match &args[0] {
		Value::Float(f) => Ok(Value::Float(*f)),
		Value::Int(n) => Ok(Value::Float(*n as f64)),
		Value::String(s) => s.parse::<f64>().map(Value::Float).map_err(|_| EvalError {
			line: span.line,
			col: span.col,
			msg: format!("cannot convert '{}' to float", s),
		}),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("cannot convert {} to float", args[0].type_name()),
		}),
	}
}

fn builtin_len(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("len() takes 1 argument, got {}", args.len()),
		});
	}
	match &args[0] {
		Value::List(l) => Ok(Value::Int(l.len() as i64)),
		Value::Map(m) => Ok(Value::Int(m.len() as i64)),
		Value::String(s) => Ok(Value::Int(s.len() as i64)),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("len() expects list/map/string, got {}", args[0].type_name()),
		}),
	}
}

fn builtin_push(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 2 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("push() takes 2 arguments, got {}", args.len()),
		});
	}
	match &args[0] {
		Value::List(l) => {
			let mut new = l.clone();
			new.push(args[1].clone());
			Ok(Value::List(new))
		}
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("push() expects list, got {}", args[0].type_name()),
		}),
	}
}

fn builtin_contains(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 2 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("contains() takes 2 arguments, got {}", args.len()),
		});
	}
	match (&args[0], &args[1]) {
		(Value::String(haystack), Value::String(needle)) => Ok(Value::Bool(haystack.contains(needle.as_str()))),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "contains() expects two strings".to_string(),
		}),
	}
}

fn builtin_starts_with(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 2 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("starts_with() takes 2 arguments, got {}", args.len()),
		});
	}
	match (&args[0], &args[1]) {
		(Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "starts_with() expects two strings".to_string(),
		}),
	}
}

fn builtin_upper(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("upper() takes 1 argument, got {}", args.len()),
		});
	}
	match &args[0] {
		Value::String(s) => Ok(Value::String(s.to_uppercase())),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "upper() expects string".to_string(),
		}),
	}
}

fn builtin_lower(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("lower() takes 1 argument, got {}", args.len()),
		});
	}
	match &args[0] {
		Value::String(s) => Ok(Value::String(s.to_lowercase())),
		_ => Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "lower() expects string".to_string(),
		}),
	}
}

fn builtin_keybind(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() < 3 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: format!("keybind() takes 3 arguments (mods, key, action), got {}", args.len()),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("keybind".to_string()));
	map.insert("mods".to_string(), args[0].clone());
	map.insert("key".to_string(), args[1].clone());
	map.insert("action".to_string(), args[2].clone());
	Ok(Value::Map(map))
}

fn builtin_rule(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.is_empty() {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "rule() takes at least 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("window_rule".to_string()));
	for (i, arg) in args.iter().enumerate() {
		if let Value::Map(m) = arg {
			for (k, v) in m {
				map.insert(k.clone(), v.clone());
			}
		} else {
			map.insert(format!("_arg{}", i), arg.clone());
		}
	}
	Ok(Value::Map(map))
}

fn builtin_spawn(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "spawn() takes 1 argument (command)".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("spawn".to_string()));
	map.insert("cmd".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_kill_focused(_args: &[Value], _span: &Span) -> Result<Value, EvalError> {
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("kill_focused".to_string()));
	Ok(Value::Map(map))
}

fn builtin_workspace(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "workspace() takes 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("switch_workspace".to_string()));
	map.insert("workspace".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_move_to_workspace(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "move_to_workspace() takes 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("move_to_workspace".to_string()));
	map.insert("workspace".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_focus_direction(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "focus_direction() takes 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("focus_direction".to_string()));
	map.insert("dir".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_move_direction(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "move_direction() takes 1 argument".to_string(),
		});
	}

	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("focus_direction".to_string()));
	map.insert("dir".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_dispatch(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "dispatch() takes 1 argument (command name)".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("dispatch".to_string()));
	map.insert("cmd".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_on_workspace(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "on_workspace() takes 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("workspace".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_floating(_args: &[Value], _span: &Span) -> Result<Value, EvalError> {
	let mut map = HashMap::new();
	map.insert("floating".to_string(), Value::Bool(true));
	Ok(Value::Map(map))
}

fn builtin_pinned(_args: &[Value], _span: &Span) -> Result<Value, EvalError> {
	let mut map = HashMap::new();
	map.insert("pin".to_string(), Value::Bool(true));
	Ok(Value::Map(map))
}

fn builtin_match_class(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "match_class() takes 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("match_class".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_match_title(args: &[Value], span: &Span) -> Result<Value, EvalError> {
	if args.len() != 1 {
		return Err(EvalError {
			line: span.line,
			col: span.col,
			msg: "match_title() takes 1 argument".to_string(),
		});
	}
	let mut map = HashMap::new();
	map.insert("match_title".to_string(), args[0].clone());
	Ok(Value::Map(map))
}

fn builtin_output(args: &[Value], _span: &Span) -> Result<Value, EvalError> {
	let mut map = HashMap::new();
	map.insert("type".to_string(), Value::String("output".to_string()));
	for (i, arg) in args.iter().enumerate() {
		if let Value::String(_) = arg {
			if i == 0 {
				map.insert("name".to_string(), arg.clone());
			}
		}
		if let Value::Map(m) = arg {
			for (k, v) in m {
				map.insert(k.clone(), v.clone());
			}
		}
	}
	Ok(Value::Map(map))
}

fn type_error(span: &Span, msg: &str) -> EvalError {
	EvalError {
		line: span.line,
		col: span.col,
		msg: msg.to_string(),
	}
}
