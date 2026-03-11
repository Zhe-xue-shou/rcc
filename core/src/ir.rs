#![allow(unused)]
mod builder;
mod fmt;
mod instruction;
mod module;
mod types;
mod value;

pub use self::{
  builder::ModuleBuilder,
  module::{
    BasicBlock, Function as IRFunction, Initializer as IRStaticInitializer,
    Module, Variable as IRGlobalValue,
  },
  types::{Type, TypeRef, TypeRefMut},
};
