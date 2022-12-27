use crate::unification::{unify, Substitutions, Term};
use std::marker::PhantomData;

// A goal defines a single function eval() that takes substitutions as an
// argument, and produces a stream of substitutions as a result.
pub trait Goal<T> {
    type SubstitutionsIterator: Iterator<Item = Substitutions<T>>;

    fn eval(&self, substs: &Substitutions<T>) -> Self::SubstitutionsIterator;
}

// The Succeed goal produces a singleton stream.
#[derive(Clone)]
pub struct Succeed {}

pub struct SucceedIterator<T> {
    substs: Option<Substitutions<T>>,
}

impl<T: Clone> Goal<T> for Succeed {
    type SubstitutionsIterator = SucceedIterator<T>;

    fn eval(&self, substs: &Substitutions<T>) -> Self::SubstitutionsIterator {
        SucceedIterator {
            substs: Some(substs.clone()),
        }
    }
}

impl<T: Clone> Iterator for SucceedIterator<T> {
    type Item = Substitutions<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.substs.is_some() {
            let result = self.substs.clone();
            self.substs = None;
            result
        } else {
            None
        }
    }
}

// The Fail goal produces the empty stream.
#[derive(Clone)]
pub struct Fail {}

pub struct FailureIterator<T> {
    phantom: PhantomData<T>,
}

impl<T> Goal<T> for Fail {
    type SubstitutionsIterator = FailureIterator<T>;

    fn eval(&self, _: &Substitutions<T>) -> FailureIterator<T> {
        FailureIterator {
            phantom: PhantomData,
        }
    }
}

impl<T> Iterator for FailureIterator<T> {
    type Item = Substitutions<T>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

// The EqualsExpr goal produces either a singleton stream, if left and
// right unify, or the empty stream.
#[derive(Clone)]
pub struct EqualsExpr<T> {
    // Left term.
    left: Term<T>,
    // Right term.
    right: Term<T>,
}

pub struct EqualsExprIterator<T> {
    // True if we've evaluated the result.
    forced: bool,
    // Left term.
    left: Term<T>,
    // Right term.
    right: Term<T>,
    // substitutions to use during unification.
    substs: Substitutions<T>,
}

impl<T: std::cmp::PartialEq + Clone> Goal<T> for EqualsExpr<T> {
    type SubstitutionsIterator = EqualsExprIterator<T>;

    fn eval(&self, substs: &Substitutions<T>) -> EqualsExprIterator<T> {
        EqualsExprIterator {
            forced: false,
            left: self.left.clone(),
            right: self.right.clone(),
            substs: substs.clone(),
        }
    }
}

impl<T: std::cmp::PartialEq + Clone> Iterator for EqualsExprIterator<T> {
    type Item = Substitutions<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.forced {
            self.forced = true;
            let unified = unify(&self.left, &self.right, &mut self.substs);
            if unified {
                let result = Some(self.substs.clone());
                self.substs.clear();
                result
            } else {
                None
            }
        } else {
            None
        }
    }
}

// The Disj2 goal produces the stream that results from interleaving
// substitutions produced by the left and the right goals, continuing until
// both streams are empty. The Disj2 goal succeeds if either of the
// left or the right goal succeeds.
#[derive(Clone)]
pub struct Disj2<T, G1: Goal<T>, G2: Goal<T>> {
    // Left goal.
    left: G1,
    // Right goal.
    right: G2,
    phantom: PhantomData<T>,
}

impl<T, G1: Goal<T>, G2: Goal<T>> Disj2<T, G1, G2> {
    fn new(left: G1, right: G2) -> Self {
        Disj2 {
            left,
            right,
            phantom: PhantomData,
        }
    }
}

pub struct Disj2Iterator<
    T,
    I1: Iterator<Item = Substitutions<T>>,
    I2: Iterator<Item = Substitutions<T>>,
> {
    // Iterator from left goal.
    left: I1,
    // Iterator from right goal.
    right: I2,
    // True if we should take a result from the left stream next.
    interleave_left: bool,
    phantom: PhantomData<T>,
}

impl<T: Clone, G1: Goal<T> + Clone, G2: Goal<T> + Clone> Goal<T> for Disj2<T, G1, G2> {
    type SubstitutionsIterator =
        Disj2Iterator<T, G1::SubstitutionsIterator, G2::SubstitutionsIterator>;

    fn eval(&self, substs: &Substitutions<T>) -> Self::SubstitutionsIterator {
        Disj2Iterator {
            left: self.left.eval(substs),
            right: self.right.eval(substs),
            interleave_left: true,
            phantom: PhantomData,
        }
    }
}

impl<T: Clone, I1: Iterator<Item = Substitutions<T>>, I2: Iterator<Item = Substitutions<T>>>
    Iterator for Disj2Iterator<T, I1, I2>
{
    type Item = Substitutions<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Interleave the two streams. If one stream is empty, just produce
        // results from the other stream.
        if self.interleave_left {
            self.interleave_left = false;
            self.left.next().or_else(|| self.right.next())
        } else {
            self.interleave_left = true;
            self.right.next().or_else(|| self.left.next())
        }
    }
}

// The Conj2 goal produces a stream of substitutions that results from mapping
// the right goal over the stream of substitutions produced by the left goal. Conj2
// succeeds only if both goals succeed.
#[derive(Clone)]
pub struct Conj2<T, G1: Goal<T>, G2: Goal<T>> {
    // Left goal.
    left: G1,
    // Right goal.
    right: G2,
    phantom: PhantomData<T>,
}

