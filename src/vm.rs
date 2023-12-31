use crate::errors::RuntimeError;
use crate::logic;
use crate::unification;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

pub type AtomType = u64;

#[derive(Debug, Clone)]
pub enum Opcode {
    // Push a new atom term to the stack.
    // -> Term
    Atom(AtomType),
    // Push a new variable term to the stack.
    // -> Term
    Variable(u64),
    // Pop two goals from the stack and construct a new Conj2 goal using them.
    // Goal Goal -> Conj2
    Conj2,
    // Pop two goals from the stack and construct a new Disj2 goal using them.
    // Goal Goal -> Disj2
    Disj2,
    // Pop two terms from the stack and attempt to unify them.
    // Term Term -> Unify
    Unify,
    // Evaluate the goal to produce a stream.
    // Goal -> Stream
    Eval,
    // Call next on the stream, pushing a table to the stack.
    // Stream -> Stream Table
    Next,
    // Pop the value from the top of the stack.
    // Value ->
    Pop,
    // Print the value from the top of the stack to stdout (for debugging).
    // Value -> Value
    Print,
}

pub enum Value {
    Term(unification::Term<AtomType>),
    Goal(Rc<dyn logic::Goal<AtomType>>),
    Stream(Box<dyn Iterator<Item = unification::Substitutions<AtomType>>>),
    Table(unification::Substitutions<AtomType>),
    None,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Term(term) => {
                write!(f, "<term {:?}>", term)
            }
            Value::Goal(_) => {
                write!(f, "<goal>")
            }
            Value::Stream(_) => write!(f, "<stream>"),
            Value::Table(substs) => {
                write!(f, "<substitions (")?;
                let mut first = true;
                for subst in substs {
                    if !first {
                        write!(f, ", {} = {}", subst.0, subst.0)?;
                    } else {
                        first = false;
                        write!(f, "{} = {:?}", subst.0, subst.1)?;
                    }
                }
                write!(f, ")>")
            }
            Value::None => write!(f, "<none>")
        }
    }
}

pub struct VirtualMachine {
    pub instructions: Vec<Opcode>,
    pub ip: usize,

    next_id: u64,
    pub interned: HashMap<u64, String>,

    pub stack: Vec<Value>,
}

macro_rules! buildgoal {
    ($vm:expr, $type:tt, $goal:tt) => {{
        match $vm.stack.pop() {
            Some(Value::$type(left)) => match $vm.stack.pop() {
                Some(Value::$type(right)) => {
                    $vm.stack
                        .push(Value::Goal(Rc::new(logic::$goal::new(left, right))));
                }
                None => {
                    return Err(RuntimeError {
                        msg: "Stack underflow.".to_string(),
                        ip: $vm.ip,
                        opcode: $vm.instructions[$vm.ip].clone(),
                    });
                }
                _ => {
                    return Err(RuntimeError {
                        msg: "Expected term.".to_string(),
                        ip: $vm.ip,
                        opcode: $vm.instructions[$vm.ip].clone(),
                    });
                }
            },
            None => {
                return Err(RuntimeError {
                    msg: "Stack underflow.".to_string(),
                    ip: $vm.ip,
                    opcode: $vm.instructions[$vm.ip].clone(),
                });
            }
            _ => {
                return Err(RuntimeError {
                    msg: "Expected term.".to_string(),
                    ip: $vm.ip,
                    opcode: $vm.instructions[$vm.ip].clone(),
                });
            }
        }
    }};
}

impl VirtualMachine {
    pub fn intern(&mut self, s: &String) -> u64 {
        self.next_id += 1;
        self.interned.insert(self.next_id, s.to_string());
        self.next_id
    }

