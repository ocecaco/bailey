use crate::ir_let::free_vars::FreeVars;
use crate::ir_let::let_expr::{
    AllocClosure, Assignment, Block, Control, Definition, Function, Instruction, Program, Simple,
    Step, TargetAddress, VariableReference,
};
use crate::lang::syntax::Expr;
use crate::result::Result;
use std::collections::HashMap;

struct LetNormalizer {
    program: Program,
    current_function_index: Option<usize>,
    current_block_index: Option<usize>,
    var_counter: u64,
    var_substitution: HashMap<String, String>,
}

impl LetNormalizer {
    fn new() -> Self {
        LetNormalizer {
            program: Program {
                functions: Vec::new(),
            },
            current_function_index: None,
            current_block_index: None,
            var_counter: 0,
            var_substitution: HashMap::new(),
        }
    }

    // TODO: Implement more efficient/less hacky variable generation (probably
    // want to do interning anyway instead of having String all over the place).
    fn fresh(&mut self, base_name: &str) -> String {
        let count = self.var_counter;
        self.var_counter += 1;
        base_name.to_owned() + "__" + &count.to_string()
    }

    fn with_substitution<F, R>(&mut self, from: String, to: String, f: F) -> R
    where
        F: FnOnce(&mut LetNormalizer) -> R,
    {
        let old_substitution = self.var_substitution.remove(&from);
        self.var_substitution.insert(from.clone(), to);

        let result = f(self);

        if let Some(old_to) = old_substitution {
            self.var_substitution.insert(from, old_to);
        } else {
            self.var_substitution.remove(&from);
        }

        result
    }

    fn with_substitutions<F, R>(
        &mut self,
        mut reverse_substitutions: Vec<(String, String)>,
        f: F,
    ) -> R
    where
        F: FnOnce(&mut LetNormalizer) -> R,
    {
        if let Some((from, to)) = reverse_substitutions.pop() {
            self.with_substitution(from, to, |comp| {
                comp.with_substitutions(reverse_substitutions, f)
            })
        } else {
            f(self)
        }
    }

    fn emit(&mut self, instruction: Instruction) {
        let current_function_index = self
            .current_function_index
            .expect("should have active function");
        let current_block_index = self.current_block_index.expect("should have active block");
        self.program.functions[current_function_index].blocks[current_block_index]
            .instructions
            .push(instruction);
    }

    fn normalize_var(&mut self, e: &Expr) -> Result<VariableReference> {
        let norm_rhs = self.normalize_rhs(e)?;

        match norm_rhs {
            Definition::Var(expr_at) => Ok(expr_at),
            Definition::Step(step) => {
                let var_name = self.fresh("__gen");
                self.emit(Instruction::Assignment(Assignment {
                    name: var_name.clone(),
                    definition: Definition::Step(step),
                }));
                Ok(VariableReference { var_name })
            }
        }
    }

    fn normalize_function_body(
        &mut self,
        name: String,
        arg_names: Vec<String>,
        e: &Expr,
    ) -> Result<AllocClosure> {
        let old_function_index = self.current_function_index;
        let new_function_index = self.program.functions.len();
        self.program.functions.push(Function {
            name: name.clone(),
            arg_names: arg_names.clone(),
            free_names: None,
            blocks: Vec::new(),
        });
        self.current_function_index = Some(new_function_index);
        let old_block_index = self.current_block_index;
        self.current_block_index = None;

        let body_address = self.normalize_block(e)?;

        let freevars: Vec<String> = FreeVars::free_vars_function(
            &self.program.functions[new_function_index].blocks,
            &name,
            &arg_names,
            body_address.block_index,
        )
        .iter()
        .map(|&x| x.to_owned())
        .collect();

        self.program.functions[new_function_index].free_names = Some(freevars.clone());

        let function = AllocClosure {
            name,
            arg_names,
            free_names: freevars,
            body: body_address,
        };

        self.current_function_index = old_function_index;
        self.current_block_index = old_block_index;

        Ok(function)
    }

