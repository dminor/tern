use crate::errors::RuntimeError;
use crate::logic;
use crate::unification;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

pub type AtomType = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CallableKind {
    Function,
    Relation,
}

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
    // Solve the goal to produce a stream.
    // Goal -> Stream
    Solve,
    // Call next on the stream, pushing a table to the stack.
    // Stream -> Stream Table
    Next,
    // Pop the value from the top of the stack.
    // Value ->
    Pop,
    // Create a new table and push it to the stack.
    // -> Table
    NewTable,
    // Set the field in the table with `key` to `value`.
    // Table key value -> Table
    SetTable,
    // Get the field in the table with `key`. Pushes `None` for missing keys.
    // Table key -> Table value
    GetTable,
    // Set a variable `name` in the environment to `value`.
    // name value ->
    SetEnv,
    // Get the value variable `name` in the environment.
    // name -> value
    GetEnv,
    // Call the callable, passing n arguments to it.
    // arg1 arg2 ... argn callable -> value
    Call,
    // Return from a call.
    Ret,
    // Create a new callable
    Callable {
        kind: CallableKind,
        parameters: Rc<Vec<u64>>,
        instructions: Rc<Vec<Opcode>>,
        ip: usize,
    },
}

impl fmt::Display for CallableKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CallableKind::Function => write!(f, "function"),
            CallableKind::Relation => write!(f, "relation"),
        }
    }
}

pub enum Value {
    Term(unification::Term<AtomType>),
    Goal(Rc<dyn logic::Goal<AtomType>>),
    Stream(Box<dyn Iterator<Item = unification::Substitutions<AtomType>>>),
    Table(HashMap<unification::Term<AtomType>, unification::Term<AtomType>>),
    None,
    Callable {
        kind: CallableKind,
        parameters: Rc<Vec<u64>>,
        instructions: Rc<Vec<Opcode>>,
        ip: usize,
    },
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
            Value::Table(values) => {
                write!(f, "<table (")?;
                let mut first = true;
                for value in values {
                    if !first {
                        write!(f, ", {:?}: {:?}", value.0, value.1)?;
                    } else {
                        first = false;
                        write!(f, "{:?}: {:?}", value.0, value.1)?;
                    }
                }
                write!(f, ")>")
            }
            Value::None => write!(f, "<none>"),
            Value::Callable {
                kind,
                parameters: _,
                instructions: _,
                ip: _,
            } => write!(f, "<{}>", kind),
        }
    }
}

pub struct VirtualMachine {
    next_id: u64,
    pub interned: HashMap<u64, String>,

    pub stack: Vec<Value>,
    pub callstack: Vec<Value>,

    // Because we don't currently support user functions, all let bindings
    // occur at global scope.
    pub env: HashMap<u64, Value>,
}

macro_rules! err {
    ($vm: expr, $msg: expr, $ip: expr) => {{
        return Err(RuntimeError {
            msg: $msg.to_string(),
            ip: $ip,
        });
    }};
}

