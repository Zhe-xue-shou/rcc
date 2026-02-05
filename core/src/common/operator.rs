#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  Default,
  ::strum_macros::Display,
  ::strum_macros::EnumString,
  ::std::marker::ConstParamTy,
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
  /// default binding power for postfix operators.
  ///
  /// Also, use this to ensure that postfix operators bind more tightly than any infix operators.
  /// like in function calls.
  pub const POSTFIX: u8 = 0xA0;
  /// use this to stop before `:`, excluding the `,` in ternary operator
  #[deprecated(note = "Based on the clang AST output, the precedence level \
                       for `? :` is lower than that of `,`. That is, `0 ? 1, \
                       2 : 3` is parsed as `0 ? (1, 2) : 3`. Use DEFAULT \
                       instead.")]
  pub const TERNARY: u8 = 0x06;
}
impl Operator {
  pub const fn unary(&self) -> bool {
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
      // increment and decrement.
        | PlusPlus
        | MinusMinus
    )
  }

  pub const fn binary(&self) -> bool {
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
        | Dot
        | Arrow
        // special cases
        | Comma
    ) || self.assignment()
  }

  pub const fn postfix(&self) -> bool {
    matches!(
      self,
      PlusPlus | MinusMinus | Dot | Arrow | LeftBracket | LeftParen
    )
  }

  pub const fn prefix_binding_power(&self) -> ((), u8) {
    debug_assert!(
      self.unary(),
      "prefix_binding_power called on non-unary operator"
    );
    let rhs = match self {
      // arithmetic
      Plus | Minus => 0x90,
      // logical
      Not => 0x90,
      // bitwise
      Tilde => 0x90,
      // indirect and address-of
      Star | Ampersand => 0x90,
      // increment and decrement.
      PlusPlus | MinusMinus => 0x90,
      _ => unreachable!(),
    };
    ((), rhs)
  }

  pub const fn postfix_binding_power(&self) -> Option<(u8, ())> {
    match self {
      PlusPlus | MinusMinus | LeftBracket | LeftParen =>
        Some((Self::POSTFIX, ())),

      _ => None,
    }
  }

  pub const fn infix_binding_power(&self) -> Option<(u8, u8)> {
    match self {
      Dot | Arrow => Some((0xC0, 0xC1)),
      // multiplicative
      Star | Slash | Percent => Some((0x80, 0x81)),
      // additive
      Plus | Minus => Some((0x70, 0x71)),
      // shift
      LeftShift | RightShift => Some((0x60, 0x61)),
      // relational
      Less | LessEqual | Greater | GreaterEqual => Some((0x50, 0x51)),
      // equality
      EqualEqual | NotEqual => Some((0x40, 0x41)),
      // bitwise AND
      Ampersand => Some((0x38, 0x39)),
      // bitwise XOR
      Caret => Some((0x20, 0x21)),
      // bitwise OR
      Pipe => Some((0x18, 0x19)),
      // logical AND
      And => Some((0x10, 0x11)),
      // logical OR
      Or => Some((0x08, 0x09)),

      // Question mark (ternary operator)
      Question => Some((0x07, 0x06)),

      // assignment
      _ if self.assignment() => Some((0x04, 0x03)),
      // comma operator
      Comma => Some((0x02, 0x01)),
      _ => None,
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
  Special,

  /// should not reach here
  Uncategorized,
}
use Category::*;
impl Operator {
  pub const fn category(&self) -> Category {
    match self {
      And | Or | Not => Logical,

      LeftShift | RightShift => BitShift,

      Tilde | Ampersand | Pipe | Caret => Bitwise,

      Plus | Minus | Star | Slash | Percent | PlusPlus | MinusMinus =>
        Arithmetic,

      Less | LessEqual | Greater | GreaterEqual | EqualEqual | NotEqual =>
        Relational,

      Assign | PlusAssign | MinusAssign | StarAssign | SlashAssign
      | PercentAssign | AmpersandAssign | PipeAssign | CaretAssign
      | LeftShiftAssign | RightShiftAssign => Assignment,

      Comma | Dot | Arrow | LeftBracket | LeftParen => Special,

      _ => Uncategorized,
    }
  }

  pub const fn assignment(&self) -> bool {
    matches!(self.category(), Assignment)
  }
}

impl PartialEq<Operator> for &Operator {
  fn eq(&self, other: &Operator) -> bool {
    *self == other
  }
}
