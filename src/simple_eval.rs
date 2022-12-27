use crate::let_expr::{LetExpr, LetExprA, LetExprB, LetExprC, LetFunction};
use crate::syntax::{BinOp, Constant};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct HeapAddress(u32);

// Using Strings everywhere it likely not very efficient (as opposed to interning
// or using offsets into stack frames), but this is just a proof-of-concept simple
// implementation.

#[derive(Clone)]
struct Closure {
    name: String,
    arg_names: Vec<String>,
    environment: HashMap<String, HeapAddress>,
    body: LetExpr,
}

struct Tuple {
    field_values: Vec<HeapAddress>,
}

enum HeapValue {
    Int(i32),
    Bool(bool),
    Tuple(Tuple),
    Closure(Closure),
}

impl HeapValue {
    fn check_closure(&self) -> &Closure {
        match self {
            HeapValue::Closure(clos) => clos,
            _ => panic!("expected closure"),
        }
    }

    fn check_int(&self) -> i32 {
        match self {
            HeapValue::Int(value) => *value,
            _ => panic!("expected int"),
        }
    }

    fn check_bool(&self) -> bool {
        match self {
            HeapValue::Bool(value) => *value,
            _ => panic!("expected bool"),
        }
    }

    fn check_tuple(&self) -> &Tuple {
        match self {
            HeapValue::Tuple(tuple) => tuple,
            _ => panic!("expected tuple"),
        }
    }

    fn check_tuple_mut(&mut self) -> &mut Tuple {
        match self {
            HeapValue::Tuple(tuple) => tuple,
            _ => panic!("expected tuple"),
        }
    }
}

struct RefCountedHeapValue {
    refcount: u32,
    heap_value: HeapValue,
}

struct Heap {
    memory: HashMap<HeapAddress, RefCountedHeapValue>,
    heap_next_address: HeapAddress,
}

impl Heap {
    fn new() -> Self {
        Heap {
            memory: HashMap::new(),
            heap_next_address: HeapAddress(0),
        }
    }

    fn alloc(&mut self, heap_value: HeapValue) -> HeapAddress {
        let address = self.heap_next_address;
        self.heap_next_address = HeapAddress(self.heap_next_address.0 + 1);
        let refcounted = RefCountedHeapValue {
            refcount: 0,
            heap_value,
        };
        self.memory.insert(address, refcounted);
        address
    }

    fn free(&mut self, heap_address: HeapAddress) {
        let destroying_value = self
            .memory
            .remove(&heap_address)
            .expect("attempt to free invalid pointer")
            .heap_value;

        match destroying_value {
            HeapValue::Int(_) => {}
            HeapValue::Bool(_) => {}
            HeapValue::Tuple(Tuple { field_values }) => {
                for addr in field_values {
                    self.dec_refcount(addr);
                }
            }
            HeapValue::Closure(Closure { environment, .. }) => {
                for addr in environment.values() {
                    self.dec_refcount(*addr);
                }
            }
        }
    }

    fn free_block_frame(&mut self, block_frame: BlockFrame) {
        for addr in block_frame.values {
            self.dec_refcount(addr);
        }
    }

    // TODO: Alternative, could just ensure that there are no more block frames
    // left when the function exits. Seems like that might be the case anyway in
    // the current implementation (need to check).
    fn free_stack_frame(&mut self, stack_frame: CallStackFrame) {
        for block_frame in stack_frame.nested_block_frames {
            self.free_block_frame(block_frame)
        }
    }

    fn deref(&self, heap_address: HeapAddress) -> &HeapValue {
        &self.memory[&heap_address].heap_value
    }

    fn deref_mut(&mut self, heap_address: HeapAddress) -> &mut HeapValue {
        &mut self
            .memory
            .get_mut(&heap_address)
            .expect("invalid pointer")
            .heap_value
    }

