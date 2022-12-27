use crate::syntax::{BinOp, Constant, Expr};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct HeapAddress(u32);

// TODO: Using Strings everywhere it likely not very efficient, but this is just
// a proof-of-concept simple implementation.
type Environment = HashMap<String, HeapAddress>;

#[derive(Clone)]
struct Closure {
    name: String,
    arg_names: Vec<String>,
    environment: Environment,
    body: Expr,
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
            refcount: 1,
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

    fn free_env(&mut self, env: &Environment) {
        for addr in env.values() {
            self.dec_refcount(*addr);
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

struct SimpleEvaluator {
    heap: Heap,
}

impl SimpleEvaluator {
    fn new() -> Self {
        SimpleEvaluator { heap: Heap::new() }
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

    fn eval(&mut self, env: &Environment, e: &Expr) -> HeapAddress {
        match e {
            Expr::Literal(Constant::Int { value }) => self.heap.alloc(HeapValue::Int(*value)),
            Expr::Literal(Constant::Bool { value }) => self.heap.alloc(HeapValue::Bool(*value)),
            Expr::Tuple { values } => {
                let mut field_values = Vec::new();

                for v in values {
                    let value_addr = self.eval(env, v);
                    field_values.push(value_addr);
                }

                self.heap.alloc(HeapValue::Tuple(Tuple { field_values }))
            }
            Expr::Fun {
                name,
                arg_names,
                body,
            } => {
                self.heap.alloc(HeapValue::Closure(Closure {
                    name: name.clone(),
                    arg_names: arg_names.clone(),
                    // TODO: Restrict the environment to only capture the free
                    // variables of the body to prevent memory leaks.
                    environment: env.clone(),
                    body: body.as_ref().clone(),
                }))
            }
            Expr::Var { var_name } => *env.get(var_name).expect("unknown variable"),
            Expr::Call { func, args } => {
                let closure_address = self.eval(env, func);

                // TODO: Cloning the closure is relatively inefficient because
                // it contains a potentially large expression. Doing it anyway
                // for now since I will refactor it later to not store a copy of
                // the expression in the closure but rather a code pointer of
                // some kind.
                let closure = self.heap.deref(closure_address).check_closure().clone();

                if closure.arg_names.len() != args.len() {
                    panic!("incorrect number of arguments");
                }

                let mut new_environment = closure.environment.clone();

                let mut args_values = Vec::new();
                for arg in args {
                    args_values.push(self.eval(env, arg));
                }

                for (arg_name, arg_value) in closure.arg_names.iter().zip(args_values) {
                    new_environment.insert(arg_name.clone(), arg_value);
                }

                // Allow the function to recursively calling itself by inserting
                // a pointer to its own closure into its environment when
                // calling it.
                new_environment.insert(closure.name.clone(), closure_address);

                let result = self.eval(&new_environment, &closure.body);

                // TODO: This is right now the only trigger for freeing memory.
                // However, it does not account for pointers that are never
                // assigned to variables in the environment, e.g. intermediate
                // values in expressions. Is there a way to deal with that
                // without converting to ANF? (i.e. giving every intermediate
                // value a name and flattening the evaluation contexts)
                self.heap.free_env(&new_environment);

                result
            }
            Expr::Let {
                name,
                definition,
                body,
            } => {
                // TODO: This can be more efficient since it is a stack, no need
                // to copy the entire environment.
                let mut new_environment = env.clone();

                let definition_value = self.eval(env, definition);
                new_environment.insert(name.clone(), definition_value);

                let result = self.eval(&new_environment, body);

                self.heap.free_env(&new_environment);

                result
            }
            Expr::BinOp { op, lhs, rhs } => {
                let lhs_address = self.eval(env, lhs);
                let rhs_address = self.eval(env, rhs);
                self.eval_binop(*op, lhs_address, rhs_address)
            }
            Expr::Set {
                tuple,
                index,
                new_expr,
            } => {
                let tuple_address = self.eval(env, tuple);
                let new_value = self.eval(env, new_expr);

                // TODO: Could check if it is a tuple before evaluating the new value for the field.
                let tuple = self.heap.deref_mut(tuple_address).check_tuple_mut();

                if (*index as usize) < tuple.field_values.len() {
                    tuple.field_values[*index as usize] = new_value;
                } else {
                    panic!("tuple index out of range during mutation");
                }

                self.heap.alloc(HeapValue::Tuple(Tuple {
                    field_values: Vec::new(),
                }))
            }
            Expr::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                let condition_address = self.eval(env, condition);
                let condition_value = self.heap.deref(condition_address).check_bool();

                if condition_value {
                    self.eval(env, branch_success)
                } else {
                    self.eval(env, branch_failure)
                }
            }
        }
    }
}
