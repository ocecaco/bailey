use crate::ir_flat::syntax as target;
use crate::ir_let::let_expr as source;

use crate::ir_flat::frame_layout::ProgramFrameLayout;

use super::frame_layout::compute_program_frame_layout;

struct Compiler<'a> {
    program: &'a source::Program,
    frame_layout: ProgramFrameLayout,
}

impl<'a> Compiler<'a> {
    fn new(program: &'a source::Program) -> Self {
        Compiler {
            program,
            frame_layout: compute_program_frame_layout(program),
        }
    }

    fn compile_function(&self, function: &source::Function) -> target::Function {
        let mut compiled_blocks = Vec::new();

        for (i, b) in function.blocks.iter().enumerate() {
            compiled_blocks.push(self.compile_block(i, b));
        }

        target::Function {
            args_size: function.arg_names.len(),
            closure_env_size: function
                .free_names
                .as_ref()
                .expect("free names should be known")
                .len(),
            blocks: compiled_blocks,
        }
    }

    fn compile_block(&self, _block_index: usize, _block: &source::Block) -> target::Block {
        unimplemented!();
    }
}
