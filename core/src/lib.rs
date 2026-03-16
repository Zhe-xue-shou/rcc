#![feature(unsized_const_params)]
#![allow(incomplete_features)]
#![feature(specialization)]
// C/C++ like default initialization in struct fields
// #![feature(default_field_values)]
// const for Box::new
#![feature(const_convert)]
// for `impl const` traits
#![feature(const_trait_impl)]
// NTTP
#![feature(adt_const_params)]
// for using core::intrinsics::breakpoint
#![allow(internal_features)]
#![feature(core_intrinsics)]
pub(crate) mod blueprints;
pub mod codegen;
pub mod common;
pub mod diagnosis;
pub mod ir;
pub mod lexer;
pub mod parse;
pub mod sema;
pub mod session;
pub mod storage;
pub mod types;
