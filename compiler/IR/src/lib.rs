#[macro_use]
mod builder;
mod context;
mod emitable;
pub mod instruction;
pub mod module;
mod types;
mod value;

pub use self::{
  builder::Builder,
  context::{Context, Session},
  module::{
    Argument, BasicBlock, Function as IRFunction,
    Initializer as IRStaticInitializer, Module, Variable as IRGlobalValue,
  },
  types::{Type, TypeRef, TypeRefMut},
  value::{Data as ValueData, Value, ValueID},
};
