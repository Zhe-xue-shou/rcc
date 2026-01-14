use ::strum_macros::{Display, EnumString};

#[derive(
  Debug, Clone, Display, EnumString, PartialEq, Eq, ::std::marker::ConstParamTy,
)]
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
  #[strum(serialize = "restrict")]
  Restrict,
  #[strum(serialize = "thread_local")]
  #[strum(serialize = "_Thread_local")]
  ThreadLocal,
  #[strum(serialize = "inline")]
  Inline,
  #[strum(serialize = "while")]
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
  #[strum(serialize = "constexpr")]
  Constexpr,
}
#[derive(
  Debug, Clone, Display, EnumString, PartialEq, Eq, ::std::marker::ConstParamTy,
)]
#[allow(unused)]
pub enum Reserved {
  #[strum(serialize = "typeof")] // C23
  #[strum(serialize = "__typeof__")]
  TypeOf,
  #[strum(serialize = "typeof_unqual")] // C23
  #[strum(serialize = "__typeof_unqual__")]
  TypeOfUnqual,
  #[strum(serialize = "nullptr")]
  Nullptr,
  #[strum(serialize = "true")]
  True,
  #[strum(serialize = "false")]
  False,
  #[strum(serialize = "_BitInt")]
  BitInt,
  // C23 optional, but I'll keep it reserved keyword
  #[strum(serialize = "_Decimal32")]
  Decimal32,
  #[strum(serialize = "_Decimal64")]
  Decimal64,
  #[strum(serialize = "_Decimal128")]
  Decimal128,
  // ^^^ C23 / C++, although not considering C++, but some words probably reserve them and emit a warning if used as identifiers vvv
  #[strum(serialize = "and")]
  And,
  #[strum(serialize = "and_eq")]
  AndEq,
  #[strum(serialize = "bitand")]
  Bitand,
  #[strum(serialize = "bitor")]
  Bitor,
  #[strum(serialize = "compl")]
  Compl,
  #[strum(serialize = "not")]
  Not,
  #[strum(serialize = "not_eq")]
  NotEq,
  #[strum(serialize = "or")]
  Or,
  #[strum(serialize = "or_eq")]
  OrEq,
  #[strum(serialize = "xor")]
  Xor,
  #[strum(serialize = "xor_eq")]
  XorEq,

  #[strum(serialize = "asm")]
  Asm,
  #[strum(serialize = "catch")]
  Catch,
  #[strum(serialize = "char8_t")]
  Char8,
  #[strum(serialize = "char16_t")]
  Char16,
  #[strum(serialize = "char32_t")]
  Char32,
  #[strum(serialize = "class")]
  Class,
  #[strum(serialize = "concept")]
  Concept,
  #[strum(serialize = "consteval")]
  Consteval,
  #[strum(serialize = "constinit")]
  Constinit,
  #[strum(serialize = "const_cast")]
  ConstCast,
  #[strum(serialize = "co_await")]
  CoAwait,
  #[strum(serialize = "co_return")]
  CoReturn,
  #[strum(serialize = "co_yield")]
  CoYield,
  #[strum(serialize = "decltype")]
  Decltype,
  #[strum(serialize = "delete")]
  Delete,
  #[strum(serialize = "dynamic_cast")]
  DynamicCast,
  #[strum(serialize = "explicit")]
  Explicit,
  #[strum(serialize = "export")]
  Export,
  #[strum(serialize = "friend")]
  Friend,
  #[strum(serialize = "mutable")]
  Mutable,
  #[strum(serialize = "namespace")]
  Namespace,
  #[strum(serialize = "new")]
  New,
  #[strum(serialize = "noexcept")]
  Noexcept,
  #[strum(serialize = "operator")]
  Operator,
  #[strum(serialize = "private")]
  Private,
  #[strum(serialize = "protected")]
  Protected,
  #[strum(serialize = "public")]
  Public,
  #[strum(serialize = "reinterpret_cast")]
  ReinterpretCast,
  #[strum(serialize = "requires")]
  Requires,
  #[strum(serialize = "static_cast")]
  StaticCast,
  #[strum(serialize = "template")]
  Template,
  #[strum(serialize = "this")]
  This,
  #[strum(serialize = "throw")]
  Throw,
  #[strum(serialize = "try")]
  Try,
  #[strum(serialize = "typeid")]
  Typeid,
  #[strum(serialize = "typename")]
  Typename,
  #[strum(serialize = "using")]
  Using,
  #[strum(serialize = "virtual")]
  Virtual,
  #[strum(serialize = "wchar_t")]
  WideChar,
  // ^^^ keywords/contextual vvv
  #[strum(serialize = "final")]
  Final,
  #[strum(serialize = "override")]
  Override,
  #[strum(serialize = "import")]
  Import,
  #[strum(serialize = "module")]
  Module,
}
