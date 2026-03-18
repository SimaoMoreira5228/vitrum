#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
	pub line: usize,
	pub col: usize,
	pub len: usize,
}

impl Span {
	pub fn new(line: usize, col: usize, len: usize) -> Self {
		Self { line, col, len }
	}
}

#[derive(Debug)]
pub struct Program<'a> {
	pub stmts: &'a [Stmt<'a>],
	pub span: Span,
}

#[derive(Debug)]
pub enum Stmt<'a> {
	Let {
		name: &'a str,
		value: Expr<'a>,
		span: Span,
	},

	Section {
		name: &'a str,
		assignments: &'a [Assignment<'a>],
		span: Span,
	},

	SectionArray {
		name: &'a str,
		items: &'a [&'a [Assignment<'a>]],
		span: Span,
	},

	Import {
		path: &'a str,
		span: Span,
	},

	ForLoop {
		var: &'a str,
		iterable: Expr<'a>,
		body: &'a [Stmt<'a>],
		span: Span,
	},

	Expr {
		expr: Expr<'a>,
		span: Span,
	},
}

#[derive(Debug)]
pub struct Assignment<'a> {
	pub key: &'a str,
	pub value: Expr<'a>,
	pub span: Span,
}

#[derive(Debug)]
pub enum Expr<'a> {
	String(&'a str, Span),

	Int(i64, Span),

	Float(f64, Span),

	Bool(bool, Span),

	Null(Span),

	Ident(&'a str, Span),

	List(&'a [Expr<'a>], Span),

	Map(&'a [MapEntry<'a>], Span),

	Binary {
		op: BinOp,
		left: &'a Expr<'a>,
		right: &'a Expr<'a>,
		span: Span,
	},

	If {
		cond: &'a Expr<'a>,
		then_body: &'a Expr<'a>,
		else_body: Option<&'a Expr<'a>>,
		span: Span,
	},

	IfBlock {
		cond: &'a Expr<'a>,
		then_stmts: &'a [Stmt<'a>],
		else_stmts: Option<&'a [Stmt<'a>]>,
		result: Option<&'a Expr<'a>>,
		span: Span,
	},

	Call {
		name: &'a str,
		args: &'a [Expr<'a>],
		span: Span,
	},

	Interpolation {
		parts: &'a [InterpPart<'a>],
		span: Span,
	},

	Range {
		start: &'a Expr<'a>,
		end: &'a Expr<'a>,
		inclusive: bool,
		span: Span,
	},
}

#[derive(Debug)]
pub enum InterpPart<'a> {
	Literal(&'a str),
	Expr(Expr<'a>),
}

#[derive(Debug)]
pub struct MapEntry<'a> {
	pub key: &'a str,
	pub value: Expr<'a>,
	pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
	Add,
	Sub,
	Mul,
	Div,
	Mod,
	Eq,
	Neq,
	Lt,
	Gt,
	Lte,
	Gte,
	And,
	Or,
}
