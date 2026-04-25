#[macro_use]
mod builder;
mod constant;
mod context;
mod emitable;
mod global;
pub mod instruction;
mod types;
mod value;

use ::rcc_ast::Constant as ConstantData;

pub use self::{
  builder::Builder,
  constant::{Constant as IRConstant, Global as GlobalValue},
  context::{Context, Session},
  global::{
    BasicBlock, Function as IRFunction, Initializer as IRStaticInitializer,
    Module, Variable as IRVariable,
  },
  types::{Type, TypeRef, TypeRefMut},
  value::{Arguments as IRArguments, Data as ValueData, Value, ValueID},
};
