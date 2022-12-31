# Bailey
This is a work-in-progress implementation of Bailey, a simple dynamically, strictly-evaluated, reference-counted programming language.

I started this side project in December 2022, mostly as a way to experiment with compiling a functional language all the way to assembly with proper memory management (based on reference counting).

## Features currently implemented
* The source language is an untyped lambda calculus with let bindings, with heap-allocated tuples, integers and booleans as basic data types.
* Compilation to a simplified intermediate language that flattens the source terms into blocks of single instructions (let-normalized form).
* An interpreter for the intermediate language that stores all values on a reference-counted heap. The interpreter is effectively a byte-code interpreter. It uses an iterative implementation with its own call stack represented as an ordinary vector (Vec) in Rust.
* A frame layout step that assigns all variables a fixed offset in a stack frame. This is in preparation for generating assembly code.

## To be implemented
* A parser for the source language. I wanted to get the project up and running quickly and wanted to focus mainly on the translation to intermediate language and interpreter. Hence, I did not add a parser yet.
* Assembly generation: the intermediate language is already somewhat close to being able to be translated into assembly, since it already uses a flat representation of the instructions. Moreover, there is also already code to determine the stack frame layout for each block/function in the program.
* A simple runtime to handle heap allocation and reference counting. To be decided whether I will implement it in C or Rust. Initially, all manipulation of the heap values will be implemented in the runtime (including reference counting and things like adding two integers stored on the heap), but I could gradually reduce the scope of the runtime so that the compiled assembly only requires an external allocator (i.e. malloc/free).