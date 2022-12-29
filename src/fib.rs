use crate::syntax::{BinOp, Constant, Expr};

fn fib_helper_def() -> Expr {
    Expr::Fun {
        name: "fib_helper".to_owned(),
        arg_names: vec!["n".to_owned(), "a".to_owned(), "b".to_owned()],
        body: Box::new(Expr::If {
            condition: Box::new(Expr::BinOp {
                op: BinOp::Eq,
                lhs: Box::new(Expr::Var {
                    var_name: "n".to_owned(),
                }),
                rhs: Box::new(Expr::Literal(Constant::Int { value: 0 })),
            }),
            branch_success: Box::new(Expr::Var {
                var_name: "b".to_owned(),
            }),
            branch_failure: Box::new(Expr::Call {
                func: Box::new(Expr::Var {
                    var_name: "fib_helper".to_owned(),
                }),
                args: vec![
                    Expr::BinOp {
                        op: BinOp::Sub,
                        lhs: Box::new(Expr::Var {
                            var_name: "n".to_owned(),
                        }),
                        rhs: Box::new(Expr::Literal(Constant::Int { value: 1 })),
                    },
                    Expr::BinOp {
                        op: BinOp::Add,
                        lhs: Box::new(Expr::Var {
                            var_name: "a".to_owned(),
                        }),
                        rhs: Box::new(Expr::Var {
                            var_name: "b".to_owned(),
                        }),
                    },
                    Expr::Var {
                        var_name: "a".to_owned(),
                    },
                ],
            }),
        }),
    }
}

fn fib_def() -> Expr {
    Expr::Fun {
        name: "fib".to_owned(),
        arg_names: vec!["n".to_owned()],
        body: Box::new(Expr::Call {
            func: Box::new(Expr::Var {
                var_name: "fib_helper".to_owned(),
            }),
            args: vec![
                Expr::Var {
                    var_name: "n".to_owned(),
                },
                Expr::Literal(Constant::Int { value: 1 }),
                Expr::Literal(Constant::Int { value: 0 }),
            ],
        }),
    }
}

pub fn fib_test(n: i32) -> Expr {
    Expr::Let {
        name: "fib_helper".to_owned(),
        definition: Box::new(fib_helper_def()),
        body: Box::new(Expr::Let {
            name: "fib".to_owned(),
            definition: Box::new(fib_def()),
            body: Box::new(Expr::Call {
                func: Box::new(Expr::Var {
                    var_name: "fib".to_owned(),
                }),
                args: vec![Expr::Literal(Constant::Int { value: n })],
            }),
        }),
    }
}
