// for using core::intrinsics::breakpoint
#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(adt_const_params)]
pub mod analyzer;
pub mod codegen;
mod common;
pub mod lexer;
pub mod parser;
pub mod types;
