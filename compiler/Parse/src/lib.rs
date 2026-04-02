#![feature(adt_const_params)]

pub mod declaration;
pub mod expression;
mod parser;
pub mod statement;

pub use self::parser::Parser;