    fn normalize_rhs(&mut self, e: &Expr) -> Result<Definition> {
        match e {
            Expr::Literal(c) => Ok(Definition::Step(Step::Simple(Simple::Literal(*c)))),
            Expr::Var { var_name } => Ok(Definition::Var(VariableReference {
                var_name: self
                    .var_substitution
                    .get(var_name)
                    .expect("could not find substitution")
                    .clone(),
            })),
            Expr::Fun {
                name: original_name,
                arg_names: original_arg_names,
                body,
            } => {
                let unique_name = self.fresh(original_name);

                let mut arg_substitutions = Vec::new();
                let mut unique_arg_names = Vec::new();
                for original_arg_name in original_arg_names.iter().rev() {
                    let unique_arg_name = self.fresh(original_arg_name);
                    arg_substitutions.push((original_arg_name.clone(), unique_arg_name.clone()));
                    unique_arg_names.push(unique_arg_name);
                }
                unique_arg_names.reverse();

                let function = self.with_substitutions(arg_substitutions, |comp| {
                    comp.with_substitution(original_name.clone(), unique_name.clone(), |comp| {
                        comp.normalize_function_body(
                            unique_name.clone(),
                            unique_arg_names.clone(),
                            body,
                        )
                    })
                })?;

                Ok(Definition::Step(Step::Simple(Simple::Fun(function))))
            }
            Expr::Call { func, args } => {
                let fun_at = self.normalize_var(func)?;
                let mut args_at = Vec::new();
                for arg in args {
                    args_at.push(self.normalize_var(arg)?);
                }
                Ok(Definition::Step(Step::Control(Control::Call {
                    func: fun_at,
                    args: args_at,
                })))
            }
            Expr::BinOp { op, lhs, rhs } => {
                let lhs_at = self.normalize_var(lhs)?;
                let rhs_at = self.normalize_var(rhs)?;
                Ok(Definition::Step(Step::Simple(Simple::BinOp {
                    op: *op,
                    lhs: lhs_at,
                    rhs: rhs_at,
                })))
            }
            Expr::Let {
                name: original_name,
                definition,
                body,
            } => {
                let def_c = self.normalize_rhs(definition)?;
                let unique_name = self.fresh(original_name);
                self.emit(Instruction::Assignment(Assignment {
                    name: unique_name.clone(),
                    definition: def_c,
                }));

                self.with_substitution(original_name.clone(), unique_name, |comp| {
                    comp.normalize_rhs(body)
                })
            }
            Expr::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                let cond_at = self.normalize_var(condition)?;
                let branch_success = self.normalize_block(branch_success)?;
                let branch_failure = self.normalize_block(branch_failure)?;
                Ok(Definition::Step(Step::Control(Control::If {
                    condition: cond_at,
                    branch_success,
                    branch_failure,
                })))
            }
            Expr::Tuple { values } => {
                let mut args_norm = Vec::new();

                for arg in values {
                    args_norm.push(self.normalize_var(arg)?);
                }

                Ok(Definition::Step(Step::Simple(Simple::Tuple {
                    args: args_norm,
                })))
            }
            Expr::Set {
                tuple,
                index,
                new_expr,
            } => {
                let tuple_at = self.normalize_var(tuple)?;
                let new_at = self.normalize_var(new_expr)?;
                Ok(Definition::Step(Step::Simple(Simple::Set {
                    tuple: tuple_at,
                    index: *index,
                    new_value: new_at,
                })))
            }
        }
    }

    fn normalize_block(&mut self, e: &Expr) -> Result<TargetAddress> {
        let current_function_index = self
            .current_function_index
            .expect("should have active function");

        let new_block_index = self.program.functions[current_function_index].blocks.len();
        self.program.functions[current_function_index]
            .blocks
            .push(Block {
                instructions: Vec::new(),
                parent_block_index: self.current_block_index,
            });

        // Save the current block index so we can restore it later.
        let old_block_index = self.current_block_index;
        self.current_block_index = Some(new_block_index);

        self.emit(Instruction::EnterBlock);
        let result = self.normalize_var(e)?;
        self.emit(Instruction::ExitBlock(result));

        // Restore the old current block index
        self.current_block_index = old_block_index;

        Ok(TargetAddress {
            function_index: current_function_index,
            block_index: new_block_index,
            instruction_index: 0,
        })
    }

    fn normalize_program(mut self, e: &Expr) -> Result<Program> {
        self.normalize_function_body("toplevel".to_owned(), vec![], e)?;
        Ok(self.program)
    }
}

pub fn let_normalize(e: &Expr) -> Result<Program> {
    let normalizer = LetNormalizer::new();
    normalizer.normalize_program(e)
}
