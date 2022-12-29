use crate::heap_value::HeapAddress;
use crate::let_expr::TargetAddress;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ReturnInfo {
    pub result_variable: String,
    pub return_address: TargetAddress,
}

#[derive(Debug)]
pub struct BlockFrame {
    pub values: Vec<HeapAddress>,
    pub variable_offsets: HashMap<String, usize>,
    pub return_info: Option<ReturnInfo>,
}

impl BlockFrame {
    fn new(return_info: ReturnInfo) -> Self {
        BlockFrame {
            values: Vec::new(),
            variable_offsets: HashMap::new(),
            return_info: Some(return_info),
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<HeapAddress> {
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

#[derive(Debug)]
struct CallStackFrame {
    nested_block_frames: Vec<BlockFrame>,
}

impl CallStackFrame {
    fn new(return_info: ReturnInfo) -> Self {
        CallStackFrame {
            nested_block_frames: vec![BlockFrame::new(return_info)],
        }
    }

    fn enter_block(&mut self, return_info: ReturnInfo) {
        self.nested_block_frames.push(BlockFrame::new(return_info))
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

#[derive(Debug)]
pub struct Stack {
    frames: Vec<CallStackFrame>,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            frames: vec![CallStackFrame {
                nested_block_frames: vec![BlockFrame {
                    values: Vec::new(),
                    variable_offsets: HashMap::new(),
                    return_info: None,
                }],
            }],
        }
    }

    pub fn enter_function(&mut self, return_info: ReturnInfo) {
        self.frames.push(CallStackFrame::new(return_info));
    }

    pub fn enter_block(&mut self, return_info: ReturnInfo) {
        self.current_frame_mut().enter_block(return_info)
    }

    pub fn exit_block(&mut self) -> BlockFrame {
        let frame = self.current_frame_mut().exit_block();

        // We pop the call stack frame upon exiting the outermost block
        // of the function.
        if self.current_frame().nested_block_frames.is_empty() {
            self.frames.pop();
        }

        frame
    }

    pub fn set_var_no_refcount(&mut self, name: String, value: HeapAddress) {
        self.current_frame_mut().set_var_no_refcount(name, value);
    }

    pub fn lookup_var(&self, name: &str) -> HeapAddress {
        self.current_frame().lookup_var(name)
    }

    fn current_frame_mut(&mut self) -> &mut CallStackFrame {
        self.frames.last_mut().expect("stack should not be empty")
    }

    fn current_frame(&self) -> &CallStackFrame {
        self.frames.last().expect("stack should not be empty")
    }
}
