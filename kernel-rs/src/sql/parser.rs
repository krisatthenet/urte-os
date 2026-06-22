//! SQL parser: tokens -> `Vec<Stmt>` (statements separated by `;`).

use crate::sql::ast::{CmpOp, Predicate, Projection, Stmt, Value};
use crate::sql::lexer::Tok;

#[derive(Debug, Clone, PartialEq)]
pub struct SqlParseError {
    pub msg: String,
}

struct P {
    toks: Vec<Tok>,
    pos: usize,
}

impl P {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos]
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, want: &Tok) -> Result<(), SqlParseError> {
        if self.peek() == want {
            self.next();
            Ok(())
        } else {
            Err(SqlParseError { msg: format!("expected {want:?}, found {:?}", self.peek()) })
        }
    }

    fn ident(&mut self) -> Result<String, SqlParseError> {
        match self.next() {
            Tok::Ident(s) => Ok(s),
            other => Err(SqlParseError { msg: format!("expected identifier, found {other:?}") }),
        }
    }

    fn value(&mut self) -> Result<Value, SqlParseError> {
        match self.next() {
            Tok::Num(n) => Ok(Value::Num(n)),
            Tok::Str(s) => Ok(Value::Str(s)),
            Tok::Ident(s) if s.eq_ignore_ascii_case("null") => Ok(Value::Null),
            other => Err(SqlParseError { msg: format!("expected value, found {other:?}") }),
        }
    }
}

pub fn parse(src: &str) -> Result<Vec<Stmt>, SqlParseError> {
    let toks = crate::sql::lexer::lex(src)
        .map_err(|e| SqlParseError { msg: format!("lex error at {}: {}", e.pos, e.msg) })?;
    let mut p = P { toks, pos: 0 };
    let mut stmts = Vec::new();

    while *p.peek() != Tok::Eof {
        let stmt = match p.peek() {
            Tok::Create => parse_create(&mut p)?,
            Tok::Insert => parse_insert(&mut p)?,
            Tok::Select => parse_select(&mut p)?,
            other => return Err(SqlParseError { msg: format!("unexpected statement start {other:?}") }),
        };
        stmts.push(stmt);
        // optional trailing ';'
        if *p.peek() == Tok::Semi {
            p.next();
        }
    }

    if stmts.is_empty() {
        return Err(SqlParseError { msg: "empty SQL request".into() });
    }
    Ok(stmts)
}

fn parse_create(p: &mut P) -> Result<Stmt, SqlParseError> {
    p.eat(&Tok::Create)?;
    p.eat(&Tok::Table)?;
    let table = p.ident()?;
    p.eat(&Tok::LParen)?;
    let mut columns = Vec::new();
    loop {
        columns.push(p.ident()?);
        match p.peek() {
            Tok::Comma => {
                p.next();
            }
            _ => break,
        }
    }
    p.eat(&Tok::RParen)?;
    Ok(Stmt::CreateTable { table, columns })
}

fn parse_insert(p: &mut P) -> Result<Stmt, SqlParseError> {
    p.eat(&Tok::Insert)?;
    p.eat(&Tok::Into)?;
    let table = p.ident()?;
    p.eat(&Tok::LParen)?;
    let mut columns = Vec::new();
    loop {
        columns.push(p.ident()?);
        match p.peek() {
            Tok::Comma => {
                p.next();
            }
            _ => break,
        }
    }
    p.eat(&Tok::RParen)?;
    p.eat(&Tok::Values)?;
    p.eat(&Tok::LParen)?;
    let mut values = Vec::new();
    loop {
        values.push(p.value()?);
        match p.peek() {
            Tok::Comma => {
                p.next();
            }
            _ => break,
        }
    }
    p.eat(&Tok::RParen)?;
    Ok(Stmt::Insert { table, columns, values })
}

fn parse_select(p: &mut P) -> Result<Stmt, SqlParseError> {
    p.eat(&Tok::Select)?;
    let projection = if *p.peek() == Tok::Star {
        p.next();
        Projection::All
    } else {
        let mut cols = Vec::new();
        loop {
            cols.push(p.ident()?);
            match p.peek() {
                Tok::Comma => {
                    p.next();
                }
                _ => break,
            }
        }
        Projection::Columns(cols)
    };
    p.eat(&Tok::From)?;
    let table = p.ident()?;

    let mut filters = Vec::new();
    if *p.peek() == Tok::Where {
        p.next();
        loop {
            filters.push(parse_predicate(p)?);
            match p.peek() {
                Tok::And => {
                    p.next();
                }
                _ => break,
            }
        }
    }
    Ok(Stmt::Select { table, projection, filters })
}

fn parse_predicate(p: &mut P) -> Result<Predicate, SqlParseError> {
    let column = p.ident()?;
    let op = match p.next() {
        Tok::Eq => CmpOp::Eq,
        Tok::Ne => CmpOp::Ne,
        Tok::Lt => CmpOp::Lt,
        Tok::Le => CmpOp::Le,
        Tok::Gt => CmpOp::Gt,
        Tok::Ge => CmpOp::Ge,
        other => return Err(SqlParseError { msg: format!("expected comparison operator, found {other:?}") }),
    };
    let value = p.value()?;
    Ok(Predicate { column, op, value })
}
