use crate::script::ast::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
	String(&'a str),
	Int(i64),
	Float(f64),
	Bool(bool),
	Null,

	Ident(&'a str),
	Let,
	If,
	Then,
	Else,
	For,
	In,
	Import,
	Or,
	And,
	Not,

	LParen,
	RParen,
	LBrace,
	RBrace,
	LBracket,
	RBracket,
	Dot,
	DotDot,
	DotDotEq,
	Comma,
	Semicolon,
	Equals,
	EqEq,
	Neq,
	Lt,
	Gt,
	Lte,
	Gte,
	Plus,
	Minus,
	Star,
	Slash,
	Percent,

	Newline,
	Comment,
	Eof,
}

#[derive(Debug, Clone, Copy)]
pub struct TokenInfo<'a> {
	pub token: Token<'a>,
	pub span: Span,
}

pub struct Lexer<'a> {
	src: &'a str,
	pos: usize,
	line: usize,
	col: usize,
}

impl<'a> Lexer<'a> {
	pub fn new(src: &'a str) -> Self {
		Self {
			src,
			pos: 0,
			line: 1,
			col: 1,
		}
	}

	pub fn tokenize(&mut self) -> Result<Vec<TokenInfo<'a>>, LexError> {
		let mut tokens = Vec::new();

		loop {
			self.skip_whitespace();
			if self.pos >= self.src.len() {
				tokens.push(TokenInfo {
					token: Token::Eof,
					span: Span::new(self.line, self.col, 0),
				});
				break;
			}

			let start_line = self.line;
			let start_col = self.col;
			let start_pos = self.pos;

			let token = self.next_token()?;
			let len = self.pos - start_pos;

			tokens.push(TokenInfo {
				token,
				span: Span::new(start_line, start_col, len),
			});
		}

