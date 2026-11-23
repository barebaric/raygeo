//! A small, host-agnostic expression evaluator for text templates.
//!
//! The language covers the subset used by sketch text templates:
//! numbers, strings, booleans, dotted names, function calls, arithmetic,
//! comparisons and logic. It is intentionally not a Python interpreter:
//! names are looked up in a [`TemplateContext`], so hosts provide their
//! own variables and zero-argument template functions.
//!
//! Math functions and constants are built in, so a wasm or native host
//! does not need a math library of its own.

use std::collections::BTreeSet;
use std::fmt;

use crate::template::placeholders::names_in_expression;

/// A value produced by an expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A whole number.
    Int(i64),
    /// A floating-point number.
    Float(f64),
    /// A string.
    Text(String),
    /// A boolean.
    Bool(bool),
    /// Python's `None`.
    None,
}

impl Value {
    /// Whether the value is truthy under Python-like rules.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Int(value) => *value != 0,
            Value::Float(value) => *value != 0.0,
            Value::Text(value) => !value.is_empty(),
            Value::Bool(value) => *value,
            Value::None => false,
        }
    }

    /// The value as a floating-point number, if possible.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(value) => Some(*value as f64),
            Value::Float(value) => Some(*value),
            Value::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// The value formatted the way a template renders it by default.
    pub fn display(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Float(value) => format_float(*value),
            Value::Text(value) => value.clone(),
            Value::Bool(value) => {
                if *value {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            Value::None => "None".to_string(),
        }
    }

    /// A stable representation used in dependency stamps.
    pub fn stamp_repr(&self) -> String {
        match self {
            Value::Text(value) => format!("{value:?}"),
            _ => self.display(),
        }
    }
}

/// An expression evaluation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalError {
    message: String,
}

impl EvalError {
    /// Creates an error with a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        EvalError {
            message: message.into(),
        }
    }

    /// The error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EvalError {}

/// The variables and functions an expression can use.
pub trait TemplateContext {
    /// Returns the value of a variable, or `None` when it is unknown.
    fn variable(&self, name: &str) -> Option<Value>;

    /// Calls a host function, or returns `None` when it is unknown.
    ///
    /// A function that exists but cannot be called returns
    /// `Some(Err(..))`.
    fn call(
        &self,
        name: &str,
        arguments: &[Value],
    ) -> Option<Result<Value, EvalError>>;
}

/// Evaluates an expression and returns its value.
pub fn evaluate(
    expression: &str,
    context: &dyn TemplateContext,
) -> Result<Value, EvalError> {
    let tokens = lex(expression)?;
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_expression()?;
    parser.expect_end()?;
    eval(&parsed, context)
}

/// Evaluates an expression as a floating-point number.
pub fn evaluate_number(
    expression: &str,
    context: &dyn TemplateContext,
) -> Result<f64, EvalError> {
    let value = evaluate(expression, context)?;
    value
        .as_f64()
        .ok_or_else(|| EvalError::new("expression is not numeric"))
}

/// Formats a value with a Python-style format spec.
pub fn format_value(value: &Value, spec: &str) -> String {
    format_with_spec(value, spec)
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(String),
    String(String),
    Name(String),
    Plus,
    Minus,
    Star,
    Slash,
    FloorDiv,
    Percent,
    Pow,
    LParen,
    RParen,
    Comma,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    NotEqual,
    Not,
    And,
    Or,
    Question,
    Colon,
    End,
}

struct Lexer {
    chars: Vec<char>,
    position: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Lexer {
            chars: input.chars().collect(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.position + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.position += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += 1;
        }
    }

