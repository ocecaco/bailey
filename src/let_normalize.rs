use crate::let_expr::{FreeVars, LetBinding, LetExpr, LetExprA, LetExprB, LetExprC, LetFunction};
use crate::result::Result;
use crate::syntax::Expr;

struct LetNormalizer {
    let_context: Vec<LetBinding>,
    varcounter: u64,
}

impl LetNormalizer {
    fn new() -> Self {
        LetNormalizer {
            let_context: Vec::new(),
            varcounter: 0,
        }
    }

    // TODO: Implement more efficient/less hacky variable generation (probably
    // want to do interning anyway instead of having String all over the place).
    fn fresh(&mut self) -> String {
        let x = String::from("__gen");
        let count = self.varcounter;
        self.varcounter += 1;
        x + &count.to_string()
    }

    fn normalize_atom(&mut self, e: &Expr) -> Result<LetExprA> {
        let norm_rhs = self.normalize_rhs(e)?;

        match norm_rhs {
            LetExprB::Atomic(expr_at) => Ok(expr_at),
            LetExprB::Complex(expr_c) => {
                let var_name = self.fresh();
                self.let_context.push(LetBinding {
                    name: var_name.clone(),
                    definition: LetExprB::Complex(expr_c),
                });
                Ok(LetExprA { var_name })
            }
        }
    }

    fn normalize_rhs(&mut self, e: &Expr) -> Result<LetExprB> {
        match e {
            Expr::Literal(c) => Ok(LetExprB::Complex(LetExprC::Literal(*c))),
            Expr::Var { var_name } => Ok(LetExprB::Atomic(LetExprA {
                var_name: var_name.clone(),
            })),
            Expr::Fun {
                name,
                arg_names,
                body,
            } => {
                // TODO: Is it a bug that we create a new normalizer here and
                // hence "reset" the variable generation counter? Same question for let normalization
                // of the branches of the if.
                let body_norm = let_normalize(&body)?;
                let freevars = FreeVars::freevars_function(&name, &arg_names, &body_norm);
                let function = LetFunction {
                    name: name.clone(),
                    arg_names: arg_names.clone(),
                    free_names: freevars.iter().map(|&x| x.to_owned()).collect(),
                    body: Box::new(body_norm),
                };

                Ok(LetExprB::Complex(LetExprC::Fun(function)))
            }
            Expr::Call { func, args } => {
                let fun_at = self.normalize_atom(func)?;
                let mut args_at = Vec::new();
                for arg in args {
                    args_at.push(self.normalize_atom(arg)?);
                }
                Ok(LetExprB::Complex(LetExprC::Call {
                    func: fun_at,
                    args: args_at,
                }))
            }
            Expr::BinOp { op, lhs, rhs } => {
                let lhs_at = self.normalize_atom(lhs)?;
                let rhs_at = self.normalize_atom(rhs)?;
                Ok(LetExprB::Complex(LetExprC::BinOp {
                    op: *op,
                    lhs: lhs_at,
                    rhs: rhs_at,
                }))
            }
            Expr::Let {
                name,
                definition,
                body,
            } => {
                let def_c = self.normalize_rhs(definition)?;
                self.let_context.push(LetBinding {
                    name: name.clone(),
                    definition: def_c,
                });
                self.normalize_rhs(body)
            }
            Expr::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                let cond_at = self.normalize_atom(condition)?;
                let branch_success = let_normalize(branch_success)?;
                let branch_failure = let_normalize(branch_failure)?;
                Ok(LetExprB::Complex(LetExprC::If {
                    condition: cond_at,
                    branch_success: Box::new(branch_success),
                    branch_failure: Box::new(branch_failure),
                }))
            }
            Expr::Tuple { values } => {
                let mut args_norm = Vec::new();

                for arg in values {
                    args_norm.push(self.normalize_atom(arg)?);
                }

                Ok(LetExprB::Complex(LetExprC::Tuple { args: args_norm }))
            }
            Expr::Set {
                tuple,
                index,
                new_expr,
            } => {
                let tuple_at = self.normalize_atom(tuple)?;
                let new_at = self.normalize_atom(new_expr)?;
                Ok(LetExprB::Complex(LetExprC::Set {
                    tuple: tuple_at,
                    index: *index,
                    new_value: new_at,
                }))
            }
        }
    }

    // This consumes the normalizer because we don't want to have the same
    // normalizer accidentally be used twice: it's only meant to be used to
    // construct a single "block" of let-bindings.
    fn normalize(mut self, e: &Expr) -> Result<LetExpr> {
        let e_rhs = self.normalize_rhs(e)?;

        let var_name = self.fresh();

        let mut let_bindings = self.let_context;
        let_bindings.push(LetBinding {
            name: var_name,
            definition: e_rhs,
        });

        Ok(LetExpr { let_bindings })
    }
}

pub fn let_normalize(e: &Expr) -> Result<LetExpr> {
    let normalizer = LetNormalizer::new();
    normalizer.normalize(e)
}
