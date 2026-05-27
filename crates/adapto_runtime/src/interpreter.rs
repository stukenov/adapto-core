use crate::error::RuntimeError;
use crate::state::StateStore;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    NumberLit(f64),
    BoolLit(bool),
    Null,
    // Operators
    Assign,       // =
    PlusAssign,   // +=
    MinusAssign,  // -=
    MulAssign,    // *=
    DivAssign,    // /=
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    Eq,           // ==
    Ne,           // !=
    Lt,           // <
    Le,           // <=
    Gt,           // >
    Ge,           // >=
    And,          // &&
    Or,           // ||
    Not,          // !
    Dot,          // .
    Comma,        // ,
    Semicolon,    // ;
    LParen,       // (
    RParen,       // )
    LBracket,     // [
    RBracket,     // ]
    LBrace,       // {
    RBrace,       // }
    // Keywords
    If,
    Else,
    For,
    In,
    Let,
    Return,
    Await,
    Eof,
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' && self.chars.get(self.pos + 1) == Some(&'/') {
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, RuntimeError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    return Ok(tokens);
                }
                Some(ch) => {
                    let tok = match ch {
                        '(' => { self.advance(); Token::LParen }
                        ')' => { self.advance(); Token::RParen }
                        '[' => { self.advance(); Token::LBracket }
                        ']' => { self.advance(); Token::RBracket }
                        '{' => { self.advance(); Token::LBrace }
                        '}' => { self.advance(); Token::RBrace }
                        ',' => { self.advance(); Token::Comma }
                        ';' => { self.advance(); Token::Semicolon }
                        '.' => { self.advance(); Token::Dot }
                        '%' => { self.advance(); Token::Percent }
                        '+' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::PlusAssign }
                            else { Token::Plus }
                        }
                        '-' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::MinusAssign }
                            else { Token::Minus }
                        }
                        '*' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::MulAssign }
                            else { Token::Star }
                        }
                        '/' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::DivAssign }
                            else { Token::Slash }
                        }
                        '=' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Eq }
                            else { Token::Assign }
                        }
                        '!' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Ne }
                            else { Token::Not }
                        }
                        '<' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Le }
                            else { Token::Lt }
                        }
                        '>' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Ge }
                            else { Token::Gt }
                        }
                        '&' => {
                            self.advance();
                            if self.peek() == Some('&') { self.advance(); Token::And }
                            else { return Err(RuntimeError::Internal("unexpected '&'".into())); }
                        }
                        '|' => {
                            self.advance();
                            if self.peek() == Some('|') { self.advance(); Token::Or }
                            else { return Err(RuntimeError::Internal("unexpected '|'".into())); }
                        }
                        '"' | '\'' => self.read_string()?,
                        c if c.is_ascii_digit() => self.read_number()?,
                        c if c.is_alphanumeric() || c == '_' => self.read_ident(),
                        other => {
                            return Err(RuntimeError::Internal(format!("unexpected char: '{}'", other)));
                        }
                    };
                    tokens.push(tok);
                }
            }
        }
    }

    fn read_string(&mut self) -> Result<Token, RuntimeError> {
        let quote = self.advance().unwrap();
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(RuntimeError::Internal("unterminated string".into())),
                Some(c) if c == quote => return Ok(Token::StringLit(s)),
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\\') => s.push('\\'),
                        Some(c) if c == quote => s.push(c),
                        _ => s.push('\\'),
                    }
                }
                Some(c) => s.push(c),
            }
        }
    }

    fn read_number(&mut self) -> Result<Token, RuntimeError> {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s.parse::<f64>()
            .map(Token::NumberLit)
            .map_err(|e| RuntimeError::Internal(format!("invalid number: {}", e)))
    }

    fn read_ident(&mut self) -> Token {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "let" => Token::Let,
            "return" => Token::Return,
            "await" => Token::Await,
            "true" => Token::BoolLit(true),
            "false" => Token::BoolLit(false),
            "null" | "None" => Token::Null,
            _ => Token::Ident(s),
        }
    }
}

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Stmt {
    Assign { target: Expr, value: Expr },
    CompoundAssign { target: Expr, op: CompoundOp, value: Expr },
    Let { name: String, value: Expr },
    If { condition: Expr, then_body: Vec<Stmt>, else_body: Vec<Stmt> },
    For { var: String, iterable: Expr, body: Vec<Stmt> },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone, Copy)]