    // TODO: Reference counting is not actively used for now in the
    // interpreter since there was a bug with destruction of intermediate values.
    fn inc_refcount(&mut self, heap_address: HeapAddress) {
        let refcounted = &mut self.memory.get_mut(&heap_address).expect("invalid pointer");
        refcounted.refcount += 1;
    }

    fn dec_refcount(&mut self, heap_address: HeapAddress) {
        let new_refcount = {
            let refcounted = &mut self.memory.get_mut(&heap_address).expect("invalid pointer");
            refcounted.refcount -= 1;
            refcounted.refcount
        };

        if new_refcount == 0 {
            self.free(heap_address);
        }
    }
}

struct BlockFrame {
    values: Vec<HeapAddress>,
    variable_offsets: HashMap<String, usize>,
}

impl BlockFrame {
    fn new() -> Self {
        BlockFrame {
            values: Vec::new(),
            variable_offsets: HashMap::new(),
        }
    }

    fn lookup_var(&self, name: &str) -> Option<HeapAddress> {
        let offset = self.variable_offsets.get(name);

        if let Some(offset) = offset {
            Some(*self.values.get(*offset).expect("stack index out of range"))
        } else {
            None
        }
    }

    fn set_var(&mut self, name: String, value: HeapAddress) {
        let new_offset = self.values.len();
        self.values.push(value);
        self.variable_offsets.insert(name, new_offset);
    }
}

struct CallStackFrame {
    nested_block_frames: Vec<BlockFrame>,
}

impl CallStackFrame {
    fn new() -> Self {
        CallStackFrame {
            nested_block_frames: vec![BlockFrame::new()],
        }
    }

    fn enter_block(&mut self) {
        self.nested_block_frames.push(BlockFrame::new())
    }

    fn exit_block(&mut self) -> BlockFrame {
        self.nested_block_frames
            .pop()
            .expect("exiting block while no more block frames")
    }

    fn current_block_mut(&mut self) -> &mut BlockFrame {
        self.nested_block_frames
            .last_mut()
            .expect("expected active block")
    }

    fn lookup_var(&self, name: &str) -> HeapAddress {
        // Walk backwards from the innermost block frame to the outermost
        // one to find the lexically closest one that binds the variable we are looking for.
        for frame in self.nested_block_frames.iter().rev() {
            if let Some(value) = frame.lookup_var(name) {
                return value;
            }
        }

        panic!("could not find variable in stack frame")
    }

    fn set_var_no_refcount(&mut self, name: String, value: HeapAddress) {
        self.current_block_mut().set_var(name, value);
    }
}

struct Stack {
    frames: Vec<CallStackFrame>,
}

impl Stack {
    fn new() -> Self {
        Stack {
            frames: vec![CallStackFrame::new()],
        }
    }

    fn enter_function(&mut self) {
        self.frames.push(CallStackFrame::new());
    }

    fn exit_function(&mut self) -> CallStackFrame {
        self.frames
            .pop()
            .expect("stack should not be empty during popping")
    }

    fn current_frame_mut(&mut self) -> &mut CallStackFrame {
        self.frames.last_mut().expect("stack should not be empty")
    }

    fn current_frame(&self) -> &CallStackFrame {
        self.frames.last().expect("stack should not be empty")
    }
}

struct SimpleEvaluator {
    heap: Heap,
    stack: Stack,
}

impl SimpleEvaluator {
    fn new() -> Self {
        SimpleEvaluator {
            heap: Heap::new(),
            stack: Stack::new(),
        }
    }

