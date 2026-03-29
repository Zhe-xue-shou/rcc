#[macro_use]
mod emitter;
mod constant;
mod context;
mod emitable;
mod fmt;
pub mod instruction;
pub mod module;
mod types;
mod value;

pub use self::{
  context::{Context, Session},
  emitter::Emitter,
  module::{
    Argument, BasicBlock, Function as IRFunction,
    Initializer as IRStaticInitializer, Module, Variable as IRGlobalValue,
  },
  // printer::{IRPrinter, Printable, Printer},
  types::{Type, TypeRef, TypeRefMut},
  value::{Data as ValueData, Value, ValueID},
};
