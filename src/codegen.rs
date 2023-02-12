use crate::parser::AST;
use crate::vm::{Opcode, VirtualMachine};

pub fn generate(ast: &AST, vm: &mut VirtualMachine) {
    match ast {
        AST::Conj(nodes) => todo!(),
        AST::Disj(nodes) => todo!(),
        AST::Equals(left, right) => {
            generate(left, vm);
            generate(right, vm);
            vm.instructions.push(Opcode::Unify);
        }
        AST::Atom(s) => {
            let id = vm.intern(s);
            vm.instructions.push(Opcode::Atom(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{codegen, parser, tokenizer, vm};
    use std::collections::HashMap;
    use std::rc::Rc;

    macro_rules! generate {
        ($input:expr, $vm:expr) => {{
            match tokenizer::scan($input) {
                Ok(tokens) => match parser::parse(tokens) {
                    Ok(ast) => {
                        codegen::generate(&ast, $vm);
                    }
                    Err(err) => assert_eq!("parse failed", err.msg),
                },
                _ => assert!(false),
            }
        }};
    }

    #[test]
    fn unify() {
        let mut vm = vm::VirtualMachine::new();
        generate!("'olive == 'olive", &mut vm);
        vm.instructions.push(vm::Opcode::Eval);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }
    }
}
