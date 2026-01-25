use ::std::{
  iter::Peekable,
  str::{Chars, FromStr},
};

use crate::{
  common::{
    Coordinate, Error, ErrorData, Keyword,
    Operator::{self, *},
    SourceSpan, Token,
  },
  // this isn't strictrly correct, i uses the same `Constant` type in lexer and the parser,
  //    yet the lexeer part distinguishes number and string, but the parser part does not
  types::Constant as NumberConstant,
};

pub struct Lexer<'a> {
  /// the SourceManager owns the String.
  source: &'a str,

  /// maybe 1~4 bytes
  chars: Peekable<Chars<'a>>,

  /// track position manually for Spans
  cursor: usize,

  /// report line/col errors *during* lexing
  coords: Coordinate,

  errors: Vec<Error>,
}
impl<'a> Lexer<'a> {
  pub fn new(source: &'a str) -> Self {
    Self {
      source,
      chars: source.chars().peekable(),
      cursor: usize::default(),
      coords: Default::default(),
      errors: Default::default(),
    }
  }

  pub fn errors(&self) -> &[Error] {
    &self.errors
  }

  fn add_error(&mut self, error: Error) {
    self.errors.push(error);
  }

  #[inline]
  fn advance_n(&mut self, offset: usize) {
    for _ in 0..offset {
      self.advance();
    }
  }

  /// Returns true if we are at the end
  fn is_at_end(&mut self) -> bool {
    self.chars.peek().is_none()
  }

  /// look ahead without consuming
  #[inline]
  fn peek(&mut self) -> char {
    // returns '\0' if EOF, copied
    *self.chars.peek().unwrap_or(&'\0')
  }

  /// double peek
  #[inline]
  fn peek_next(&self) -> char {
    let mut iter = self.chars.clone();
    iter.next();
    iter.next().unwrap_or('\0')
  }

  #[inline]
  fn peek_n(&self, n: usize) -> char {
    let mut iter = self.chars.clone();
    for _ in 0..n {
      iter.next();
    }
    iter.next().unwrap_or('\0')
  }

  #[inline]
  fn recall(&self) -> char {
    self.chars.clone().rev().next().expect(
      "should not fail, unless the number appears at the start of the file(?)",
    )
  }

  /// consumes the next character and updates position
  fn advance(&mut self) -> char {
    match self.chars.next() {
      Some(c) => {
        self.cursor += c.len_utf8();

        if c == '\n' {
          self.coords.line += 1;
          self.coords.column = 1;
        } else {
          self.coords.column += 1;
        }
        c
      },
      None => '\0',
    }
  }

  // /// match a specific char
  // fn advance_if(&mut self, expected: char) -> bool {
  //   if self.peek() == expected {
  //     self.advance();
  //     true
  //   } else {
  //     false
  //   }
  // }

  fn span(&self, start: usize) -> SourceSpan {
    debug_assert!(start != self.cursor, "{start}");
    SourceSpan {
      file_index: 0,
      start: start as u32,
      end: self.cursor as u32,
    }
  }

  fn slice(&self, start: usize, end: usize) -> &str {
    &self.source[start..end]
  }

  pub fn lex_all(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    while !self.is_at_end() {
      if let Some(token) = self.next_token() {
        tokens.push(token);
      }
    }
    // add 1 to form [a,a+1)
    tokens.push(Token::operator(EOF, self.span(self.cursor + 1)));
    tokens
  }

