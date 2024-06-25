mod codegen;
mod errors;
mod logic;
mod parser;
mod tokenizer;
mod unification;
mod vm;

use std::cmp::max;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, Write};

fn display_error(filename: &str, src: &str, err_type: &str, err_msg: &str, err_offset: usize) {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut line = 0;
    let mut col = 0;
    let mut count = 0;
    for ch in src.chars() {
        col += 1;
        count += 1;
        if count == err_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        }
    }
    let width = line.to_string().len() + 2;
    println!("{}: {}", err_type, err_msg);
    println!("{s:>width$}|", s = " ", width = width);
    println!(" {} | {}", line, lines[line - 1]);
    print!("{s:>width$}|", s = " ", width = width);
    println!("{s:>width$}^", s = " ", width = col);
    println!("--> {}:{}", filename, line);
}

fn eval(filename: &str, src: &str, ctx: &mut codegen::Context, vm: &mut vm::VirtualMachine) {
    match tokenizer::scan(src) {
        Ok(tokens) => match parser::parse(tokens) {
            Ok(ast) => match codegen::generate(&ast, ctx, vm) {
                Ok(()) => match vm.run() {
                    Ok(()) => match vm.stack.pop() {
                        Some(vm::Value::Table(substs)) => {
                            if substs.is_empty() {
                                println!("Ok.");
                            } else {
                                for subst in substs {
                                    match subst.0 {
                                        unification::Term::Atom(a)
                                        | unification::Term::Variable(a) => {
                                            if let Some(interned) = vm.lookup_interned(&a) {
                                                print!("{}: ", interned);
                                            } else {
                                                print!("{}: ", a);
                                            }
                                        }
                                        _ => {
                                            println!("{:?}", subst.1);
                                        }
                                    }
                                    match subst.1 {
                                        unification::Term::Atom(a)
                                        | unification::Term::Variable(a) => {
                                            if let Some(interned) = vm.lookup_interned(&a) {
                                                println!("{}", interned);
                                            } else {
                                                println!("{}", a);
                                            }
                                        }
                                        _ => {
                                            println!("{:?}", subst.1);
                                        }
                                    }
                                }
                            }
                        }
                        Some(vm::Value::None) => {
                            println!("No.");
                        }
                        Some(value) => {
                            println!("InternalError: Unexpected value on stack: {}.", value);
                        }
                        None => {
                            println!("InternalError: Stack underflow.");
                        }
                    },
                    Err(err) => {
                        println!("RuntimeError: {}", err.msg);
                        println!("Instructions:");
                        let start_ip = max(0, err.ip as i64 - 10) as usize;
                        for ip in start_ip..err.ip + 1 {
                            println!("{:04}| {:?}", ip, vm.instructions[ip]);
                        }
                        if vm.stack.is_empty() {
                            println!("Empty stack.");
                        } else {
                            println!("Stack:");
                            for sp in 0..vm.stack.len() {
                                println!("{:04}| {}", sp, vm.stack[sp]);
                            }
                        }
                    }
                },
                Err(err) => {
                    display_error(filename, src, "SyntaxError", &err.msg, err.offset);
                }
            },
            Err(err) => {
                display_error(filename, src, "SyntaxError", &err.msg, err.offset);
            }
        },
        Err(err) => {
            display_error(filename, src, "TokenizerError", &err.msg, err.offset);
        }
    }
}

fn main() -> io::Result<()> {
    let mut ctx = codegen::Context::new();
    let mut vm = vm::VirtualMachine::new();
    let args: Vec<String> = env::args().collect();
    for i in 1..args.len() {
        let filename = &args[i];
        let mut file = File::open(filename)?;
        let mut program = String::new();
        file.read_to_string(&mut program)?;
        eval(&filename, &program, &mut ctx, &mut vm);
    }

    // Not running interactively.
    if args.len() > 1 {
        return Ok(());
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    println!("Welcome to Tern!");
    print!("> ");
    stdout.flush()?;

    for line in stdin.lock().lines() {
        match line {
            Ok(src) => {
                eval("<stdin>", &src, &mut ctx, &mut vm);
            }
            _ => break,
        }
        print!("> ");
        stdout.flush()?;
    }

    Ok(())
}