enum CompoundOp { Add, Sub, Mul, Div }

#[derive(Debug, Clone)]
enum Expr {
    Literal(Value),
    Ident(String),
    DotAccess(Box<Expr>, String),
    IndexAccess(Box<Expr>, Box<Expr>),
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryNot(Box<Expr>),
    UnaryNeg(Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
}

#[derive(Debug, Clone, Copy)]
enum BinOp { Add, Sub, Mul, Div, Mod, Eq, Ne, Lt, Le, Gt, Ge, And, Or }

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), RuntimeError> {
        let got = self.advance();
        if std::mem::discriminant(&got) == std::mem::discriminant(expected) {
            Ok(())
        } else {
            Err(RuntimeError::Internal(format!(
                "expected {:?}, got {:?}", expected, got
            )))
        }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, RuntimeError> {
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::Eof) {
            stmts.push(self.parse_stmt()?);
            // Skip optional semicolons between statements
            while matches!(self.peek(), Token::Semicolon) {
                self.advance();
            }
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, RuntimeError> {
        match self.peek() {
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::For => self.parse_for(),
            Token::Return => self.parse_return(),
            _ => self.parse_assign_or_expr(),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, RuntimeError> {
        self.advance(); // consume 'let'
        let name = match self.advance() {
            Token::Ident(n) => n,
            other => return Err(RuntimeError::Internal(format!("expected identifier after let, got {:?}", other))),
        };
        self.expect(&Token::Assign)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let { name, value })
    }

    fn parse_if(&mut self) -> Result<Stmt, RuntimeError> {
        self.advance(); // consume 'if'
        let condition = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let then_body = self.parse_block()?;
        self.expect(&Token::RBrace)?;

        let else_body = if matches!(self.peek(), Token::Else) {
            self.advance(); // consume 'else'
            if matches!(self.peek(), Token::If) {
                vec![self.parse_if()?]
            } else {
                self.expect(&Token::LBrace)?;
                let body = self.parse_block()?;
                self.expect(&Token::RBrace)?;
                body
            }
        } else {
            vec![]
        };

        Ok(Stmt::If { condition, then_body, else_body })
    }

    fn parse_for(&mut self) -> Result<Stmt, RuntimeError> {
        self.advance(); // consume 'for'
        let var = match self.advance() {
            Token::Ident(n) => n,
            other => return Err(RuntimeError::Internal(format!("expected identifier after for, got {:?}", other))),
        };
        self.expect(&Token::In)?;
        let iterable = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::For { var, iterable, body })
    }

    fn parse_return(&mut self) -> Result<Stmt, RuntimeError> {
        self.advance(); // consume 'return'
        if matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
            Ok(Stmt::Return(None))
        } else {
            let expr = self.parse_expr()?;
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, RuntimeError> {
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            stmts.push(self.parse_stmt()?);
            while matches!(self.peek(), Token::Semicolon) {
                self.advance();
            }
        }
        Ok(stmts)
    }

    fn parse_assign_or_expr(&mut self) -> Result<Stmt, RuntimeError> {
        let expr = self.parse_expr()?;
        match self.peek() {
            Token::Assign => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Stmt::Assign { target: expr, value })
            }
            Token::PlusAssign => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Stmt::CompoundAssign { target: expr, op: CompoundOp::Add, value })
            }
            Token::MinusAssign => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Stmt::CompoundAssign { target: expr, op: CompoundOp::Sub, value })
            }
            Token::MulAssign => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Stmt::CompoundAssign { target: expr, op: CompoundOp::Mul, value })
            }
            Token::DivAssign => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Stmt::CompoundAssign { target: expr, op: CompoundOp::Div, value })
            }
            _ => Ok(Stmt::Expr(expr)),
        }
    }

    // Expression parsing: precedence climbing
    fn parse_expr(&mut self) -> Result<Expr, RuntimeError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, RuntimeError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, RuntimeError> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek(), Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, RuntimeError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, RuntimeError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, RuntimeError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, RuntimeError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, RuntimeError> {
        match self.peek() {
            Token::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryNot(Box::new(expr)))
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryNeg(Box::new(expr)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, RuntimeError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let field = match self.advance() {
                        Token::Ident(name) => name,
                        other => return Err(RuntimeError::Internal(format!("expected field name after '.', got {:?}", other))),
                    };
                    if matches!(self.peek(), Token::LParen) {
                        self.advance();
                        let args = self.parse_args()?;
                        self.expect(&Token::RParen)?;
                        expr = Expr::MethodCall(Box::new(expr), field, args);
                    } else {
                        expr = Expr::DotAccess(Box::new(expr), field);
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::IndexAccess(Box::new(expr), Box::new(index));
                }
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, RuntimeError> {
        match self.peek().clone() {
            Token::NumberLit(n) => {
                let n = n;
                self.advance();
                if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
                    Ok(Expr::Literal(Value::Number(serde_json::Number::from(n as i64))))
                } else {
                    Ok(Expr::Literal(Value::Number(
                        serde_json::Number::from_f64(n).unwrap_or(serde_json::Number::from(0)),
                    )))
                }
            }
            Token::StringLit(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Literal(Value::String(s)))
            }
            Token::BoolLit(b) => {
                self.advance();
                Ok(Expr::Literal(Value::Bool(b)))
            }
            Token::Null => {
                self.advance();
                Ok(Expr::Literal(Value::Null))
            }
            Token::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                    items.push(self.parse_expr()?);
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::Array(items))
            }
            Token::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                    let key = match self.advance() {
                        Token::Ident(k) => k,
                        Token::StringLit(k) => k,
                        other => return Err(RuntimeError::Internal(format!("expected object key, got {:?}", other))),
                    };
                    // Support shorthand { name } as { name: name }
                    if matches!(self.peek(), Token::Comma | Token::RBrace) {
                        pairs.push((key.clone(), Expr::Ident(key)));
                    } else {
                        // Expect colon — but we don't have a Colon token, we'll
                        // accept Assign (=) as object separator for DSL syntax.
                        // Actually, let's just skip the next token as separator.
                        self.advance(); // skip : or =
                        let val = self.parse_expr()?;
                        pairs.push((key, val));
                    }
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::Object(pairs))
            }
            other => Err(RuntimeError::Internal(format!("unexpected token: {:?}", other))),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, RuntimeError> {
        let mut args = Vec::new();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            args.push(self.parse_expr()?);
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
        }
        Ok(args)
    }
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

