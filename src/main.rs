mod codegen;
mod errors;
mod logic;
mod parser;
mod tokenizer;
mod unification;
mod vm;

use std::cmp::{max, min};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, Write};
use std::rc::Rc;

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
    if line > 0 {
        println!(" {} | {}", line, lines[line - 1]);
    }
    print!("{s:>width$}|", s = " ", width = width);
    println!("{s:>width$}^", s = " ", width = col);
    println!("--> {}:{}", filename, line);
}

fn eval(filename: &str, src: &str, ctx: &mut codegen::Context, vm: &mut vm::VirtualMachine) {
    vm.stack.clear();
    vm.callstack.clear();
    match tokenizer::scan(src) {
        Ok(tokens) => match parser::parse(tokens) {
            Ok(ast) => {
                let mut instr = Vec::new();
                match codegen::generate(&ast, ctx, vm, &mut instr) {
                    Ok(()) => match vm.run(Rc::new(instr)) {
                        Ok(()) => {
                            match vm.stack.pop() {
                                Some(vm::Value::Table(substs)) => {
                                    if substs.is_empty() {
                                        println!("Ok.");
                                    } else {
                                        for subst in substs {
                                            match subst.0 {
                                                unification::Term::Variable(a) => {
                                                    if let Some(name) = vm.lookup_variable(&a) {
                                                        print!("{}: ", name);
                                                    } else {
                                                        print!("{}: ", a);
                                                    }
                                                }
                                                _ => unreachable!(
                                                    "expected variable as substitution key"
                                                ),
                                            }
                                            match subst.1 {
                                            unification::Term::Atom(a) => {
                                                if let Some(interned) = vm.lookup_interned(&a) {
                                                    println!("{}", interned);
                                                } else {
                                                    println!("{}", a);
                                                }
                                            },
                                            unification::Term::Variable(a) => {
                                                if let Some(name) = vm.lookup_variable(&a) {
                                                    println!("{}", name);
                                                } else {
                                                    println!("{}", a);
                                                }
                                            }
                                            _ => unreachable!("expected atom or variable as substitution value")
                                        }
                                        }
                                    }
                                }
                                Some(vm::Value::None) => {
                                    println!("No.");
                                }
                                Some(value) => {
                                    println!("{}", value);
                                }
                                _ => {}
                            }
                        }
                        Err(err) => {
                            println!("RuntimeError: {}", err.msg);
                            if vm.callstack.is_empty() {
                                println!("Empty call stack.");
                            } else {
                                println!("Call stack:");
                                for callable in vm.callstack.iter().rev() {
                                    if let vm::Value::Callable {
                                        kind,
                                        parameters,
                                        instructions,
                                        ip: callable_ip,
                                    } = callable
                                    {
                                        let start_ip = max(0, *callable_ip as i64 - 10) as usize;
                                        let end_ip = min(instructions.len(), *callable_ip + 10);
                                        for ip in start_ip..end_ip {
                                            if ip == *callable_ip {
                                                println!("->  {:04}| {:?}", ip, instructions[ip]);
                                            } else {
                                                println!("    {:04}| {:?}", ip, instructions[ip]);
                                            }
                                        }
                                    }
                                }
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
                }
            }
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
    let mut run_interactive = args.len() == 1;
    for i in 1..args.len() {
        if args[i] == "--interactive" {
            run_interactive = true;
            continue;
        }
        let filename = &args[i];
        let mut file = File::open(filename)?;
        let mut program = String::new();
        file.read_to_string(&mut program)?;
        eval(&filename, &program, &mut ctx, &mut vm);
    }

    // Not running interactively.
    if !run_interactive {
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
