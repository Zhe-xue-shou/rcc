use std::marker::ConstParamTy;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Display, EnumString, PartialEq, Eq, ConstParamTy)]
pub enum Keyword {
  // C
  #[strum(serialize = "auto")]
  Auto,
  #[strum(serialize = "break")]
  Break,
  #[strum(serialize = "case")]
  Case,
  #[strum(serialize = "char")]
  Char,
  #[strum(serialize = "const")]
  Const,
  #[strum(serialize = "continue")]
  Continue,
  #[strum(serialize = "default")]
  Default,
  #[strum(serialize = "do")]
  Do,
  #[strum(serialize = "double")]
  Double,
  #[strum(serialize = "else")]
  Else,
  #[strum(serialize = "enum")]
  Enum,
  #[strum(serialize = "extern")]
  Extern,
  #[strum(serialize = "float")]
  Float,
  #[strum(serialize = "for")]
  For,
  #[strum(serialize = "goto")]
  Goto,
  #[strum(serialize = "if")]
  If,
  #[strum(serialize = "int")]
  Int,
  #[strum(serialize = "long")]
  Long,
  #[strum(serialize = "register")]
  Register,
  #[strum(serialize = "return")]
  Return,
  #[strum(serialize = "short")]
  Short,
  #[strum(serialize = "signed")]
  Signed,
  #[strum(serialize = "sizeof")]
  Sizeof,
  #[strum(serialize = "static")]
  Static,
  #[strum(serialize = "struct")]
  Struct,
  #[strum(serialize = "switch")]
  Switch,
  #[strum(serialize = "typedef")]
  Typedef,
  #[strum(serialize = "union")]
  Union,
  #[strum(serialize = "unsigned")]
  Unsigned,
  #[strum(serialize = "void")]
  Void,
  #[strum(serialize = "volatile")]
  Volatile,
  #[strum(serialize = "while")]
  While,
}
