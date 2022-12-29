use crate::ir_let::interpreter::heap::Heap;
use crate::ir_let::interpreter::heap_value::{Closure, HeapAddress, HeapValue, Tuple};
use crate::ir_let::interpreter::stack::{ReturnInfo, Stack};
use crate::ir_let::let_expr::{
    Assignment, Control, Definition, Function, Instruction, Program, Simple, Step, TargetAddress,
    VariableReference,
};
use crate::lang::syntax::{BinOp, Constant};
use std::collections::HashMap;

#[derive(Debug)]
struct InstructionEvaluator {
    heap: Heap,
    stack: Stack,
}

impl InstructionEvaluator {
    fn new() -> Self {
        InstructionEvaluator {
            heap: Heap::new(),
            stack: Stack::new(),
        }
    }

    fn set_var(&mut self, name: String, address: HeapAddress) {
        self.heap.inc_refcount(address);
        self.stack.set_var_no_refcount(name, address);
    }

    fn eval_binop(
        &mut self,
        op: BinOp,
        lhs_addr: HeapAddress,
        rhs_addr: HeapAddress,
    ) -> HeapAddress {
        match op {
            BinOp::Add => {
                let lhs_value = self.heap.deref(lhs_addr).check_int();
                let rhs_value = self.heap.deref(rhs_addr).check_int();
                self.heap.alloc(HeapValue::Int(lhs_value + rhs_value))
            }
            BinOp::Sub => {
                let lhs_value = self.heap.deref(lhs_addr).check_int();
                let rhs_value = self.heap.deref(rhs_addr).check_int();
                self.heap.alloc(HeapValue::Int(lhs_value - rhs_value))
            }
            BinOp::Eq => {
                let lhs_value = self.heap.deref(lhs_addr).check_int();
                let rhs_value = self.heap.deref(rhs_addr).check_int();
                self.heap.alloc(HeapValue::Bool(lhs_value == rhs_value))
            }
            BinOp::Get => {
                let tuple = self.heap.deref(lhs_addr).check_tuple();
                let index = self.heap.deref(rhs_addr).check_int();

                match tuple.field_values.get(index as usize) {
                    Some(value) => *value,
                    None => panic!("field index out of range"),
                }
            }
        }
    }

    fn eval_var(&mut self, e: &VariableReference) -> HeapAddress {
        self.stack.lookup_var(&e.var_name)
    }

    fn eval_simple(&mut self, e: &Simple) -> HeapAddress {
        match e {
            Simple::Literal(Constant::Int { value }) => self.heap.alloc(HeapValue::Int(*value)),
            Simple::Literal(Constant::Bool { value }) => self.heap.alloc(HeapValue::Bool(*value)),
            Simple::Tuple { args } => {
                let mut field_values = Vec::new();

                for arg in args {
                    let value_addr = self.eval_var(arg);
                    field_values.push(value_addr);
                }

                for addr in &field_values {
                    self.heap.inc_refcount(*addr);
                }

                self.heap.alloc(HeapValue::Tuple(Tuple { field_values }))
            }
            Simple::Fun(Function {
                name,
                arg_names,
                free_names,
                body,
            }) => {
                let mut closure_environment = HashMap::new();

                for free_name in free_names {
                    let value_addr = self.stack.lookup_var(free_name);

                    closure_environment.insert(free_name.clone(), value_addr);
                }

                for value_addr in closure_environment.values() {
                    self.heap.inc_refcount(*value_addr);
                }

                self.heap.alloc(HeapValue::Closure(Closure {
                    name: name.clone(),
                    arg_names: arg_names.clone(),
                    environment: closure_environment,
                    body: *body,
                }))
            }
            Simple::BinOp { op, lhs, rhs } => {
                let lhs_address = self.eval_var(lhs);
                let rhs_address = self.eval_var(rhs);
                self.eval_binop(*op, lhs_address, rhs_address)
            }
            Simple::Set {
                tuple,
                index,
                new_value,
            } => {
                let tuple_address = self.eval_var(tuple);
                let new_value = self.eval_var(new_value);

                let tuple = self.heap.deref_mut(tuple_address).check_tuple_mut();

                if (*index as usize) < tuple.field_values.len() {
                    let old_value = tuple.field_values[*index as usize];
                    tuple.field_values[*index as usize] = new_value;

                    // Ordering is important here, because in case new_value == old_value we do
                    // not want to destroy the value we are assigning, as would happen when we swap the lines.
                    self.heap.inc_refcount(new_value);
                    self.heap.dec_refcount(old_value);
                } else {
                    panic!("tuple index out of range during mutation");
                }

                self.heap.alloc(HeapValue::Tuple(Tuple {
                    field_values: Vec::new(),
                }))
            }
        }
    }

