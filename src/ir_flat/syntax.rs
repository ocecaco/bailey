use crate::lang::syntax::{BinOp, Constant};

#[derive(Debug, Copy, Clone)]
pub enum Reference {
    Local(LocalReference),
    Argument(ArgumentReference),
    Closure(ClosureReference),
    This,
}

#[derive(Debug, Copy, Clone)]
pub struct LocalReference(pub usize);

#[derive(Debug, Copy, Clone)]
pub struct ArgumentReference(pub usize);

#[derive(Debug, Copy, Clone)]
pub struct ClosureReference(pub usize);

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub args_size: usize,
    pub closure_env_size: usize,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub frame_size: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    EnterBlock,
    ExitBlock,
    Assignment(Assignment),
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: LocalReference,
    pub definition: Definition,
}

#[derive(Debug, Clone)]
pub enum Definition {
    Var(Reference),
    Step(Step),
}

#[derive(Debug, Copy, Clone)]
pub struct TargetAddress {
    pub function_index: usize,
    pub block_index: usize,
    pub instruction_index: usize,
}

#[derive(Debug, Clone)]
pub struct AllocClosure {
    pub name: String,
    pub arg_names: Vec<String>,
    pub free_names: Vec<String>,
    pub body: TargetAddress,
}

#[derive(Debug, Clone)]
pub enum Simple {
    Literal(Constant),
    Fun(AllocClosure),
    BinOp {
        op: BinOp,
        lhs: Reference,
        rhs: Reference,
    },
    Tuple {
        args: Vec<Reference>,
    },
    Set {
        tuple: Reference,
        index: u32,
        new_value: Reference,
    },
}

#[derive(Debug, Clone)]
pub enum Control {
    Call {
        func: Reference,
        args: Vec<Reference>,
    },
    If {
        condition: Reference,
        branch_success: TargetAddress,
        branch_failure: TargetAddress,
    },
}

#[derive(Debug, Clone)]
pub enum Step {
    Simple(Simple),
    Control(Control),
}