impl<T, G1: Goal<T>, G2: Goal<T>> Conj2<T, G1, G2> {
    fn new(left: G1, right: G2) -> Self {
        Conj2 {
            left,
            right,
            phantom: PhantomData,
        }
    }
}

pub struct Conj2Iterator<
    T: Clone,
    G: Goal<T>,
    I1: Iterator<Item = Substitutions<T>>,
    I2: Iterator<Item = Substitutions<T>>,
> {
    // Right goal.
    right: G,
    // Stream produced from applying right goal to substitutions from the left terator.
    right_iterator: Option<I1>,
    // Left iterator.
    left_iterator: I2,
    phantom: PhantomData<T>,
}

impl<T: Clone, G1: Goal<T> + Clone, G2: Goal<T> + Clone> Goal<T> for Conj2<T, G1, G2> {
    type SubstitutionsIterator =
        Conj2Iterator<T, G2, G2::SubstitutionsIterator, G1::SubstitutionsIterator>;

    fn eval(&self, substs: &Substitutions<T>) -> Self::SubstitutionsIterator {
        Conj2Iterator {
            right: self.right.clone(),
            right_iterator: None,
            left_iterator: self.left.eval(substs),
            phantom: PhantomData,
        }
    }
}

impl<
        T: Clone,
        G: Goal<T, SubstitutionsIterator = I1>,
        I1: Iterator<Item = Substitutions<T>>,
        I2: Iterator<Item = Substitutions<T>>,
    > Iterator for Conj2Iterator<T, G, I1, I2>
{
    type Item = Substitutions<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have a stream from applying the goal to a binding from the
        // left iterator, we iterate over that until it is empty. If it's
        // empty, we reset the iterator, and call next() to attempt to
        // apply the goal again.
        if let Some(iterator) = &mut self.right_iterator {
            let result = iterator.next();
            if result.is_none() {
                self.right_iterator = None;
                self.next()
            } else {
                result
            }
        } else {
            // If we get a new substitutions from the left iterator, we evalate the goal
            // using the new substitutions, and call next() to use that stream of
            // substitutions. If the left iterator is empty, we're done.
            if let Some(substs) = self.left_iterator.next() {
                self.right_iterator = Some(self.right.eval(&substs));
                self.next()
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::logic::*;

    #[test]
    fn test_succeed() {
        let substs: HashMap<i64, Term<u32>> = HashMap::new();
        let success = Succeed {};
        let mut iter = success.eval(&substs);
        assert_eq!(iter.next().unwrap(), substs);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_fail() {
        let substs: HashMap<i64, Term<u32>> = HashMap::new();
        let failure = Fail {};
        let mut iter = failure.eval(&substs);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_equalsexpr() {
        let substs = HashMap::new();
        let equals = EqualsExpr {
            left: Term::Atom("olive".to_string()),
            right: Term::Atom("olive".to_string()),
        };
        let mut iter = equals.eval(&substs);
        assert_eq!(iter.next().unwrap(), substs);

        let equals = EqualsExpr {
            left: Term::Atom("olive".to_string()),
            right: Term::Atom("oil".to_string()),
        };
        let mut iter = equals.eval(&substs);
        assert_eq!(iter.next(), None);

        let equals = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let mut iter = equals.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_disj2() {
        let substs = HashMap::new();
        let disj2 = Disj2::new(Fail {}, Fail {});
        let mut iter = disj2.eval(&substs);
        assert_eq!(iter.next(), None);

        let left = Fail {};
        let right = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        };
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("oil".to_string()));
        assert_eq!(iter.next(), None);

        let left = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let right = Fail {};
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(iter.next(), None);

        let left = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let right = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        };
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("oil".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_conj2() {
        let substs = HashMap::new();
        let conj2 = Conj2::new(Fail {}, Fail {});
        let mut iter = conj2.eval(&substs);
        assert_eq!(iter.next(), None);

        let left = Fail {};
        let right = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        };
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.eval(&substs);
        assert_eq!(iter.next(), None);

        let left = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let right = Fail {};
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.eval(&substs);
        assert_eq!(iter.next(), None);

        let left = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let right = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        };
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.eval(&substs);
        assert_eq!(iter.next(), None);

        let left = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let right = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(iter.next(), None);

        let left = EqualsExpr {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let right = EqualsExpr {
            left: Term::Variable(2),
            right: Term::Atom("oil".to_string()),
        };
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(*result.get(&2).unwrap(), Term::Atom("oil".to_string()));
        assert_eq!(iter.next(), None);

        let left = Conj2::new(
            EqualsExpr {
                left: Term::Variable(1),
                right: Term::Atom("split".to_string()),
            },
            EqualsExpr {
                left: Term::Variable(2),
                right: Term::Atom("pea".to_string()),
            },
        );
        let right = Conj2::new(
            EqualsExpr {
                left: Term::Variable(1),
                right: Term::Atom("red".to_string()),
            },
            EqualsExpr {
                left: Term::Variable(2),
                right: Term::Atom("bean".to_string()),
            },
        );
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.eval(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("split".to_string()));
        assert_eq!(*result.get(&2).unwrap(), Term::Atom("pea".to_string()));
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("red".to_string()));
        assert_eq!(*result.get(&2).unwrap(), Term::Atom("bean".to_string()));
        assert_eq!(iter.next(), None);
    }
}
