#[macro_use]
mod emitter;
mod context;
mod dump;
mod emitable;
mod fmt;
mod instruction;
mod module;
mod types;
mod value;

pub use self::{
  context::Context,
  dump::IRDumper,
  emitter::Emitter,
  module::{
    Argument, BasicBlock, Function as IRFunction,
    Initializer as IRStaticInitializer, Module, Variable as IRGlobalValue,
  },
  types::{Type, TypeRef, TypeRefMut},
  value::{Data as ValueData, Value, ValueID},
};
