use crate::ir_let::free_vars::FreeVars;
use crate::ir_let::let_expr::{
    Assignment, Block, Control, Definition, Function, Instruction, Program, Simple, Step,
    TargetAddress, VariableReference,
};
use crate::lang::syntax::Expr;
use crate::result::Result;

struct LetNormalizer {
    program: Program,
    current_block_index: Option<usize>,
    var_counter: u64,
}

impl LetNormalizer {
    fn new() -> Self {
        LetNormalizer {
            program: Program { blocks: Vec::new() },
            current_block_index: None,
            var_counter: 0,
        }
    }

    // TODO: Implement more efficient/less hacky variable generation (probably
    // want to do interning anyway instead of having String all over the place).
    fn fresh(&mut self) -> String {
        let x = String::from("__gen");
        let count = self.var_counter;
        self.var_counter += 1;
        x + &count.to_string()
    }

    fn emit(&mut self, instruction: Instruction) {
        let current_block_index = self.current_block_index.expect("should have active block");
        self.program.blocks[current_block_index]
            .instructions
            .push(instruction);
    }

    fn normalize_atom(&mut self, e: &Expr) -> Result<VariableReference> {
        let norm_rhs = self.normalize_rhs(e)?;

        match norm_rhs {
            Definition::Var(expr_at) => Ok(expr_at),
            Definition::Step(step) => {
                let var_name = self.fresh();
                self.emit(Instruction::Assignment(Assignment {
                    name: var_name.clone(),
                    definition: Definition::Step(step),
                }));
                Ok(VariableReference { var_name })
            }
        }
    }

    fn normalize_rhs(&mut self, e: &Expr) -> Result<Definition> {
        match e {
            Expr::Literal(c) => Ok(Definition::Step(Step::Simple(Simple::Literal(*c)))),
            Expr::Var { var_name } => Ok(Definition::Var(VariableReference {
                var_name: var_name.clone(),
            })),
            Expr::Fun {
                name,
                arg_names,
                body,
            } => {
                let body_address = self.normalize_block(&body)?;
                let freevars =
                    FreeVars::free_vars_function(&self.program, &name, &arg_names, body_address);
                let function = Function {
                    name: name.clone(),
                    arg_names: arg_names.clone(),
                    free_names: freevars.iter().map(|&x| x.to_owned()).collect(),
                    body: body_address,
                };

                Ok(Definition::Step(Step::Simple(Simple::Fun(function))))
            }
            Expr::Call { func, args } => {
                let fun_at = self.normalize_atom(func)?;
                let mut args_at = Vec::new();
                for arg in args {
                    args_at.push(self.normalize_atom(arg)?);
                }
                Ok(Definition::Step(Step::Control(Control::Call {
                    func: fun_at,
                    args: args_at,
                })))
            }
            Expr::BinOp { op, lhs, rhs } => {
                let lhs_at = self.normalize_atom(lhs)?;
                let rhs_at = self.normalize_atom(rhs)?;
                Ok(Definition::Step(Step::Simple(Simple::BinOp {
                    op: *op,
                    lhs: lhs_at,
                    rhs: rhs_at,
                })))
            }
            Expr::Let {
                name,
                definition,
                body,
            } => {
                let def_c = self.normalize_rhs(definition)?;
                self.emit(Instruction::Assignment(Assignment {
                    name: name.clone(),
                    definition: def_c,
                }));
                self.normalize_rhs(body)
            }
            Expr::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                let cond_at = self.normalize_atom(condition)?;
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
                    args_norm.push(self.normalize_atom(arg)?);
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
                let tuple_at = self.normalize_atom(tuple)?;
                let new_at = self.normalize_atom(new_expr)?;
                Ok(Definition::Step(Step::Simple(Simple::Set {
                    tuple: tuple_at,
                    index: *index,
                    new_value: new_at,
                })))
            }
        }
    }

    fn normalize_block(&mut self, e: &Expr) -> Result<TargetAddress> {
        let new_block_index = self.program.blocks.len();
        self.program.blocks.push(Block {
            instructions: Vec::new(),
        });
        // Save the current block index so we can restore it later.
        let old_block_index = self.current_block_index;
        self.current_block_index = Some(new_block_index);

        self.emit(Instruction::EnterBlock);
        let result = self.normalize_atom(e)?;
        self.emit(Instruction::ExitBlock(result));

        // Restore the old current block index
        self.current_block_index = old_block_index;

        Ok(TargetAddress {
            block_index: new_block_index,
            instruction_index: 0,
        })
    }

    fn normalize_program(mut self, e: &Expr) -> Result<Program> {
        self.normalize_block(e)?;
        Ok(self.program)
    }
}

pub fn let_normalize(e: &Expr) -> Result<Program> {
    let normalizer = LetNormalizer::new();
    normalizer.normalize_program(e)
}
