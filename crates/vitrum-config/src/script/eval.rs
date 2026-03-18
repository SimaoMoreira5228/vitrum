use std::collections::HashMap;

use crate::script::arena::Arena;
use crate::script::ast::*;
use crate::script::de::ConfigAccumulator;
use crate::script::scope::Scope;

#[derive(Debug, Clone)]
pub enum Value {
	String(String),
	Int(i64),
	Float(f64),
	Bool(bool),
	List(Vec<Value>),
	Map(HashMap<String, Value>),
	Null,
}

impl Value {
	pub fn as_string(&self) -> Option<&str> {
		match self {
			Value::String(s) => Some(s),
			_ => None,
		}
	}

	pub fn as_int(&self) -> Option<i64> {
		match self {
			Value::Int(n) => Some(*n),
			Value::Float(f) => Some(*f as i64),
			_ => None,
		}
	}

	pub fn as_bool(&self) -> bool {
		match self {
			Value::Bool(b) => *b,
			Value::Null => false,
			Value::Int(n) => *n != 0,
			Value::String(s) => !s.is_empty(),
			Value::List(l) => !l.is_empty(),
			Value::Map(m) => !m.is_empty(),
			Value::Float(f) => *f != 0.0,
		}
	}

	pub fn type_name(&self) -> &'static str {
		match self {
			Value::String(_) => "string",
			Value::Int(_) => "int",
			Value::Float(_) => "float",
			Value::Bool(_) => "bool",
			Value::List(_) => "list",
			Value::Map(_) => "map",
			Value::Null => "null",
		}
	}
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::String(s) => write!(f, "{}", s),
			Value::Int(n) => write!(f, "{}", n),
			Value::Float(n) => write!(f, "{}", n),
			Value::Bool(b) => write!(f, "{}", b),
			Value::List(items) => {
				write!(f, "[")?;
				for (i, item) in items.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}
					write!(f, "{}", item)?;
				}
				write!(f, "]")
			}
			Value::Map(map) => {
				write!(f, "{{")?;
				for (i, (k, v)) in map.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}
					write!(f, "{} = {}", k, v)?;
				}
				write!(f, "}}")
			}
			Value::Null => write!(f, "null"),
		}
	}
}

#[derive(Debug)]
pub struct EvalError {
	pub line: usize,
	pub col: usize,
	pub msg: String,
}

impl std::fmt::Display for EvalError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}: {}", self.line, self.col, self.msg)
	}
}

pub trait BuiltinHandler {
	fn call(&self, name: &str, args: &[Value], span: &Span) -> Result<Value, EvalError>;
}

pub struct Evaluator<'a> {
	pub arena: &'a Arena,
	handler: Option<&'a dyn BuiltinHandler>,
}

impl<'a> Evaluator<'a> {
	pub fn new(arena: &'a Arena) -> Self {
		Self { arena, handler: None }
	}

	pub fn with_handler(arena: &'a Arena, handler: &'a dyn BuiltinHandler) -> Self {
		Self {
			arena,
			handler: Some(handler),
		}
	}

	pub fn eval_program(&self, program: &Program<'a>) -> Result<ConfigAccumulator, EvalError> {
		let mut scope = Scope::new();
		let mut acc = ConfigAccumulator::new();

		for stmt in program.stmts {
			self.eval_stmt(stmt, &mut scope, &mut acc)?;
		}

		Ok(acc)
	}

	pub fn eval_stmt(&self, stmt: &Stmt<'a>, scope: &mut Scope<'a>, acc: &mut ConfigAccumulator) -> Result<(), EvalError> {
		match stmt {
			Stmt::Let { name, value, .. } => {
				let val = self.eval_expr(value, scope)?;
				scope.define(name, val);
			}
			Stmt::Section { name, assignments, .. } => {
				let mut section = HashMap::new();
				for assign in *assignments {
					let val = self.eval_expr(&assign.value, scope)?;
					section.insert(assign.key.to_string(), val);
				}
				acc.sections.insert(name.to_string(), Value::Map(section));
			}
			Stmt::SectionArray { name, items, .. } => {
				for assignments in *items {
					let mut map = HashMap::new();
					map.insert("type".to_string(), Value::String(name.to_string()));
					for assign in *assignments {
						let val = self.eval_expr(&assign.value, scope)?;
						map.insert(assign.key.to_string(), val);
					}
					acc.push_tagged(map);
				}
			}
			Stmt::ForLoop {
				var,
				iterable,
				body,
				span,
			} => {
				let iter_val = self.eval_expr(iterable, scope)?;
				match iter_val {
					Value::List(items) => {
						for item in items {
							let mut child_scope = Scope::child(scope);
							child_scope.define(var, item);
							for stmt in *body {
								self.eval_stmt(stmt, &mut child_scope, acc)?;
							}
						}
					}
					Value::Map(map) => {
						for (key, val) in map {
							let mut child_scope = Scope::child(scope);
							child_scope.define(self.arena.alloc_str(&key), val);
							for stmt in *body {
								self.eval_stmt(stmt, &mut child_scope, acc)?;
							}
						}
					}
					_ => {
						return Err(EvalError {
							line: span.line,
							col: span.col,
							msg: format!("cannot iterate over {}", iter_val.type_name()),
						});
					}
				}
			}
			Stmt::Expr { expr, .. } => {
				let val = self.eval_expr(expr, scope)?;
				if let Value::Map(map) = val {
					acc.push_tagged(map);
				}
			}
			Stmt::Import { .. } => {}
		}
		Ok(())
	}