    fn lex_number(&mut self) -> String {
        let start = self.position;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.position += 1;
        }
        if self.peek() == Some('.') {
            self.position += 1;
            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.position += 1;
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.position += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.position += 1;
            }
            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.position += 1;
            }
        }
        self.chars[start..self.position].iter().collect()
    }

    fn lex_name(&mut self) -> String {
        let start = self.position;
        while self
            .peek()
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
        {
            self.position += 1;
        }
        while self.peek() == Some('.')
            && self
                .peek_next()
                .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
        {
            self.position += 1;
            while self
                .peek()
                .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
            {
                self.position += 1;
            }
        }
        self.chars[start..self.position].iter().collect()
    }

    fn lex_string(&mut self, quote: char) -> Result<String, EvalError> {
        let mut value = String::new();
        loop {
            let Some(ch) = self.advance() else {
                return Err(EvalError::new("unterminated string"));
            };
            if ch == quote {
                return Ok(value);
            }
            if ch == '\\' {
                let Some(escaped) = self.advance() else {
                    return Err(EvalError::new("unterminated escape"));
                };
                value.push(match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    other => other,
                });
            } else {
                value.push(ch);
            }
        }
    }
}

fn lex(input: &str) -> Result<Vec<Token>, EvalError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        lexer.skip_whitespace();
        let Some(ch) = lexer.peek() else {
            tokens.push(Token::End);
            return Ok(tokens);
        };
        let token = match ch {
            '0'..='9' => Token::Number(lexer.lex_number()),
            '.' if lexer.peek_next().is_some_and(|c| c.is_ascii_digit()) => {
                Token::Number(lexer.lex_number())
            }
            '\'' | '"' => {
                lexer.advance();
                Token::String(lexer.lex_string(ch)?)
            }
            '+' => {
                lexer.advance();
                Token::Plus
            }
            '-' => {
                lexer.advance();
                Token::Minus
            }
            '*' => {
                lexer.advance();
                if lexer.peek() == Some('*') {
                    lexer.advance();
                    Token::Pow
                } else {
                    Token::Star
                }
            }
            '/' => {
                lexer.advance();
                if lexer.peek() == Some('/') {
                    lexer.advance();
                    Token::FloorDiv
                } else {
                    Token::Slash
                }
            }
            '%' => {
                lexer.advance();
                Token::Percent
            }
            '(' => {
                lexer.advance();
                Token::LParen
            }
            ')' => {
                lexer.advance();
                Token::RParen
            }
            ',' => {
                lexer.advance();
                Token::Comma
            }
            '<' => {
                lexer.advance();
                if lexer.peek() == Some('=') {
                    lexer.advance();
                    Token::LessEqual
                } else {
                    Token::Less
                }
            }
            '>' => {
                lexer.advance();
                if lexer.peek() == Some('=') {
                    lexer.advance();
                    Token::GreaterEqual
                } else {
                    Token::Greater
                }
            }
            '=' => {
                lexer.advance();
                if lexer.peek() == Some('=') {
                    lexer.advance();
                    Token::EqualEqual
                } else {
                    return Err(EvalError::new("unexpected '='"));
                }
            }
            '!' => {
                lexer.advance();
                if lexer.peek() == Some('=') {
                    lexer.advance();
                    Token::NotEqual
                } else {
                    Token::Not
                }
            }
            '?' => {
                lexer.advance();
                Token::Question
            }
            ':' => {
                lexer.advance();
                Token::Colon
            }
            ch if ch.is_alphabetic() || ch == '_' => {
                let name = lexer.lex_name();
                match name.as_str() {
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    _ => Token::Name(name),
                }
            }
            other => {
                return Err(EvalError::new(format!(
                    "unexpected character '{other}'"
                )));
            }
        };
        tokens.push(token);
    }
}

