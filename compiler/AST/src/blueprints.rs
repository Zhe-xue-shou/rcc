//! TODO: integrate this into parser crate, since the sema AST migration is completed.

mod rawdecl;
mod rawexpr;
mod rawstmt;

pub use self::{
  rawdecl::VarDeclKind,
  rawexpr::{
    RawArraySubscript, RawBinary, RawCStyleCast, RawCall, RawCompoundLiteral,
    RawConstant, RawMemberAccess, RawParen, RawSizeOf, RawSizeOfKind,
    RawTernary, RawUnary,
  },
  rawstmt::{
    RawBreak, RawCase, RawCompound, RawContinue, RawDefault, RawDoWhile,
    RawFor, RawGoto, RawIf, RawLabel, RawReturn, RawSwitch, RawWhile,
  },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ::strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
pub enum UnaryKind {
  Prefix,
  Postfix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Placeholder;

impl const From<Placeholder> for () {
  #[inline(always)]
  fn from(_: Placeholder) -> Self {}
}
impl const From<()> for Placeholder {
  #[inline(always)]
  fn from(_: ()) -> Self {
    Self
  }
}
impl ::std::fmt::Display for Placeholder {
  #[inline(always)]
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    write!(f, "")
  }
}
