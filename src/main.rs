//! Naive x86-64 code generator for expression in reverse polish form.
//! Takes an expression on the command-line and emit nasm assembly on stdout.
//!
//! As the goal is to play with code generation, the input language is minimal.
//! There is notably no lexical analyzer.  All tokens are one ASCII character long
//! and spaces between tokens are not allowed.
//!
//! Grammar:
//! program -> expr | program ';' expr
//! expr -> primary | expr expr binary_operator
//! primary -> number | variable
//! number -> '0' .. '9'
//! variable -> 'A' .. 'Z' | 'a' .. 'z'
//! binary_operator -> '+' | '*' | '='

use std::collections::HashSet;
use std::env;
use std::fmt;

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    if args.len() != 1 {
        panic!("usage: input_string");
    }
    compile(&args[0]);
}

/// Parses expression and calls code generator.
fn compile(input: &str) {
    let mut cg = CodeGen::new();
    cg.prologue();
    for ch in input.chars() {
        match ch {
            '0'..='9' => cg.number(ch.to_digit(10).unwrap()),
            'a'..='z' | 'A'..='Z' => cg.variable(ch),
            '+' => cg.add(),
            '-' => cg.sub(),
            '*' => cg.mul(),
            ';' => cg.end_of_expr(),
            '=' => cg.assign(),
            _ => panic!("unexpected input: {}", ch),
        }
    }
    cg.epilogue();
}

/// Naive code generator.
/// Exposes "semantic actions" called from the parser.
#[derive(Debug)]
struct CodeGen {
    // Keeps track of location of all terms of expression to generate code for.
    stack: Vec<Location>,
    symbols: HashSet<char>,
}

/// Operand location.
#[derive(Debug)]
enum Location {
    OnOperandStack(Operand),
    InAccumulator,
    OnCpuStack,
}

/// Operand flavors.
#[derive(Debug)]
enum Operand {
    Integer(u32),
    Variable(char),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Integer(n) => write!(f, "{}", n),
            Operand::Variable(v) => write!(f, "[rel {}]", v),
        }
    }
}

impl CodeGen {
    fn new() -> CodeGen {
        CodeGen {
            stack: vec![],
            symbols: HashSet::new(),
        }
    }

    fn prologue(&mut self) {
        println!("global _evaluate");
        println!("section .text");
        println!("_evaluate:");
    }

    fn epilogue(&mut self) {
        self.end_of_expr();
        println!("\tret");

        if self.symbols.len() > 0 {
            println!("section .data");
            for s in &self.symbols {
                println!("{}: dd 0", *s);
            }
        }
    }

    fn end_of_expr(&mut self) {
        match self.stack.pop() {
            Some(Location::OnOperandStack(o)) => println!("\tmov eax, {}", o),
            Some(Location::OnCpuStack) => panic!("unbalanced stack: {:?}", self.stack),
            Some(Location::InAccumulator) | None => (),
        }
        assert_eq!(self.stack.len(), 0);
    }

    fn number(&mut self, n: u32) {
        self.stack
            .push(Location::OnOperandStack(Operand::Integer(n)))
    }

    fn variable(&mut self, v: char) {
        self.symbols.insert(v);
        self.stack
            .push(Location::OnOperandStack(Operand::Variable(v)))
    }

    fn add(&mut self) {
        self.rvalue_binop(|n| println!("\tadd eax, {}", n));
    }

    fn sub(&mut self) {
        self.rvalue_binop(|n| println!("\tsub eax, {}", n));
    }

    fn mul(&mut self) {
        self.rvalue_binop(|n| {
            println!("\tmov ebx, {}", n);
            println!("\tmul ebx");
        });
    }

    fn assign(&mut self) {
        match self.prepare_binop() {
            (Location::OnOperandStack(Operand::Variable(v)), Location::OnOperandStack(r)) => {
                println!("\tmov eax, {}", r);
                println!("\tmov dword [rel {}], eax", v);
                self.stack.push(Location::InAccumulator);
            }
            (Location::OnOperandStack(Operand::Variable(v)), Location::InAccumulator) => {
                println!("\tmov dword [rel {}], eax", v);
                self.stack.push(Location::InAccumulator);
            }
            (lhs, rhs) => panic!("unexpected stack: {:?} {:?} {:?}", self.stack, lhs, rhs),
        }
    }

    /// Emits code for binary operation with rvalue operands.
    fn rvalue_binop<F: FnOnce(&str)>(&mut self, emit_binop: F) {
        let (lhs, rhs) = self.prepare_binop();
        match (lhs, rhs) {
            (Location::OnOperandStack(l), Location::OnOperandStack(r)) => {
                println!("\tmov eax, {}", l);
                emit_binop(&r.to_string());
                self.stack.push(Location::InAccumulator);
            }
            (Location::OnOperandStack(l), Location::InAccumulator) => {
                println!("\tmov ebx, eax");
                println!("\tmov eax, {}", l);
                emit_binop("ebx");
                self.stack.push(Location::InAccumulator);
            }
            (Location::InAccumulator, Location::OnOperandStack(r)) => {
                emit_binop(&r.to_string());
                self.stack.push(Location::InAccumulator);
            }
            (Location::OnCpuStack, Location::InAccumulator) => {
                println!("\tpop rbx");
                emit_binop("ebx");
                self.stack.push(Location::InAccumulator);
            }
            (lhs, rhs) => panic!("unexpected stack: {:?} {:?} {:?}", self.stack, lhs, rhs),
        }
    }

    /// Pops operands for binary operation and spill if needed.
    fn prepare_binop(&mut self) -> (Location, Location) {
        // Get location of operands.
        debug_assert!(self.stack.len() >= 2);
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.pop().unwrap();

        // Spill partial result for lower-precedence operation.
        let len = self.stack.len();
        for (i, ol) in self.stack.iter_mut().enumerate() {
            match ol {
                Location::OnOperandStack(Operand::Integer(_)) => {}
                Location::OnOperandStack(Operand::Variable(_)) => {}
                Location::OnCpuStack => (),
                Location::InAccumulator => {
                    if i != len - 1 {
                        panic!("unexpected stack: {:?}", self.stack);
                    }
                    println!("\tpush rax");
                    *ol = Location::OnCpuStack;
                }
            }
        }

        (lhs, rhs)
    }
}