pub struct Interpreter;

impl Interpreter {
    /// Execute an action body string against the state store.
    pub fn execute(
        body: &str,
        state: &mut StateStore,
        args: &Value,
    ) -> Result<Value, RuntimeError> {
        let tokens = Lexer::new(body).tokenize()?;
        let stmts = Parser::new(tokens).parse_program()?;
        let mut env = Env::new(state, args);
        env.exec_block(&stmts)
    }

    /// Evaluate a single expression string against the state store (read-only).
    pub fn eval_expr(expr: &str, state: &StateStore) -> Result<Value, RuntimeError> {
        let tokens = Lexer::new(expr).tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_expr()?;
        let empty_args = Value::Null;
        let mut state_clone = state.clone();
        let env = Env::new(&mut state_clone, &empty_args);
        env.eval(&ast)
    }
}

struct Env<'a> {
    state: &'a mut StateStore,
    args: &'a Value,
    locals: std::collections::HashMap<String, Value>,
}

impl<'a> Env<'a> {
    fn new(state: &'a mut StateStore, args: &'a Value) -> Self {
        Self {
            state,
            args,
            locals: std::collections::HashMap::new(),
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Value, RuntimeError> {
        let mut result = Value::Null;
        for stmt in stmts {
            result = self.exec_stmt(stmt)?;
        }
        Ok(result)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Value, RuntimeError> {
        match stmt {
            Stmt::Assign { target, value } => {
                let val = self.eval(value)?;
                self.assign_target(target, val)?;
                Ok(Value::Null)
            }
            Stmt::CompoundAssign { target, op, value } => {
                let current = self.eval(target)?;
                let rhs = self.eval(value)?;
                let result = match op {
                    CompoundOp::Add => numeric_op(&current, &rhs, |a, b| a + b),
                    CompoundOp::Sub => numeric_op(&current, &rhs, |a, b| a - b),
                    CompoundOp::Mul => numeric_op(&current, &rhs, |a, b| a * b),
                    CompoundOp::Div => numeric_op(&current, &rhs, |a, b| if b != 0.0 { a / b } else { 0.0 }),
                };
                self.assign_target(target, result)?;
                Ok(Value::Null)
            }
            Stmt::Let { name, value } => {
                let val = self.eval(value)?;
                self.locals.insert(name.clone(), val);
                Ok(Value::Null)
            }
            Stmt::If { condition, then_body, else_body } => {
                let cond = self.eval(condition)?;
                if is_truthy(&cond) {
                    self.exec_block(then_body)
                } else {
                    self.exec_block(else_body)
                }
            }
            Stmt::For { var, iterable, body } => {
                let iter_val = self.eval(iterable)?;
                if let Value::Array(items) = iter_val {
                    for item in items {
                        self.locals.insert(var.clone(), item);
                        self.exec_block(body)?;
                    }
                }
                Ok(Value::Null)
            }
            Stmt::Expr(expr) => self.eval(expr),
            Stmt::Return(expr) => {
                match expr {
                    Some(e) => self.eval(e),
                    None => Ok(Value::Null),
                }
            }
        }
    }

    fn eval(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(v) => Ok(v.clone()),
            Expr::Ident(name) => {
                if let Some(val) = self.locals.get(name) {
                    return Ok(val.clone());
                }
                if let Some(val) = self.state.get(name) {
                    return Ok(val.clone());
                }
                if let Some(val) = self.args.get(name) {
                    return Ok(val.clone());
                }
                Ok(Value::Null)
            }
            Expr::DotAccess(obj, field) => {
                let val = self.eval(obj)?;
                Ok(val.get(field).cloned().unwrap_or(Value::Null))
            }
            Expr::IndexAccess(obj, index) => {
                let val = self.eval(obj)?;
                let idx = self.eval(index)?;
                match (&val, &idx) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let i = n.as_u64().unwrap_or(0) as usize;
                        Ok(arr.get(i).cloned().unwrap_or(Value::Null))
                    }
                    (Value::Object(map), Value::String(key)) => {
                        Ok(map.get(key).cloned().unwrap_or(Value::Null))
                    }
                    _ => Ok(Value::Null),
                }
            }
            Expr::BinaryOp(left, op, right) => {
                let lval = self.eval(left)?;
                // Short-circuit for && and ||
                match op {
                    BinOp::And => {
                        if !is_truthy(&lval) { return Ok(lval); }
                        return self.eval(right);
                    }
                    BinOp::Or => {
                        if is_truthy(&lval) { return Ok(lval); }
                        return self.eval(right);
                    }
                    _ => {}
                }
                let rval = self.eval(right)?;
                Ok(eval_binop(&lval, op, &rval))
            }
            Expr::UnaryNot(inner) => {
                let val = self.eval(inner)?;
                Ok(Value::Bool(!is_truthy(&val)))
            }
            Expr::UnaryNeg(inner) => {
                let val = self.eval(inner)?;
                Ok(numeric_op(&Value::Number(0.into()), &val, |_, b| -b))
            }
            Expr::Call(func, args) => {
                let evaled_args: Vec<Value> = args.iter().map(|a| self.eval(a)).collect::<Result<_, _>>()?;
                self.call_builtin(func, &evaled_args)
            }
            Expr::MethodCall(obj, method, args) => {
                let obj_val = self.eval(obj)?;
                let evaled_args: Vec<Value> = args.iter().map(|a| self.eval(a)).collect::<Result<_, _>>()?;
                self.call_method(&obj_val, method, &evaled_args)
            }
            Expr::Array(items) => {
                let vals: Vec<Value> = items.iter().map(|e| self.eval(e)).collect::<Result<_, _>>()?;
                Ok(Value::Array(vals))
            }
            Expr::Object(pairs) => {
                let mut map = serde_json::Map::new();
                for (key, val_expr) in pairs {
                    map.insert(key.clone(), self.eval(val_expr)?);
                }
                Ok(Value::Object(map))
            }
        }
    }

