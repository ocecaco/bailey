use crate::ir_let::free_vars::FreeVars;
use crate::ir_let::let_expr::{
    AllocClosure, Assignment, Block, Control, Definition, Function, Instruction, Program, Simple,
    Step, TargetAddress, VariableReference,
};
use crate::lang::syntax::Expr;
use crate::result::Result;

struct LetNormalizer {
    program: Program,
    current_function_index: Option<usize>,
    current_block_index: Option<usize>,
    var_counter: u64,
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
                let var_name = self.fresh();
                self.emit(Instruction::Assignment(Assignment {
                    name: var_name.clone(),
                    definition: Definition::Step(step),
                }));
                Ok(VariableReference { var_name })
            }
        }
    }

    fn normalize_function_body(&mut self, e: &Expr) -> Result<TargetAddress> {
        let old_function_index = self.current_function_index;
        let new_function_index = self.program.functions.len();
        self.program.functions.push(Function { blocks: Vec::new() });
        self.current_function_index = Some(new_function_index);
        let old_block_index = self.current_block_index;
        self.current_block_index = None;

        let body_address = self.normalize_block(e)?;

        self.current_function_index = old_function_index;
        self.current_block_index = old_block_index;

        Ok(body_address)
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
                let function_body_address = self.normalize_function_body(body)?;

                let freevars = FreeVars::free_vars_function(
                    &self.program.functions[function_body_address.function_index].blocks,
                    &name,
                    &arg_names,
                    function_body_address.block_index,
                );
                let function = AllocClosure {
                    name: name.clone(),
                    arg_names: arg_names.clone(),
                    free_names: freevars.iter().map(|&x| x.to_owned()).collect(),
                    body: function_body_address,
                };

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
        self.normalize_function_body(e)?;
        Ok(self.program)
    }
}

pub fn let_normalize(e: &Expr) -> Result<Program> {
    let normalizer = LetNormalizer::new();
    normalizer.normalize_program(e)
}
