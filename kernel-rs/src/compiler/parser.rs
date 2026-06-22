//! Recursive-descent parser: tokens -> AST.

use crate::compiler::ast::{Cmp, Pipeline, Program, Stmt};
use crate::compiler::lexer::{Tok, Token};
use crate::guardrail::InterventionKind;
use crate::types::{ScaleLevel, Stage};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub msg: String,
}

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Parser { toks, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn advance(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError { line: self.peek().line, msg: msg.into() })
    }

    fn expect(&mut self, want: &Tok) -> Result<(), ParseError> {
        if &self.peek().tok == want {
            self.advance();
            Ok(())
        } else {
            self.err(format!("expected {want:?}, found {:?}", self.peek().tok))
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut pipelines = Vec::new();
        while self.peek().tok != Tok::Eof {
            pipelines.push(self.parse_pipeline()?);
        }
        if pipelines.is_empty() {
            return self.err("empty program: expected at least one `pipeline`");
        }
        Ok(Program { pipelines })
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline, ParseError> {
        self.expect(&Tok::Pipeline)?;
        let name = match self.advance().tok {
            Tok::Str(s) => s,
            other => return self.err(format!("expected pipeline name string, found {other:?}")),
        };
        self.expect(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek().tok != Tok::RBrace {
            if self.peek().tok == Tok::Eof {
                return self.err("unexpected EOF inside pipeline body");
            }
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Tok::RBrace)?;
        Ok(Pipeline { name, stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        let stmt = match self.peek().tok.clone() {
            Tok::Scale => {
                self.advance();
                let id = self.ident()?;
                let lvl = ScaleLevel::from_ident(&id)
                    .ok_or_else(|| ParseError { line: self.peek().line, msg: format!("unknown scale '{id}'") })?;
                Stmt::Scale(lvl)
            }
            Tok::Stage => {
                self.advance();
                let id = self.ident()?;
                let st = Stage::from_ident(&id)
                    .ok_or_else(|| ParseError { line: self.peek().line, msg: format!("unknown stage '{id}'") })?;
                Stmt::Stage(st)
            }
            Tok::Guard => self.parse_guard()?,
            Tok::Emit => {
                self.advance();
                match self.advance().tok {
                    Tok::Str(s) => Stmt::Emit(s),
                    other => return self.err(format!("expected string after `emit`, found {other:?}")),
                }
            }
            other => return self.err(format!("unexpected statement start {other:?}")),
        };
        self.expect(&Tok::Semi)?;
        Ok(stmt)
    }

    fn parse_guard(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Tok::Guard)?;
        let kind_id = self.ident()?;
        let kind = match kind_id.as_str() {
            "observe" => InterventionKind::Observe,
            "simulate" => InterventionKind::Simulate,
            "actuate" => InterventionKind::Actuate,
            "release" => InterventionKind::Release,
            _ => return self.err(format!("unknown intervention kind '{kind_id}'")),
        };
        self.expect(&Tok::If)?;
        let field = self.ident()?;
        let cmp = match self.advance().tok {
            Tok::Gt => Cmp::Gt,
            Tok::Ge => Cmp::Ge,
            Tok::Lt => Cmp::Lt,
            Tok::Le => Cmp::Le,
            Tok::EqEq => Cmp::Eq,
            other => return self.err(format!("expected comparison operator, found {other:?}")),
        };
        let value = match self.advance().tok {
            Tok::Num(n) => n,
            other => return self.err(format!("expected number in guard, found {other:?}")),
        };
        Ok(Stmt::Guard { kind, field, cmp, value })
    }

    fn ident(&mut self) -> Result<String, ParseError> {
        match self.advance().tok {
            Tok::Ident(s) => Ok(s),
            other => self.err(format!("expected identifier, found {other:?}")),
        }
    }
}
