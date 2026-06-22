//! URTE Pipeline Language (UPL) compiler layer.
//!
//! Pipeline: source -> [lexer] -> tokens -> [parser] -> AST -> [codegen] -> IR.
//! The IR (`ir::Module`) is consumed by the `interpreter` layer.

pub mod ast;
pub mod ir;
pub mod lexer;
pub mod parser;

use ast::{Program, Stmt};
use ir::{Chunk, Module, Op};
use lexer::LexError;
use parser::{ParseError, Parser};

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    Lex(LexError),
    Parse(ParseError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Lex(e) => write!(f, "lex error (line {}): {}", e.line, e.msg),
            CompileError::Parse(e) => write!(f, "parse error (line {}): {}", e.line, e.msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile UPL source text into an executable IR module.
pub fn compile(src: &str) -> Result<Module, CompileError> {
    let toks = lexer::lex(src).map_err(CompileError::Lex)?;
    let program = Parser::new(toks).parse_program().map_err(CompileError::Parse)?;
    Ok(codegen(&program))
}

/// Lower the AST to IR. Each pipeline becomes one chunk terminated by `Halt`.
fn codegen(program: &Program) -> Module {
    let mut module = Module::default();
    for pl in &program.pipelines {
        let mut ops = Vec::with_capacity(pl.stmts.len() + 1);
        for stmt in &pl.stmts {
            match stmt {
                Stmt::Scale(lvl) => ops.push(Op::SetScale(*lvl)),
                Stmt::Stage(st) => ops.push(Op::RunStage(*st)),
                Stmt::Emit(s) => ops.push(Op::Emit(s.clone())),
                Stmt::Guard { kind, field, cmp, value } => ops.push(Op::Guard {
                    kind: *kind,
                    field: field.clone(),
                    cmp: *cmp,
                    threshold: *value,
                }),
            }
        }
        ops.push(Op::Halt);
        module.chunks.push(Chunk { name: pl.name.clone(), ops });
    }
    module
}
