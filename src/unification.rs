use std::collections::HashMap;
use std::fmt::Debug;

pub type Substitutions<T> = HashMap<u64, Term<T>>;

#[derive(Debug, Eq, Hash)]
pub enum Term<T> {
    Atom(T),
    Variable(u64),
    Tuple(Vec<Term<T>>),
}

impl<T: Clone> Clone for Term<T> {
    fn clone(&self) -> Self {
        match self {
            Term::Atom(u) => Term::Atom(u.clone()),
            Term::Variable(u) => Term::Variable(*u),
            Term::Tuple(u) => Term::Tuple(u.to_vec()),
        }
    }
}

impl<T: std::cmp::PartialEq> PartialEq for Term<T> {
    fn eq(&self, other: &Term<T>) -> bool {
        match self {
            Term::Atom(u) => {
                if let Term::Atom(v) = other {
                    u == v
                } else {
                    false
                }
            }
            Term::Variable(u) => {
                if let Term::Variable(v) = other {
                    u == v
                } else {
                    false
                }
            }
            Term::Tuple(u) => {
                if let Term::Tuple(v) = other {
                    u == v
                } else {
                    false
                }
            }
        }
    }
}

// Resolve the value of x in the substitutions.
//
// `walk` is a utility function that walks the substitutions, recursively resolving variables
// until an unbound variable or an atom is encountered. E.g, given substitutions that map x -> y,
// y -> z, and z -> "ceviche", calling `walk` with the variable `x` will result in the atom
// "ceviche".
fn walk<'a, T: Clone>(x: &'a Term<T>, substs: &'a Substitutions<T>) -> &'a Term<T> {
    if let Term::Variable(var) = x {
        if let Some(t) = substs.get(var) {
            walk(t, substs)
        } else {
            x
        }
    } else {
        x
    }
}

// Attempt to unify the left and right hand terms using the given substitutions, returning
// true if the terms unify, false otherwise.
//
// If one of the terms is an unbound variable, it will be bound to the other term,
// extending the substitutions. If both terms are bound variables or atoms, the unification
// will succeed if the value of the bound variable or the atom is equal to the other
// term. An unbound variable can be bound to an atom, to a bound variable, or another
// unbound variable. Once bound, a variable can not be bound to another term.
pub fn unify<T: std::cmp::PartialEq + Clone>(
    left: &Term<T>,
    right: &Term<T>,
    substs: &mut Substitutions<T>,
) -> bool {
    match left {
        Term::Atom(u) => match right {
            Term::Atom(v) => u == v,
            Term::Variable(_) => {
                let x = walk(right, substs);
                if let Term::Variable(var) = x {
                    substs.insert(*var, left.clone());
                    true
                } else {
                    left == x
                }
            }
            Term::Tuple(_) => false,
        },
        Term::Variable(_) => {
            let x = walk(left, substs);
            // Check for equality early to avoid binding a variable to itself,
            // which will lead to infinite recursion while unifying.
            if right == x {
                true
            } else if let Term::Variable(var) = x {
                let y = walk(right, substs);
                // Only introduce a binding if y resolves to something other
                // than x, to avoid cycles that will lead to infinite recursion
                // while unifying.
                if x != y {
                    substs.insert(*var, right.clone());
                }
                true
            } else {
                false
            }
        }
        Term::Tuple(u) => match right {
            Term::Atom(_) => false,
            Term::Variable(var) => {
                let x = walk(right, substs);
                if let Term::Variable(var) = x {
                    substs.insert(*var, left.clone());
                    true
                } else {
                    right == x
                }
            }
            Term::Tuple(v) => {
                if u.len() != v.len() {
                    return false;
                }
                for (u0, v0) in u.iter().zip(v.iter()) {
                    if !unify(u0, v0, substs) {
                        return false;
                    }
                }
                return true;
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::unification::*;

    #[test]
    fn test_walk() {
        let substs = HashMap::<u64, Term<i32>>::new();
        assert_eq!(walk(&Term::Variable(1), &substs), &Term::Variable(1));
        assert_eq!(walk(&Term::Atom(42), &substs), &Term::Atom(42));

        let mut substs = HashMap::new();
        substs.insert(1, Term::Variable(2));
        substs.insert(3, Term::Atom("a".to_string()));
        substs.insert(2, Term::Variable(3));
        assert_eq!(
            walk(&Term::Variable(1), &substs),
            &Term::Atom("a".to_string())
        );
    }

    #[test]
    fn test_unify() {
        let mut substs = HashMap::new();
        assert!(unify(
            &Term::Atom("a".to_string()),
            &Term::Atom("a".to_string()),
            &mut substs
        ));
        assert!(!unify(
            &Term::Atom("a".to_string()),
            &Term::Atom("ab".to_string()),
            &mut substs
        ));
        assert_eq!(substs.len(), 0);

        let mut substs = HashMap::new();
        assert!(unify(&Term::Atom(1), &Term::Atom(1), &mut substs));
        assert_eq!(substs.len(), 0);

        let mut substs = HashMap::new();
        assert!(unify(&Term::Variable(1), &Term::Atom(1), &mut substs));
        assert_eq!(substs.len(), 1);
        assert_eq!(*substs.get(&1).unwrap(), Term::Atom(1));
        assert!(unify(&Term::Variable(1), &Term::Atom(1), &mut substs));
        assert!(!unify(&Term::Variable(1), &Term::Atom(2), &mut substs));

        let mut substs = HashMap::new();
        assert!(unify(
            &Term::Tuple(vec!(Term::Variable(1), Term::Atom(1))),
            &Term::Tuple(vec!(Term::Atom(2), Term::Variable(2))),
            &mut substs
        ));
        assert_eq!(substs.len(), 2);
        assert_eq!(*substs.get(&1).unwrap(), Term::Atom(2));
        assert_eq!(*substs.get(&2).unwrap(), Term::Atom(1));

        let mut substs = HashMap::<u64, Term<i32>>::new();
        assert!(unify(&Term::Variable(1), &Term::Variable(2), &mut substs));
        assert_eq!(substs.len(), 1);
        assert_eq!(*substs.get(&1).unwrap(), Term::Variable(2));

        let mut substs = HashMap::<u64, Term<i32>>::new();
        assert!(unify(
            &Term::Tuple(vec!(Term::Variable(1), Term::Variable(1))),
            &Term::Tuple(vec!(Term::Variable(2), Term::Variable(2))),
            &mut substs
        ));
        assert_eq!(substs.len(), 1);
        assert_eq!(*substs.get(&1).unwrap(), Term::Variable(2));

        let mut substs = HashMap::<u64, Term<i32>>::new();
        assert!(unify(
            &Term::Tuple(vec!(
                Term::Variable(1),
                Term::Variable(1),
                Term::Variable(1)
            )),
            &Term::Tuple(vec!(
                Term::Variable(2),
                Term::Variable(2),
                Term::Variable(2)
            )),
            &mut substs
        ));
        assert_eq!(substs.len(), 1);
        assert_eq!(*substs.get(&1).unwrap(), Term::Variable(2));

        let mut substs = HashMap::<u64, Term<i32>>::new();
        assert!(unify(
            &Term::Tuple(vec!(Term::Variable(1), Term::Variable(2), Term::Atom(42))),
            &Term::Tuple(vec!(
                Term::Variable(2),
                Term::Variable(1),
                Term::Variable(1)
            )),
            &mut substs
        ));
        assert_eq!(substs.len(), 2);
        assert_eq!(*substs.get(&1).unwrap(), Term::Variable(2));
        assert_eq!(*substs.get(&2).unwrap(), Term::Atom(42));
    }
}
