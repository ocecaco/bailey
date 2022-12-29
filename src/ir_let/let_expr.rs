use crate::lang::syntax::{BinOp, Constant};

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
