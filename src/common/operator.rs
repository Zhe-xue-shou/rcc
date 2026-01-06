use strum_macros::{Display, EnumString};
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
  #[strum(serialize = "::")]
  DoubleColon, // in C this is only for [[prefix::attribute]] in C23 and later

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
  pub fn unary(&self) -> bool {
    matches!(
      self,
      Operator::Plus
        | Operator::Minus
        | Operator::Star
        | Operator::Not
        | Operator::Tilde
        | Operator::Ampersand
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
        | Operator::Assign // Comma
    )
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
      Operator::Assign => 0x04,
      // comma operator
      Operator::Comma => 0x02,
      _ => unreachable!(),
    }
  }
  pub fn is_right_associative(&self) -> bool {
    matches!(
      self,
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
