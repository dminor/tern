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
mod tests {}
