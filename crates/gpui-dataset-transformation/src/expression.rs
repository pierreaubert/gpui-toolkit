//! Bounded expression compilation with borrowed fields and reusable evaluation storage.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, DataSet};

// Bound parser recursion, bytecode and source storage independently. These limits
// permit ordinary scientific formulas without input-controlled stack growth.
const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 128;
const MAX_INSTRUCTIONS: usize = 4096;

#[derive(Clone, Copy)]
enum Unary {
    Neg,
    Sin,
    Cos,
    Tan,
    Sqrt,
    Exp,
    Log,
    Abs,
}

#[derive(Clone, Copy)]
enum Binary {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Min,
    Max,
}

#[derive(Clone, Copy)]
enum Instruction<'a> {
    Number(f64),
    Coord(usize),
    Field {
        values: &'a [f64],
        valid: Option<&'a [bool]>,
    },
    Unary(Unary),
    Binary(Binary),
}

pub(crate) struct Expression<'a> {
    code: Vec<Instruction<'a>>,
    stack: Vec<f64>,
}

impl<'a> Expression<'a> {
    /// The dataset must be validated before binding field references.
    pub(crate) fn new(dataset: &'a DataSet, source: &str) -> Result<Self, TransformError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(TransformError::Degenerate(
                "calculator source exceeds 64 KiB".into(),
            ));
        }
        let mut parser = Parser {
            chars: source.chars().collect(),
            pos: 0,
            depth: 0,
            dataset,
            code: Vec::new(),
            stack: 0,
            max_stack: 0,
        };
        parser.expr()?;
        if parser.peek().is_some() {
            return Err(parser.error("trailing characters"));
        }
        debug_assert_eq!(parser.stack, 1);
        Ok(Self {
            code: parser.code,
            stack: Vec::with_capacity(parser.max_stack),
        })
    }

    pub(crate) fn evaluate(&mut self, index: usize, point: [f64; 3]) -> f64 {
        self.stack.clear();
        for instruction in &self.code {
            match *instruction {
                Instruction::Number(value) => self.stack.push(value),
                Instruction::Coord(axis) => self.stack.push(point[axis]),
                Instruction::Field { values, valid } => {
                    self.stack.push(if valid.is_none_or(|mask| mask[index]) {
                        values[index]
                    } else {
                        f64::NAN
                    });
                }
                Instruction::Unary(op) => {
                    let value = self.stack.last_mut().expect("compiled unary operand");
                    let result = if value.is_finite() {
                        match op {
                            Unary::Neg => -*value,
                            Unary::Sin => value.sin(),
                            Unary::Cos => value.cos(),
                            Unary::Tan => value.tan(),
                            Unary::Sqrt => value.sqrt(),
                            Unary::Exp => value.exp(),
                            Unary::Log => value.ln(),
                            Unary::Abs => value.abs(),
                        }
                    } else {
                        f64::NAN
                    };
                    *value = if result.is_finite() { result } else { f64::NAN };
                }
                Instruction::Binary(op) => {
                    let b = self.stack.pop().expect("compiled right operand");
                    let a = self.stack.last_mut().expect("compiled left operand");
                    let result = if a.is_finite() && b.is_finite() {
                        match op {
                            Binary::Add => *a + b,
                            Binary::Sub => *a - b,
                            Binary::Mul => *a * b,
                            Binary::Div => *a / b,
                            Binary::Pow => a.powf(b),
                            Binary::Min => a.min(b),
                            Binary::Max => a.max(b),
                        }
                    } else {
                        f64::NAN
                    };
                    *a = if result.is_finite() { result } else { f64::NAN };
                }
            }
        }
        self.stack[0]
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    depth: usize,
    dataset: &'a DataSet,
    code: Vec<Instruction<'a>>,
    stack: usize,
    max_stack: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, message: &str) -> TransformError {
        TransformError::Degenerate(format!(
            "calculator parse error at character {}: {message}",
            self.pos
        ))
    }

    fn peek(&mut self) -> Option<char> {
        while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
            self.pos += 1;
        }
        self.chars.get(self.pos).copied()
    }

    fn emit(
        &mut self,
        instruction: Instruction<'a>,
        consumed: usize,
    ) -> Result<(), TransformError> {
        if self.code.len() >= MAX_INSTRUCTIONS {
            return Err(self.error("more than 4096 instructions"));
        }
        debug_assert!(self.stack >= consumed);
        self.stack = self.stack - consumed + 1;
        self.max_stack = self.max_stack.max(self.stack);
        self.code.push(instruction);
        Ok(())
    }

    fn expr(&mut self) -> Result<(), TransformError> {
        self.term()?;
        loop {
            let op = match self.peek() {
                Some('+') => Binary::Add,
                Some('-') => Binary::Sub,
                _ => return Ok(()),
            };
            self.pos += 1;
            self.term()?;
            self.emit(Instruction::Binary(op), 2)?;
        }
    }

    fn term(&mut self) -> Result<(), TransformError> {
        self.unary()?;
        loop {
            let op = match self.peek() {
                Some('*') => Binary::Mul,
                Some('/') => Binary::Div,
                _ => return Ok(()),
            };
            self.pos += 1;
            self.unary()?;
            self.emit(Instruction::Binary(op), 2)?;
        }
    }

    fn unary(&mut self) -> Result<(), TransformError> {
        if self.depth >= MAX_DEPTH {
            return Err(self.error("nesting exceeds 128 levels"));
        }
        self.depth += 1;
        let result = (|| match self.peek() {
            Some('-') => {
                self.pos += 1;
                self.unary()?;
                self.emit(Instruction::Unary(Unary::Neg), 1)
            }
            Some('+') => {
                self.pos += 1;
                self.unary()
            }
            _ => self.power(),
        })();
        self.depth -= 1;
        result
    }

    fn power(&mut self) -> Result<(), TransformError> {
        self.primary()?;
        if self.peek() == Some('^') {
            self.pos += 1;
            self.unary()?;
            self.emit(Instruction::Binary(Binary::Pow), 2)?;
        }
        Ok(())
    }

    fn primary(&mut self) -> Result<(), TransformError> {
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                self.expr()?;
                if self.peek() != Some(')') {
                    return Err(self.error("expected ')'"));
                }
                self.pos += 1;
                Ok(())
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_alphabetic() || c == '_' => self.name(),
            _ => Err(self.error("unexpected character")),
        }
    }

    fn number(&mut self) -> Result<(), TransformError> {
        let start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_digit() || *c == '.')
        {
            self.pos += 1;
        }
        if self
            .chars
            .get(self.pos)
            .is_some_and(|c| matches!(c, 'e' | 'E'))
        {
            self.pos += 1;
            if self
                .chars
                .get(self.pos)
                .is_some_and(|c| matches!(c, '+' | '-'))
            {
                self.pos += 1;
            }
            while self.chars.get(self.pos).is_some_and(char::is_ascii_digit) {
                self.pos += 1;
            }
        }
        let number = self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse::<f64>()
            .map_err(|_| self.error("bad number"))?;
        if !number.is_finite() {
            return Err(self.error("number must be finite"));
        }
        self.emit(Instruction::Number(number), 0)
    }

    fn name(&mut self) -> Result<(), TransformError> {
        let start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            self.pos += 1;
        }
        let name: String = self.chars[start..self.pos].iter().collect();
        if self.peek() == Some('(') {
            return self.call(&name);
        }
        let instruction = match name.as_str() {
            "x" => Instruction::Coord(0),
            "y" => Instruction::Coord(1),
            "z" => Instruction::Coord(2),
            _ => {
                let field = self
                    .dataset
                    .field(Association::Point, &name)
                    .ok_or_else(|| TransformError::FieldNotFound(name.clone()))?;
                let values = field
                    .as_scalar()
                    .ok_or_else(|| TransformError::WrongFieldShape {
                        field: name.clone(),
                        reason: "calculator requires a scalar field".into(),
                    })?;
                Instruction::Field {
                    values,
                    valid: field.valid.as_deref(),
                }
            }
        };
        self.emit(instruction, 0)
    }

    fn call(&mut self, name: &str) -> Result<(), TransformError> {
        let (instruction, arity) = match name {
            "sin" => (Instruction::Unary(Unary::Sin), 1),
            "cos" => (Instruction::Unary(Unary::Cos), 1),
            "tan" => (Instruction::Unary(Unary::Tan), 1),
            "sqrt" => (Instruction::Unary(Unary::Sqrt), 1),
            "exp" => (Instruction::Unary(Unary::Exp), 1),
            "log" => (Instruction::Unary(Unary::Log), 1),
            "abs" => (Instruction::Unary(Unary::Abs), 1),
            "min" => (Instruction::Binary(Binary::Min), 2),
            "max" => (Instruction::Binary(Binary::Max), 2),
            _ => return Err(self.error("unknown function")),
        };
        self.pos += 1;
        let mut arguments = 0;
        if self.peek() != Some(')') {
            loop {
                self.expr()?;
                arguments += 1;
                if self.peek() != Some(',') {
                    break;
                }
                self.pos += 1;
            }
        }
        if self.peek() != Some(')') {
            return Err(self.error("expected ')' after arguments"));
        }
        self.pos += 1;
        if arguments != arity {
            return Err(self.error("wrong function arity"));
        }
        self.emit(instruction, arity)
    }
}