#[derive(Debug)]
enum Expr {
    Value(Value),
    Variable(String),
    Call(String, Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Clone, Copy, Debug)]
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    FloorDivide,
    Modulo,
    Power,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        self.position += 1;
        token
    }

    fn expect_end(&self) -> Result<(), EvalError> {
        if matches!(self.peek(), Token::End) {
            Ok(())
        } else {
            Err(EvalError::new("unexpected token at end of expression"))
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, EvalError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, EvalError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(BinaryOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, EvalError> {
        let mut left = self.parse_not()?;
        while matches!(self.peek(), Token::And) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::Binary(BinaryOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, EvalError> {
        if matches!(self.peek(), Token::Not) {
            self.advance();
            let operand = self.parse_not()?;
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(operand)));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, EvalError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Less => BinaryOp::Less,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::Greater => BinaryOp::Greater,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                Token::EqualEqual => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, EvalError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, EvalError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::FloorDiv => BinaryOp::FloorDivide,
                Token::Percent => BinaryOp::Modulo,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, EvalError> {
        match self.peek() {
            Token::Plus => {
                self.advance();
                Ok(Expr::Unary(UnaryOp::Plus, Box::new(self.parse_unary()?)))
            }
            Token::Minus => {
                self.advance();
                Ok(Expr::Unary(UnaryOp::Minus, Box::new(self.parse_unary()?)))
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Expr, EvalError> {
        let left = self.parse_primary()?;
        if matches!(self.peek(), Token::Pow) {
            self.advance();
            let right = self.parse_unary()?;
            return Ok(Expr::Binary(
                BinaryOp::Power,
                Box::new(left),
                Box::new(right),
            ));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, EvalError> {
        match self.advance() {
            Token::Number(literal) => Ok(Expr::Value(parse_number(&literal))),
            Token::String(value) => Ok(Expr::Value(Value::Text(value))),
            Token::Name(name) => match name.as_str() {
                "True" => Ok(Expr::Value(Value::Bool(true))),
                "False" => Ok(Expr::Value(Value::Bool(false))),
                "None" => Ok(Expr::Value(Value::None)),
                _ => {
                    if matches!(self.peek(), Token::LParen) {
                        self.advance();
                        let mut arguments = Vec::new();
                        if !matches!(self.peek(), Token::RParen) {
                            loop {
                                arguments.push(self.parse_expression()?);
                                if matches!(self.peek(), Token::Comma) {
                                    self.advance();
                                    continue;
                                }
                                break;
                            }
                        }
                        if !matches!(self.advance(), Token::RParen) {
                            return Err(EvalError::new("expected ')'"));
                        }
                        Ok(Expr::Call(name, arguments))
                    } else {
                        Ok(Expr::Variable(name))
                    }
                }
            },
            Token::LParen => {
                let expression = self.parse_expression()?;
                if !matches!(self.advance(), Token::RParen) {
                    return Err(EvalError::new("expected ')'"));
                }
                Ok(expression)
            }
            _ => Err(EvalError::new("expected a value")),
        }
    }
}

fn parse_number(literal: &str) -> Value {
    if literal.contains('.') || literal.contains('e') || literal.contains('E') {
        literal
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or(Value::Float(0.0))
    } else {
        literal.parse::<i64>().map(Value::Int).unwrap_or_else(|_| {
            literal
                .parse::<f64>()
                .map(Value::Float)
                .unwrap_or(Value::Float(0.0))
        })
    }
}

fn eval(
    expression: &Expr,
    context: &dyn TemplateContext,
) -> Result<Value, EvalError> {
    match expression {
        Expr::Value(value) => Ok(value.clone()),
        Expr::Variable(name) => lookup_variable(name, context),
        Expr::Call(name, arguments) => {
            let values = arguments
                .iter()
                .map(|argument| eval(argument, context))
                .collect::<Result<Vec<_>, _>>()?;
            call_function(name, &values, context)
        }
        Expr::Unary(operator, operand) => {
            let value = eval(operand, context)?;
            match operator {
                UnaryOp::Plus => numeric_value(&value),
                UnaryOp::Minus => match numeric_value(&value)? {
                    Value::Int(value) => Ok(Value::Int(-value)),
                    Value::Float(value) => Ok(Value::Float(-value)),
                    _ => Err(EvalError::new("expected a number")),
                },
                UnaryOp::Not => Ok(Value::Bool(!value.truthy())),
            }
        }
        Expr::Binary(operator, left, right) => {
            if matches!(operator, BinaryOp::And | BinaryOp::Or) {
                let left_value = eval(left, context)?;
                return match operator {
                    BinaryOp::And if !left_value.truthy() => Ok(left_value),
                    BinaryOp::Or if left_value.truthy() => Ok(left_value),
                    _ => eval(right, context),
                };
            }
            let left_value = eval(left, context)?;
            let right_value = eval(right, context)?;
            binary(*operator, &left_value, &right_value)
        }
    }
}

fn lookup_variable(
    name: &str,
    context: &dyn TemplateContext,
) -> Result<Value, EvalError> {
    if let Some(value) = context.variable(name) {
        return Ok(value);
    }
    if let Some(value) = constant(name) {
        return Ok(value);
    }
    if let Some(last) = name.rsplit('.').next() {
        if let Some(value) = constant(last) {
            return Ok(value);
        }
    }
    Err(EvalError::new(format!("unknown variable '{name}'")))
}

fn call_function(
    name: &str,
    arguments: &[Value],
    context: &dyn TemplateContext,
) -> Result<Value, EvalError> {
    if let Some(result) = context.call(name, arguments) {
        return result;
    }
    if let Some(last) = name.rsplit('.').next() {
        if last != name {
            if let Some(result) = context.call(last, arguments) {
                return result;
            }
        }
    }
    if let Some(result) = call_math(name, arguments) {
        return result;
    }
    if let Some(last) = name.rsplit('.').next() {
        if last != name {
            if let Some(result) = call_math(last, arguments) {
                return result;
            }
        }
    }
    Err(EvalError::new(format!("unknown function '{name}'")))
}

fn constant(name: &str) -> Option<Value> {
    match name {
        "pi" => Some(Value::Float(std::f64::consts::PI)),
        "e" => Some(Value::Float(std::f64::consts::E)),
        "tau" => Some(Value::Float(std::f64::consts::TAU)),
        _ => None,
    }
}

fn numeric_value(value: &Value) -> Result<Value, EvalError> {
    match value {
        Value::Int(_) | Value::Float(_) => Ok(value.clone()),
        Value::Bool(value) => Ok(Value::Int(if *value { 1 } else { 0 })),
        _ => Err(EvalError::new("expected a number")),
    }
}

fn as_number(value: &Value) -> Result<f64, EvalError> {
    value
        .as_f64()
        .ok_or_else(|| EvalError::new("expected a number"))
}

fn both_int(left: &Value, right: &Value) -> Option<(i64, i64)> {
    match (left, right) {
        (Value::Int(left), Value::Int(right)) => Some((*left, *right)),
        (Value::Bool(left), Value::Bool(right)) => {
            Some((i64::from(*left), i64::from(*right)))
        }
        (Value::Int(left), Value::Bool(right)) => {
            Some((*left, i64::from(*right)))
        }
        (Value::Bool(left), Value::Int(right)) => {
            Some((i64::from(*left), *right))
        }
        _ => None,
    }
}

fn binary(
    operator: BinaryOp,
    left: &Value,
    right: &Value,
) -> Result<Value, EvalError> {
    match operator {
        BinaryOp::Add => add(left, right),
        BinaryOp::Subtract => match both_int(left, right) {
            Some((left, right)) => Ok(Value::Int(left - right)),
            None => Ok(Value::Float(as_number(left)? - as_number(right)?)),
        },
        BinaryOp::Multiply => match both_int(left, right) {
            Some((left, right)) => Ok(Value::Int(left * right)),
            None => Ok(Value::Float(as_number(left)? * as_number(right)?)),
        },
        BinaryOp::Divide => {
            let divisor = as_number(right)?;
            if divisor == 0.0 {
                return Err(EvalError::new("division by zero"));
            }
            Ok(Value::Float(as_number(left)? / divisor))
        }
        BinaryOp::FloorDivide => match both_int(left, right) {
            Some((left, right)) => {
                if right == 0 {
                    return Err(EvalError::new("division by zero"));
                }
                Ok(Value::Int(floor_div(left, right)))
            }
            None => {
                let divisor = as_number(right)?;
                if divisor == 0.0 {
                    return Err(EvalError::new("division by zero"));
                }
                Ok(Value::Float((as_number(left)? / divisor).floor()))
            }
        },
        BinaryOp::Modulo => match both_int(left, right) {
            Some((left, right)) => {
                if right == 0 {
                    return Err(EvalError::new("division by zero"));
                }
                Ok(Value::Int(floor_mod(left, right)))
            }
            None => {
                let divisor = as_number(right)?;
                if divisor == 0.0 {
                    return Err(EvalError::new("division by zero"));
                }
                let quotient = (as_number(left)? / divisor).floor();
                Ok(Value::Float(as_number(left)? - quotient * divisor))
            }
        },
        BinaryOp::Power => match both_int(left, right) {
            Some((left, right)) if (0..=32).contains(&right) => {
                Ok(Value::Int(left.pow(right as u32)))
            }
            _ => {
                Ok(Value::Float(libm::pow(as_number(left)?, as_number(right)?)))
            }
        },
        BinaryOp::Less => compare_values(left, right)
            .map(|ordering| Value::Bool(ordering == std::cmp::Ordering::Less)),
        BinaryOp::LessEqual => compare_values(left, right).map(|ordering| {
            Value::Bool(ordering != std::cmp::Ordering::Greater)
        }),
        BinaryOp::Greater => compare_values(left, right).map(|ordering| {
            Value::Bool(ordering == std::cmp::Ordering::Greater)
        }),
        BinaryOp::GreaterEqual => compare_values(left, right)
            .map(|ordering| Value::Bool(ordering != std::cmp::Ordering::Less)),
        BinaryOp::Equal => Ok(Value::Bool(left == right)),
        BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
        BinaryOp::And | BinaryOp::Or => unreachable!(),
    }
}

fn add(left: &Value, right: &Value) -> Result<Value, EvalError> {
    if let (Value::Text(left), Value::Text(right)) = (left, right) {
        return Ok(Value::Text(format!("{left}{right}")));
    }
    if matches!(left, Value::Text(_)) || matches!(right, Value::Text(_)) {
        return Err(EvalError::new("cannot add a string and a number"));
    }
    match both_int(left, right) {
        Some((left, right)) => Ok(Value::Int(left + right)),
        None => Ok(Value::Float(as_number(left)? + as_number(right)?)),
    }
}

fn compare_values(
    left: &Value,
    right: &Value,
) -> Result<std::cmp::Ordering, EvalError> {
    match (left, right) {
        (Value::Text(left), Value::Text(right)) => Ok(left.cmp(right)),
        _ => {
            let left = as_number(left)?;
            let right = as_number(right)?;
            left.partial_cmp(&right)
                .ok_or_else(|| EvalError::new("cannot compare values"))
        }
    }
}

fn floor_div(left: i64, right: i64) -> i64 {
    let quotient = left / right;
    let remainder = left % right;
    if remainder != 0 && (remainder < 0) != (right < 0) {
        quotient - 1
    } else {
        quotient
    }
}

fn floor_mod(left: i64, right: i64) -> i64 {
    let remainder = left % right;
    if remainder != 0 && (remainder < 0) != (right < 0) {
        remainder + right
    } else {
        remainder
    }
}

fn call_math(
    name: &str,
    arguments: &[Value],
) -> Option<Result<Value, EvalError>> {
    if !matches!(
        name,
        "sqrt"
            | "abs"
            | "round"
            | "floor"
            | "ceil"
            | "trunc"
            | "exp"
            | "ln"
            | "log10"
            | "log2"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "pow"
            | "hypot"
            | "min"
            | "max"
            | "log"
    ) {
        return None;
    }
    Some((|| -> Result<Value, EvalError> {
        let number = |index: usize| -> Result<f64, EvalError> {
            arguments
                .get(index)
                .ok_or_else(|| EvalError::new("missing argument"))
                .and_then(as_number)
        };
        match name {
            "sqrt" => Ok(Value::Float(libm::sqrt(number(0)?))),
            "abs" => match arguments.first() {
                Some(Value::Int(value)) => Ok(Value::Int(value.abs())),
                Some(value) => Ok(Value::Float(as_number(value)?.abs())),
                None => Err(EvalError::new("missing argument")),
            },
            "round" => match arguments.len() {
                0 => Err(EvalError::new("missing argument")),
                1 => Ok(Value::Int(as_number(&arguments[0])?.round() as i64)),
                _ => {
                    let digits = as_number(&arguments[1])? as i32;
                    let factor = 10f64.powi(digits);
                    Ok(Value::Float(
                        (as_number(&arguments[0])? * factor).round() / factor,
                    ))
                }
            },
            "floor" => Ok(Value::Int(libm::floor(number(0)?) as i64)),
            "ceil" => Ok(Value::Int(libm::ceil(number(0)?) as i64)),
            "trunc" => Ok(Value::Int(number(0)?.trunc() as i64)),
            "exp" => Ok(Value::Float(libm::exp(number(0)?))),
            "ln" => Ok(Value::Float(libm::log(number(0)?))),
            "log10" => Ok(Value::Float(libm::log10(number(0)?))),
            "log2" => Ok(Value::Float(libm::log2(number(0)?))),
            "sin" => Ok(Value::Float(libm::sin(number(0)?))),
            "cos" => Ok(Value::Float(libm::cos(number(0)?))),
            "tan" => Ok(Value::Float(libm::tan(number(0)?))),
            "asin" => Ok(Value::Float(libm::asin(number(0)?))),
            "acos" => Ok(Value::Float(libm::acos(number(0)?))),
            "atan" => Ok(Value::Float(libm::atan(number(0)?))),
            "atan2" => Ok(Value::Float(libm::atan2(number(0)?, number(1)?))),
            "pow" => Ok(Value::Float(libm::pow(number(0)?, number(1)?))),
            "hypot" => {
                let mut sum = 0.0;
                for argument in arguments {
                    let value = as_number(argument)?;
                    sum += value * value;
                }
                Ok(Value::Float(libm::sqrt(sum)))
            }
            "min" | "max" => {
                if arguments.is_empty() {
                    Err(EvalError::new("missing argument"))
                } else {
                    let mut best = as_number(&arguments[0])?;
                    for argument in &arguments[1..] {
                        let value = as_number(argument)?;
                        if (name == "min" && value < best)
                            || (name == "max" && value > best)
                        {
                            best = value;
                        }
                    }
                    if arguments.iter().all(|value| {
                        matches!(value, Value::Int(_) | Value::Bool(_))
                    }) {
                        Ok(Value::Int(best as i64))
                    } else {
                        Ok(Value::Float(best))
                    }
                }
            }
            "log" => match arguments.len() {
                1 => Ok(Value::Float(libm::log(number(0)?))),
                2 => Ok(Value::Float(
                    libm::log(number(0)?) / libm::log(number(1)?),
                )),
                _ => Err(EvalError::new("wrong number of arguments")),
            },
            _ => unreachable!(),
        }
    })())
}

fn format_float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    let text = format!("{value}");
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

#[derive(Default)]
struct FormatSpec {
    fill: Option<char>,
    align: Option<char>,
    sign: Option<char>,
    zero: bool,
    width: usize,
    comma: bool,
    precision: Option<usize>,
    ty: Option<char>,
}

fn parse_format_spec(spec: &str) -> FormatSpec {
    let mut parsed = FormatSpec::default();
    let chars: Vec<char> = spec.chars().collect();
    let mut index = 0;
    if chars.len() >= 2 && matches!(chars[1], '<' | '>' | '^' | '=') {
        parsed.fill = Some(chars[0]);
        parsed.align = Some(chars[1]);
        index = 2;
    } else if chars
        .first()
        .is_some_and(|ch| matches!(ch, '<' | '>' | '^' | '='))
    {
        parsed.align = Some(chars[0]);
        index = 1;
    }
    if let Some(ch) = chars.get(index) {
        if matches!(ch, '+' | '-' | ' ') {
            parsed.sign = Some(*ch);
            index += 1;
        }
    }
    while let Some(ch) = chars.get(index) {
        match ch {
            '0' => {
                parsed.zero = true;
                index += 1;
            }
            '#' | 'z' => index += 1,
            _ => break,
        }
    }
    while chars.get(index).is_some_and(char::is_ascii_digit) {
        parsed.width =
            parsed.width * 10 + chars[index].to_digit(10).unwrap_or(0) as usize;
        index += 1;
    }
    if chars.get(index) == Some(&',') {
        parsed.comma = true;
        index += 1;
    }
    if chars.get(index) == Some(&'.') {
        index += 1;
        let mut precision = 0;
        while chars.get(index).is_some_and(char::is_ascii_digit) {
            precision = precision * 10
                + chars[index].to_digit(10).unwrap_or(0) as usize;
            index += 1;
        }
        parsed.precision = Some(precision);
    }
    if let Some(ch) = chars.get(index) {
        if matches!(ch, 'd' | 'i' | 'f' | 'F' | 's' | 'e' | 'E' | 'g' | 'G') {
            parsed.ty = Some(*ch);
            index += 1;
        }
    }
    if index < chars.len() {
        return FormatSpec::default();
    }
    parsed
}

fn format_with_spec(value: &Value, spec: &str) -> String {
    if spec.is_empty() {
        return value.display();
    }
    let format = parse_format_spec(spec);
    if format.width == 0
        && format.precision.is_none()
        && format.ty.is_none()
        && format.sign.is_none()
    {
        return value.display();
    }

    let is_numeric =
        matches!(value, Value::Int(_) | Value::Float(_) | Value::Bool(_));
    let mut text = match format.ty {
        Some('d' | 'i') => int_text(value),
        Some('f' | 'F') => {
            let precision = format.precision.unwrap_or(6);
            format!("{:.*}", precision, value.as_f64().unwrap_or_default())
        }
        Some('e' | 'E') => {
            let precision = format.precision.unwrap_or(6);
            format!("{:.*e}", precision, value.as_f64().unwrap_or_default())
        }
        Some('g' | 'G') => {
            let precision = format.precision.unwrap_or(6);
            format!("{:.*}", precision, value.as_f64().unwrap_or_default())
        }
        Some('s') => value.display(),
        _ => match (value, format.precision) {
            (Value::Float(value), Some(precision)) => {
                format!("{:.*}", precision, value)
            }
            (Value::Text(text), Some(precision)) => {
                text.chars().take(precision).collect()
            }
            _ => value.display(),
        },
    };

    if format.comma && is_numeric {
        text = group_thousands(&text);
    }

    let negative = text.starts_with('-');
    let positive_sign = format.sign == Some('+') && !negative;
    let sign = if negative {
        text.remove(0);
        "-"
    } else if positive_sign {
        "+"
    } else if format.sign == Some(' ') {
        " "
    } else {
        ""
    };

    let mut fill = format.fill.unwrap_or(' ');
    let align = format.align.unwrap_or(if is_numeric { '>' } else { '<' });
    if format.zero && format.align.is_none() && is_numeric {
        fill = '0';
    }
    let padding = format.width.saturating_sub(sign.len() + text.len());
    if padding == 0 {
        return format!("{sign}{text}");
    }
    match align {
        '<' => format!("{sign}{text}{}", fill.to_string().repeat(padding)),
        '^' => {
            let left = padding / 2;
            let right = padding - left;
            format!(
                "{}{sign}{text}{}",
                fill.to_string().repeat(left),
                fill.to_string().repeat(right)
            )
        }
        '=' => format!("{sign}{}{text}", fill.to_string().repeat(padding)),
        _ => {
            if fill == '0' {
                format!("{sign}{}{text}", fill.to_string().repeat(padding))
            } else {
                format!("{}{sign}{text}", fill.to_string().repeat(padding))
            }
        }
    }
}

fn int_text(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Bool(value) => {
            if *value {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        Value::Float(value) if value.fract() == 0.0 => {
            format!("{}", *value as i64)
        }
        _ => value.display(),
    }
}

fn group_thousands(text: &str) -> String {
    let (integer, fraction) = match text.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (text, None),
    };
    let negative = integer.starts_with('-');
    let digits = integer.trim_start_matches('-');
    let mut grouped = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let mut result: String = grouped.chars().rev().collect();
    if negative {
        result.insert(0, '-');
    }
    if let Some(fraction) = fraction {
        result.push('.');
        result.push_str(fraction);
    }
    result
}

/// The variable names an expression reads, excluding string literals.
pub fn referenced_expression_names(expression: &str) -> BTreeSet<String> {
    names_in_expression(expression)
}
