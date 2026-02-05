mod rawdecl;
mod rawexpr;
mod rawstmt;

pub use self::{
  // rawdecl::*,
  rawexpr::{
    RawArraySubscript, RawBinary, RawCStyleCast, RawCall, RawCompoundLiteral,
    RawConstant, RawMemberAccess, RawParen, RawSizeOf, RawSizeOfKind,
    RawTernary, RawUnary, RawUnaryKind,
  },
  rawstmt::{
    RawBreak, RawCase, RawCompound, RawContinue, RawDefault, RawDoWhile,
    RawFor, RawGoto, RawIf, RawLabel, RawReturn, RawStmt, RawSwitch, RawWhile,
  },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Placeholder;

impl From<Placeholder> for () {
  #[inline(always)]
  fn from(_: Placeholder) -> Self {}
}
impl From<()> for Placeholder {
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
