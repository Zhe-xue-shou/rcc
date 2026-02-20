// C/C++ like default initialization in struct fields
#![feature(default_field_values)]
// const for Box::new
#![feature(const_convert)]
// for `impl const` traits
#![feature(const_trait_impl)]
// operator `?` overloading
#![feature(try_trait_v2)]
// NTTP
#![feature(adt_const_params)]
// for using core::intrinsics::breakpoint
#![allow(internal_features)]
#![feature(core_intrinsics)]
pub(crate) mod blueprints;
pub mod common;
pub mod diagnosis;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod sema;
pub mod session;
pub mod types;
