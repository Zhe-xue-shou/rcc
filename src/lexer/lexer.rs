use crate::common::keyword::Keyword;
use crate::common::operator::Operator;
use crate::common::token::{SourceLocation, Token};
use std::str::FromStr;

pub struct Lexer {
  source: String,
  chars: Vec<char>,
  byte_positions: Vec<usize>,
  cursor: usize,
  cur_line: u32,
  cur_column: u32,
  errors: Vec<String>,
}

impl Lexer {
  pub fn new(source: String) -> Self {
    let chars: Vec<char> = source.chars().collect();
    let byte_positions: Vec<usize> = source.char_indices().map(|(pos, _)| pos).collect();

    Self {
      source,
      chars,
      byte_positions,
      cursor: 0,
      cur_line: 1,
      cur_column: 1,
      errors: Vec::new(),
    }
  }
  pub fn errors(self) -> Vec<String> {
    self.errors
  }

  fn is_at_end(&self) -> bool {
    self.cursor >= self.chars.len()
  }

  fn peek(&self, offset: usize) -> char {
    self
      .chars
      .get(self.cursor + offset)
      .copied()
      .unwrap_or('\0')
  }

  fn advance_n(&mut self, offset: usize) {
    for _ in 0..offset {
      self.advance();
    }
  }

  fn advance(&mut self) -> Option<char> {
    if self.is_at_end() {
      return None;
    }

    let ch = self.chars[self.cursor];
    self.cursor += 1;

    if ch == '\n' {
      self.cur_line += 1;
      self.cur_column = 1;
    } else {
      self.cur_column += 1;
    }

    Some(ch)
  }

  fn loc(&self) -> SourceLocation {
    SourceLocation {
      line: self.cur_line,
      column: self.cur_column,
    }
  }

  fn slice_str(&self, start: usize, end: usize) -> &str {
    let byte_start = self
      .byte_positions
      .get(start)
      .copied()
      .unwrap_or(self.source.len());
    let byte_end = self
      .byte_positions
      .get(end)
      .copied()
      .unwrap_or(self.source.len());
    &self.source[byte_start..byte_end]
  }

  pub fn lex_all(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    while !self.is_at_end() {
      if let Some(token) = self.next_token() {
        tokens.push(token);
      }
    }
    tokens.push(Token::operator(Operator::EOF, self.loc()));
    tokens
  }

  fn next_token(&mut self) -> Option<Token> {
    let start = self.cursor;
    let start_loc = self.loc();

    let ch = self.advance()?;

    match ch {
      // whitespace
      ' ' | '\t' | '\r' | '\n' => None,

      // identifiers and keywords
      c if Self::is_ident_start(c) => Some(self.lex_identifier(start, start_loc)),

      // numbers
      '0'..='9' => Some(self.lex_number(start, start_loc, false)),

      // strings
      '"' => Some(self.lex_string(start_loc)),

      // dot (operator/floating point)
      '.' => {
        if self.peek(0).is_ascii_hexdigit() {
          Some(self.lex_number(start, start_loc, true))
        } else {
          Some(Token::operator(Operator::Dot, start_loc))
        }
      }

      // comments/division
      '/' => match self.peek(0) {
        '/' => {
          self.skip_line_comment();
          None
        }
        '=' => {
          self.advance();
          Some(Token::operator(Operator::SlashAssign, start_loc))
        }
        _ => Some(Token::operator(Operator::Slash, start_loc)),
      },

      // multi-character operators
      '+' => self.lex_compound_op(
        start_loc,
        Operator::Plus,
        &[("++", Operator::PlusPlus), ("+=", Operator::PlusAssign)],
      ),
      '-' => self.lex_compound_op(
        start_loc,
        Operator::Minus,
        &[
          ("--", Operator::MinusMinus),
          ("-=", Operator::MinusAssign),
          ("->", Operator::Arrow),
        ],
      ),
      '*' => self.lex_compound_op(start_loc, Operator::Star, &[("*=", Operator::StarAssign)]),
      '%' => self.lex_compound_op(
        start_loc,
        Operator::Percent,
        &[("%=", Operator::PercentAssign)],
      ),
      '=' => self.lex_compound_op(start_loc, Operator::Assign, &[("==", Operator::EqualEqual)]),
      '!' => self.lex_compound_op(start_loc, Operator::Not, &[("!=", Operator::NotEqual)]),
      '<' => self.lex_compound_op(
        start_loc,
        Operator::Less,
        &[
          ("<<=", Operator::LeftShiftAssign),
          ("<<", Operator::LeftShift),
          ("<=", Operator::LessEqual),
        ],
      ),
      '>' => self.lex_compound_op(
        start_loc,
        Operator::Greater,
        &[
          (">>=", Operator::RightShiftAssign),
          (">>", Operator::RightShift),
          (">=", Operator::GreaterEqual),
        ],
      ),
      '&' => self.lex_compound_op(
        start_loc,
        Operator::Ampersand,
        &[("&&", Operator::And), ("&=", Operator::AmpersandAssign)],
      ),
      '|' => self.lex_compound_op(
        start_loc,
        Operator::Pipe,
        &[("||", Operator::Or), ("|=", Operator::PipeAssign)],
      ),
      '^' => self.lex_compound_op(start_loc, Operator::Caret, &[("^=", Operator::CaretAssign)]),

      '#' => self.lex_compound_op(start_loc, Operator::Hash, &[("##", Operator::HashHash)]),

      // single-character operators
      ',' => Some(Token::operator(Operator::Comma, start_loc)),
      ';' => Some(Token::operator(Operator::Semicolon, start_loc)),
      '(' => Some(Token::operator(Operator::LeftParen, start_loc)),
      ')' => Some(Token::operator(Operator::RightParen, start_loc)),
      '{' => Some(Token::operator(Operator::LeftBrace, start_loc)),
      '}' => Some(Token::operator(Operator::RightBrace, start_loc)),
      '[' => Some(Token::operator(Operator::LeftBracket, start_loc)),
      ']' => Some(Token::operator(Operator::RightBracket, start_loc)),
      '~' => Some(Token::operator(Operator::Tilde, start_loc)),
      '?' => Some(Token::operator(Operator::Question, start_loc)),
      ':' => Some(Token::operator(Operator::Colon, start_loc)),

      _ => {
        self.errors.push(format!(
          "Unknown character '{}' at {}:{}",
          ch, start_loc.line, start_loc.column
        ));
        None
      }
    }
  }

  fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
  }

  fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
  }

  fn lex_identifier(&mut self, start: usize, start_loc: SourceLocation) -> Token {
    while matches!(self.peek(0), c if Self::is_ident_continue(c)) {
      self.advance();
    }

    let text = self.slice_str(start, self.cursor);

    match Keyword::from_str(text) {
      Ok(keyword) => Token::keyword(keyword, start_loc),
      Err(_) => Token::identifier(text.to_string(), start_loc),
    }
  }

  fn lex_number(
    &mut self,
    start: usize,
    start_loc: SourceLocation,
    started_with_dot: bool,
  ) -> Token {
    let base = if !started_with_dot && self.cursor > 0 {
      match (self.chars.get(start).unwrap(), self.peek(0)) {
        ('0', 'x' | 'X') => {
          self.advance();
          16
        }
        ('0', 'b' | 'B') => {
          self.advance();
          2
        }
        ('0', 'd' | 'D') => {
          self.advance();
          10
        }
        ('0', '0'..'7') => {
          self.advance();
          8
        }
        _ => 10,
      }
    } else {
      10
    };

    // digits
    while matches!(self.peek(0), c if Self::is_digit_of_base(c, base)) {
      self.advance();
    }

    // decimal point for base-10 numbers
    if base == 10 && matches!(self.peek(0), '.') && matches!(self.peek(1), ('0'..='9')) {
      self.advance(); // consume '.'
      while matches!(self.peek(0), ('0'..='9')) {
        self.advance();
      }
    }

    let text = self.slice_str(start, self.cursor);
    Token::number(text.to_string(), start_loc)
  }

  fn is_digit_of_base(c: char, base: u32) -> bool {
    match base {
      2 => matches!(c, '0' | '1'),
      8 => matches!(c, '0'..='7'),
      10 => c.is_ascii_digit(),
      16 => c.is_ascii_hexdigit(),
      _ => false,
    }
  }

  fn lex_string(&mut self, start_loc: SourceLocation) -> Token {
    let start = self.cursor;

    while !self.is_at_end() && self.peek(0) != ('"') {
      self.advance();
    }

    if self.is_at_end() {
      self.errors.push(format!(
        "Unterminated string at {}:{}",
        start_loc.line, start_loc.column
      ));
      let text = self.slice_str(start, self.cursor);
      return Token::string(text.to_string(), start_loc);
    }

    let end = self.cursor;
    self.advance(); // consume closing quote

    let text = self.slice_str(start, end);
    Token::string(text.to_string(), start_loc)
  }

  fn skip_line_comment(&mut self) {
    while !self.is_at_end() && self.peek(0) != ('\n') {
      self.advance();
    }
  }

  fn lex_compound_op(
    &mut self,
    start_loc: SourceLocation,
    default: Operator,
    patterns: &[(&str, Operator)],
  ) -> Option<Token> {
    debug_assert!(
      patterns.windows(2).all(|w| w[0].0.len() >= w[1].0.len()),
      "compound operator patterns should be sorted from longest to shortest"
    );
    // note: called after consuming the first character already.
    for (pattern, op) in patterns {
      debug_assert!(
        self.peek(pattern.chars().count() - 1) != '\0',
        "should not match past end of input 
        (if this happens, simply continue to the next pattern;
        but here I assert to catch logic errors)"
      );
      debug_assert!(
        pattern.chars().count() >= 2,
        "compound operator pattern should be >= 2 chars"
      );
      // pattern includes the first char (already consumed by next_token),
      // so compare pattern[1..] against peek(0..).
      if self.matches_ahead(pattern.chars().skip(1)) {
        self.advance_n(pattern.chars().count() - 1);
        return Some(Token::operator(op.clone(), start_loc));
      }
    }
    Some(Token::operator(default, start_loc))
  }

  fn matches_ahead(&self, pattern: impl Iterator<Item = char>) -> bool {
    pattern.enumerate().all(|(i, ch)| self.peek(i) == (ch))
  }
}
