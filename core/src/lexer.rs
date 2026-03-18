use ::rcc_utils::{IntoWith, SmallString};
use ::std::{
  iter::Peekable,
  str::{Chars, FromStr},
};

use crate::{
  common::{
    Coordinate, Keyword,
    Operator::{self, *},
    SourceSpan, Token,
  },
  diagnosis::{DiagData::*, Diagnosis},
  session::{Session, SessionRef},
  // this isn't strictrly correct, i uses the same `Constant` type in lexer and the parser,
  //    yet the lexeer part distinguishes number and string, but the parser part does not
  types::Constant as NumberConstant,
};

pub struct Lexer<'c> {
  /// the [`SourceManager`](crate::common::SourceManager) owns the [`String`].
  source: &'c str,

  /// maybe 1~4 bytes
  chars: Peekable<Chars<'c>>,

  /// track position manually for Spans
  cursor: usize,

  /// report line/col errors *during* lexing
  coords: Coordinate,

  /// Context.
  session: SessionRef<'c>,
}
impl<'a> ::std::ops::Deref for Lexer<'a> {
  type Target = Session<'a>;

  fn deref(&self) -> &Self::Target {
    self.session
  }
}
impl<'c> Lexer<'c> {
  pub fn new(session: SessionRef<'c>) -> Self {
    let chars = session
      .src()
      .files
      .first()
      .unwrap()
      .source
      .chars()
      .peekable();
    Self {
      source: &session.src().files.first().unwrap().source,
      chars,
      cursor: Default::default(),
      coords: Default::default(),
      session,
    }
  }

  /// Returns true if we are at the end
  fn is_at_end(&mut self) -> bool {
    self.chars.peek().is_none()
  }

