use crate::script::arena::Arena;
use crate::script::ast::*;
use crate::script::lexer::{Token, TokenInfo};

pub struct Parser<'a> {
	tokens: &'a [TokenInfo<'a>],
	pos: usize,
	arena: &'a Arena,
}

#[derive(Debug)]
pub struct ParseError {
	pub line: usize,
	pub col: usize,
	pub msg: String,
}

impl std::fmt::Display for ParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "line {}, col {}: {}", self.line, self.col, self.msg)
	}
}

impl<'a> Parser<'a> {
	pub fn new(tokens: &'a [TokenInfo<'a>], arena: &'a Arena) -> Self {
		Self { tokens, pos: 0, arena }
	}

	pub fn parse(&mut self) -> Result<Program<'a>, ParseError> {
		let stmts = self.parse_statements()?;
		let span = Span::new(1, 1, 0);
		Ok(Program { stmts, span })
	}

	fn parse_statements(&mut self) -> Result<&'a [Stmt<'a>], ParseError> {
		let mut stmts: Vec<Stmt<'a>> = Vec::new();

		loop {
			self.skip_newlines();
			if self.is_eof() {
				break;
			}

			stmts.push(self.parse_statement()?);
		}

		Ok(&*self.arena.alloc_slice(&stmts))
	}

	fn parse_statement(&mut self) -> Result<Stmt<'a>, ParseError> {
		match &self.peek().token {
			Token::Let => self.parse_let(),
			Token::Import => self.parse_import(),
			Token::For => self.parse_for(),
			Token::LBracket => self.parse_section(),
			_ => self.parse_expr_stmt(),
		}
	}

	fn parse_let(&mut self) -> Result<Stmt<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::Let)?;
		let name = self.expect_ident()?;
		self.expect(Token::Equals)?;
		let value = self.parse_expr()?;
		self.optional_semicolon();
		Ok(Stmt::Let { name, value, span })
	}

	fn parse_import(&mut self) -> Result<Stmt<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::Import)?;
		let path = self.expect_string()?;
		self.optional_semicolon();
		Ok(Stmt::Import { path, span })
	}

	fn parse_for(&mut self) -> Result<Stmt<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::For)?;
		let var = self.expect_ident()?;
		self.expect(Token::In)?;
		let iterable = self.parse_expr()?;
		self.expect(Token::LBrace)?;
		let body = self.parse_statements_until(Token::RBrace)?;
		self.expect(Token::RBrace)?;
		Ok(Stmt::ForLoop {
			var,
			iterable,
			body,
			span,
		})
	}

	fn parse_section(&mut self) -> Result<Stmt<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::LBracket)?;

		let is_array = if self.check(Token::LBracket) {
			self.advance();
			true
		} else {
			false
		};

		let name = self.expect_ident()?;

		if is_array {
			self.expect(Token::RBracket)?;
			self.expect(Token::RBracket)?;

			let mut items = Vec::new();
			loop {
				self.skip_newlines();
				if self.check(Token::LBracket) {
					let save = self.pos;
					self.advance();
					if self.check(Token::LBracket) {
						self.pos = save;
						break;
					}
					self.pos = save;
				}
				if self.is_eof() {
					break;
				}
				let assignments = self.parse_assignments()?;
				let alloced: &[Assignment<'a>] = &*self.arena.alloc_slice(&assignments);
				items.push(alloced);
			}

			let items_in_arena: &mut [&[Assignment<'a>]] = self.arena.alloc_slice(&items);

			Ok(Stmt::SectionArray {
				name,
				items: items_in_arena,
				span,
			})
		} else {
			self.expect(Token::RBracket)?;
			let assignments = self.parse_assignments()?;
			let alloced: &[Assignment<'a>] = self.arena.alloc_slice(&assignments);
			Ok(Stmt::Section {
				name,
				assignments: alloced,
				span,
			})
		}
	}

	fn parse_assignments(&mut self) -> Result<Vec<Assignment<'a>>, ParseError> {
		let mut assignments = Vec::new();

		loop {
			self.skip_newlines();
			if self.is_eof() || self.check(Token::LBracket) {
				break;
			}

			let span = self.current_span();
			let key = self.expect_ident()?;
			self.expect(Token::Equals)?;
			let value = self.parse_expr()?;
			self.optional_semicolon();

			assignments.push(Assignment { key, value, span });
		}

		Ok(assignments)
	}

	fn parse_expr_stmt(&mut self) -> Result<Stmt<'a>, ParseError> {
		let span = self.current_span();
		let expr = self.parse_expr()?;
		self.optional_semicolon();
		Ok(Stmt::Expr { expr, span })
	}

	fn parse_expr(&mut self) -> Result<Expr<'a>, ParseError> {
		self.parse_or()
	}

	fn parse_or(&mut self) -> Result<Expr<'a>, ParseError> {
		let mut left = self.parse_and()?;

		while self.check(Token::Or) {
			let span = self.current_span();
			self.advance();
			let right = self.parse_and()?;
			left = Expr::Binary {
				op: BinOp::Or,
				left: &*self.arena.alloc(left),
				right: &*self.arena.alloc(right),
				span,
			};
		}

		Ok(left)
	}

	fn parse_and(&mut self) -> Result<Expr<'a>, ParseError> {
		let mut left = self.parse_equality()?;

		while self.check(Token::And) {
			let span = self.current_span();
			self.advance();
			let right = self.parse_equality()?;
			left = Expr::Binary {
				op: BinOp::And,
				left: &*self.arena.alloc(left),
				right: &*self.arena.alloc(right),
				span,
			};
		}

		Ok(left)
	}

	fn parse_equality(&mut self) -> Result<Expr<'a>, ParseError> {
		let mut left = self.parse_comparison()?;

		loop {
			let op = match &self.peek().token {
				Token::EqEq => BinOp::Eq,
				Token::Neq => BinOp::Neq,
				_ => break,
			};
			let span = self.current_span();
			self.advance();
			let right = self.parse_comparison()?;
			left = Expr::Binary {
				op,
				left: &*self.arena.alloc(left),
				right: &*self.arena.alloc(right),
				span,
			};
		}

		Ok(left)
	}

	fn parse_comparison(&mut self) -> Result<Expr<'a>, ParseError> {
		let mut left = self.parse_additive()?;

		loop {
			let op = match &self.peek().token {
				Token::Lt => BinOp::Lt,
				Token::Gt => BinOp::Gt,
				Token::Lte => BinOp::Lte,
				Token::Gte => BinOp::Gte,
				_ => break,
			};
			let span = self.current_span();
			self.advance();
			let right = self.parse_additive()?;
			left = Expr::Binary {
				op,
				left: &*self.arena.alloc(left),
				right: &*self.arena.alloc(right),
				span,
			};
		}

		Ok(left)
	}

	fn parse_additive(&mut self) -> Result<Expr<'a>, ParseError> {
		let mut left = self.parse_multiplicative()?;

		loop {
			let op = match &self.peek().token {
				Token::Plus => BinOp::Add,
				Token::Minus => BinOp::Sub,
				_ => break,
			};
			let span = self.current_span();
			self.advance();
			let right = self.parse_multiplicative()?;
			left = Expr::Binary {
				op,
				left: &*self.arena.alloc(left),
				right: &*self.arena.alloc(right),
				span,
			};
		}

		Ok(left)
	}

	fn parse_multiplicative(&mut self) -> Result<Expr<'a>, ParseError> {
		let mut left = self.parse_unary()?;

		loop {
			let op = match &self.peek().token {
				Token::Star => BinOp::Mul,
				Token::Slash => BinOp::Div,
				Token::Percent => BinOp::Mod,
				_ => break,
			};
			let span = self.current_span();
			self.advance();
			let right = self.parse_unary()?;
			left = Expr::Binary {
				op,
				left: &*self.arena.alloc(left),
				right: &*self.arena.alloc(right),
				span,
			};
		}

		Ok(left)
	}

	fn parse_unary(&mut self) -> Result<Expr<'a>, ParseError> {
		if self.check(Token::Not) {
			let span = self.current_span();
			self.advance();
			let expr = self.parse_unary()?;

			let false_lit = Expr::Bool(false, span);
			return Ok(Expr::Binary {
				op: BinOp::Eq,
				left: &*self.arena.alloc(expr),
				right: &*self.arena.alloc(false_lit),
				span,
			});
		}

		if self.check(Token::Minus) {
			let span = self.current_span();
			self.advance();
			let expr = self.parse_unary()?;
			let zero = Expr::Int(0, span);
			return Ok(Expr::Binary {
				op: BinOp::Sub,
				left: &*self.arena.alloc(zero),
				right: &*self.arena.alloc(expr),
				span,
			});
		}

		self.parse_range()
	}

	fn parse_range(&mut self) -> Result<Expr<'a>, ParseError> {
		let span = self.current_span();
		let left = self.parse_primary()?;

		if self.check(Token::DotDotEq) {
			self.advance();
			let right = self.parse_primary()?;
			return Ok(Expr::Range {
				start: &*self.arena.alloc(left),
				end: &*self.arena.alloc(right),
				inclusive: true,
				span,
			});
		}

		if self.check(Token::DotDot) {
			self.advance();
			let right = self.parse_primary()?;
			return Ok(Expr::Range {
				start: &*self.arena.alloc(left),
				end: &*self.arena.alloc(right),
				inclusive: false,
				span,
			});
		}

		Ok(left)
	}

	fn parse_primary(&mut self) -> Result<Expr<'a>, ParseError> {
		let span = self.current_span();

		match self.peek().token.clone() {
			Token::Int(n) => {
				self.advance();
				Ok(Expr::Int(n, span))
			}
			Token::Float(f) => {
				self.advance();
				Ok(Expr::Float(f, span))
			}
			Token::Bool(b) => {
				self.advance();
				Ok(Expr::Bool(b, span))
			}
			Token::Null => {
				self.advance();
				Ok(Expr::Null(span))
			}
			Token::String(s) => {
				self.advance();
				if s.contains('{') {
					self.parse_interpolation(s, span)
				} else {
					Ok(Expr::String(s, span))
				}
			}
			Token::Ident(name) => {
				self.advance();
				if self.check(Token::LParen) {
					self.parse_call(name, span)
				} else {
					Ok(Expr::Ident(name, span))
				}
			}
			Token::If => self.parse_if_expr(),
			Token::LBracket => self.parse_list_literal(),
			Token::LBrace => self.parse_map_literal(),
			Token::LParen => {
				self.advance();
				let expr = self.parse_expr()?;
				self.expect(Token::RParen)?;
				Ok(expr)
			}
			_ => Err(ParseError {
				line: self.peek().span.line,
				col: self.peek().span.col,
				msg: format!("unexpected token: {:?}", self.peek().token),
			}),
		}
	}

	fn parse_call(&mut self, name: &'a str, span: Span) -> Result<Expr<'a>, ParseError> {
		self.expect(Token::LParen)?;
		let mut args = Vec::new();

		if !self.check(Token::RParen) {
			args.push(self.parse_expr()?);
			while self.check(Token::Comma) {
				self.advance();
				args.push(self.parse_expr()?);
			}
		}

		self.expect(Token::RParen)?;

		Ok(Expr::Call {
			name,
			args: &*self.arena.alloc_slice(&args),
			span,
		})
	}

	fn parse_if_expr(&mut self) -> Result<Expr<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::If)?;
		let cond = self.parse_expr()?;

		if self.check(Token::Then) {
			self.advance();
			let then_body = self.parse_expr()?;
			let else_body = if self.check(Token::Else) {
				self.advance();
				Some(&*self.arena.alloc(self.parse_expr()?))
			} else {
				None
			};
			Ok(Expr::If {
				cond: &*self.arena.alloc(cond),
				then_body: &*self.arena.alloc(then_body),
				else_body,
				span,
			})
		} else {
			self.expect(Token::LBrace)?;
			let then_stmts = self.parse_statements_until(Token::RBrace)?;
			self.expect(Token::RBrace)?;

			let else_stmts = if self.check(Token::Else) {
				self.advance();
				self.expect(Token::LBrace)?;
				let stmts = self.parse_statements_until(Token::RBrace)?;
				self.expect(Token::RBrace)?;
				Some(stmts)
			} else {
				None
			};

			Ok(Expr::IfBlock {
				cond: &*self.arena.alloc(cond),
				then_stmts,
				else_stmts,
				result: None,
				span,
			})
		}
	}

	fn parse_list_literal(&mut self) -> Result<Expr<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::LBracket)?;
		let mut items = Vec::new();

		if !self.check(Token::RBracket) {
			items.push(self.parse_expr()?);
			while self.check(Token::Comma) {
				self.advance();
				items.push(self.parse_expr()?);
			}
		}

		self.expect(Token::RBracket)?;
		Ok(Expr::List(&*self.arena.alloc_slice(&items), span))
	}

	fn parse_map_literal(&mut self) -> Result<Expr<'a>, ParseError> {
		let span = self.current_span();
		self.expect(Token::LBrace)?;
		let mut entries = Vec::new();

		if !self.check(Token::RBrace) {
			entries.push(self.parse_map_entry()?);
			while self.check(Token::Comma) {
				self.advance();
				if self.check(Token::RBrace) {
					break;
				}
				entries.push(self.parse_map_entry()?);
			}
		}

		self.expect(Token::RBrace)?;
		Ok(Expr::Map(&*self.arena.alloc_slice(&entries), span))
	}

	fn parse_map_entry(&mut self) -> Result<MapEntry<'a>, ParseError> {
		let span = self.current_span();
		let key = self.expect_ident()?;
		self.expect(Token::Equals)?;
		let value = self.parse_expr()?;
		Ok(MapEntry { key, value, span })
	}

	fn parse_interpolation(&self, s: &'a str, _span: Span) -> Result<Expr<'a>, ParseError> {
		let mut parts: Vec<InterpPart<'a>> = Vec::new();
		let mut rest = s;

		while let Some(pos) = rest.find('{') {
			if pos > 0 {
				parts.push(InterpPart::Literal(&rest[..pos]));
			}
			rest = &rest[pos + 1..];

			if let Some(end) = rest.find('}') {
				let expr_str = &rest[..end];
				let mut lexer = crate::script::lexer::Lexer::new(expr_str);
				let tokens = lexer.tokenize().map_err(|e| ParseError {
					line: 1,
					col: 1,
					msg: format!("interpolation error: {}", e),
				})?;
				let arena = Arena::new(4096);
				let mut parser = Parser::new(&tokens, &arena);
				let expr = parser.parse_expr()?;

				let _ = expr;
				parts.push(InterpPart::Literal(&rest[..end]));
				rest = &rest[end + 1..];
			}
		}

		if !rest.is_empty() {
			parts.push(InterpPart::Literal(rest));
		}

		Ok(Expr::Interpolation {
			parts: &*self.arena.alloc_slice(&parts),
			span: Span::new(1, 1, 0),
		})
	}

	fn parse_statements_until(&mut self, end: Token) -> Result<&'a [Stmt<'a>], ParseError> {
		let mut stmts = Vec::new();

		loop {
			self.skip_newlines();
			if self.is_eof() || self.check(end.clone()) {
				break;
			}
			stmts.push(self.parse_statement()?);
		}

		Ok(&*self.arena.alloc_slice(&stmts))
	}

	fn peek(&self) -> &TokenInfo<'a> {
		&self.tokens[self.pos]
	}

	fn advance(&mut self) {
		self.pos += 1;
	}

	fn check(&self, token: Token) -> bool {
		std::mem::discriminant(&self.peek().token) == std::mem::discriminant(&token)
	}

	fn is_eof(&self) -> bool {
		matches!(self.peek().token, Token::Eof)
	}

	fn skip_newlines(&mut self) {
		while self.check(Token::Newline) || self.check(Token::Comment) {
			self.advance();
		}
	}

	fn expect(&mut self, token: Token) -> Result<(), ParseError> {
		if self.check(token.clone()) {
			self.advance();
			Ok(())
		} else {
			Err(ParseError {
				line: self.peek().span.line,
				col: self.peek().span.col,
				msg: format!("expected {:?}, got {:?}", token, self.peek().token),
			})
		}
	}

	fn expect_ident(&mut self) -> Result<&'a str, ParseError> {
		match self.peek().token.clone() {
			Token::Ident(name) => {
				self.advance();
				Ok(name)
			}
			_ => Err(ParseError {
				line: self.peek().span.line,
				col: self.peek().span.col,
				msg: format!("expected identifier, got {:?}", self.peek().token),
			}),
		}
	}

	fn expect_string(&mut self) -> Result<&'a str, ParseError> {
		match self.peek().token.clone() {
			Token::String(s) => {
				self.advance();
				Ok(s)
			}
			_ => Err(ParseError {
				line: self.peek().span.line,
				col: self.peek().span.col,
				msg: format!("expected string, got {:?}", self.peek().token),
			}),
		}
	}

	fn optional_semicolon(&mut self) {
		if self.check(Token::Semicolon) {
			self.advance();
		}
	}

	fn current_span(&self) -> Span {
		self.peek().span
	}
}
