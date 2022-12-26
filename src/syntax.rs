#[derive(Clone)]
pub enum Expr {
    ConstInt {
        value: i32,
    },
    ConstBool {
        value: bool,
    },
    Tuple {
        values: Vec<Expr>,
    },
    Fun {
        name: String,
        arg_names: Vec<String>,
        body: Box<Expr>,
    },
    Var {
        name: String,
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
    Add {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Sub {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Eq {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Get {
        tuple: Box<Expr>,
        index: u32,
    },
    Set {
        tuple: Box<Expr>,
        index: u32,
        new_expr: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        branch_success: Box<Expr>,
        branch_failure: Box<Expr>,
    },
}
