use crate::errors::SyntaxError;
use crate::parser::AST;
use crate::vm::{Opcode, VirtualMachine};
use std::collections::HashMap;

pub struct Context {
    pub bindings: Vec<HashMap<String, u64>>,
}

impl Context {
    pub fn new() -> Context {
        Context {
            bindings: vec![HashMap::new()],
        }
    }

    pub fn lookup(&self, s: &str) -> Option<u64> {
        for binding in self.bindings.iter().rev() {
            if let Some(id) = binding.get(s) {
                return Some(*id);
            }
        }
        None
    }

    pub fn push(&mut self) {
        self.bindings.push(HashMap::new())
    }

    pub fn pop(&mut self) {
        self.bindings.pop();
        assert!(
            !self.bindings.is_empty(),
            "Internal error: empty context while doing codegen"
        );
    }

    pub fn insert(&mut self, id: u64, value: &str) {
        self.bindings
            .last_mut()
            .unwrap()
            .insert(value.to_string(), id);
    }
}

pub fn generate(ast: &AST, ctx: &mut Context, vm: &mut VirtualMachine) -> Result<(), SyntaxError> {
    match ast {
        AST::Conj(nodes) => {
            let mut first = true;
            for node in nodes.iter() {
                generate(node, ctx, vm)?;
                if !first {
                    vm.instructions.push(Opcode::Conj2);
                } else {
                    first = false;
                }
            }
        }
        AST::Disj(nodes) => {
            let mut first = true;
            for node in nodes.iter() {
                generate(node, ctx, vm)?;
                if !first {
                    vm.instructions.push(Opcode::Disj2);
                } else {
                    first = false;
                }
            }
        }
        AST::Equals(left, right) => {
            generate(left, ctx, vm)?;
            generate(right, ctx, vm)?;
            vm.instructions.push(Opcode::Unify);
        }
        AST::Var(declarations, body) => {
            ctx.push();
            for declaration in declarations {
                if let AST::Variable(v, _) = declaration {
                    let id = vm.intern(v);
                    ctx.insert(id, v);
                } else {
                    unreachable!()
                }
            }
            generate(body, ctx, vm)?;
            ctx.pop();
        }
        AST::Atom(s) => {
            if let Some(id) = ctx.lookup(s) {
                vm.instructions.push(Opcode::Atom(id));
            } else {
                let id = vm.intern(s);
                ctx.insert(id, s);
                vm.instructions.push(Opcode::Atom(id));
            }
        }
        AST::Variable(v, offset) => {
            if let Some(id) = ctx.lookup(v) {
                vm.instructions.push(Opcode::Variable(id));
            } else {
                return Err(SyntaxError {
                    msg: "Expected { after conj.".to_string(),
                    offset: *offset,
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{codegen, parser, tokenizer, unification::Term, vm};
    use std::collections::HashMap;
    use std::rc::Rc;

    macro_rules! generate {
        ($input:expr, $ctx: expr, $vm:expr) => {{
            match tokenizer::scan($input) {
                Ok(tokens) => match parser::parse(tokens) {
                    Ok(ast) => {
                        codegen::generate(&ast, $ctx, $vm);
                    }
                    Err(err) => assert_eq!("parse failed", err.msg),
                },
                _ => assert!(false),
            }
        }};
    }

    #[test]
    fn conj() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!("conj {'olive == 'olive, 'oil == 'oil }", &mut ctx, &mut vm);
        vm.instructions.push(vm::Opcode::Solve);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        for v in &vm.stack {
            println!("{}", v);
        }
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn disj() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!(
            "disj {'olive == 'olive| 'olive == 'oil }",
            &mut ctx,
            &mut vm
        );
        vm.instructions.push(vm::Opcode::Solve);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn unify() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!("'olive == 'olive", &mut ctx, &mut vm);
        vm.instructions.push(vm::Opcode::Solve);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn var() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!("var (q) { q == 'olive }", &mut ctx, &mut vm);
        vm.instructions.push(vm::Opcode::Solve);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.len(), 1);
            assert_eq!(substs.get(&1).unwrap(), &Term::Atom(2));
            assert_eq!(vm.lookup_interned(&1).unwrap(), "q");
            assert_eq!(vm.lookup_interned(&2).unwrap(), "olive");
        } else {
            assert!(false);
        }
    }
}