		Ok(tokens)
	}

	fn next_token(&mut self) -> Result<Token<'a>, LexError> {
		let ch = self.peek();

		match ch {
			'#' => {
				self.advance();
				while self.pos < self.src.len() && self.peek() != '\n' {
					self.advance();
				}
				Ok(Token::Comment)
			}
			'\n' => {
				self.advance();
				Ok(Token::Newline)
			}
			'"' | '\'' => self.read_string(ch),
			'0'..='9' => self.read_number(),
			'a'..='z' | 'A'..='Z' | '_' => self.read_ident_or_keyword(),
			'=' => {
				self.advance();
				if self.peek() == '=' {
					self.advance();
					Ok(Token::EqEq)
				} else {
					Ok(Token::Equals)
				}
			}
			'!' => {
				self.advance();
				if self.peek() == '=' {
					self.advance();
					Ok(Token::Neq)
				} else {
					Ok(Token::Not)
				}
			}
			'<' => {
				self.advance();
				if self.peek() == '=' {
					self.advance();
					Ok(Token::Lte)
				} else {
					Ok(Token::Lt)
				}
			}
			'>' => {
				self.advance();
				if self.peek() == '=' {
					self.advance();
					Ok(Token::Gte)
				} else {
					Ok(Token::Gt)
				}
			}
			'+' => {
				self.advance();
				Ok(Token::Plus)
			}
			'-' => {
				self.advance();
				Ok(Token::Minus)
			}
			'*' => {
				self.advance();
				Ok(Token::Star)
			}
			'/' => {
				self.advance();
				Ok(Token::Slash)
			}
			'%' => {
				self.advance();
				Ok(Token::Percent)
			}
			'(' => {
				self.advance();
				Ok(Token::LParen)
			}
			')' => {
				self.advance();
				Ok(Token::RParen)
			}
			'{' => {
				self.advance();
				Ok(Token::LBrace)
			}
			'}' => {
				self.advance();
				Ok(Token::RBrace)
			}
			'[' => {
				self.advance();
				Ok(Token::LBracket)
			}
			']' => {
				self.advance();
				Ok(Token::RBracket)
			}
			',' => {
				self.advance();
				Ok(Token::Comma)
			}
			';' => {
				self.advance();
				Ok(Token::Semicolon)
			}
			'.' => {
				self.advance();
				if self.peek() == '.' {
					self.advance();
					if self.peek() == '=' {
						self.advance();
						Ok(Token::DotDotEq)
					} else {
						Ok(Token::DotDot)
					}
				} else {
					Ok(Token::Dot)
				}
			}
			_ => Err(LexError {
				line: self.line,
				col: self.col,
				msg: format!("unexpected character: '{}'", ch),
			}),
		}
	}

	fn read_string(&mut self, quote: char) -> Result<Token<'a>, LexError> {
		self.advance();
		let start = self.pos;

		while self.pos < self.src.len() && self.peek() != quote {
			if self.peek() == '\\' {
				self.advance();
			}
			self.advance();
		}

		if self.pos >= self.src.len() {
			return Err(LexError {
				line: self.line,
				col: self.col,
				msg: "unterminated string".to_string(),
			});
		}

		let content = &self.src[start..self.pos];
		self.advance();

		Ok(Token::String(content))
	}

	fn read_number(&mut self) -> Result<Token<'a>, LexError> {
		let start = self.pos;
		let mut is_float = false;

		while self.pos < self.src.len() && (self.peek().is_ascii_digit() || self.peek() == '.') {
			if self.peek() == '.' {
				if is_float {
					break;
				}
				let next = self.src[self.pos + 1..].chars().next().unwrap_or('\0');
				if next == '.' {
					break;
				}
				is_float = true;
			}
			self.advance();
		}

		let s = &self.src[start..self.pos];

		if is_float {
			s.parse::<f64>().map(Token::Float).map_err(|_| LexError {
				line: self.line,
				col: self.col,
				msg: format!("invalid float: {}", s),
			})
		} else {
			s.parse::<i64>().map(Token::Int).map_err(|_| LexError {
				line: self.line,
				col: self.col,
				msg: format!("invalid integer: {}", s),
			})
		}
	}

	fn read_ident_or_keyword(&mut self) -> Result<Token<'a>, LexError> {
		let start = self.pos;

		while self.pos < self.src.len() && (self.peek().is_alphanumeric() || self.peek() == '_') {
			self.advance();
		}

		let word = &self.src[start..self.pos];

		match word {
			"let" => Ok(Token::Let),
			"if" => Ok(Token::If),
			"then" => Ok(Token::Then),
			"else" => Ok(Token::Else),
			"for" => Ok(Token::For),
			"in" => Ok(Token::In),
			"import" => Ok(Token::Import),
			"or" => Ok(Token::Or),
			"and" => Ok(Token::And),
			"not" => Ok(Token::Not),
			"true" => Ok(Token::Bool(true)),
			"false" => Ok(Token::Bool(false)),
			"null" => Ok(Token::Null),
			_ => Ok(Token::Ident(word)),
		}
	}

	fn peek(&self) -> char {
		self.src[self.pos..].chars().next().unwrap_or('\0')
	}

	fn advance(&mut self) {
		if self.peek() == '\n' {
			self.line += 1;
			self.col = 1;
		} else {
			self.col += 1;
		}
		self.pos += self.peek().len_utf8();
	}

	fn skip_whitespace(&mut self) {
		while self.pos < self.src.len() && self.peek().is_ascii_whitespace() && self.peek() != '\n' {
			self.advance();
		}
	}
}

#[derive(Debug)]
pub struct LexError {
	pub line: usize,
	pub col: usize,
	pub msg: String,
}

impl std::fmt::Display for LexError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "line {}, col {}: {}", self.line, self.col, self.msg)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_basic_tokens() {
		let mut lexer = Lexer::new("let x = 42\n");
		let tokens = lexer.tokenize().unwrap();
		assert_eq!(tokens[0].token, Token::Let);
		assert_eq!(tokens[1].token, Token::Ident("x"));
		assert_eq!(tokens[2].token, Token::Equals);
		assert_eq!(tokens[3].token, Token::Int(42));
	}

	#[test]
	fn test_string() {
		let mut lexer = Lexer::new("\"hello world\"");
		let tokens = lexer.tokenize().unwrap();
		assert_eq!(tokens[0].token, Token::String("hello world"));
	}
}