  fn next_token(&mut self) -> Option<Token> {
    let start = self.cursor;

    match self.advance() {
      // whitespace or EOF
      ' ' | '\t' | '\r' | '\n' | '\0' => None,

      // identifiers and keywords
      c if Self::is_ident_start(c) => Some(self.lex_identifier(start)),

      // numbers
      '0'..='9' => Some(self.lex_number(start, false)),

      // strings
      '"' => Some(self.lex_string(start)),

      // dot (operator/floating point)
      '.' =>
        if self.peek().is_ascii_hexdigit() {
          Some(self.lex_number(start, true))
        } else {
          self.lex_compound_op(start, Dot, &[("...", Ellipsis)])
        },

      // comments/division
      '/' => match self.peek() {
        '/' => {
          self.skip_line_comment();
          None
        },
        '*' => {
          self.advance();
          self.skip_block_comment();
          None
        },
        '=' => {
          self.advance();
          Some(Token::operator(SlashAssign, self.span(start)))
        },
        _ => Some(Token::operator(Slash, self.span(start))),
      },

      // multi-character operators
      '+' => self.lex_compound_op(
        start,
        Plus,
        &[("++", PlusPlus), ("+=", PlusAssign)],
      ),
      '-' => self.lex_compound_op(
        start,
        Minus,
        &[("--", MinusMinus), ("-=", MinusAssign), ("->", Arrow)],
      ),
      '*' => self.lex_compound_op(start, Star, &[("*=", StarAssign)]),
      '%' => self.lex_compound_op(start, Percent, &[("%=", PercentAssign)]),
      '=' => self.lex_compound_op(start, Assign, &[("==", EqualEqual)]),
      '!' => self.lex_compound_op(start, Not, &[("!=", NotEqual)]),
      '<' => self.lex_compound_op(
        start,
        Less,
        &[
          ("<<=", LeftShiftAssign),
          ("<<", LeftShift),
          ("<=", LessEqual),
        ],
      ),
      '>' => self.lex_compound_op(
        start,
        Greater,
        &[
          (">>=", RightShiftAssign),
          (">>", RightShift),
          (">=", GreaterEqual),
        ],
      ),
      '&' => self.lex_compound_op(
        start,
        Ampersand,
        &[("&&", And), ("&=", AmpersandAssign)],
      ),
      '|' =>
        self.lex_compound_op(start, Pipe, &[("||", Or), ("|=", PipeAssign)]),
      '^' => self.lex_compound_op(start, Caret, &[("^=", CaretAssign)]),
      ':' => self.lex_compound_op(start, Colon, &[("::", DoubleColon)]),
      '[' =>
        self.lex_compound_op(start, LeftBracket, &[("[[", DoubleLeftBracket)]),
      ']' =>
        self.lex_compound_op(start, RightBracket, &[("]]", DoubleRightBracket)]),

      '#' => self.lex_compound_op(start, Hash, &[("##", HashHash)]),

      // single-character operators
      ',' => Some(Token::operator(Comma, self.span(start))),
      ';' => Some(Token::operator(Semicolon, self.span(start))),
      '(' => Some(Token::operator(LeftParen, self.span(start))),
      ')' => Some(Token::operator(RightParen, self.span(start))),
      '{' => Some(Token::operator(LeftBrace, self.span(start))),
      '}' => Some(Token::operator(RightBrace, self.span(start))),
      '~' => Some(Token::operator(Tilde, self.span(start))),
      '?' => Some(Token::operator(Question, self.span(start))),
      '\\' => todo!("character literals not implemented yet"),

      _ => {
        // self.errors.push(format!(
        //   "Unknown character '{}' at {}:{}",
        //   ch, start.line, start.column
        // ));
        None
      },
    }
  }

  fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
  }

  fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
  }

  fn lex_identifier(&mut self, start: usize) -> Token {
    while matches!(self.peek(), c if Self::is_ident_continue(c)) {
      self.advance();
    }

    let text = self.slice(start, self.cursor);

    match Keyword::from_str(text) {
      Ok(keyword) => Token::keyword(keyword, self.span(start)),
      Err(_) => Token::identifier(text.to_string(), self.span(start)),
    }
  }

  fn lex_number(&mut self, start: usize, started_with_dot: bool) -> Token {
    let base = if !started_with_dot && self.cursor > 0 {
      match (self.recall(), self.peek()) {
        ('0', 'x' | 'X') => {
          self.advance();
          16
        },
        ('0', 'b' | 'B') => {
          self.advance();
          2
        },
        ('0', 'd' | 'D') => {
          self.advance();
          10
        },
        ('0', '0'..'7') => {
          self.advance();
          8
        },
        _ => 10,
      }
    } else {
      10
    };

    // digits
    while matches!(self.peek(), c if Self::is_digit_of_base(c, base)) {
      self.advance();
    }

    let mut is_floating = false;

    // decimal point for base-10 numbers
    if base == 10
      && matches!(self.peek(), '.')
      && self.peek_next().is_ascii_digit()
    {
      self.advance(); // consume '.'
      is_floating = true;
      while self.peek().is_ascii_digit() {
        self.advance();
      }
    }

    // exponent part for base-10 (e.g., 1.5e-10, 3E+5, 2e10)
    if base == 10 && matches!(self.peek(), 'e' | 'E') {
      is_floating = true;
      self.advance(); // consume 'e' or 'E'

      // optional sign
      if matches!(self.peek(), '+' | '-') {
        self.advance();
      }

      // exponent digits, required
      if !self.peek().is_ascii_digit() {
        // self.add_error("Expected digits after exponent marker".to_string());
        self.add_error(Error::new(
          self.span(start),
          ErrorData::InvalidNumberFormat(
            "Expected digits after exponent marker".to_string(),
          ),
        ));
      } else {
        while self.peek().is_ascii_digit() {
          self.advance();
        }
      }
    }

    // hexadecimal floating point exponent (e.g., 0x1.5p-3)
    if base == 16 && matches!(self.peek(), 'p' | 'P') {
      is_floating = true;
      self.advance(); // consume 'p' or 'P'

      // optional sign
      if matches!(self.peek(), '+' | '-') {
        self.advance();
      }

      if !self.peek().is_ascii_digit() {
        self.add_error(Error::new(
          self.span(start),
          ErrorData::InvalidNumberFormat(
            "Expected digits after hexadecimal exponent marker".to_string(),
          ),
        ));
      } else {
        while self.peek().is_ascii_digit() {
          self.advance();
        }
      }
    }

    let head = self.cursor;
    let num = self.slice(start, head).to_string();

    let suffix = if matches!(self.peek(), c if Self::is_ident_start(c)) {
      while matches!(self.peek(), c if Self::is_ident_start(c)) {
        self.advance();
      }
      let s = self.slice(head, self.cursor);
      match is_floating {
        true =>
          if NumberConstant::FLOATING_SUFFIXES.contains(&s) {
            Some(s)
          } else {
            self.add_error(Error::new(
              self.span(start),
              ErrorData::InvalidNumberFormat(format!(
                "Invalid floating point literal suffix '{}', ignoring",
                s
              )),
            ));
            None
          },
        false =>
          if NumberConstant::INTEGER_SUFFIXES.contains(&s) {
            Some(s)
          } else {
            self.add_error(Error::new(
              self.span(start),
              ErrorData::InvalidNumberFormat(format!(
                "Invalid integer literal suffix '{}', ignoring",
                s
              )),
            ));
            None
          },
      }
    } else {
      None
    };

    let (constant, error) = NumberConstant::parse(&num, suffix, is_floating);
    if let Some(e) = error {
      self.add_error(Error::new(
        self.span(start),
        ErrorData::InvalidNumberFormat(e),
      ));
    }

    Token::number(constant, self.span(start))
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

  fn lex_string(&mut self, start: usize) -> Token {
    while !self.is_at_end() && self.peek() != ('"') {
      self.advance();
    }

    if self.is_at_end() {
      self.add_error(Error::new(
        self.span(start),
        ErrorData::UnterminatedString,
      ));
      let text = self.slice(start, self.cursor);
      return Token::string(text.to_string(), self.span(start));
    }

    let end = self.cursor;
    self.advance(); // consume closing quote

    let text = self.slice(start, end);
    Token::string(text.to_string(), self.span(start))
  }

  fn skip_block_comment(&mut self) {
    while !self.is_at_end() {
      if self.peek() == ('*') && self.peek_next() == ('/') {
        self.advance_n(2); // consume '*/'
        break;
      } else {
        self.advance();
      }
    }
  }

  fn skip_line_comment(&mut self) {
    while !self.is_at_end() && self.peek() != ('\n') {
      self.advance();
    }
  }

  fn lex_compound_op(
    &mut self,
    start: usize,
    default: Operator,
    patterns: &'static [(&'static str, Operator)],
  ) -> Option<Token> {
    debug_assert!(
      patterns.windows(2).all(|w| w[0].0.len() >= w[1].0.len()),
      "compound operator patterns should be sorted from longest to shortest"
    );
    // note: called after consuming the first character already.
    for (pattern, op) in patterns {
      debug_assert!(
        self.peek_n(pattern.chars().count() - 1) != '\0',
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
        return Some(Token::operator(op.clone(), self.span(start)));
      }
    }
    Some(Token::operator(default, self.span(start)))
  }

  fn matches_ahead(&self, pattern: impl Iterator<Item = char>) -> bool {
    pattern.enumerate().all(|(i, ch)| self.peek_n(i) == (ch))
  }
}