  /// look ahead without consuming. returns '\0' if EOF
  #[inline]
  fn peek(&mut self) -> &char {
    self.chars.peek().unwrap_or(&'\0')
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

  /// this is a worst case, but we only call this when we see a digit, so it should be fine in practice.
  ///
  /// Assuming it is ascii.
  #[inline]
  fn peek_back(&self) -> char {
    self.source.as_bytes()[self.cursor.saturating_sub(1)] as char
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

  #[inline]
  fn advance_n(&mut self, offset: usize) {
    for _ in 0..offset {
      self.advance();
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

  /// this is random access, fast.
  #[inline(always)]
  fn slice(&self, start: usize, end: usize) -> &str {
    &self.source[start..end]
  }

  pub fn lex(&mut self) -> Vec<Token<'c>> {
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

  fn next_token(&mut self) -> Option<Token<'c>> {
    let start = self.cursor;

    match self.advance() {
      // whitespace or EOF
      ' ' | '\t' | '\r' | '\n' | '\0' => None,

      // identifiers and keywords
      c if Self::is_ident_start(&c) => Some(self.identifier(start)),

      // numbers
      '0'..='9' => Some(self.number(start, false)),

      // strings
      '"' => self.string(start),

      // dot (operator/floating point)
      '.' =>
        if self.peek().is_ascii_hexdigit() {
          Some(self.number(start, true))
        } else {
          self.lex_compound_operator(start, Dot, &[("...", Ellipsis)])
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
      '+' => self.lex_compound_operator(
        start,
        Plus,
        &[("++", PlusPlus), ("+=", PlusAssign)],
      ),
      '-' => self.lex_compound_operator(
        start,
        Minus,
        &[("--", MinusMinus), ("-=", MinusAssign), ("->", Arrow)],
      ),
      '*' => self.lex_compound_operator(start, Star, &[("*=", StarAssign)]),
      '%' => self.lex_compound_operator(
        start,
        Percent,
        &[
          // [tab:lex.diagraph]
          ("%:%:", HashHash),
          //
          ("%=", PercentAssign),
          //
          ("%>", RightBrace),
          ("%:", Hash),
        ],
      ),
      '=' => self.lex_compound_operator(start, Assign, &[("==", EqualEqual)]),
      '!' => self.lex_compound_operator(start, Not, &[("!=", NotEqual)]),
      '<' => self.lex_compound_operator(
        start,
        Less,
        &[
          ("<<=", LeftShiftAssign),
          ("<<", LeftShift),
          ("<=", LessEqual),
          //
          ("<%", LeftBrace),
          ("<:", LeftBracket),
        ],
      ),
      '>' => self.lex_compound_operator(
        start,
        Greater,
        &[
          (">>=", RightShiftAssign),
          (">>", RightShift),
          (">=", GreaterEqual),
        ],
      ),
      '&' => self.lex_compound_operator(
        start,
        Ampersand,
        &[("&&", And), ("&=", AmpersandAssign)],
      ),
      '|' => self.lex_compound_operator(
        start,
        Pipe,
        &[("||", Or), ("|=", PipeAssign)],
      ),
      '^' => self.lex_compound_operator(start, Caret, &[("^=", CaretAssign)]),
      ':' => self.lex_compound_operator(
        start,
        Colon,
        &[
          ("::", DoubleColon),
          //
          (":>", RightBracket),
        ],
      ),
      '[' => self.lex_compound_operator(
        start,
        LeftBracket,
        &[("[[", DoubleLeftBracket)],
      ),
      ']' => self.lex_compound_operator(
        start,
        RightBracket,
        &[("]]", DoubleRightBracket)],
      ),

      '#' => self.lex_compound_operator(
        start,
        Hash,
        &[("##", HashHash), ("#@", HashAt)],
      ),

      // single-character operators
      ',' => Some(Token::operator(Comma, self.span(start))),
      ';' => Some(Token::operator(Semicolon, self.span(start))),
      '(' => Some(Token::operator(LeftParen, self.span(start))),
      ')' => Some(Token::operator(RightParen, self.span(start))),
      '{' => Some(Token::operator(LeftBrace, self.span(start))),
      '}' => Some(Token::operator(RightBrace, self.span(start))),
      '~' => Some(Token::operator(Tilde, self.span(start))),
      '?' => Some(Token::operator(Question, self.span(start))),
      '\'' => self.character(start),
      '\\' => self.line_escape(start),

      ch => {
        self.diag().add_error(
          UnexpectedCharacter((ch.to_string(), None).into()),
          self.span(start),
        );
        None
      },
    }
  }

  #[inline(always)]
  fn is_ident_start(c: &char) -> bool {
    c.is_alphabetic() || c == &'_'
  }

  #[inline(always)]
  fn is_ident_continue(c: &char) -> bool {
    c.is_alphanumeric() || c == &'_'
  }

  fn identifier(&mut self, start: usize) -> Token<'c> {
    while matches!(self.peek(), c if Self::is_ident_continue(c)) {
      self.advance();
    }

    let text = self.slice(start, self.cursor);

    match Keyword::from_str(text) {
      Ok(keyword) =>
        Token::keyword(keyword, self.span(start)).transform_alternative(),
      Err(_) =>
        Token::identifier(self.ast().intern_str(text), self.span(start)),
    }
  }

  fn number(&mut self, start: usize, started_with_dot: bool) -> Token<'c> {
    let (base, offset) = if !started_with_dot && self.cursor > 0 {
      match (self.peek_back(), self.peek()) {
        ('0', 'x' | 'X') => {
          self.advance();
          (16, 2)
        },
        ('0', 'b' | 'B') => {
          self.advance();
          (2, 2)
        },
        ('0', 'd' | 'D') => {
          self.advance();
          (10, 2)
        },
        ('0', '0'..'7') => {
          self.advance();
          (8, 2)
        },
        _ => (10, 0),
      }
    } else {
      (10, 0)
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
        self.diag().add_error(
          InvalidNumberFormat(
            "Expected digits after exponent marker".to_string(),
          ),
          self.span(start),
        );
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
        self.diag().add_error(
          InvalidNumberFormat(
            "Expected digits after hexadecimal exponent marker".to_string(),
          ),
          self.span(start),
        );
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
            self.diag().add_error(
              InvalidNumberFormat(format!(
                "Invalid floating point literal suffix '{}', ignoring",
                s
              )),
              self.span(start),
            );
            None
          },
        false =>
          if NumberConstant::INTEGER_SUFFIXES.contains(&s) {
            Some(s)
          } else {
            self.diag().add_error(
              InvalidNumberFormat(format!(
                "Invalid integer literal suffix '{}', ignoring",
                s
              )),
              self.span(start),
            );
            None
          },
      }
    } else {
      None
    };

    let (constant, error) =
      NumberConstant::parse(&num[offset..], base, suffix, is_floating);
    if let Some(e) = error {
      self.diag().add_diag(e.into_with(self.span(start)));
    }

    Token::number(constant, self.span(start))
  }

