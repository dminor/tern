use crate::unification::{unify, Substitutions, Term};
use std::marker::PhantomData;
use std::rc::Rc;

// A goal defines a single function solve() that takes substitutions as an
// argument, and produces a stream of substitutions as a result.
pub trait Goal<T> {
    fn solve(&self, substs: &Substitutions<T>) -> Box<dyn Iterator<Item = Substitutions<T>>>;
}

// The EqualsExpr goal produces either a singleton stream, if left and
// right unify, or the empty stream.
#[derive(Clone)]
pub struct Unify<T> {
    // Left term.
    left: Term<T>,
    // Right term.
    right: Term<T>,
}

impl<T> Unify<T> {
    pub fn new(left: Term<T>, right: Term<T>) -> Self {
        Unify { left, right }
    }
}

pub struct UnifyIterator<T> {
    // True if we've evaluated the result.
    forced: bool,
    // Left term.
    left: Term<T>,
    // Right term.
    right: Term<T>,
    // substitutions to use during unification.
    substs: Substitutions<T>,
}

impl<T: std::cmp::PartialEq + Clone + 'static> Goal<T> for Unify<T> {
    fn solve(&self, substs: &Substitutions<T>) -> Box<dyn Iterator<Item = Substitutions<T>>> {
        Box::new(UnifyIterator {
            forced: false,
            left: self.left.clone(),
            right: self.right.clone(),
            substs: substs.clone(),
        })
    }
}

impl<T: std::cmp::PartialEq + Clone> Iterator for UnifyIterator<T> {
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
pub struct Disj2<T> {
    // Left goal.
    left: Rc<dyn Goal<T>>,
    // Right goal.
    right: Rc<dyn Goal<T>>,
    phantom: PhantomData<T>,
}

impl<T> Disj2<T> {
    pub fn new(left: Rc<dyn Goal<T>>, right: Rc<dyn Goal<T>>) -> Self {
        Disj2 {
            left: left.clone(),
            right: right.clone(),
            phantom: PhantomData,
        }
    }
}

pub struct Disj2Iterator<T> {
    // Iterator from left goal.
    left: Box<dyn Iterator<Item = Substitutions<T>>>,
    // Iterator from right goal.
    right: Box<dyn Iterator<Item = Substitutions<T>>>,
    // True if we should take a result from the left stream next.
    interleave_left: bool,
    phantom: PhantomData<T>,
}

impl<T: Clone + 'static> Goal<T> for Disj2<T> {
    fn solve(&self, substs: &Substitutions<T>) -> Box<dyn Iterator<Item = Substitutions<T>>> {
        Box::new(Disj2Iterator {
            left: self.left.solve(substs),
            right: self.right.solve(substs),
            interleave_left: true,
            phantom: PhantomData,
        })
    }
}

impl<T: Clone> Iterator for Disj2Iterator<T> {
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
pub struct Conj2<T> {
    // Left goal.
    left: Rc<dyn Goal<T>>,
    // Right goal.
    right: Rc<dyn Goal<T>>,
    phantom: PhantomData<T>,
}

impl<T> Conj2<T> {
    pub fn new(left: Rc<dyn Goal<T>>, right: Rc<dyn Goal<T>>) -> Self {
        Conj2 {
            left: left.clone(),
            right: right.clone(),
            phantom: PhantomData,
        }
    }
}

pub struct Conj2Iterator<T: Clone> {
    // Right goal.
    right: Rc<dyn Goal<T>>,
    // Stream produced from applying right goal to substitutions from the left terator.
    right_iterator: Option<Box<dyn Iterator<Item = Substitutions<T>>>>,
    // Left iterator.
    left_iterator: Box<dyn Iterator<Item = Substitutions<T>>>,
    phantom: PhantomData<T>,
}

impl<T: Clone + 'static> Goal<T> for Conj2<T> {
    fn solve(&self, substs: &Substitutions<T>) -> Box<dyn Iterator<Item = Substitutions<T>>> {
        Box::new(Conj2Iterator {
            right: self.right.clone(),
            right_iterator: None,
            left_iterator: self.left.solve(substs),
            phantom: PhantomData,
        })
    }
}

impl<T: Clone> Iterator for Conj2Iterator<T> {
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
                self.right_iterator = Some(self.right.solve(&substs));
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
    use std::rc::Rc;

    use crate::logic::*;