	pub fn eval_expr(&self, expr: &Expr<'a>, scope: &Scope<'a>) -> Result<Value, EvalError> {
		match expr {
			Expr::String(s, _) => Ok(Value::String(s.to_string())),
			Expr::Int(n, _) => Ok(Value::Int(*n)),
			Expr::Float(f, _) => Ok(Value::Float(*f)),
			Expr::Bool(b, _) => Ok(Value::Bool(*b)),
			Expr::Null(_) => Ok(Value::Null),
			Expr::Ident(name, span) => scope.get(name).cloned().ok_or_else(|| EvalError {
				line: span.line,
				col: span.col,
				msg: format!("undefined variable: {}", name),
			}),
			Expr::List(items, _) => {
				let mut values = Vec::new();
				for item in *items {
					values.push(self.eval_expr(item, scope)?);
				}
				Ok(Value::List(values))
			}
			Expr::Map(entries, _) => {
				let mut map = HashMap::new();
				for entry in *entries {
					let val = self.eval_expr(&entry.value, scope)?;
					map.insert(entry.key.to_string(), val);
				}
				Ok(Value::Map(map))
			}
			Expr::Binary { op, left, right, span } => {
				let l = self.eval_expr(left, scope)?;
				let r = self.eval_expr(right, scope)?;
				self.eval_binop(*op, &l, &r, span)
			}
			Expr::If {
				cond,
				then_body,
				else_body,
				span,
			} => {
				let cond_val = self.eval_expr(cond, scope)?;
				if cond_val.as_bool() {
					self.eval_expr(then_body, scope)
				} else if let Some(else_expr) = else_body {
					self.eval_expr(else_expr, scope)
				} else {
					Err(EvalError {
						line: span.line,
						col: span.col,
						msg: "if without else cannot be used as a value".to_string(),
					})
				}
			}
			Expr::IfBlock {
				cond,
				then_stmts,
				else_stmts,
				result,
				span: _,
			} => {
				let cond_val = self.eval_expr(cond, scope)?;
				let stmts = if cond_val.as_bool() {
					Some(*then_stmts)
				} else {
					else_stmts.clone()
				};
				if let Some(stmts) = stmts {
					let mut child_scope = Scope::child(scope);
					let mut dummy_acc = ConfigAccumulator::new();
					for s in stmts {
						self.eval_stmt(s, &mut child_scope, &mut dummy_acc)?;
					}
					if let Some(result_expr) = result {
						self.eval_expr(result_expr, &child_scope)
					} else {
						Ok(Value::Null)
					}
				} else if let Some(result_expr) = result {
					self.eval_expr(result_expr, scope)
				} else {
					Ok(Value::Null)
				}
			}
			Expr::Call { name, args, span } => self.eval_call(name, args, scope, span),
			Expr::Interpolation { parts, .. } => {
				let mut result = String::new();
				for part in *parts {
					match part {
						InterpPart::Literal(s) => result.push_str(s),
						InterpPart::Expr(e) => {
							let val = self.eval_expr(e, scope)?;
							result.push_str(&val.to_string());
						}
					}
				}
				Ok(Value::String(result))
			}
			Expr::Range {
				start,
				end,
				inclusive,
				span,
			} => {
				let start_val = self.eval_expr(start, scope)?;
				let end_val = self.eval_expr(end, scope)?;
				match (start_val, end_val) {
					(Value::Int(s), Value::Int(e)) => {
						let range: Vec<Value> = if *inclusive {
							(s..=e).map(Value::Int).collect()
						} else {
							(s..e).map(Value::Int).collect()
						};
						Ok(Value::List(range))
					}
					(s, e) => Err(EvalError {
						line: span.line,
						col: span.col,
						msg: format!("range bounds must be int, got {} and {}", s.type_name(), e.type_name()),
					}),
				}
			}
		}
	}