    fn set_var(&mut self, name: String, address: HeapAddress) {
        self.heap.inc_refcount(address);
        self.stack
            .current_frame_mut()
            .set_var_no_refcount(name, address);
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

    fn eval_atomic(&mut self, e: &LetExprA) -> HeapAddress {
        self.stack.current_frame().lookup_var(&e.var_name)
    }

    fn eval_complex(&mut self, e: &LetExprC) -> HeapAddress {
        match e {
            LetExprC::Literal(Constant::Int { value }) => self.heap.alloc(HeapValue::Int(*value)),
            LetExprC::Literal(Constant::Bool { value }) => self.heap.alloc(HeapValue::Bool(*value)),
            LetExprC::Tuple { args } => {
                let mut field_values = Vec::new();

                for arg in args {
                    let value_addr = self.eval_atomic(arg);
                    field_values.push(value_addr);
                }

                for addr in &field_values {
                    self.heap.inc_refcount(*addr);
                }

                self.heap.alloc(HeapValue::Tuple(Tuple { field_values }))
            }
            LetExprC::Fun(LetFunction {
                name,
                arg_names,
                free_names,
                body,
            }) => {
                let mut closure_environment = HashMap::new();

                for free_name in free_names {
                    let value_addr = self.stack.current_frame().lookup_var(free_name);

                    closure_environment.insert(free_name.clone(), value_addr);
                }

                for value_addr in closure_environment.values() {
                    self.heap.inc_refcount(*value_addr);
                }

                self.heap.alloc(HeapValue::Closure(Closure {
                    name: name.clone(),
                    arg_names: arg_names.clone(),
                    environment: closure_environment,
                    body: body.as_ref().clone(),
                }))
            }
            LetExprC::Call { func, args } => {
                let closure_address = self.eval_atomic(func);

                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.eval_atomic(arg));
                }

                // TODO: Cloning the closure is relatively inefficient because
                // it contains a potentially large expression. Doing it anyway
                // for now since I will refactor it later to not store a copy of
                // the expression in the closure but rather a code pointer of
                // some kind.
                let closure = self.heap.deref(closure_address).check_closure().clone();

                if closure.arg_names.len() != args.len() {
                    panic!("incorrect number of arguments");
                }

                self.stack.enter_function();

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

                let result = self.eval_block(&closure.body);

                // TODO: This still uses the host stack, switch to a fully iterative interpreter implementation.
                let frame = self.stack.exit_function();
                self.heap.free_stack_frame(frame);

                result
            }
            LetExprC::BinOp { op, lhs, rhs } => {
                let lhs_address = self.eval_atomic(lhs);
                let rhs_address = self.eval_atomic(rhs);
                self.eval_binop(*op, lhs_address, rhs_address)
            }
            LetExprC::Set {
                tuple,
                index,
                new_value,
            } => {
                let tuple_address = self.eval_atomic(tuple);
                let new_value = self.eval_atomic(new_value);

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
            LetExprC::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                let condition_address = self.eval_atomic(condition);
                let condition_value = self.heap.deref(condition_address).check_bool();

                if condition_value {
                    self.eval_block(branch_success)
                } else {
                    self.eval_block(branch_failure)
                }
            }
        }
    }

    fn eval_rhs(&mut self, e: &LetExprB) -> HeapAddress {
        match e {
            LetExprB::Atomic(e_atomic) => self.eval_atomic(e_atomic),
            LetExprB::Complex(e_complex) => self.eval_complex(e_complex),
        }
    }

    fn eval_block(&mut self, e: &LetExpr) -> HeapAddress {
        self.stack.current_frame_mut().enter_block();

        let (last_instruction, initial_instructions) = e
            .let_bindings
            .split_last()
            .expect("should be at least one instruction");

        for instruction in initial_instructions {
            let new_var_value = self.eval_rhs(&instruction.definition);

            self.set_var(instruction.name.clone(), new_var_value);
        }

        let block_return_value = self.eval_rhs(&last_instruction.definition);
        // We do not assign the block return value to a local variable in the stack frame,
        // so that its reference count does not get decreased when the frame is destroyed,
        // since that would lead to immediate destruction of the result value.

        // TODO: Stop using host stack, use iterative implementation
        let frame = self.stack.current_frame_mut().exit_block();
        self.heap.free_block_frame(frame);

        block_return_value
    }
}
