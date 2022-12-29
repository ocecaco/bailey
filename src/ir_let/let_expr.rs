use crate::lang::syntax::{BinOp, Constant};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

impl Program {
    pub fn get_instruction(&self, address: TargetAddress) -> &Instruction {
        let function = self
            .functions
            .get(address.function_index)
            .expect("invalid function index");
        let block = function
            .blocks
            .get(address.block_index)
            .expect("invalid block index");
        block
            .instructions
            .get(address.instruction_index)
            .expect("invalid instruction index")
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "program\n")?;

        for (i, func) in self.functions.iter().enumerate() {
            write!(f, "begin function {}\n", i)?;
            write!(f, "{}", func)?;
            write!(f, "end function {}\n\n", i)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub blocks: Vec<Block>,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, block) in self.blocks.iter().enumerate() {
            write!(f, "begin block {}\n", i)?;
            write!(f, "{}", block)?;
            write!(f, "begin block {}\n\n", i)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    // The last instruction represents the result of evaluating the
    // entire sequence of instructions.
    pub instructions: Vec<Instruction>,
    pub parent_block_index: Option<usize>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(parent_block_index) = self.parent_block_index {
            write!(f, "parent block {}\n", parent_block_index)?;
        } else {
            write!(f, "no parent block\n")?;
        }

        for instruction in self.instructions.iter() {
            write!(f, "{}\n", instruction)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    EnterBlock,
    ExitBlock(VariableReference),
    Assignment(Assignment),
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Instruction::EnterBlock => write!(f, "enterblock")?,
            Instruction::ExitBlock(var) => write!(f, "exitblock({})", var)?,
            Instruction::Assignment(Assignment { name, definition }) => {
                write!(f, "{} = {}", name, definition)?
            }
        };

        Ok(())
    }
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

impl fmt::Display for Definition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Definition::Var(var) => write!(f, "{}", var)?,
            Definition::Step(step) => write!(f, "{}", step)?,
        };

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TargetAddress {
    pub function_index: usize,
    pub block_index: usize,
    pub instruction_index: usize,
}

impl TargetAddress {
    pub fn next(&self) -> TargetAddress {
        TargetAddress {
            function_index: self.function_index,
            block_index: self.block_index,
            instruction_index: self.instruction_index + 1,
        }
    }
}

impl fmt::Display for TargetAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({},{},{})",
            self.function_index, self.block_index, self.instruction_index
        )?;

        Ok(())
    }
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

impl fmt::Display for Simple {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Simple::Literal(Constant::Int { value }) => write!(f, "{}", value)?,
            Simple::Literal(Constant::Bool { value }) => write!(f, "{}", value)?,
            Simple::Fun(AllocClosure {
                name,
                arg_names,
                free_names,
                body,
            }) => {
                write!(f, "closure({}, {}, [", name, body)?;
                for a in arg_names {
                    write!(f, "{} ", a)?;
                }
                write!(f, "], [")?;
                for free_name in free_names {
                    write!(f, "{} ", free_name)?;
                }
                write!(f, "])")?;
            }
            Simple::BinOp { op, lhs, rhs } => {
                write!(f, "{} ", lhs)?;
                match op {
                    BinOp::Add => write!(f, "+")?,
                    BinOp::Sub => write!(f, "-")?,
                    BinOp::Eq => write!(f, "==")?,
                    BinOp::Get => write!(f, "!!")?,
                };
                write!(f, " {}", rhs)?
            }
            Simple::Tuple { args } => {
                write!(f, "(")?;
                for arg in args {
                    write!(f, "{}, ", arg)?;
                }
                write!(f, ")")?
            }
            Simple::Set {
                tuple,
                index,
                new_value,
            } => write!(f, "{}.{} = {}", tuple, index, new_value)?,
        };

        Ok(())
    }
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

impl fmt::Display for Control {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Control::Call { func, args } => {
                write!(f, "{}(", func)?;

                if let Some((first, rest)) = args.split_first() {
                    write!(f, "{}", first)?;

                    for arg in rest {
                        write!(f, ", {}", arg)?;
                    }
                }

                write!(f, ")")?;
            }
            Control::If {
                condition,
                branch_success,
                branch_failure,
            } => {
                write!(
                    f,
                    "if {} then {} else {}",
                    condition, branch_success, branch_failure
                )?;
            }
        };

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Step {
    Simple(Simple),
    Control(Control),
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Step::Simple(simple) => write!(f, "{}", simple)?,
            Step::Control(control) => write!(f, "{}", control)?,
        };

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct VariableReference {
    pub var_name: String,
}

impl fmt::Display for VariableReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.var_name)?;

        Ok(())
    }
}