	fn eval_binop(&self, op: BinOp, l: &Value, r: &Value, span: &Span) -> Result<Value, EvalError> {
		match op {
			BinOp::Add => match (l, r) {
				(Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
				(Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
				(Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
				(Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
				(Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
				_ => Err(EvalError {
					line: span.line,
					col: span.col,
					msg: format!("cannot add {} and {}", l.type_name(), r.type_name()),
				}),
			},
			BinOp::Sub => match (l, r) {
				(Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
				(Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
				(Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
				(Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
				_ => Err(EvalError {
					line: span.line,
					col: span.col,
					msg: format!("cannot subtract {} and {}", l.type_name(), r.type_name()),
				}),
			},
			BinOp::Mul => match (l, r) {
				(Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
				(Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
				(Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
				(Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
				_ => Err(EvalError {
					line: span.line,
					col: span.col,
					msg: format!("cannot multiply {} and {}", l.type_name(), r.type_name()),
				}),
			},
			BinOp::Div => match (l, r) {
				(Value::Int(a), Value::Int(b)) if *b != 0 => Ok(Value::Int(a / b)),
				(Value::Float(a), Value::Float(b)) if *b != 0.0 => Ok(Value::Float(a / b)),
				(Value::Int(a), Value::Float(b)) if *b != 0.0 => Ok(Value::Float(*a as f64 / b)),
				(Value::Float(a), Value::Int(b)) if *b != 0 => Ok(Value::Float(a / *b as f64)),
				_ => Err(EvalError {
					line: span.line,
					col: span.col,
					msg: "division by zero or incompatible types".to_string(),
				}),
			},
			BinOp::Mod => match (l, r) {
				(Value::Int(a), Value::Int(b)) if *b != 0 => Ok(Value::Int(a % b)),
				(Value::Float(a), Value::Float(b)) if *b != 0.0 => Ok(Value::Float(a % b)),
				(Value::Int(a), Value::Float(b)) if *b != 0.0 => Ok(Value::Float(*a as f64 % b)),
				(Value::Float(a), Value::Int(b)) if *b != 0 => Ok(Value::Float(a % *b as f64)),
				_ => Err(EvalError {
					line: span.line,
					col: span.col,
					msg: "modulo by zero or incompatible types".to_string(),
				}),
			},
			BinOp::Eq => Ok(Value::Bool(self.values_equal(l, r))),
			BinOp::Neq => Ok(Value::Bool(!self.values_equal(l, r))),
			BinOp::Lt => Ok(Value::Bool(self.compare_values(l, r, span)? < 0)),
			BinOp::Gt => Ok(Value::Bool(self.compare_values(l, r, span)? > 0)),
			BinOp::Lte => Ok(Value::Bool(self.compare_values(l, r, span)? <= 0)),
			BinOp::Gte => Ok(Value::Bool(self.compare_values(l, r, span)? >= 0)),
			BinOp::And => Ok(Value::Bool(l.as_bool() && r.as_bool())),
			BinOp::Or => Ok(Value::Bool(l.as_bool() || r.as_bool())),
		}
	}

	fn values_equal(&self, a: &Value, b: &Value) -> bool {
		match (a, b) {
			(Value::Int(a), Value::Int(b)) => a == b,
			(Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
			(Value::String(a), Value::String(b)) => a == b,
			(Value::Bool(a), Value::Bool(b)) => a == b,
			(Value::Null, Value::Null) => true,
			_ => false,
		}
	}

	fn compare_values(&self, a: &Value, b: &Value, span: &Span) -> Result<i32, EvalError> {
		match (a, b) {
			(Value::Int(a), Value::Int(b)) => Ok(a.cmp(b) as i32),
			(Value::Float(a), Value::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32),
			(Value::String(a), Value::String(b)) => Ok(a.cmp(b) as i32),
			_ => Err(EvalError {
				line: span.line,
				col: span.col,
				msg: format!("cannot compare {} and {}", a.type_name(), b.type_name()),
			}),
		}
	}

	fn eval_call(&self, name: &str, args: &[Expr<'a>], scope: &Scope<'a>, span: &Span) -> Result<Value, EvalError> {
		let mut values = Vec::new();
		for arg in args {
			values.push(self.eval_expr(arg, scope)?);
		}
		if let Some(h) = self.handler {
			if let Ok(val) = h.call(name, &values, span) {
				return Ok(val);
			}
		}
		super::builtins::call_builtin(name, &values, span)
	}
}