    fn assign_target(&mut self, target: &Expr, value: Value) -> Result<(), RuntimeError> {
        match target {
            Expr::Ident(name) => {
                if self.locals.contains_key(name) {
                    self.locals.insert(name.clone(), value);
                } else {
                    self.state.set(name, value);
                }
                Ok(())
            }
            Expr::DotAccess(obj, field) => {
                let path = collect_path(target);
                if let Some((root, rest)) = path.as_ref().and_then(|p| p.split_first()) {
                    let is_local = self.locals.contains_key(*root);
                    let mut current = if is_local {
                        self.locals.get(*root).cloned().unwrap_or(Value::Null)
                    } else {
                        self.state.get(*root).cloned().unwrap_or(Value::Null)
                    };

                    set_nested(&mut current, rest, value);

                    if is_local {
                        self.locals.insert(root.to_string(), current);
                    } else {
                        self.state.set(root, current);
                    }
                    Ok(())
                } else {
                    Err(RuntimeError::Internal(format!("cannot assign to expression: {:?}.{}", obj, field)))
                }
            }
            Expr::IndexAccess(obj, index) => {
                let idx_val = self.eval(index)?;
                let mut obj_val = self.eval(obj)?;
                match (&mut obj_val, &idx_val) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let i = n.as_u64().unwrap_or(0) as usize;
                        if i < arr.len() {
                            arr[i] = value;
                        }
                    }
                    (Value::Object(map), Value::String(key)) => {
                        map.insert(key.clone(), value);
                    }
                    _ => {}
                }
                self.assign_target(obj, obj_val)
            }
            _ => Err(RuntimeError::Internal("invalid assignment target".into())),
        }
    }

    fn call_builtin(&self, func: &Expr, _args: &[Value]) -> Result<Value, RuntimeError> {
        match func {
            Expr::Ident(name) => {
                Err(RuntimeError::Internal(format!("unknown function: {}", name)))
            }
            _ => Err(RuntimeError::Internal("not a callable expression".into())),
        }
    }

    fn call_method(&self, obj: &Value, method: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match (obj, method) {
            (Value::Array(arr), "len" | "length") => {
                Ok(Value::Number((arr.len() as i64).into()))
            }
            (Value::Array(arr), "push") => {
                let mut new_arr = arr.clone();
                for arg in args {
                    new_arr.push(arg.clone());
                }
                Ok(Value::Array(new_arr))
            }
            (Value::Array(arr), "filter") => {
                Ok(Value::Array(arr.clone()))
            }
            (Value::Array(arr), "map") => {
                Ok(Value::Array(arr.clone()))
            }
            (Value::Array(arr), "contains" | "includes") => {
                let needle = args.first().unwrap_or(&Value::Null);
                Ok(Value::Bool(arr.contains(needle)))
            }
            (Value::Array(arr), "is_empty" | "isEmpty") => {
                Ok(Value::Bool(arr.is_empty()))
            }
            (Value::String(s), "len" | "length") => {
                Ok(Value::Number((s.len() as i64).into()))
            }
            (Value::String(s), "to_lowercase" | "toLowerCase") => {
                Ok(Value::String(s.to_lowercase()))
            }
            (Value::String(s), "to_uppercase" | "toUpperCase") => {
                Ok(Value::String(s.to_uppercase()))
            }
            (Value::String(s), "trim") => {
                Ok(Value::String(s.trim().to_string()))
            }
            (Value::String(s), "contains" | "includes") => {
                let needle = args.first().and_then(|v| v.as_str()).unwrap_or("");
                Ok(Value::Bool(s.contains(needle)))
            }
            (Value::String(s), "starts_with" | "startsWith") => {
                let prefix = args.first().and_then(|v| v.as_str()).unwrap_or("");
                Ok(Value::Bool(s.starts_with(prefix)))
            }
            (Value::String(s), "ends_with" | "endsWith") => {
                let suffix = args.first().and_then(|v| v.as_str()).unwrap_or("");
                Ok(Value::Bool(s.ends_with(suffix)))
            }
            (Value::String(s), "split") => {
                let sep = args.first().and_then(|v| v.as_str()).unwrap_or(",");
                let parts: Vec<Value> = s.split(sep).map(|p| Value::String(p.to_string())).collect();
                Ok(Value::Array(parts))
            }
            (Value::String(s), "replace") => {
                let from = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let to = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
                Ok(Value::String(s.replace(from, to)))
            }
            (Value::String(s), "is_empty" | "isEmpty") => {
                Ok(Value::Bool(s.is_empty()))
            }
            (Value::Object(_), "keys") => {
                if let Value::Object(map) = obj {
                    let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
                    Ok(Value::Array(keys))
                } else {
                    Ok(Value::Array(vec![]))
                }
            }
            (Value::Object(_), "values") => {
                if let Value::Object(map) = obj {
                    let vals: Vec<Value> = map.values().cloned().collect();
                    Ok(Value::Array(vals))
                } else {
                    Ok(Value::Array(vec![]))
                }
            }
            (Value::Number(_), "abs") => {
                let n = to_f64(obj);
                Ok(to_json_number(n.abs()))
            }
            _ => Err(RuntimeError::Internal(format!(
                "unknown method '{}' on {:?}", method, value_type_name(obj)
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn to_f64(val: &Value) -> f64 {
    match val {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::Bool(true) => 1.0,
        Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn to_json_number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
        Value::Number((n as i64).into())
    } else {
        Value::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
    }
}

fn numeric_op(left: &Value, right: &Value, op: fn(f64, f64) -> f64) -> Value {
    // String concatenation for Add
    if let (Value::String(a), Value::String(b)) = (left, right) {
        return Value::String(format!("{}{}", a, b));
    }
    if let Value::String(a) = left {
        return Value::String(format!("{}{}", a, value_display(right)));
    }
    let a = to_f64(left);
    let b = to_f64(right);
    to_json_number(op(a, b))
}

fn eval_binop(left: &Value, op: &BinOp, right: &Value) -> Value {
    match op {
        BinOp::Add => numeric_op(left, right, |a, b| a + b),
        BinOp::Sub => numeric_op(left, right, |a, b| a - b),
        BinOp::Mul => numeric_op(left, right, |a, b| a * b),
        BinOp::Div => numeric_op(left, right, |a, b| if b != 0.0 { a / b } else { 0.0 }),
        BinOp::Mod => numeric_op(left, right, |a, b| if b != 0.0 { a % b } else { 0.0 }),
        BinOp::Eq => Value::Bool(left == right),
        BinOp::Ne => Value::Bool(left != right),
        BinOp::Lt => Value::Bool(to_f64(left) < to_f64(right)),
        BinOp::Le => Value::Bool(to_f64(left) <= to_f64(right)),
        BinOp::Gt => Value::Bool(to_f64(left) > to_f64(right)),
        BinOp::Ge => Value::Bool(to_f64(left) >= to_f64(right)),
        BinOp::And | BinOp::Or => unreachable!("handled in eval"),
    }
}

fn collect_path<'a>(expr: &'a Expr) -> Option<Vec<&'a str>> {
    match expr {
        Expr::Ident(name) => Some(vec![name.as_str()]),
        Expr::DotAccess(parent, field) => {
            let mut path = collect_path(parent)?;
            path.push(field.as_str());
            Some(path)
        }
        _ => None,
    }
}

fn set_nested(val: &mut Value, path: &[&str], new_val: Value) {
    if path.is_empty() {
        *val = new_val;
        return;
    }
    if path.len() == 1 {
        if let Value::Object(map) = val {
            map.insert(path[0].to_string(), new_val);
        }
        return;
    }
    if let Value::Object(map) = val {
        let child = map.entry(path[0].to_string()).or_insert(Value::Object(serde_json::Map::new()));
        set_nested(child, &path[1..], new_val);
    }
}

fn value_display(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

fn value_type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn exec(body: &str, state: &mut StateStore) -> Result<Value, RuntimeError> {
        Interpreter::execute(body, state, &Value::Null)
    }

    fn exec_with_args(body: &str, state: &mut StateStore, args: &Value) -> Result<Value, RuntimeError> {
        Interpreter::execute(body, state, args)
    }

    // -- Simple assignments --

    #[test]
    fn assign_number() {
        let mut state = StateStore::new();
        exec("count = 42", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(42)));
    }

    #[test]
    fn assign_string() {
        let mut state = StateStore::new();
        exec("name = \"Alice\"", &mut state).unwrap();
        assert_eq!(state.get("name"), Some(&json!("Alice")));
    }

    #[test]
    fn assign_bool() {
        let mut state = StateStore::new();
        exec("loading = true", &mut state).unwrap();
        assert_eq!(state.get("loading"), Some(&json!(true)));
    }

    // -- Compound assignments --

    #[test]
    fn plus_assign() {
        let mut state = StateStore::new();
        state.set("count", json!(10));
        exec("count += 1", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(11)));
    }

    #[test]
    fn minus_assign() {
        let mut state = StateStore::new();
        state.set("count", json!(10));
        exec("count -= 3", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(7)));
    }

    #[test]
    fn mul_assign() {
        let mut state = StateStore::new();
        state.set("val", json!(5));
        exec("val *= 4", &mut state).unwrap();
        assert_eq!(state.get("val"), Some(&json!(20)));
    }

    #[test]
    fn div_assign() {
        let mut state = StateStore::new();
        state.set("val", json!(20));
        exec("val /= 4", &mut state).unwrap();
        assert_eq!(state.get("val"), Some(&json!(5)));
    }

    // -- Dot path assignment --

    #[test]
    fn nested_dot_assign() {
        let mut state = StateStore::new();
        state.set("user", json!({"name": "Old", "age": 25}));
        exec("user.name = \"New\"", &mut state).unwrap();
        let user = state.get("user").unwrap();
        assert_eq!(user.get("name"), Some(&json!("New")));
        assert_eq!(user.get("age"), Some(&json!(25)));
    }

    // -- Expressions --

    #[test]
    fn arithmetic_expression() {
        let mut state = StateStore::new();
        exec("result = 2 + 3 * 4", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!(14)));
    }

    #[test]
    fn comparison_expression() {
        let mut state = StateStore::new();
        state.set("x", json!(5));
        exec("bigger = x > 3", &mut state).unwrap();
        assert_eq!(state.get("bigger"), Some(&json!(true)));
    }

    #[test]
    fn string_concat() {
        let mut state = StateStore::new();
        state.set("first", json!("Hello"));
        state.set("second", json!(" World"));
        exec("greeting = first + second", &mut state).unwrap();
        assert_eq!(state.get("greeting"), Some(&json!("Hello World")));
    }

    // -- If/else in action --

    #[test]
    fn if_else_statement() {
        let mut state = StateStore::new();
        state.set("x", json!(10));
        exec("if x > 5 { result = \"big\" } else { result = \"small\" }", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!("big")));

        state.set("x", json!(2));
        exec("if x > 5 { result = \"big\" } else { result = \"small\" }", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!("small")));
    }

    // -- For loop in action --

    #[test]
    fn for_loop() {
        let mut state = StateStore::new();
        state.set("items", json!([1, 2, 3]));
        state.set("sum", json!(0));
        exec("for item in items { sum += item }", &mut state).unwrap();
        assert_eq!(state.get("sum"), Some(&json!(6)));
    }

    // -- Let binding --

    #[test]
    fn let_binding() {
        let mut state = StateStore::new();
        exec("let x = 10; result = x * 2", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!(20)));
        assert!(state.get("x").is_none(), "let binding should not leak to state");
    }

    // -- Method calls --

    #[test]
    fn string_methods() {
        let mut state = StateStore::new();
        state.set("s", json!("Hello World"));
        exec("lower = s.to_lowercase()", &mut state).unwrap();
        assert_eq!(state.get("lower"), Some(&json!("hello world")));

        exec("has = s.contains(\"World\")", &mut state).unwrap();
        assert_eq!(state.get("has"), Some(&json!(true)));
    }

    #[test]
    fn array_methods() {
        let mut state = StateStore::new();
        state.set("arr", json!([1, 2, 3]));
        exec("size = arr.len()", &mut state).unwrap();
        assert_eq!(state.get("size"), Some(&json!(3)));

        exec("empty = arr.is_empty()", &mut state).unwrap();
        assert_eq!(state.get("empty"), Some(&json!(false)));
    }

    // -- Args access --

    #[test]
    fn access_args() {
        let mut state = StateStore::new();
        let args = json!({"value": 42, "name": "test"});
        exec_with_args("count = value; label = name", &mut state, &args).unwrap();
        assert_eq!(state.get("count"), Some(&json!(42)));
        assert_eq!(state.get("label"), Some(&json!("test")));
    }

    // -- Logical operators --

    #[test]
    fn logical_and_or() {
        let mut state = StateStore::new();
        state.set("a", json!(true));
        state.set("b", json!(false));
        exec("r1 = a && b; r2 = a || b", &mut state).unwrap();
        assert_eq!(state.get("r1"), Some(&json!(false)));
        assert_eq!(state.get("r2"), Some(&json!(true)));
    }

    #[test]
    fn unary_not() {
        let mut state = StateStore::new();
        state.set("flag", json!(true));
        exec("flipped = !flag", &mut state).unwrap();
        assert_eq!(state.get("flipped"), Some(&json!(false)));
    }

    // -- Multi-statement --

    #[test]
    fn multi_statement() {
        let mut state = StateStore::new();
        state.set("count", json!(0));
        exec("count += 1; count += 1; count += 1", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(3)));
    }

    // -- Array/Object literals --

    #[test]
    fn array_literal() {
        let mut state = StateStore::new();
        exec("items = [1, 2, 3]", &mut state).unwrap();
        assert_eq!(state.get("items"), Some(&json!([1, 2, 3])));
    }

    // -- Index access --

    #[test]
    fn index_access() {
        let mut state = StateStore::new();
        state.set("items", json!(["a", "b", "c"]));
        exec("first = items[0]", &mut state).unwrap();
        assert_eq!(state.get("first"), Some(&json!("a")));
    }

    // -- Dirty tracking --

    #[test]
    fn dirty_tracking() {
        let mut state = StateStore::new();
        state.set("count", json!(0));
        state.clear_dirty();

        exec("count += 1", &mut state).unwrap();
        assert!(state.is_dirty("count"));
    }

    // -- Complex scenario: counter --

    #[test]
    fn counter_increment() {
        let mut state = StateStore::new();
        state.set("count", json!(0));
        exec("count += 1", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(1)));
        exec("count += 1", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(2)));
    }

    // -- Complex scenario: toggle --

    #[test]
    fn toggle_boolean() {
        let mut state = StateStore::new();
        state.set("show", json!(true));
        exec("show = !show", &mut state).unwrap();
        assert_eq!(state.get("show"), Some(&json!(false)));
        exec("show = !show", &mut state).unwrap();
        assert_eq!(state.get("show"), Some(&json!(true)));
    }

    // -- Complex scenario: form handler --

    #[test]
    fn form_handler() {
        let mut state = StateStore::new();
        state.set("items", json!([]));
        state.set("input", json!("New Item"));
        let args = json!({"text": "From Form"});
        exec_with_args(
            "let new_item = text; items = items.push(new_item); input = \"\"",
            &mut state,
            &args,
        ).unwrap();
        let items = state.get("items").unwrap();
        assert!(items.as_array().unwrap().contains(&json!("From Form")));
        assert_eq!(state.get("input"), Some(&json!("")));
    }

    // -- Edge cases --

    #[test]
    fn division_by_zero() {
        let mut state = StateStore::new();
        exec("result = 10 / 0", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!(0)));
    }

    #[test]
    fn empty_body() {
        let mut state = StateStore::new();
        let result = exec("", &mut state);
        assert!(result.is_ok());
    }

    #[test]
    fn comments() {
        let mut state = StateStore::new();
        exec("// this is a comment\ncount = 1", &mut state).unwrap();
        assert_eq!(state.get("count"), Some(&json!(1)));
    }

    #[test]
    fn nested_if() {
        let mut state = StateStore::new();
        state.set("x", json!(15));
        exec(
            "if x > 10 { if x > 20 { r = \"huge\" } else { r = \"big\" } } else { r = \"small\" }",
            &mut state,
        ).unwrap();
        assert_eq!(state.get("r"), Some(&json!("big")));
    }

    #[test]
    fn string_number_coercion() {
        let mut state = StateStore::new();
        state.set("label", json!("Count: "));
        state.set("count", json!(42));
        exec("display = label + count", &mut state).unwrap();
        assert_eq!(state.get("display"), Some(&json!("Count: 42")));
    }

    #[test]
    fn deeply_nested_dot_access() {
        let mut state = StateStore::new();
        state.set("data", json!({"user": {"profile": {"name": "Alice"}}}));
        exec("name = data.user.profile.name", &mut state).unwrap();
        assert_eq!(state.get("name"), Some(&json!("Alice")));
    }

    #[test]
    fn modulo_operator() {
        let mut state = StateStore::new();
        exec("result = 10 % 3", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!(1)));
    }

    #[test]
    fn equality_comparison() {
        let mut state = StateStore::new();
        state.set("status", json!("active"));
        exec("is_active = status == \"active\"", &mut state).unwrap();
        assert_eq!(state.get("is_active"), Some(&json!(true)));

        exec("is_pending = status != \"active\"", &mut state).unwrap();
        assert_eq!(state.get("is_pending"), Some(&json!(false)));
    }

    #[test]
    fn parenthesized_expression() {
        let mut state = StateStore::new();
        exec("result = (2 + 3) * 4", &mut state).unwrap();
        assert_eq!(state.get("result"), Some(&json!(20)));
    }

    #[test]
    fn negative_number() {
        let mut state = StateStore::new();
        exec("x = -5", &mut state).unwrap();
        assert_eq!(state.get("x"), Some(&json!(-5)));
    }
}
