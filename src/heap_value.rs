use crate::let_expr::TargetAddress;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapAddress(pub u32);

// Using Strings everywhere it likely not very efficient (as opposed to interning
// or using offsets into stack frames), but this is just a proof-of-concept simple
// implementation.

#[derive(Debug, Clone)]
pub struct Closure {
    pub name: String,
    pub arg_names: Vec<String>,
    pub environment: HashMap<String, HeapAddress>,
    pub body: TargetAddress,
}

#[derive(Debug, Clone)]
pub struct Tuple {
    pub field_values: Vec<HeapAddress>,
}

#[derive(Debug, Clone)]
pub enum HeapValue {
    Int(i32),
    Bool(bool),
    Tuple(Tuple),
    Closure(Closure),
}

impl HeapValue {
    pub fn check_closure(&self) -> &Closure {
        match self {
            HeapValue::Closure(clos) => clos,
            _ => panic!("expected closure"),
        }
    }

    pub fn check_int(&self) -> i32 {
        match self {
            HeapValue::Int(value) => *value,
            _ => panic!("expected int"),
        }
    }

    pub fn check_bool(&self) -> bool {
        match self {
            HeapValue::Bool(value) => *value,
            _ => panic!("expected bool"),
        }
    }

    pub fn check_tuple(&self) -> &Tuple {
        match self {
            HeapValue::Tuple(tuple) => tuple,
            _ => panic!("expected tuple"),
        }
    }

    pub fn check_tuple_mut(&mut self) -> &mut Tuple {
        match self {
            HeapValue::Tuple(tuple) => tuple,
            _ => panic!("expected tuple"),
        }
    }
}

#[derive(Debug)]
pub struct RefCountedHeapValue {
    pub refcount: u32,
    pub heap_value: HeapValue,
}
