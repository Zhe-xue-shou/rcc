#![allow(incomplete_features)]
#![feature(unsized_const_params)]
#![feature(specialization)]
// C/C++ like default initialization in struct fields
// #![feature(default_field_values)]
// const for Box::new
#![feature(const_convert)]
// for `impl const` traits
#![feature(const_trait_impl)]
#![feature(derive_const)]
#![feature(const_clone)]
#![feature(const_cmp)]
#![feature(const_try)]
#![feature(const_ops)]
// NTTP
#![feature(adt_const_params)]
// for using core::intrinsics::breakpoint
#![allow(internal_features)]
// for const_eval_select
#![feature(core_intrinsics)]
#![feature(const_eval_select)]
pub(crate) mod blueprints;
pub mod codegen;
#[macro_use]
pub mod common;
pub mod diagnosis;
pub mod ir;
pub mod lexer;
pub mod parse;
pub mod sema;
pub mod session;
pub mod storage;
pub mod types;
