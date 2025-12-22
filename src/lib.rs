#![allow(internal_features)]
// for using core::intrinsics::breakpoint
#![feature(core_intrinsics)]
#![feature(adt_const_params)]
#![allow(incomplete_features)]
#![feature(unsized_const_params)]
pub(crate) mod common;
pub mod lexer;
pub mod parser;
// pub mod preprocessor;
pub mod analyzer;
pub mod codegen;
pub mod utils;