    // The Succeed goal produces a singleton stream.
    pub struct Succeed {}

    pub struct SucceedIterator<T> {
        substs: Option<Substitutions<T>>,
    }

    impl<T: Clone + 'static> Goal<T> for Succeed {
        fn solve(&self, substs: &Substitutions<T>) -> Box<dyn Iterator<Item = Substitutions<T>>> {
            Box::new(SucceedIterator {
                substs: Some(substs.clone()),
            })
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
    pub struct Fail {}

    pub struct FailureIterator<T> {
        phantom: PhantomData<T>,
    }

    impl<T: 'static> Goal<T> for Fail {
        fn solve(&self, _: &Substitutions<T>) -> Box<dyn Iterator<Item = Substitutions<T>>> {
            Box::new(FailureIterator {
                phantom: PhantomData,
            })
        }
    }

    impl<T> Iterator for FailureIterator<T> {
        type Item = Substitutions<T>;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    #[test]
    fn test_succeed() {
        let substs: HashMap<u64, Term<u32>> = HashMap::new();
        let success = Succeed {};
        let mut iter = success.solve(&substs);
        assert_eq!(iter.next().unwrap(), substs);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_fail() {
        let substs: HashMap<u64, Term<u32>> = HashMap::new();
        let failure = Fail {};
        let mut iter = failure.solve(&substs);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_equalsexpr() {
        let substs = HashMap::new();
        let equals = Unify {
            left: Term::Atom("olive".to_string()),
            right: Term::Atom("olive".to_string()),
        };
        let mut iter = equals.solve(&substs);
        assert_eq!(iter.next().unwrap(), substs);

        let equals = Unify {
            left: Term::Atom("olive".to_string()),
            right: Term::Atom("oil".to_string()),
        };
        let mut iter = equals.solve(&substs);
        assert_eq!(iter.next(), None);

        let equals = Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        };
        let mut iter = equals.solve(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_disj2() {
        let substs = HashMap::new();
        let disj2 = Disj2::new(Rc::new(Fail {}), Rc::new(Fail {}));
        let mut iter = disj2.solve(&substs);
        assert_eq!(iter.next(), None);

        let left = Rc::new(Fail {});
        let right = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        });
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.solve(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("oil".to_string()));
        assert_eq!(iter.next(), None);

        let left = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let right = Rc::new(Fail {});
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.solve(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(iter.next(), None);

        let left = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let right = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        });
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.solve(&substs);
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
        let conj2 = Conj2::new(Rc::new(Fail {}), Rc::new(Fail {}));
        let mut iter = conj2.solve(&substs);
        assert_eq!(iter.next(), None);

        let left = Rc::new(Fail {});
        let right = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        });
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.solve(&substs);
        assert_eq!(iter.next(), None);

        let left = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let right = Rc::new(Fail {});
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.solve(&substs);
        assert_eq!(iter.next(), None);

        let left = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let right = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("oil".to_string()),
        });
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.solve(&substs);
        assert_eq!(iter.next(), None);

        let left = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let right = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.solve(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(iter.next(), None);

        let left = Rc::new(Unify {
            left: Term::Variable(1),
            right: Term::Atom("olive".to_string()),
        });
        let right = Rc::new(Unify {
            left: Term::Variable(2),
            right: Term::Atom("oil".to_string()),
        });
        let conj2 = Conj2::new(left, right);
        let mut iter = conj2.solve(&substs);
        let result = iter.next().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(*result.get(&1).unwrap(), Term::Atom("olive".to_string()));
        assert_eq!(*result.get(&2).unwrap(), Term::Atom("oil".to_string()));
        assert_eq!(iter.next(), None);

        let left = Rc::new(Conj2::new(
            Rc::new(Unify {
                left: Term::Variable(1),
                right: Term::Atom("split".to_string()),
            }),
            Rc::new(Unify {
                left: Term::Variable(2),
                right: Term::Atom("pea".to_string()),
            }),
        ));
        let right = Rc::new(Conj2::new(
            Rc::new(Unify {
                left: Term::Variable(1),
                right: Term::Atom("red".to_string()),
            }),
            Rc::new(Unify {
                left: Term::Variable(2),
                right: Term::Atom("bean".to_string()),
            }),
        ));
        let disj2 = Disj2::new(left, right);
        let mut iter = disj2.solve(&substs);
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
