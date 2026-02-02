#[derive(
  Debug,
  Clone,
  Copy,
  ::strum_macros::Display,
  ::strum_macros::EnumString,
  PartialEq,
  Eq,
  ::std::marker::ConstParamTy,
  Default,
)]
pub enum Operator {
  // one-character operators
  #[strum(serialize = "+")]
  Plus,
  #[strum(serialize = "-")]
  Minus,
  #[strum(serialize = "*")]
  Star,
  #[strum(serialize = "/")]
  Slash,
  #[strum(serialize = "%")]
  Percent,
  #[strum(serialize = ",")]
  Comma,
  #[strum(serialize = ";")]
  Semicolon,
  #[strum(serialize = "(")]
  LeftParen,
  #[strum(serialize = ")")]
  RightParen,
  #[strum(serialize = "{")]
  LeftBrace,
  #[strum(serialize = "}")]
  RightBrace,
  #[strum(serialize = "[")]
  LeftBracket,
  #[strum(serialize = "]")]
  RightBracket,
  #[strum(serialize = "=")]
  Assign,
  #[strum(serialize = "!")]
  Not,
  #[strum(serialize = "<")]
  Less,
  #[strum(serialize = ">")]
  Greater,
  #[strum(serialize = "&")]
  Ampersand,
  #[strum(serialize = "|")]
  Pipe,
  #[strum(serialize = "^")]
  Caret,
  #[strum(serialize = "~")]
  Tilde,
  #[strum(serialize = ".")]
  Dot,
  #[strum(serialize = "?")]
  Question,
  #[strum(serialize = ":")]
  Colon,
  // multi-character operators
  #[strum(serialize = "++")]
  PlusPlus,
  #[strum(serialize = "--")]
  MinusMinus,
  #[strum(serialize = "+=")]
  PlusAssign,
  #[strum(serialize = "-=")]
  MinusAssign,
  #[strum(serialize = "*=")]
  StarAssign,
  #[strum(serialize = "/=")]
  SlashAssign,
  #[strum(serialize = "%=")]
  PercentAssign,
  #[strum(serialize = "==")]
  EqualEqual,
  #[strum(serialize = "!=")]
  NotEqual,
  #[strum(serialize = "<=")]
  LessEqual,
  #[strum(serialize = ">=")]
  GreaterEqual,
  #[strum(serialize = "&&")]
  And,
  #[strum(serialize = "||")]
  Or,
  #[strum(serialize = "<<")]
  LeftShift,
  #[strum(serialize = ">>")]
  RightShift,
  #[strum(serialize = "&=")]
  AmpersandAssign,
  #[strum(serialize = "|=")]
  PipeAssign,
  #[strum(serialize = "^=")]
  CaretAssign,
  #[strum(serialize = "<<=")]
  LeftShiftAssign,
  #[strum(serialize = ">>=")]
  RightShiftAssign,
  #[strum(serialize = "->")]
  Arrow,

  // only for [[prefix::attribute]] in C23 and later
  #[strum(serialize = "::")]
  DoubleColon,
  #[strum(serialize = "[[")]
  DoubleLeftBracket,
  #[strum(serialize = "]]")]
  DoubleRightBracket,

  // currently ignored operators
  #[strum(serialize = "...")]
  Ellipsis,
  // preprocessor
  #[strum(serialize = "#")]
  Hash,
  #[strum(serialize = "##")]
  HashHash,

  #[strum(disabled)]
  #[default]
  EOF,
}
use Operator::*;
impl Operator {
  /// default precedence level when parsing expressions
  pub const DEFAULT: u8 = 0x00;
  /// when parsing function call arguments or functiondecl, use this to stop at ',' or ')'
  pub const EXCOMMA: u8 = 0x04;
  /// use this to stop before `:`, excluding the `,` in ternary operator
  pub const TERNARY: u8 = 0x06;
}
impl Operator {
  pub fn unary(&self) -> bool {
    matches!(
      self,
      // arithmetic
      Plus
        | Minus
      // logical
        | Not
      // bitwise
        | Tilde
      // dereference and address-of
        | Star
        | Ampersand
      // increment and decrement
        | PlusPlus
        | MinusMinus
    )
  }

  pub fn binary(&self) -> bool {
    matches!(
      self,
      Star
        | Slash
        | Percent
        | Plus
        | Minus
        | LeftShift
        | RightShift
        | Less
        | LessEqual
        | Greater
        | GreaterEqual
        | EqualEqual
        | NotEqual
        | Ampersand
        | Caret
        | Pipe
        | And
        | Or
        // special cases
        | Comma
    ) || self.assignment()
  }

  // left-.
  pub fn precedence(&self) -> u8 {
    debug_assert!(self.binary(), "precedence called on non-binary operator");
    match self {
      // multiplicative
      Star => 0x80,
      Slash => 0x80,
      Percent => 0x80,
      // additive
      Plus => 0x70,
      Minus => 0x70,
      // shift
      LeftShift => 0x60,
      RightShift => 0x60,
      // relational
      Less => 0x50,
      LessEqual => 0x50,
      Greater => 0x50,
      GreaterEqual => 0x50,
      // equality
      EqualEqual => 0x40,
      NotEqual => 0x40,
      // bitwise AND
      Ampersand => 0x38,
      // bitwise XOR
      Caret => 0x20,
      // bitwise OR
      Pipe => 0x18,
      // logical AND
      And => 0x10,
      // logical OR
      Or => 0x08,
      // Question mark: 0x06,
      // assignment - it's a trick since it's mostly right associative
      _ if self.assignment() => 0x04,
      // comma operator
      Comma => 0x02,
      _ => unreachable!(),
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
  /// applies to integral, floating-point, and pointer types
  Logical,
  /// integer op integer
  Bitwise,
  /// integer op unsigned integer
  BitShift,
  /// all
  Arithmetic,
  /// all
  Relational,

  /// hmm...
  Assignment,
  /// ditto...
  Comma,
}
impl Operator {
  pub fn category(&self) -> Category {
    match self {
      And | Or | Not => Category::Logical,

      LeftShift | RightShift => Category::BitShift,

      Tilde | Ampersand | Pipe | Caret => Category::Bitwise,

      Plus | Minus | Star | Slash | Percent => Category::Arithmetic,

      Less | LessEqual | Greater | GreaterEqual | EqualEqual | NotEqual =>
        Category::Relational,

      Assign | PlusAssign | MinusAssign | StarAssign | SlashAssign
      | PercentAssign | AmpersandAssign | PipeAssign | CaretAssign
      | LeftShiftAssign | RightShiftAssign => Category::Assignment,

      Comma => Category::Comma,
      _ => panic!(),
    }
  }

  pub fn assignment(&self) -> bool {
    matches!(
      self,
      Assign
        | PlusAssign
        | MinusAssign
        | StarAssign
        | SlashAssign
        | PercentAssign
        | AmpersandAssign
        | PipeAssign
        | CaretAssign
        | LeftShiftAssign
        | RightShiftAssign
    )
  }

  pub fn is_arithmetic(&self) -> bool {
    matches!(self, Plus | Minus | Star | Slash | Percent)
  }
}

impl PartialEq<Operator> for &Operator {
  fn eq(&self, other: &Operator) -> bool {
    *self == other
  }
}
