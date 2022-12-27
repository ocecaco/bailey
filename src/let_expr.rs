use std::collections::HashSet;

use crate::syntax::{BinOp, Constant};

#[derive(Debug, Clone)]
pub struct LetExpr {
    // The last let binding represents the result of evaluating the
    // entire sequence of let expressions.
    pub let_bindings: Vec<LetBinding>,
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub definition: LetExprB,
}

#[derive(Debug, Clone)]
pub enum LetExprB {
    Complex(LetExprC),
    Atomic(LetExprA),
}

#[derive(Debug, Clone)]
pub struct LetFunction {
    pub name: String,
    pub arg_names: Vec<String>,
    pub free_names: Vec<String>,
    pub body: Box<LetExpr>,
}

#[derive(Debug, Clone)]
pub enum LetExprC {
    Literal(Constant),
    Fun(LetFunction),
    Call {
        func: LetExprA,
        args: Vec<LetExprA>,
    },
    If {
        condition: LetExprA,
        branch_success: Box<LetExpr>,
        branch_failure: Box<LetExpr>,
    },
    BinOp {
        op: BinOp,
        lhs: LetExprA,
        rhs: LetExprA,
    },
    Tuple {
        args: Vec<LetExprA>,
    },
    Set {
        tuple: LetExprA,
        index: u32,
        new_value: LetExprA,
    },
}

#[derive(Debug, Clone)]
pub struct LetExprA {
    pub var_name: String,
}

// Free variable determination
pub struct FreeVars<'a> {
    freevars: HashSet<&'a str>,
}

impl<'a> FreeVars<'a> {
    pub fn freevars_function(
        funname: &'a str,
        argnames: &'a [String],
        body: &'a LetExpr,
    ) -> HashSet<&'a str> {
        let mut collector = FreeVars::new();
        collector.collect_function(funname, argnames, body);
        collector.done()
    }

    fn new() -> Self {
        FreeVars {
            freevars: HashSet::new(),
        }
    }

    fn collect(&mut self, expr: &'a LetExpr) {
        // We iterate in reverse in order to determine the free variables of the
        // inner (nested) scopes first, because the first let binding scopes
        // over the entirety of the remaining let bindings.
        for binding in expr.let_bindings.iter().rev() {
            // The ordering of these two lines is important: the name of the let
            // binding does NOT scope over its right-hand side, and therefore it
            // should not be removed after processing the definition.
            self.freevars.remove(binding.name.as_str());
            self.collect_b(&binding.definition);
        }
    }

    fn collect_b(&mut self, expr: &'a LetExprB) {
        match expr {
            LetExprB::Atomic(e) => self.collect_a(e),
            LetExprB::Complex(e) => self.collect_c(e),
        }
    }

    fn collect_function(&mut self, funname: &'a str, argnames: &'a [String], body: &'a LetExpr) {
        self.collect(body);

        self.freevars.remove(funname);

        for argname in argnames.iter() {
            self.freevars.remove(argname as &'a str);
        }
    }

    fn collect_c(&mut self, expr: &'a LetExprC) {
        match expr {
            LetExprC::Literal(_) => {}
            LetExprC::Tuple { args } => {
                for arg in args {
                    self.collect_a(arg);
                }
            }
            LetExprC::Set {
                tuple,
                index: _index,
                new_value,
            } => {
                self.collect_a(tuple);
                self.collect_a(new_value);
            }
            LetExprC::Call { func, args } => {
                self.collect_a(func);
                for arg in args {
                    self.collect_a(arg);
                }
            }
            LetExprC::BinOp { op: _op, lhs, rhs } => {
                self.collect_a(lhs);
                self.collect_a(rhs);
            }
            LetExprC::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                self.collect_a(condition);
                self.collect(branch_success);
                self.collect(branch_failure);
            }
            LetExprC::Fun(f) => {
                for x in &f.free_names {
                    self.freevars.insert(x);
                }
            }
        }
    }

    fn collect_a(&mut self, expr: &'a LetExprA) {
        self.freevars.insert(&expr.var_name);
    }

    fn done(self) -> HashSet<&'a str> {
        self.freevars
    }
}
