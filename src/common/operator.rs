use ::strum_macros::{Display, EnumString};
#[derive(Debug, Clone, Display, EnumString, PartialEq, Eq, ::std::marker::ConstParamTy)]
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
  EOF,
}
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
      Operator::Plus
        | Operator::Minus
        | Operator::Star
        | Operator::Not
        | Operator::Tilde
        | Operator::Ampersand
        // vvv not sure
        | Operator::PlusPlus
        | Operator::MinusMinus
    )
  }
  pub fn binary(&self) -> bool {
    matches!(
      self,
      Operator::Star
        | Operator::Slash
        | Operator::Percent
        | Operator::Plus
        | Operator::Minus
        | Operator::LeftShift
        | Operator::RightShift
        | Operator::Less
        | Operator::LessEqual
        | Operator::Greater
        | Operator::GreaterEqual
        | Operator::EqualEqual
        | Operator::NotEqual
        | Operator::Ampersand
        | Operator::Caret
        | Operator::Pipe
        | Operator::And
        | Operator::Or
        // special cases
        | Operator::Comma
    ) || self.assignment()
  }
  // left-.
  pub fn precedence(&self) -> u8 {
    debug_assert!(self.binary(), "precedence called on non-binary operator");
    match self {
      // multiplicative
      Operator::Star => 0x80,
      Operator::Slash => 0x80,
      Operator::Percent => 0x80,
      // additive
      Operator::Plus => 0x70,
      Operator::Minus => 0x70,
      // shift
      Operator::LeftShift => 0x60,
      Operator::RightShift => 0x60,
      // relational
      Operator::Less => 0x50,
      Operator::LessEqual => 0x50,
      Operator::Greater => 0x50,
      Operator::GreaterEqual => 0x50,
      // equality
      Operator::EqualEqual => 0x40,
      Operator::NotEqual => 0x40,
      // bitwise AND
      Operator::Ampersand => 0x38,
      // bitwise XOR
      Operator::Caret => 0x20,
      // bitwise OR
      Operator::Pipe => 0x18,
      // logical AND
      Operator::And => 0x10,
      // logical OR
      Operator::Or => 0x08,
      // Question mark: 0x06,
      // assignment - it's a trick since it's mostly right associative
      _ if self.assignment() => 0x04,
      // comma operator
      Operator::Comma => 0x02,
      _ => unreachable!(),
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
  Logical,
  Bitwise,
  BitShift,
  Arithmetic,
  Relational,
  Assignment,
  Comma,
}
impl Operator {
  #[rustfmt::skip]
  pub fn category(&self) -> Category {
    match self {
      Operator::And 
      | Operator::Or 
      | Operator::Not => Category::Logical,

      Operator::LeftShift
      | Operator::RightShift => Category::BitShift,

      Operator::Tilde
      | Operator::Ampersand
      | Operator::Pipe
      | Operator::Caret => Category::Bitwise,

      Operator::Plus 
      | Operator::Minus 
      | Operator::Star 
      | Operator::Slash 
      | Operator::Percent => Category::Arithmetic,

      Operator::Less
      | Operator::LessEqual
      | Operator::Greater
      | Operator::GreaterEqual
      | Operator::EqualEqual
      | Operator::NotEqual => Category::Relational,

      Operator::Assign
      | Operator::PlusAssign
      | Operator::MinusAssign
      | Operator::StarAssign
      | Operator::SlashAssign
      | Operator::PercentAssign
      | Operator::AmpersandAssign
      | Operator::PipeAssign
      | Operator::CaretAssign
      | Operator::LeftShiftAssign
      | Operator::RightShiftAssign => Category::Assignment,

      Operator::Comma => Category::Comma,
      _ => panic!(),
    }
  }
  pub fn assignment(&self) -> bool {
    matches!(self,
      Operator::Assign
      | Operator::PlusAssign
      | Operator::MinusAssign
      | Operator::StarAssign
      | Operator::SlashAssign
      | Operator::PercentAssign
      | Operator::AmpersandAssign
      | Operator::PipeAssign
      | Operator::CaretAssign
      | Operator::LeftShiftAssign
      | Operator::RightShiftAssign
    )
  }
}
