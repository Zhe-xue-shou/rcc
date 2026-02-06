#![feature(const_trait_impl)]
// operator `?` overloading
#![feature(try_trait_v2)]
// for using core::intrinsics::breakpoint
#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(adt_const_params)]
pub mod analyzer;
pub(crate) mod blueprints;
pub mod codegen;
pub mod common;
pub mod diagnosis;
pub mod lexer;
pub mod parser;
pub mod types;