  #[inline]
  fn is_digit_of_base(c: &char, base: u32) -> bool {
    match base {
      2 => matches!(c, '0' | '1'),
      8 => matches!(c, '0'..='7'),
      10 => c.is_ascii_digit(),
      16 => c.is_ascii_hexdigit(),
      _ => false,
    }
  }

  fn line_escape(&mut self, start: usize) -> Option<Token<'c>> {
    match self.peek() {
      '\n' => {
        self.advance();
        None
      },
      ' ' => {
        self
          .session
          .diag()
          .add_warning(WhitespaceAfterLineEscape, self.span(start));
        while self.peek() == &' ' {
          self.advance();
        }
        None
      },

      _ => None,
    }
  }

  /// handles escape sequences in character literals, called after consuming the backslash.
  ///
  /// note: this is only for escape sequences in character literals, line continuation should not use this.
  fn escape_character(&mut self, start: usize) -> Option<char> {
    // 1. \n, \t, etc.
    // 2. \e \033, \x1b, ... todo.
    // 3. \U or \u -- not supported.
    match self.advance() {
      'n' => Some('\n'),
      't' => Some('\t'),
      'r' => Some('\r'),
      '\\' => Some('\\'),
      '\'' => Some('\''),
      '"' => Some('"'),
      '0' => Some('\0'),

      c => {
        self
          .session
          .diag()
          .add_error(InvalidEscapeSequence(format!("\\{c}")), self.span(start));
        None
      },
    }
  }

  /// currently only supports ascii char.
  fn character(&mut self, start: usize) -> Option<Token<'c>> {
    let mut char_content = SmallString::default();
    while !self.is_at_end() && self.peek() != &'\'' {
      let ch = if self.peek() == &'\\' {
        self.advance();
        self.escape_character(start).unwrap_or('\0')
      } else {
        self.advance()
      };
      char_content.push(ch);
    }

    if self.is_at_end() {
      self
        .session
        .diag()
        .add_error(UnterminatedString, self.span(start));
      return None;
    }

    self.advance(); // consume closing quote

    match char_content.len() {
      0 => {
        self
          .session
          .diag()
          .add_error(Custom("Expect expression".into()), self.span(start));
        None
      },
      1 => {
        let ch = char_content.chars().next().unwrap();
        Some(Token::character(ch, self.span(start)))
      },
      _ => {
        self
          .session
          .diag()
          .add_error(CharacterTooLong(char_content.into()), self.span(start));
        None
      },
    }
  }

  fn string(&mut self, start: usize) -> Option<Token<'c>> {
    while !self.is_at_end() && self.peek() != (&'"') {
      self.advance();
    }

    if self.is_at_end() {
      self
        .session
        .diag()
        .add_error(UnterminatedString, self.span(start));
      return None;
    }

    let end = self.cursor;
    self.advance(); // consume closing quote

    let text = self.slice(start, end);
    Some(Token::string(self.ast().intern_str(text), self.span(start)))
  }

  fn skip_block_comment(&mut self) {
    while !self.is_at_end() {
      if self.peek() == (&'*') && self.peek_next() == ('/') {
        self.advance_n(2); // consume '*/'
        break;
      } else {
        self.advance();
      }
    }
  }

  fn skip_line_comment(&mut self) {
    while !self.is_at_end() && self.peek() != (&'\n') {
      self.advance();
    }
  }

  fn lex_compound_operator(
    &mut self,
    start: usize,
    default: Operator,
    patterns: &'static [(&'static str, Operator)],
  ) -> Option<Token<'c>> {
    debug_assert!(
      patterns.windows(2).all(|w| w[0].0.len() >= w[1].0.len()),
      "compound operator patterns should be sorted from longest to shortest"
    );
    // note: called after consuming the first character already.
    for (pattern, op) in patterns {
      debug_assert!(
        pattern.chars().count() >= 2,
        "compound operator pattern should be >= 2 chars"
      );
      // pattern includes the first char (already consumed by next_token),
      // so compare pattern[1..] against peek(0..).
      if self.matches_ahead(pattern.chars().skip(1)) {
        self.advance_n(pattern.chars().count() - 1);
        return Some(Token::operator(*op, self.span(start)));
      }
    }
    Some(Token::operator(default, self.span(start)))
  }

  fn matches_ahead(&self, pattern: impl Iterator<Item = char>) -> bool {
    pattern.enumerate().all(|(i, ch)| self.peek_n(i) == (ch))
  }
}
