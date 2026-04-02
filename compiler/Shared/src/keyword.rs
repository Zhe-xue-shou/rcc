#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  ::strum_macros::Display,
  ::strum_macros::EnumString,
  ::strum_macros::AsRefStr,
  ::strum_macros::IntoStaticStr,
  ::std::marker::ConstParamTy,
)]
#[strum(serialize_all = "snake_case")]
pub enum Keyword {
  // C
  Auto,
  Break,
  Case,
  Char,
  Const,
  Continue,
  Default,
  Do,
  Double,
  Else,
  Enum,
  Extern,
  Float,
  For,
  Goto,
  If,
  Int,
  Long,
  Register,
  Return,
  Short,
  Signed,
  Sizeof,
  Static,
  Struct,
  Switch,
  Typedef,
  #[strum(serialize = "thread_local")]
  #[strum(serialize = "_Thread_local")]
  ThreadLocal,
  Union,
  Unsigned,
  Void,
  Volatile,
  Restrict,
  Inline,
  While,
  // ^^^ focus / not considered vvv
  #[strum(serialize = "_Bool")]
  #[strum(serialize = "bool")] // C23
  Bool, // no type of bool in C99/C11
  #[strum(serialize = "_Generic")]
  Generic,
  #[strum(serialize = "atomic")] // C23
  #[strum(serialize = "_Atomic")]
  Atomic,
  #[strum(serialize = "_Noreturn")]
  Noreturn,
  #[strum(serialize = "alignas")] // C23
  #[strum(serialize = "_Alignas")]
  Alignas,
  #[strum(serialize = "alignof")] // C23
  #[strum(serialize = "_Alignof")]
  Alignof,
  #[strum(serialize = "static_assert")] // C23
  #[strum(serialize = "_Static_assert")]
  StaticAssert,
  #[strum(serialize = "complex")] // macro
  #[strum(serialize = "_Complex")]
  Complex,
  #[strum(serialize = "imaginary")] // macro
  #[strum(serialize = "_Imaginary")]
  Imaginary,
  // ^^^ pre C23 / C23 vvv
  Constexpr,
  /// predefined constant, treat it as keyword anyway
  Nullptr,
  True,
  False,
  // make these deliberately keyword.
  And,
  AndEq,
  Bitand,
  Bitor,
  Compl,
  Not,
  NotEq,
  Or,
  OrEq,
  Xor,
  XorEq,

  // some stuffs.
  #[strum(serialize = "__auto_type")]
  AutoType,
  #[strum(serialize = "_Nullable")]
  Nullable,
  #[strum(serialize = "_Nonnull")]
  Nonnull,
  #[strum(serialize = "_Null_unspecified")]
  NullUnspecified,
  #[strum(serialize = "__func__")]
  Func,
  #[strum(serialize = "__FUNCTION__")]
  Function,
  #[strum(serialize = "__PRETTY_FUNCTION__")]
  PrettyFunction,
}
#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  ::strum_macros::Display,
  ::strum_macros::EnumString,
  ::strum_macros::IntoStaticStr,
  ::strum_macros::AsRefStr,
  ::std::marker::ConstParamTy,
)]
#[allow(unused)]
#[strum(serialize_all = "snake_case")]
/// kept as reserved keyword.
pub enum Reserved {
  #[strum(serialize = "typeof")] // C23
  #[strum(serialize = "__typeof__")]
  TypeOf,
  #[strum(serialize = "typeof_unqual")] // C23
  #[strum(serialize = "__typeof_unqual__")]
  TypeOfUnqual,
  #[strum(serialize = "_BitInt")]
  BitInt,
  #[strum(serialize = "_Decimal32")]
  Decimal32,
  #[strum(serialize = "_Decimal64")]
  Decimal64,
  #[strum(serialize = "_Decimal128")]
  Decimal128,
  // ^^^ C23 / C++ vvv
  Asm,
  Catch,
  #[strum(serialize = "char8_t")]
  Char8,
  #[strum(serialize = "char16_t")]
  Char16,
  #[strum(serialize = "char32_t")]
  Char32,
  Class,
  Concept,
  Consteval,
  Constinit,
  ConstCast,
  CoAwait,
  CoReturn,
  CoYield,
  Decltype,
  Delete,
  DynamicCast,
  Explicit,
  Export,
  Friend,
  Mutable,
  Namespace,
  New,
  Noexcept,
  Operator,
  Private,
  Protected,
  Public,
  ReinterpretCast,
  Requires,
  StaticCast,
  Template,
  This,
  Throw,
  Try,
  Typeid,
  Typename,
  Using,
  Virtual,
  #[strum(serialize = "wchar_t")]
  WideChar,
  // ^^^ keywords/contextual vvv
  Final,
  Override,
  Import,
  Module,
}

impl PartialEq<Keyword> for &Keyword {
  fn eq(&self, other: &Keyword) -> bool {
    **self == *other
  }
}

#[cfg(test)]
mod tests {
  use ::std::str::FromStr;
  use Keyword::*;
  use Reserved::*;

  use super::*;

  #[test]
  fn simple() {
    assert_eq!(Auto.to_string(), "auto");
    assert_eq!(Auto, Keyword::from_str("auto").unwrap());
  }

  #[test]
  fn reserved() {
    assert_eq!(Asm.to_string(), "asm");
    assert_eq!(Asm, Reserved::from_str("asm").unwrap());
  }
  #[test]
  fn snake_case() {
    assert_eq!(AutoType.to_string(), "__auto_type");
    assert_eq!(AutoType, Keyword::from_str("__auto_type").unwrap());
    assert!(Keyword::from_str("auto_type").is_err());
  }

  #[test]
  fn multi() {
    assert_eq!(ThreadLocal.to_string(), "_Thread_local");
    assert_eq!(ThreadLocal, Keyword::from_str("_Thread_local").unwrap());
    assert_eq!(ThreadLocal, Keyword::from_str("thread_local").unwrap());
  }
  #[test]
  fn renamed() {
    assert_eq!(Char8.to_string(), "char8_t");
    assert_eq!(Char8, Reserved::from_str("char8_t").unwrap());
    assert!(Reserved::from_str("char8").is_err());
  }
}