    fn eval_control(&mut self, control: &Control, return_info: ReturnInfo) -> TargetAddress {
        match control {
            Control::Call { func, args } => {
                let closure_address = self.eval_var(func);

                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.eval_var(arg));
                }

                let closure = self.heap.deref(closure_address).check_closure().clone();

                if closure.arg_names.len() != args.len() {
                    panic!("incorrect number of arguments");
                }

                self.stack.enter_function(return_info);

                for (name, value) in closure.environment.iter() {
                    self.set_var(name.clone(), *value);
                }

                for (name, arg_value) in closure.arg_names.iter().zip(arg_values) {
                    self.set_var(name.clone(), arg_value);
                }

                // Allow the function to recursively calling itself by inserting
                // a pointer to its own closure into its environment when
                // calling it.
                self.set_var(closure.name.clone(), closure_address);

                closure.body
            }
            Control::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                let condition_address = self.eval_var(condition);
                let condition_value = self.heap.deref(condition_address).check_bool();

                if condition_value {
                    *branch_success
                } else {
                    *branch_failure
                }
            }
        }
    }

    fn eval_instruction(
        &mut self,
        address: TargetAddress,
        instruction: &Assignment,
    ) -> TargetAddress {
        match &instruction.definition {
            Definition::Var(var) => {
                let value = self.eval_var(&var);
                self.set_var(instruction.name.clone(), value);
                address.next()
            }
            Definition::Step(Step::Simple(simple)) => {
                let value = self.eval_simple(&simple);
                self.set_var(instruction.name.clone(), value);
                address.next()
            }
            Definition::Step(Step::Control(control)) => {
                let return_info = ReturnInfo {
                    result_variable: instruction.name.clone(),
                    return_address: address.next(),
                };
                self.eval_control(&control, return_info)
            }
        }
    }
}

#[derive(Debug)]
pub struct ProgramEvaluator {
    program: Program,
    instruction_evaluator: InstructionEvaluator,
    program_counter: TargetAddress,
}

impl ProgramEvaluator {
    pub fn new(program: Program) -> Self {
        ProgramEvaluator {
            program,
            instruction_evaluator: InstructionEvaluator::new(),
            program_counter: TargetAddress {
                block_index: 0,
                instruction_index: 0,
            },
        }
    }

    pub fn run(&mut self) -> HeapValue {
        loop {
            let result = self.step();

            if let Some(result) = result {
                return result;
            }
        }
    }

    fn step(&mut self) -> Option<HeapValue> {
        println!("PC: {:?}", self.program_counter);

        let current_instruction = self.program.get_instruction(self.program_counter);

        println!("instruction: {:?}", current_instruction);

        match current_instruction {
            Instruction::EnterBlock => {
                self.program_counter = self.program_counter.next();
                None
            }
            Instruction::ExitBlock(return_var) => {
                // If there is no return address, the program is finished and we
                // can return the final value from this function.
                let block = self.instruction_evaluator.stack.exit_block();

                let return_value = block
                    .lookup_var(&return_var.var_name)
                    .expect("could not find return value of block in block local variables");

                // TODO: Some code duplication here
                match block.return_info {
                    None => {
                        let result = self.instruction_evaluator.heap.deref(return_value).clone();

                        // Decrease reference counts on the locals that are
                        // going out of scope. In the current implementation,
                        // this can only happen after we have assigned the
                        // return value into the caller stack frame, since doing
                        // that will increment the reference count, keeping the
                        // return value alive instead of potentially destroying
                        // it at the block exit.
                        for address in &block.values {
                            self.instruction_evaluator.heap.dec_refcount(*address);
                        }

                        Some(result)
                    }
                    Some(return_info) => {
                        // Put the return value into the caller's stack frame.
                        self.instruction_evaluator
                            .set_var(return_info.result_variable, return_value);

                        // Decrease reference counts on the locals that are
                        // going out of scope. In the current implementation,
                        // this can only happen after we have assigned the
                        // return value into the caller stack frame, since doing
                        // that will increment the reference count, keeping the
                        // return value alive instead of potentially destroying
                        // it at the block exit.
                        for address in &block.values {
                            self.instruction_evaluator.heap.dec_refcount(*address);
                        }

                        self.program_counter = return_info.return_address;
                        None
                    }
                }
            }
            Instruction::Assignment(assignment) => {
                let next_address = self
                    .instruction_evaluator
                    .eval_instruction(self.program_counter, assignment);
                self.program_counter = next_address;
                None
            }
        }
    }
}
