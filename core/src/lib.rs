// for using core::intrinsics::breakpoint
#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(adt_const_params)]
// for std::fmt::Formatter
#![feature(formatting_options)]
pub mod analyzer;
pub mod codegen;
pub mod common;
pub mod lexer;
pub mod parser;
pub mod types;
