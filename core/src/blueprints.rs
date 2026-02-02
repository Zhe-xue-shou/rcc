mod rawdecl;
mod rawexpr;
mod rawstmt;

pub use self::{
  // rawdecl::*,
  rawexpr::{
    RawArraySubscript, RawBinary, RawCStyleCast, RawCall, RawCompoundLiteral,
    RawConstant, RawMemberAccess, RawParen, RawSizeOf, RawSizeOfKind,
    RawTernary, RawUnary,
  },
  rawstmt::{
    RawBreak, RawCase, RawCompound, RawContinue, RawDefault, RawDoWhile,
    RawFor, RawGoto, RawIf, RawLabel, RawReturn, RawStmt, RawSwitch, RawWhile,
  },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Placeholder;
