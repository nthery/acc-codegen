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
//! primary -> digit
//! binary_operator -> '+' | '*'

use std::env;

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
            '+' => cg.add(),
            '*' => cg.mul(),
            ';' => cg.end_of_expr(),
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
}

/// Operand location.
#[derive(Debug)]
enum Location {
    OnOperandStack(u32),
    InAccumulator,
    OnCpuStack,
}

impl CodeGen {
    fn new() -> CodeGen {
        CodeGen { stack: vec![] }
    }

    fn prologue(&mut self) {
        println!("global _evaluate");
        println!("section .text");
        println!("_evaluate:");
    }

    fn epilogue(&mut self) {
        self.end_of_expr();
        println!("\tret");
    }

    fn end_of_expr(&mut self) {
        match self.stack.pop() {
            Some(Location::OnOperandStack(n)) => println!("\tmov eax, {}", n),
            Some(Location::OnCpuStack) => panic!("unbalanced stack: {:?}", self.stack),
            Some(Location::InAccumulator) | None => (),
        }
        assert_eq!(self.stack.len(), 0);
    }

    fn number(&mut self, n: u32) {
        self.stack.push(Location::OnOperandStack(n))
    }

    fn add(&mut self) {
        self.binop(|n| println!("\tadd eax, {}", n));
    }

    fn mul(&mut self) {
        self.binop(|n| {
            println!("\tmov ebx, {}", n);
            println!("\tmul ebx");
        });
    }

    fn binop<F: FnOnce(&str)>(&mut self, emit_binop: F) {
        // Get location of operands.
        let len = self.stack.len();
        debug_assert!(len >= 2);
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.pop().unwrap();

        // Spill pending operands for lower-precedence operators.
        for (i, o) in self.stack.iter_mut().enumerate() {
            match o {
                Location::OnOperandStack(n) => {
                    println!("\tmov rax, {}", n);
                    println!("\tpush rax");
                    *o = Location::OnCpuStack;
                }
                Location::OnCpuStack => (),
                Location::InAccumulator => {
                    debug_assert_eq!(i, len - 1);
                    println!("\tpush rax");
                    *o = Location::OnCpuStack;
                }
            }
        }

        // Emit operation
        match (lhs, rhs) {
            (Location::OnOperandStack(l), Location::OnOperandStack(r)) => {
                println!("\tmov eax, {}", l);
                emit_binop(&r.to_string());
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
}