macro_rules! buildgoal {
    ($vm:expr, $type:tt, $goal:tt, $ip: expr) => {{
        match $vm.stack.pop() {
            Some(Value::$type(left)) => match $vm.stack.pop() {
                Some(Value::$type(right)) => {
                    $vm.stack
                        .push(Value::Goal(Rc::new(logic::$goal::new(left, right))));
                }
                None => {
                    err!($vm, "Stack underflow.", $ip);
                }
                _ => {
                    err!($vm, "Expected term.", $ip);
                }
            },
            None => {
                err!($vm, "Stack underflow.", $ip);
            }
            _ => {
                err!($vm, "Expected term.", $ip);
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

    pub fn run(&mut self, instr: Rc<Vec<Opcode>>) -> Result<(), RuntimeError> {
        self.callstack.push(Value::Callable {
            kind: CallableKind::Function,
            parameters: Rc::new(Vec::new()),
            instructions: instr,
            ip: 0,
        });

        loop {
            let (ip, opcode) = match self.callstack.last_mut() {
                Some(Value::Callable {
                    kind: _,
                    parameters: _,
                    instructions,
                    ip,
                }) => (*ip, &instructions[*ip]),
                None => {
                    return Ok(());
                }
                _ => {
                    unreachable!("Callstack must only contain callables.");
                }
            };
            match opcode {
                Opcode::Atom(atom) => self.stack.push(Value::Term(unification::Term::Atom(*atom))),
                Opcode::Variable(var) => self
                    .stack
                    .push(Value::Term(unification::Term::Variable(*var))),
                Opcode::Conj2 => buildgoal!(self, Goal, Conj2, ip),
                Opcode::Disj2 => buildgoal!(self, Goal, Disj2, ip),
                Opcode::Unify => buildgoal!(self, Term, Unify, ip),
                Opcode::Solve => match self.stack.pop() {
                    Some(Value::Goal(goal)) => {
                        let substs = HashMap::new();
                        self.stack.push(Value::Stream(goal.solve(&substs)));
                    }
                    None => {
                        err!(self, "Stack underflow.", ip);
                    }
                    _ => {
                        err!(self, "TypeError: Expected goal.", ip);
                    }
                },
                Opcode::Next => match self.stack.pop() {
                    Some(Value::Stream(mut stream)) => match stream.next() {
                        Some(substs) => {
                            let mut table = HashMap::new();
                            for subst in substs {
                                table.insert(unification::Term::Variable(subst.0), subst.1);
                            }
                            self.stack.push(Value::Stream(stream));
                            self.stack.push(Value::Table(table));
                        }
                        None => self.stack.push(Value::None),
                    },
                    None => {
                        err!(self, "Stack underflow.", ip);
                    }
                    Some(_) => {
                        // TODO: Add value to error message...
                        err!(self, "Unexpected value.", ip);
                    }
                },
                Opcode::Pop => {
                    if self.stack.pop().is_none() {
                        err!(self, "Stack underflow.", ip);
                    }
                }
                Opcode::NewTable => {
                    let table = HashMap::new();
                    self.stack.push(Value::Table(table));
                }
                Opcode::SetTable => {
                    let value = if let Some(value) = self.stack.pop() {
                        if let Value::Term(term) = value {
                            term
                        } else {
                            err!(self, "TypeError: Expected term.", ip);
                        }
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                    let key = if let Some(value) = self.stack.pop() {
                        if let Value::Term(term) = value {
                            term
                        } else {
                            err!(self, "TypeError: Expected term.", ip);
                        }
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                    if let Some(table) = self.stack.last_mut() {
                        if let Value::Table(table) = table {
                            table.insert(key, value);
                        } else {
                            err!(self, "Expected table.", ip);
                        }
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                }
                Opcode::GetTable => {
                    let key = if let Some(value) = self.stack.pop() {
                        if let Value::Term(term) = value {
                            term
                        } else {
                            err!(self, "TypeError: Expected Term.", ip);
                        }
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                    if let Some(table) = self.stack.last_mut() {
                        if let Value::Table(table) = table {
                            if let Some(term) = table.get(&key) {
                                let term = term.clone();
                                self.stack.push(Value::Term(term));
                            } else {
                                self.stack.push(Value::None);
                            }
                        } else {
                            err!(self, "TypeError: Expected table.", ip);
                        }
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                }
                Opcode::SetEnv => {
                    if let Some(value) = self.stack.pop() {
                        let key = if let Some(value) = self.stack.pop() {
                            if let Value::Term(term) = value {
                                if let unification::Term::Variable(v) = term {
                                    v
                                } else {
                                    err!(self, "TypeError: Expected variable.", ip);
                                }
                            } else {
                                err!(self, "TypeError: Expected variable.", ip);
                            }
                        } else {
                            err!(self, "Stack underflow.", ip);
                        };
                        self.env.insert(key, value);
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                }
                Opcode::GetEnv => {
                    let key = if let Some(value) = self.stack.pop() {
                        if let Value::Term(term) = value {
                            if let unification::Term::Variable(v) = term {
                                v
                            } else {
                                err!(self, "TypeError: Expected variable.", ip);
                            }
                        } else {
                            err!(self, "TypeError: Expected variable.", ip);
                        }
                    } else {
                        err!(self, "Stack underflow.", ip);
                    };
                    if let Some(value) = self.env.get(&key) {
                        match value {
                            Value::Term(t) => {
                                self.stack.push(Value::Term(t.clone()));
                            }
                            Value::Goal(g) => {
                                self.stack.push(Value::Goal(g.clone()));
                            }
                            Value::None => {
                                self.stack.push(Value::None);
                            }
                            Value::Stream(_) => {
                                // TODO: We'll probably want to use a RefCell
                                // and introduce a reference value to get this
                                // working properly.
                                return Err(RuntimeError {
                                    msg: "Accessing streams through variables is not implemented."
                                        .to_string(),
                                    ip: ip,
                                });
                            }
                            Value::Table(t) => {
                                // TODO: Right now, tables are inmutable, so
                                // we can just return a copy. But we'll need
                                // to reconsider this if we make tables
                                // mutable.
                                self.stack.push(Value::Table(t.clone()));
                            }
                            Value::Callable {
                                kind,
                                parameters,
                                instructions,
                                ip,
                            } => {
                                self.stack.push(Value::Callable {
                                    kind: *kind,
                                    parameters: parameters.clone(),
                                    instructions: instructions.clone(),
                                    ip: *ip,
                                });
                            }
                        }
                    } else {
                        // TODO: include name in error message
                        err!(self, "Undefined variable.", ip);
                    }
                }
                Opcode::Call => match self.stack.pop() {
                    Some(Value::Callable {
                        kind,
                        parameters,
                        instructions,
                        ip,
                    }) => {
                        self.callstack.push(Value::Callable {
                            kind,
                            parameters,
                            instructions,
                            ip,
                        });
                        continue;
                    }
                    None => {
                        err!(self, "Stack underflow.", ip);
                    }
                    _ => {
                        err!(self, "TypeError: Expected callable.", ip);
                    }
                },
                Opcode::Ret => {
                    match self.callstack.pop() {
                        Some(Value::Callable {
                            kind,
                            parameters,
                            instructions,
                            ip,
                        }) => {
                            if kind == CallableKind::Relation {
                                // The relation should have pushed a Goal to the stack.
                                match self.stack.pop() {
                                    Some(Value::Goal(goal)) => {
                                        // Create a series of Conj that bind each parameter to argument using unify.
                                        let mut result = goal;
                                        for parameter in parameters.iter().rev() {
                                            match self.stack.pop() {
                                                Some(Value::Term(term)) => {
                                                    result = Rc::new(logic::Conj2::new(
                                                        Rc::new(logic::Unify::new(
                                                            unification::Term::Variable(*parameter),
                                                            term,
                                                        )),
                                                        result,
                                                    ));
                                                }
                                                Some(_) => {
                                                    // TODO: include type in message.
                                                    err!(self, "TypeError: Expected term as argument to relation.", ip);
                                                }
                                                None => {
                                                    err!(self, "Stack underflow.", ip);
                                                }
                                            }
                                        }
                                        self.stack.push(Value::Goal(result));
                                    }
                                    Some(_) => {
                                        // TODO: include type in message.
                                        err!(
                                            self,
                                            "TypeError: Unexpected type returned from relation.",
                                            ip
                                        );
                                    }
                                    None => {
                                        err!(self, "Stack underflow.", ip);
                                    }
                                }
                            }
                        }
                        None => {
                            err!(self, "Call stack underflow.", ip);
                        }
                        _ => {
                            err!(self, "TypeError: Expected callable.", ip);
                        }
                    }
                }
                Opcode::Callable {
                    kind,
                    parameters,
                    instructions,
                    ip,
                } => {
                    self.stack.push(Value::Callable {
                        kind: *kind,
                        parameters: parameters.clone(),
                        instructions: instructions.clone(),
                        ip: *ip,
                    });
                }
            }
            match self.callstack.last_mut() {
                Some(Value::Callable {
                    kind: _,
                    parameters: _,
                    instructions,
                    ip,
                }) => {
                    *ip += 1;
                    // Implicit return if we hit the end of the buffer.
                    if *ip == instructions.len() {
                        self.callstack.pop();
                    }
                }
                None => unreachable!("Empty callstack at bottom of interpreter loop."),
                _ => unreachable!("Callstack must only contain callables."),
            };
        }
    }

    pub fn new() -> Self {
        Self {
            next_id: 0,
            interned: HashMap::new(),
            stack: Vec::new(),
            callstack: Vec::new(),
            env: HashMap::new(),
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
                .push(vm::Value::Stream(Box::new(goal.solve(&substs))));
        }
        let table = HashMap::new();
        vm.stack.push(vm::Value::Table(table));
        vm.stack.push(vm::Value::None);
        vm.stack.push(vm::Value::Callable {
            kind: vm::CallableKind::Relation,
            parameters: Rc::new(Vec::new()),
            instructions: Rc::new(Vec::new()),
            ip: 0,
        });
        assert_eq!(vm.stack.len(), 9);
    }

    #[test]
    fn unify() {
        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Atom(1));
        instr.push(vm::Opcode::Atom(1));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }

        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Atom(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        match vm.stack.last() {
            Some(vm::Value::None) => {}
            _ => assert!(false),
        }

        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(
                substs.get(&unification::Term::Variable(1)).unwrap(),
                &unification::Term::Atom(2)
            );
        } else {
            assert!(false);
        }
    }

    #[test]
    fn disj2() {
        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(1));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Disj2);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(
                substs.get(&unification::Term::Variable(1)).unwrap(),
                &unification::Term::Atom(2)
            );
        } else {
            assert!(false);
        }
    }

    #[test]
    fn conj2() {
        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(1));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Variable(2));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::Conj2);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(
                substs.get(&unification::Term::Variable(1)).unwrap(),
                &unification::Term::Atom(1)
            );
        } else {
            assert!(false);
        }
    }

    #[test]
    fn table() {
        // Test NewTable.
        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::NewTable);
        assert!(vm.run(Rc::new(instr)).is_ok());
        if let Some(vm::Value::Table(table)) = vm.stack.last() {
            assert_eq!(table.len(), 0);
        } else {
            assert!(false);
        }

        // Test SetTable.
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::NewTable);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::SetTable);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 1);
        if let Some(vm::Value::Table(table)) = vm.stack.last() {
            assert_eq!(table.len(), 1);
            assert_eq!(
                table.get(&unification::Term::Variable(1)).unwrap(),
                &unification::Term::Atom(2)
            );
        } else {
            assert!(false);
        }

        // Test GetTable.
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::NewTable);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::SetTable);
        instr.push(vm::Opcode::Variable(2));
        instr.push(vm::Opcode::GetTable);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 2);
        if let Some(vm::Value::None) = vm.stack.last() {
            // Ok.
        } else {
            assert!(false);
        }

        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::NewTable);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::SetTable);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::GetTable);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 2);
        if let Some(vm::Value::Term(unification::Term::Atom(2))) = vm.stack.last() {
            // Ok.
        } else {
            assert!(false);
        }
    }

    #[test]
    fn env() {
        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::SetEnv);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 0);

        // Terms
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::SetEnv);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::GetEnv);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 1);
        if let Some(vm::Value::Term(unification::Term::Atom(2))) = vm.stack.last() {
            // Ok.
        } else {
            assert!(false);
        }

        // Goals
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::SetEnv);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::GetEnv);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 1);
        if let Some(vm::Value::Goal(_)) = vm.stack.last() {
            // Ok.
        } else {
            assert!(false);
        }

        // Streams
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::Unify);
        instr.push(vm::Opcode::NewTable);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::SetEnv);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::GetEnv);
        assert!(!vm.run(Rc::new(instr)).is_ok());

        // Tables
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::NewTable);
        instr.push(vm::Opcode::SetEnv);
        instr.push(vm::Opcode::Variable(1));
        instr.push(vm::Opcode::GetEnv);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 1);
        if let Some(vm::Value::Table(_)) = vm.stack.last() {
            // Ok.
        } else {
            assert!(false);
        }

        // Try to use atom as a variable name.
        vm = vm::VirtualMachine::new();
        instr = Vec::new();
        instr.push(vm::Opcode::Atom(1));
        instr.push(vm::Opcode::Atom(2));
        instr.push(vm::Opcode::SetEnv);
        assert!(!vm.run(Rc::new(instr)).is_ok());
    }

    #[test]
    fn callable() {
        let mut vm = vm::VirtualMachine::new();
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Callable {
            kind: vm::CallableKind::Relation,
            parameters: Rc::new(Vec::new()),
            instructions: Rc::new(vec![
                vm::Opcode::Atom(1),
                vm::Opcode::Atom(1),
                vm::Opcode::Unify,
                vm::Opcode::Ret,
            ]),
            ip: 0,
        });
        instr.push(vm::Opcode::Call);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 2);
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert!(substs.is_empty());
        } else {
            assert!(false);
        }

        let mut vm = vm::VirtualMachine::new();
        vm.stack
            .push(vm::Value::Term(unification::Term::Variable(1)));
        vm.stack.push(vm::Value::Callable {
            kind: vm::CallableKind::Relation,
            parameters: Rc::new(vec![1]),
            instructions: Rc::new(vec![
                vm::Opcode::Variable(1),
                vm::Opcode::Atom(2),
                vm::Opcode::Unify,
                vm::Opcode::Ret,
            ]),
            ip: 0,
        });
        let mut instr = Vec::new();
        instr.push(vm::Opcode::Call);
        instr.push(vm::Opcode::Solve);
        instr.push(vm::Opcode::Next);
        assert!(vm.run(Rc::new(instr)).is_ok());
        assert_eq!(vm.stack.len(), 2);
        if let Some(vm::Value::Table(substs)) = vm.stack.last() {
            assert_eq!(
                substs.get(&unification::Term::Variable(1)).unwrap(),
                &unification::Term::Atom(2)
            );
        } else {
            assert!(false);
        }
    }
}
