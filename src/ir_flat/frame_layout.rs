use std::collections::HashMap;

use crate::ir_flat::syntax as target;
use crate::ir_let::let_expr as source;

fn compute_layout(base_offset: usize, names: &[String]) -> HashMap<String, usize> {
    let original_length = names.len();
    let mut result = HashMap::new();

    for (i, n) in names.iter().enumerate() {
        result.insert(n.clone(), base_offset + i);
    }

    // Ensure that all names were unique in the environment as a sanity check.
    assert!(original_length == result.len());

    result
}

#[derive(Debug)]
pub struct ProgramFrameLayout {
    functions: Vec<FunctionFrameLayout>,
}

impl ProgramFrameLayout {
    pub fn frame_size(&self, function_index: usize, block_index: usize) -> usize {
        let function_layout = self
            .functions
            .get(function_index)
            .expect("unknown function");

        let block_layout = function_layout
            .blocks
            .get(block_index)
            .expect("unknown block");

        block_layout.offsets.len()
    }

    pub fn lookup_var(
        &self,
        function_index: usize,
        block_index: usize,
        name: &str,
    ) -> target::Reference {
        let function_layout = self
            .functions
            .get(function_index)
            .expect("unknown function");

        // First we search local variables, from innermost to outermost
        // enclosing block frame.
        let mut current_block_index = Some(block_index);
        while let Some(block_index) = current_block_index {
            let block_layout = function_layout
                .blocks
                .get(block_index)
                .expect("unknown block");

            if let Some(offset) = block_layout.offsets.get(name) {
                return target::Reference::Local(*offset);
            }

            current_block_index = block_layout.parent_block_index;
        }

        // Otherwise we check function arguments, function name itself (for
        // recursive calls), and finally closure environment.
        if let Some(offset) = function_layout.offsets_arguments.get(name) {
            return target::Reference::Argument(*offset);
        }

        if function_layout.this_name == name {
            return target::Reference::This;
        }

        if let Some(offset) = function_layout.offsets_free_vars.get(name) {
            return target::Reference::Closure(*offset);
        }

        panic!("Failed to resolve variable offset");
    }
}

#[derive(Debug)]
struct FunctionFrameLayout {
    this_name: String,
    offsets_arguments: HashMap<String, target::ArgumentReference>,
    offsets_free_vars: HashMap<String, target::ClosureReference>,
    blocks: Vec<BlockFrameLayout>,
}

#[derive(Debug)]
struct BlockFrameLayout {
    // Starting offset from the base of the function stack frame
    start_offset: usize,
    offsets: HashMap<String, target::LocalReference>,
    parent_block_index: Option<usize>,
}

impl BlockFrameLayout {
    // This is the first free offset counting from the base of the function stack
    // frame that is not occupied by this block. Blocks nested inside of this
    // block should therefore start from this offset.
    fn end_offset(&self) -> usize {
        self.start_offset + self.offsets.len()
    }
}

pub fn compute_program_frame_layout(program: &source::Program) -> ProgramFrameLayout {
    let mut function_layouts = Vec::new();

    for f in &program.functions {
        function_layouts.push(compute_function_frame_layout(f));
    }

    ProgramFrameLayout {
        functions: function_layouts,
    }
}

fn compute_function_frame_layout(function: &source::Function) -> FunctionFrameLayout {
    let mut block_layouts: Vec<BlockFrameLayout> = Vec::new();

    for b in &function.blocks {
        let parent_layout = if let Some(parent_index) = b.parent_block_index {
            Some(
                block_layouts
                    .get(parent_index)
                    .expect("parent block should have been already processed"),
            )
        } else {
            None
        };

        let start_offset = if let Some(parent_layout) = parent_layout {
            parent_layout.end_offset()
        } else {
            0
        };

        let block_layout = BlockFrameLayout {
            start_offset,
            offsets: compute_layout(start_offset, &b.block_names())
                .drain()
                .map(|(name, offset)| (name, target::LocalReference(offset)))
                .collect(),
            parent_block_index: b.parent_block_index,
        };

        block_layouts.push(block_layout);
    }

    FunctionFrameLayout {
        this_name: function.name.clone(),
        offsets_arguments: compute_layout(0, &function.arg_names)
            .drain()
            .map(|(name, offset)| (name, target::ArgumentReference(offset)))
            .collect(),
        offsets_free_vars: compute_layout(
            0,
            function
                .free_names
                .as_ref()
                .expect("free names should be known"),
        )
        .drain()
        .map(|(name, offset)| (name, target::ClosureReference(offset)))
        .collect(),
        blocks: block_layouts,
    }
}
