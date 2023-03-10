use crate::ir_let::let_expr::{
    Block, Control, Definition, Instruction, Simple, Step, VariableReference,
};
use std::collections::HashSet;

// TODO: I could add some asserts to check that there are no invalid
// interprocedural jumps in conditionals and that calls/jumps always go to the
// first instruction of a block.
pub struct FreeVars<'a> {
    function_blocks: &'a [Block],
    free_vars: HashSet<&'a str>,
}

impl<'a> FreeVars<'a> {
    pub fn free_vars_function(
        function_blocks: &'a [Block],
        funname: &'a str,
        argnames: &'a [String],
        initial_block_index: usize,
    ) -> HashSet<&'a str> {
        let mut collector = FreeVars::new(function_blocks);
        collector.collect_function(funname, argnames, initial_block_index);
        collector.done()
    }

    fn new(function_blocks: &'a [Block]) -> Self {
        FreeVars {
            function_blocks,
            free_vars: HashSet::new(),
        }
    }

    fn collect_block(&mut self, block_index: usize) {
        let block = &self.function_blocks[block_index];

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

    fn collect_function(
        &mut self,
        funname: &'a str,
        argnames: &'a [String],
        initial_block_index: usize,
    ) {
        self.collect_block(initial_block_index);

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
                self.collect_block(branch_success.block_index);
                self.collect_block(branch_failure.block_index);
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
