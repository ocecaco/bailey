// TODO: Remove this when the implementation is reasonably complete and there
// are no more unused parts.
#![allow(dead_code)]
mod ir_let;
mod lang;
mod result;

use crate::ir_let::compiler::let_normalize;
use crate::ir_let::interpreter::simple_eval::ProgramEvaluator;
use crate::lang::test::fib::fib_test;

fn main() {
    let fib_program = fib_test(10);
    let compiled_program = let_normalize(&fib_program).expect("expected program");
    println!("{:#?}", compiled_program);

    let mut evaluator = ProgramEvaluator::new(compiled_program);
    let result = evaluator.run();

    println!("{:#?}", evaluator);

    println!("{:#?}", result);
}
