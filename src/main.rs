// TODO: Remove this when the implementation is reasonably complete and there
// are no more unused parts.
#![allow(dead_code)]
mod fib;
mod heap;
mod heap_value;
mod let_expr;
mod let_normalize;
mod result;
mod simple_eval;
mod stack;
mod syntax;

use crate::fib::fib_test;
use crate::let_normalize::let_normalize;
use crate::simple_eval::ProgramEvaluator;

fn main() {
    let fib_program = fib_test(10);
    let compiled_program = let_normalize(&fib_program).expect("expected program");
    println!("{:#?}", compiled_program);

    let mut evaluator = ProgramEvaluator::new(compiled_program);
    let result = evaluator.run();

    println!("{:#?}", evaluator);

    println!("{:#?}", result);
}
