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
                if let AST::Variable(v) = declaration {
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
        AST::Variable(v) => {
            if let Some(id) = ctx.lookup(v) {
                vm.instructions.push(Opcode::Variable(id));
            } else {
                let id = vm.intern(v);
                ctx.insert(id, v);
                vm.instructions.push(Opcode::Variable(id));
            }
        }
        AST::FnCall(name, args, offset) => {
            for arg in args {
                generate(arg, ctx, vm)?;
            }
            // So far, we just have two builtin functions...
            if name == "solve" {
                vm.instructions.push(Opcode::Solve);
            } else if name == "next" {
                vm.instructions.push(Opcode::Next);
            } else {
                let msg = "Undefined function: ".to_string() + name;
                return Err(SyntaxError {
                    msg,
                    offset: *offset,
                });
            }
        }
        AST::Program(statements) => {
            for statement in statements {
                generate(statement, ctx, vm)?;
            }
        }
        AST::Table(fields) => {
            vm.instructions.push(Opcode::NewTable);
            let mut gen_set_table = false;
            for field in fields {
                generate(field, ctx, vm)?;
                if gen_set_table {
                    vm.instructions.push(Opcode::SetTable);
                }
                gen_set_table = !gen_set_table;
            }
        }
        AST::LetBinding(name, value) => {
            if let Some(id) = ctx.lookup(name) {
                vm.instructions.push(Opcode::Variable(id));
            } else {
                let id = vm.intern(name);
                ctx.insert(id, name);
                vm.instructions.push(Opcode::Variable(id));
            }
            generate(value, ctx, vm)?;
            vm.instructions.push(Opcode::SetEnv);
        }
        AST::BindingRef(name) => {
            if let Some(id) = ctx.lookup(name) {
                vm.instructions.push(Opcode::Variable(id));
            } else {
                let id = vm.intern(name);
                ctx.insert(id, name);
                vm.instructions.push(Opcode::Variable(id));
            }
            vm.instructions.push(Opcode::GetEnv);
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
                    Ok(ast) => match codegen::generate(&ast, $ctx, $vm) {
                        Ok(()) => {}
                        Err(err) => assert_eq!("code generation failed", err.msg),
                    },
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
        vm.instructions.push(vm::Opcode::NewTable);
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
        vm.instructions.push(vm::Opcode::NewTable);
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
        vm.instructions.push(vm::Opcode::NewTable);
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
        vm.instructions.push(vm::Opcode::NewTable);
        vm.instructions.push(vm::Opcode::Solve);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.len(), 1);
            assert_eq!(substs.get(&Term::Variable(1)).unwrap(), &Term::Atom(2));
            assert_eq!(vm.lookup_interned(&1).unwrap(), "q");
            assert_eq!(vm.lookup_interned(&2).unwrap(), "olive");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn fncall() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!(
            "next(solve(var (q) { q == 'olive }, {}))",
            &mut ctx,
            &mut vm
        );
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.len(), 1);
            assert_eq!(substs.get(&Term::Variable(1)).unwrap(), &Term::Atom(2));
            assert_eq!(vm.lookup_interned(&1).unwrap(), "q");
            assert_eq!(vm.lookup_interned(&2).unwrap(), "olive");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn program() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!(
            "next(solve(var (q) { q == 'olive }, {}))\nnext(solve(var (q) { q == 'oil }, {}))",
            &mut ctx,
            &mut vm
        );
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.len(), 1);
            assert_eq!(substs.get(&Term::Variable(3)).unwrap(), &Term::Atom(4));
            assert_eq!(vm.lookup_interned(&3).unwrap(), "q");
            assert_eq!(vm.lookup_interned(&4).unwrap(), "oil");
        } else {
            assert!(false);
        }
        vm.stack.pop();
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.len(), 1);
            assert_eq!(substs.get(&Term::Variable(1)).unwrap(), &Term::Atom(2));
            assert_eq!(vm.lookup_interned(&1).unwrap(), "q");
            assert_eq!(vm.lookup_interned(&2).unwrap(), "olive");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn table() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!("{x: 'olive, y: 'oil}", &mut ctx, &mut vm);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(table)) = vm.stack.last() {
            assert_eq!(table.len(), 2);
            assert_eq!(table.get(&Term::Variable(1)).unwrap(), &Term::Atom(2));
            assert_eq!(table.get(&Term::Variable(3)).unwrap(), &Term::Atom(4));
            assert_eq!(vm.lookup_interned(&1).unwrap(), "x");
            assert_eq!(vm.lookup_interned(&2).unwrap(), "olive");
            assert_eq!(vm.lookup_interned(&3).unwrap(), "y");
            assert_eq!(vm.lookup_interned(&4).unwrap(), "oil");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn letbindings() {
        let mut ctx = codegen::Context::new();
        let mut vm = vm::VirtualMachine::new();
        generate!(
            "let x = {x: 'olive, y: 'oil}\nlet y = 'banana == 'apple\nlet z = solve('banana == 'banana, {}) x y",
            &mut ctx,
            &mut vm
        );
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Goal(_)) = vm.stack.pop() {
            // Ok.
        } else {
            assert!(false);
        }
        if let Some(vm::Value::Table(table)) = vm.stack.pop() {
            assert_eq!(table.len(), 2);
            assert_eq!(table.get(&Term::Variable(1)).unwrap(), &Term::Atom(2));
            assert_eq!(table.get(&Term::Variable(3)).unwrap(), &Term::Atom(4));
            assert_eq!(vm.lookup_interned(&1).unwrap(), "x");
            assert_eq!(vm.lookup_interned(&2).unwrap(), "olive");
            assert_eq!(vm.lookup_interned(&3).unwrap(), "y");
            assert_eq!(vm.lookup_interned(&4).unwrap(), "oil");
        } else {
            assert!(false);
        }
        // TODO: Add test for retrieving stream from let binding when we support it.
    }
}
