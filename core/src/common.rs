#[macro_use]
mod dumper;
mod environment;
mod floating;
mod integral;
mod keyword;
mod operator;
mod source_info;
mod storage;
mod token;
pub use self::{
  dumper::{Default as TreeDumper, Dumpable, Dumper, FakeDumpRes, Palette},
  environment::{Environment, Symbol, SymbolRef, UnitScope, VarDeclKind},
  floating::{Floating, Format as FloatFormat},
  integral::{Integral, Signedness},
  keyword::Keyword,
  operator::{Category as OperatorCategory, Operator},
  source_info::{
    Coordinate, Display as SourceDisplay, File as SourceFile,
    Id as SourceFileId, Id as FileId, Index as SourceSpanIndex,
    Index as SpanIndex, Manager as SourceManager, Span as SourceSpan,
    SpanDisplay,
  },
  storage::Storage,
  token::{Literal, Token},
};

pub type StrRef<'c> = &'c str;

pub trait RefEq {
  fn ref_eq(lhs: Self, rhs: Self) -> bool
  where
    Self: PartialEq + Sized;
}

impl RefEq for StrRef<'_> {
  fn ref_eq(lhs: Self, rhs: Self) -> bool
  where
    Self: PartialEq + Sized,
  {
    let ref_eq = ::std::ptr::eq(lhs, rhs);
    if const { cfg!(debug_assertions) } {
      let actual_eq = lhs == rhs;
      if ref_eq != actual_eq {
        eprintln!(
          "INTERNAL ERROR: comparing by pointer address result did not match 
          the actual result: {:p}: {:?} and {:p}: {:?}
        ",
          lhs, lhs, rhs, rhs
        );
      }
      return actual_eq;
    }
    ref_eq
  }
}
