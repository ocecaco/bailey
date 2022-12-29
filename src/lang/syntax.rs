#[derive(Debug, Copy, Clone)]
pub enum Constant {
    Int { value: i32 },
    Bool { value: bool },
}

#[derive(Debug, Copy, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Eq,
    Get,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Constant),
    Var {
        var_name: String,
    },
    Fun {
        name: String,
        arg_names: Vec<String>,
        body: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Let {
        name: String,
        definition: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        branch_success: Box<Expr>,
        branch_failure: Box<Expr>,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Tuple {
        values: Vec<Expr>,
    },
    Set {
        tuple: Box<Expr>,
        index: u32,
        new_expr: Box<Expr>,
    },
}
