// for `impl const` traits
#![feature(const_trait_impl)]
// operator `?` overloading
#![feature(try_trait_v2)]
// NTTP
#![feature(adt_const_params)]
// for using core::intrinsics::breakpoint
#![allow(internal_features)]
#![feature(core_intrinsics)]
pub mod analyzer;
pub(crate) mod blueprints;
pub mod codegen;
pub mod common;
pub mod diagnosis;
pub mod lexer;
pub mod parser;
pub mod types;
