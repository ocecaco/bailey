use std::collections::HashSet;

use crate::syntax::{BinOp, Constant};

#[derive(Debug, Clone)]
pub struct Program {
    pub blocks: Vec<Block>,
}

impl Program {
    pub fn get_instruction(&self, address: TargetAddress) -> &Instruction {
        let block = self.blocks.get(address.block_index).expect("invalid block");
        block
            .instructions
            .get(address.instruction_index)
            .expect("invalid instruction index")
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    // The last instruction represents the result of evaluating the
    // entire sequence of instructions.
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    EnterBlock,
    ExitBlock(VariableReference),
    Assignment(Assignment),
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: String,
    pub definition: Definition,
}

#[derive(Debug, Clone)]
pub enum Definition {
    Var(VariableReference),
    Step(Step),
}

#[derive(Debug, Copy, Clone)]
pub struct TargetAddress {
    pub block_index: usize,
    pub instruction_index: usize,
}

impl TargetAddress {
    pub fn next(&self) -> TargetAddress {
        TargetAddress {
            block_index: self.block_index,
            instruction_index: self.instruction_index + 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub arg_names: Vec<String>,
    pub free_names: Vec<String>,
    pub body: TargetAddress,
}

#[derive(Debug, Clone)]
pub enum Simple {
    Literal(Constant),
    Fun(Function),
    BinOp {
        op: BinOp,
        lhs: VariableReference,
        rhs: VariableReference,
    },
    Tuple {
        args: Vec<VariableReference>,
    },
    Set {
        tuple: VariableReference,
        index: u32,
        new_value: VariableReference,
    },
}

#[derive(Debug, Clone)]
pub enum Control {
    Call {
        func: VariableReference,
        args: Vec<VariableReference>,
    },
    If {
        condition: VariableReference,
        branch_success: TargetAddress,
        branch_failure: TargetAddress,
    },
}

#[derive(Debug, Clone)]
pub enum Step {
    Simple(Simple),
    Control(Control),
}

#[derive(Debug, Clone)]
pub struct VariableReference {
    pub var_name: String,
}

// Free variable determination
pub struct FreeVars<'a> {
    program: &'a Program,
    free_vars: HashSet<&'a str>,
}

impl<'a> FreeVars<'a> {
    pub fn free_vars_function(
        program: &'a Program,
        funname: &'a str,
        argnames: &'a [String],
        body: TargetAddress,
    ) -> HashSet<&'a str> {
        let mut collector = FreeVars::new(program);
        collector.collect_function(funname, argnames, body);
        collector.done()
    }

    fn new(program: &'a Program) -> Self {
        FreeVars {
            program,
            free_vars: HashSet::new(),
        }
    }

    fn collect_block(&mut self, block_address: TargetAddress) {
        let block = &self.program.blocks[block_address.block_index];

        // We iterate in reverse in order to determine the free variables of the
        // inner (nested) scopes first, because the first let binding scopes
        // over the entirety of the remaining let bindings.
        for instruction in block.instructions.iter().rev() {
            match instruction {
                Instruction::EnterBlock => {}
                Instruction::ExitBlock(return_var) => {
                    self.collect_var(return_var);
                }
                Instruction::Assignment(assignment) => {
                    // The ordering of these two lines is important: the name of the let
                    // binding does NOT scope over its right-hand side, and therefore it
                    // should not be removed after processing the definition.
                    self.free_vars.remove(assignment.name.as_str());
                    self.collect_definition(&assignment.definition);
                }
            }
        }
    }

    fn collect_definition(&mut self, expr: &'a Definition) {
        match expr {
            Definition::Var(var_ref) => self.collect_var(var_ref),
            Definition::Step(Step::Simple(e)) => self.collect_simple(e),
            Definition::Step(Step::Control(e)) => self.collect_control(e),
        }
    }

    fn collect_function(&mut self, funname: &'a str, argnames: &'a [String], body: TargetAddress) {
        self.collect_block(body);

        self.free_vars.remove(funname);

        for argname in argnames.iter() {
            self.free_vars.remove(argname as &'a str);
        }
    }

    fn collect_control(&mut self, expr: &'a Control) {
        match expr {
            Control::Call { func, args } => {
                self.collect_var(func);
                for arg in args {
                    self.collect_var(arg);
                }
            }
            Control::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                self.collect_var(condition);
                self.collect_block(*branch_success);
                self.collect_block(*branch_failure);
            }
        }
    }

    fn collect_simple(&mut self, expr: &'a Simple) {
        match expr {
            Simple::Literal(_) => {}
            Simple::Tuple { args } => {
                for arg in args {
                    self.collect_var(arg);
                }
            }
            Simple::Set {
                tuple,
                index: _index,
                new_value,
            } => {
                self.collect_var(tuple);
                self.collect_var(new_value);
            }
            Simple::BinOp { op: _op, lhs, rhs } => {
                self.collect_var(lhs);
                self.collect_var(rhs);
            }
            Simple::Fun(f) => {
                for x in &f.free_names {
                    self.free_vars.insert(x);
                }
            }
        }
    }

    fn collect_var(&mut self, expr: &'a VariableReference) {
        self.free_vars.insert(&expr.var_name);
    }

    fn done(self) -> HashSet<&'a str> {
        self.free_vars
    }
}