    pub fn lookup_interned(&mut self, id: &u64) -> Option<&String> {
        self.interned.get(id)
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        while self.ip < self.instructions.len() {
            match &self.instructions[self.ip] {
                Opcode::Atom(atom) => self.stack.push(Value::Term(unification::Term::Atom(*atom))),
                Opcode::Variable(var) => self
                    .stack
                    .push(Value::Term(unification::Term::Variable(*var))),
                Opcode::Conj2 => buildgoal!(self, Goal, Conj2),
                Opcode::Disj2 => buildgoal!(self, Goal, Disj2),
                Opcode::Unify => buildgoal!(self, Term, Unify),
                Opcode::Eval => match self.stack.pop() {
                    Some(Value::Goal(goal)) => {
                        //TODO: eventually, we'll pop substs from the stack as well...
                        let substs = HashMap::new();
                        self.stack.push(Value::Stream(goal.eval(&substs)));
                    }
                    None => {
                        return Err(RuntimeError {
                            msg: "Stack underflow.".to_string(),
                            ip: self.ip,
                            opcode: self.instructions[self.ip].clone(),
                        });
                    }
                    _ => {
                        return Err(RuntimeError {
                            msg: "Expected goal.".to_string(),
                            ip: self.ip,
                            opcode: self.instructions[self.ip].clone(),
                        });
                    }
                },
                Opcode::Next => match self.stack.pop() {
                    Some(Value::Stream(mut stream)) => match stream.next() {
                        Some(substs) => self.stack.push(Value::Table(substs)),
                        None => self.stack.push(Value::None),
                    },
                    None => {
                        return Err(RuntimeError {
                            msg: "Stack underflow.".to_string(),
                            ip: self.ip,
                            opcode: self.instructions[self.ip].clone(),
                        });
                    }
                    _ => {
                        return Err(RuntimeError {
                            msg: "Unexpected value.".to_string(),
                            ip: self.ip,
                            opcode: self.instructions[self.ip].clone(),
                        });
                    }
                },
                Opcode::Pop => {
                    if self.stack.pop().is_none() {
                        return Err(RuntimeError {
                            msg: "Stack underflow.".to_string(),
                            ip: self.ip,
                            opcode: self.instructions[self.ip].clone(),
                        });
                    }
                }
                Opcode::Print => match self.stack.last() {
                    Some(value) => match value {
                        Value::Term(term) => {
                            println!("{:?}", term);
                        }
                        Value::Goal(goal) => println!("goal#{:?}", std::ptr::addr_of!(goal)),
                        Value::Stream(stream) => {
                            println!("stream#{:?}", std::ptr::addr_of!(stream))
                        }
                        Value::Table(table) => {
                            println!("{:?}", table);
                        }
                        Value::None => println!("None"),
                    },
                    None => {
                        return Err(RuntimeError {
                            msg: "Stack underflow.".to_string(),
                            ip: self.ip,
                            opcode: self.instructions[self.ip].clone(),
                        });
                    }
                },
            }
            self.ip += 1;
        }
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            ip: 0,
            next_id: 0,
            interned: HashMap::new(),
            stack: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{logic, unification, vm};
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn values() {
        let mut vm = vm::VirtualMachine::new();
        vm.stack.push(vm::Value::Term(unification::Term::Atom(1)));
        vm.stack
            .push(vm::Value::Term(unification::Term::Variable(1)));
        vm.stack.push(vm::Value::Goal(Rc::new(logic::Unify::new(
            unification::Term::Variable(1),
            unification::Term::Atom(1),
        ))));
        vm.stack.push(vm::Value::Goal(Rc::new(logic::Disj2::new(
            Rc::new(logic::Unify::new(
                unification::Term::Variable(1),
                unification::Term::Atom(1),
            )),
            Rc::new(logic::Unify::new(
                unification::Term::Variable(1),
                unification::Term::Atom(2),
            )),
        ))));
        vm.stack.push(vm::Value::Goal(Rc::new(logic::Conj2::new(
            Rc::new(logic::Disj2::new(
                Rc::new(logic::Unify::new(
                    unification::Term::Variable(1),
                    unification::Term::Atom(1),
                )),
                Rc::new(logic::Unify::new(
                    unification::Term::Variable(1),
                    unification::Term::Atom(2),
                )),
            )),
            Rc::new(logic::Unify::new(
                unification::Term::Variable(1),
                unification::Term::Atom(2),
            )),
        ))));
        let substs = HashMap::new();
        if let Some(vm::Value::Goal(goal)) = vm.stack.last() {
            vm.stack
                .push(vm::Value::Stream(Box::new(goal.eval(&substs))));
        }
        vm.stack.push(vm::Value::Table(substs));
        vm.stack.push(vm::Value::None);
        assert_eq!(vm.stack.len(), 8);
    }

    #[test]
    fn unify() {
        let mut vm = vm::VirtualMachine::new();
        vm.instructions.push(vm::Opcode::Atom(1));
        vm.instructions.push(vm::Opcode::Atom(1));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Eval);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }

        let mut vm = vm::VirtualMachine::new();
        vm.instructions.push(vm::Opcode::Atom(1));
        vm.instructions.push(vm::Opcode::Atom(2));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Eval);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        match vm.stack.last() {
            Some(vm::Value::None) => {}
            _ => assert!(false),
        }

        let mut vm = vm::VirtualMachine::new();
        vm.instructions.push(vm::Opcode::Variable(1));
        vm.instructions.push(vm::Opcode::Atom(2));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Eval);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.get(&1).unwrap(), &unification::Term::Atom(2));
        } else {
            assert!(false);
        }
    }

    #[test]
    fn disj2() {
        let mut vm = vm::VirtualMachine::new();
        vm.instructions.push(vm::Opcode::Variable(1));
        vm.instructions.push(vm::Opcode::Atom(1));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Variable(1));
        vm.instructions.push(vm::Opcode::Atom(2));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Disj2);
        vm.instructions.push(vm::Opcode::Eval);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.get(&1).unwrap(), &unification::Term::Atom(2));
        } else {
            assert!(false);
        }
    }

    #[test]
    fn conj2() {
        let mut vm = vm::VirtualMachine::new();
        vm.instructions.push(vm::Opcode::Variable(1));
        vm.instructions.push(vm::Opcode::Atom(1));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Variable(2));
        vm.instructions.push(vm::Opcode::Atom(2));
        vm.instructions.push(vm::Opcode::Unify);
        vm.instructions.push(vm::Opcode::Conj2);
        vm.instructions.push(vm::Opcode::Eval);
        vm.instructions.push(vm::Opcode::Next);
        assert!(vm.run().is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(substs.get(&1).unwrap(), &unification::Term::Atom(1));
        } else {
            assert!(false);
        }
    }
}
