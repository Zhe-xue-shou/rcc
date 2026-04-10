use ::rcc_adt::Integral;
use ::rcc_ast::{
  Session, SessionRef, UnitScope,
  types::{FunctionSpecifier, Qualifiers, TypeInfo},
};
use ::rcc_shared::{
  DiagData::{self, *},
  Diagnosis, Keyword, Literal, OpDiag,
  Operator::{self, *},
  SourceSpan, Storage, Token,
};
use ::rcc_utils::{IntoWith, StrRef, contract_assert, not_implemented_feature};

use crate::{
  declaration::{
    ArrayModifier, DeclSpecs, Declaration, Declarator,
    DeclaratorType::{self, *},
    Designated, Designator, Function, FunctionSignature, Initializer,
    InitializerList, InitializerListEntry, Modifier, Parameter, Program,
    TypeSpecifier, VarDef,
  },
  expression::{
    ArraySubscript, Binary, Call, ConstantLiteral as CL, Expression, Paren,
    SizeOfKind, Ternary, Unary, UnprocessedType, Variable,
  },
  statement::{
    Break, Case, Compound, Continue, Default, DoWhile, For, Goto, If, Label,
    Return, Statement, Switch, While,
  },
};

#[derive(Debug)]
pub struct Parser<'c> {
  tokens: Vec<Token<'c>>,
  cursor: usize,
  // contest-sensitive part - needed to parse `T * x`.
  typedefs: UnitScope<'c>,
  session: SessionRef<'c, OpDiag<'c>>,
}
impl<'a> ::std::ops::Deref for Parser<'a> {
  type Target = Session<'a, OpDiag<'a>>;

  fn deref(&self) -> &Self::Target {
    self.session
  }
}
impl<'c> Parser<'c> {
  pub fn new(
    tokens: Vec<Token<'c>>,
    session: SessionRef<'c, OpDiag<'c>>,
  ) -> Self {
    assert_eq!(
      tokens.last().map(|t| &t.literal),
      Some(&Literal::Operator(EOF))
    );
    Self {
      tokens,
      cursor: usize::default(),
      typedefs: UnitScope::new(),
      session,
    }
  }
}
#[allow(unused)]
impl<'c> Parser<'c> {
  pub fn parse(&mut self) -> Program<'c> {
    let mut program = Program::new();

    self.typedefs.push_scope(); // global scope
    while !self.is_at_end() {
      debug_assert!(self.typedefs.is_top_level());
      program.declarations.push(self.next_declaration());
    }
    self.typedefs.pop_scope();

    contract_assert!(
      self.peek_lit() == EOF,
      "expected EOF token, found: {:?}",
      self.peek_lit()
    );

    program
  }

  #[inline]
  fn is_at_end(&self) -> bool {
    self.tokens.len() <= self.cursor + 1
  }

  #[inline]
  fn peek(&self) -> &Token<'c> {
    &self.tokens[self.cursor]
  }

  #[inline]
  fn peek_with_offset(&self, offset: usize) -> &Token<'c> {
    &self.tokens[self.cursor + offset]
  }

  #[inline]
  fn peek_lit(&self) -> &Literal<'c> {
    &self.tokens[self.cursor].literal
  }

  #[inline]
  fn peek_lit_with_offset(&self, offset: usize) -> &Literal<'c> {
    &self.tokens[self.cursor + offset].literal
  }

  #[inline]
  fn peek_loc(&self) -> &SourceSpan {
    &self.tokens[self.cursor].location
  }

  #[inline]
  fn peek_loc_with_offset(&self, offset: usize) -> &SourceSpan {
    if self.is_at_end() {
      &self.tokens[self.cursor].location
    } else {
      &self.tokens[self.cursor + offset].location
    }
  }

  #[inline]
  fn peek_prev_lit(&self) -> &Literal<'c> {
    &self.tokens[self.cursor - 1].literal
  }

  #[inline]
  fn peek_prev_loc(&self) -> &SourceSpan {
    &self.tokens[self.cursor - 1].location
  }

  #[inline]
  fn peek_backward_loc(&self, offset: isize) -> &SourceSpan {
    contract_assert!(
      offset < (self.cursor as isize),
      "peek_backward_loc: offset out of bounds"
    );
    contract_assert!(
      offset.is_negative(),
      "peek_backward_loc: offset must be negative"
    );
    &self.tokens[(self.cursor as isize + offset) as usize].location
  }

  #[inline]
  fn peek_backward_lit_with_offset(&self, offset: isize) -> &Literal<'c> {
    contract_assert!(
      offset < (self.cursor as isize),
      "peek_backward_lit: offset out of bounds"
    );
    contract_assert!(
      offset.is_negative(),
      "peek_backward_lit: offset must be negative"
    );
    &self.tokens[(self.cursor as isize + offset) as usize].literal
  }

  /// identical to `self.get()`, but will panic if the next token is not KEY. useful for debugging.
  #[inline]
  fn must_get_key<const KEY: Keyword>(&mut self) -> usize {
    let index = self.get();
    contract_assert!(
      !matches!(&self.tokens[index].literal, Literal::Keyword(kw) if kw != KEY),
      "expected: {:?}, found: {:?}",
      KEY,
      self.tokens[index].literal
    );
    index
  }

  /// ditto; consume and return the index of the token if it's OP; else, panic.
  #[inline]
  fn must_get_op<const OP: Operator>(&mut self) -> usize {
    let index = self.get();
    contract_assert!(
      !matches!(&self.tokens[index].literal, Literal::Operator(op) if op != OP),
      "expected: {:?}, found: {:?}",
      OP,
      self.tokens[index].literal
    );
    index
  }

  #[inline]
  fn get(&mut self) -> usize {
    self.get_with_offset(1)
  }

  #[inline]
  fn get_with_offset(&mut self, offset: usize) -> usize {
    assert!(self.cursor < self.tokens.len());
    let index = self.cursor;
    self.cursor += offset;
    index
  }

  /// if the next token is OP, consume it; else, report an error - but does not consume it.
  fn recoverable_get<const OP: Operator>(&mut self) {
    if self.peek_lit() != OP {
      self.add_error(
        UnexpectedCharacter(
          (self.peek_lit().to_string(), Some(OP.to_string())).into(),
        ),
        *self.peek_loc(),
      );
    } else {
      self.must_get_op::<OP>();
    }
  }

  /// get if the next token is OP; otherwise, do nothing.
  fn silent_get_if<const OP: Operator>(&mut self) {
    if self.peek_lit() == OP {
      self.must_get_op::<OP>();
    }
  }

  /// end-location.
  #[inline(always)]
  fn eloc(&self, location: SourceSpan) -> SourceSpan {
    SourceSpan {
      end: self.peek_loc().end,
      ..location
    }
  }
}
/// diagnostic functions
impl<'c> Parser<'c> {
  fn add_error(&self, data: DiagData<'c>, span: SourceSpan) {
    self.diag().add_error(data, span);
  }

  fn add_warning(&self, data: DiagData<'c>, span: SourceSpan) {
    self.diag().add_warning(data, span);
  }
}
/// opt checks
impl<'c> Parser<'c> {
  fn ios_c_strict_check_for_decl(&self, statement: &Statement) {
    if self.ast().langopts() < 23
      && matches!(statement, Statement::Declaration(_))
    {
      self.add_warning(DeprecatedStmtDeclCvt, *self.peek_loc());
    }
  }
}
/// meta parse
impl<'c> Parser<'c> {
  #[inline]
  fn parse_type_specifier(&self) -> Option<TypeSpecifier<'c>> {
    self.parse_type_specifier_with_offset(0)
  }

  fn parse_type_specifier_with_offset(
    &self,
    offset: usize,
  ) -> Option<TypeSpecifier<'c>> {
    match self.peek_lit_with_offset(offset) {
      Literal::Keyword(Keyword::Struct) => todo!(),
      Literal::Keyword(Keyword::Union) => todo!(),
      Literal::Keyword(Keyword::Enum) => todo!(),
      Literal::Keyword(Keyword::Auto) =>
        if self.ast().langopts() >= 23 {
          Some(TypeSpecifier::AutoType)
        } else {
          None
        },
      Literal::Keyword(keyword) => TypeSpecifier::try_from(keyword).ok(),
      Literal::Identifier(ident) =>
        if self.typedefs.contains(ident) {
          Some(TypeSpecifier::Typedef(ident))
        } else {
          None
        },
      _ => None,
    }
  }

  fn parse_function_specifier(&self) -> Option<FunctionSpecifier> {
    FunctionSpecifier::try_from(self.peek_lit()).ok()
  }

  fn parse_qualifier(&mut self) -> Qualifiers {
    debug_assert!(
      self.peek_lit().is_qualifier(),
      "parse_qualifier called on non-qualifier token: {:?}",
      self.peek_lit()
    );
    let qualifier = Qualifiers::from(self.peek_lit());
    self.get(); // get the qualifier
    qualifier
  }

  fn parse_qualifiers(&mut self) -> Qualifiers {
    let mut qualifiers = Qualifiers::empty();
    while self.peek_lit().is_qualifier() {
      let qualifier = self.parse_qualifier();
      if qualifiers.contains(qualifier) {
        self.add_warning(
          RedundantQualifier(qualifier.to_string()),
          *self.peek_loc(),
        );
      }
      qualifiers |= qualifier;
    }
    qualifiers
  }

  fn parse_declspecs(&mut self) -> DeclSpecs<'c> {
    let location = *self.peek_loc();

    let mut qualifiers = Qualifiers::empty();
    let mut storage: Option<Storage> = None;
    let mut type_specifiers = Vec::new();
    let mut function_specifiers = FunctionSpecifier::empty();

    loop {
      if self.peek_lit().is_qualifier() {
        let qualifier = self.parse_qualifier();
        // qualifiers is a bitfield
        if qualifiers.contains(qualifier) {
          self.add_warning(
            RedundantQualifier(qualifier.to_string()),
            self.eloc(location),
          );
        } else {
          qualifiers |= qualifier;
        }
      } else if let Ok(storage_class) = Storage::try_from(self.peek_lit())
        && (!matches!(storage_class, Storage::Automatic)
          || (self.ast().langopts() >= 23
            && self.parse_type_specifier_with_offset(1).is_some()))
      {
        match storage {
          Some(ref existing_storage) if existing_storage == storage_class => {
            self.add_warning(
              RedundantStorageSpecs(storage_class),
              self.eloc(location),
            );
          },
          Some(ref existing_storage) => {
            self.add_error(
              StorageSpecsUnmergeable(*existing_storage, storage_class),
              self.eloc(location),
            );
          },
          None => {
            storage = Some(storage_class);
          },
        }
        self.get(); // get the storage class
      // it's a bit tricky to parse type specifiers here
      } else if let Some(specifier) = self.parse_type_specifier() {
        if type_specifiers.is_empty() && !specifier.is_builtin() {
          type_specifiers.push(specifier);
          self.get();
          break;
        } else if !specifier.is_builtin() {
          break;
        } else {
          type_specifiers.push(specifier);
          self.get();
        }
      } else if let Some(kw) = self.parse_function_specifier() {
        function_specifiers |= kw;
        self.get();
      } else {
        break;
      }
    }

    if type_specifiers.is_empty() {
      self.add_error(MissingTypeSpecifier, self.eloc(location));
      type_specifiers.push(TypeSpecifier::Int);
    }

    DeclSpecs::new(
      storage,
      qualifiers,
      type_specifiers,
      function_specifiers,
      self.eloc(location),
    )
  }

  /// `TYPE`: [`DeclaratorType`]
  ///
  /// - [`Named`]: normal declarator, must have a name;
  /// - [`Maybe`]: may have a name;
  /// - [`Abstract`]: abstract-declarator, no name.
  ///
  /// `AGGRESSIVE`: [`bool`] if true, will try to recover from missing identifier by consuming the next token.
  fn parse_declarator<const TYPE: DeclaratorType, const AGGRESSIVE: bool>(
    &mut self,
  ) -> Declarator<'c> {
    let location = *self.peek_loc();

    let mut pointers = Vec::new();

    while self.peek_lit() == Star {
      self.must_get_op::<{ Star }>();

      pointers.push(Modifier::Pointer(self.parse_qualifiers()));
    }

    let (name, mut modifiers) = if self.peek_lit() == LeftParen
    // FIXME: i didnt know it is ok to check this cond?
      && !self.is_function_params_ahead()
    {
      self.must_get_op::<{ LeftParen }>();
      let inner_declarator = self.parse_declarator::<TYPE, AGGRESSIVE>();
      self.recoverable_get::<{ RightParen }>();
      (inner_declarator.name, inner_declarator.modifiers)
    } else {
      let name = if TYPE != Abstract {
        if let Literal::Identifier(ident) = *self.peek_lit() {
          self.get(); // consume the ident
          Some(ident)
        } else {
          if TYPE == Named {
            self.add_error(
              MissingIdentifier("Expect identifier in declarator".to_string()),
              self.eloc(location),
            );
            if const { AGGRESSIVE } {
              self.get();
            }
          }
          None
        }
      } else {
        None
      };
      (name, Vec::new())
    };
    while matches!(
      self.peek_lit(),
      Literal::Operator(LeftParen) | Literal::Operator(LeftBracket)
    ) {
      if self.peek_lit() == LeftParen {
        self.must_get_op::<{ LeftParen }>();
        let parameters = self.parse_function_params();
        self.recoverable_get::<{ RightParen }>();
        modifiers.push(Modifier::Function(parameters));
      }
      if self.peek_lit() == LeftBracket {
        self.must_get_op::<{ LeftBracket }>();
        let array_modifier = self.parse_array_declarator();
        self.recoverable_get::<{ RightBracket }>();
        modifiers.push(Modifier::Array(array_modifier));
      }
    }
    modifiers.extend(pointers.into_iter().rev());
    Declarator::new(name, modifiers, self.eloc(location))
  }

  fn parse_array_declarator(&mut self) -> ArrayModifier<'c> {
    let location = *self.peek_loc();

    if self.peek_lit() == RightBracket {
      return ArrayModifier::new(
        Qualifiers::empty(),
        false,
        None,
        self.eloc(location),
      );
    }
    let mut is_static = false;
    if self.peek_lit() == Keyword::Static {
      self.must_get_key::<{ Keyword::Static }>();
      is_static = true;
    }
    let qualifiers_front = self.parse_qualifiers();

    if self.peek_lit() == Keyword::Static {
      self.must_get_key::<{ Keyword::Static }>();
      if is_static {
        // clang chooses to report an error here directly, nonetheless I choose to warn only.
        self.add_warning(
          RedundantStorageSpecs(Storage::Static),
          *self.peek_loc(),
        );
      } else {
        is_static = true;
      }
    }

    let qualifiers_back = self.parse_qualifiers();

    // clang also reports error if both `front` and `back` qualifiers are present, I choose to merge them.
    let qualifiers = qualifiers_front | qualifiers_back;
    let bound = if let Literal::Operator(Star) = self.peek_lit() {
      self.must_get_op::<{ Star }>();
      None
    } else if self.peek_lit() == RightBracket {
      None
    } else {
      let expr = self.next_expression(Operator::DEFAULT);
      Some(expr)
    };
    if is_static && bound.is_none() {
      self.add_error(StaticArrayWithoutBound, self.eloc(location));
    }
    ArrayModifier::new(qualifiers, is_static, bound, self.eloc(location))
  }

  /// parse function parameter list, use [`Parser::_parse_argument_list`] for function call.
  fn parse_function_params(&mut self) -> FunctionSignature<'c> {
    // (functionnoproto type, deprecated in C23) a function declaration without a parameter list
    //  or function body provides no information about that function’s parameters
    // but I won't support that obselete feature :(
    if self.peek_lit() == Keyword::Void
      && self.peek_lit_with_offset(1) == RightParen
    {
      // single void parameter
      self.must_get_key::<{ Keyword::Void }>();
      FunctionSignature::default()
    } else if self.peek_lit() == RightParen {
      // empty parameter list -- assuming no parameters
      FunctionSignature::default()
    } else {
      let mut parameters = Vec::new();
      loop {
        let location = *self.peek_loc();
        let mut declspecs = self.parse_declspecs();
        let declarator = self.parse_declarator::<{ Maybe }, false>();
        if let Some(storage) = &declspecs.storage_class
          && storage != Storage::Register
        {
          self.add_error(ExtraneousStorageSpecs(*storage), self.eloc(location));
          declspecs.storage_class = None;
        }
        parameters.push(Parameter::new(
          declspecs,
          declarator,
          self.eloc(location),
        ));

        match self.peek_lit() {
          Literal::Operator(RightParen) => break,
          Literal::Operator(Comma) => {
            self.must_get_op::<{ Comma }>();
            if self.peek_lit() == RightParen {
              self.add_error(
                ExtraneousComma(
                  "Trailing comma in parameter list is not allowed in C.",
                ),
                *self.peek_loc(),
              );
              break;
            }
          },
          _ => {
            if self.parse_type_specifier().is_none() {
              self.add_error(
                UnclosedParameterList(
                  "Expect ',', ')' or type specifier in parameter list"
                    .to_string(),
                ),
                *self.peek_loc(),
              );
              break;
            }
            // continuing parsing
          },
        }
      }
      FunctionSignature::new(parameters, false)
    }
  }

  /// distinguish between function params and parenthesized declarator.
  ///
  /// becayse: `direct_declarator := '(' declarator ')' | ...`
  fn is_function_params_ahead(&self) -> bool {
    if self.peek_lit_with_offset(1) == RightParen {
      // ()
      true
    } else if self.peek_lit_with_offset(1) == Keyword::Void
      && self.peek_lit_with_offset(2) == RightParen
    {
      // (void)
      true
    } else {
      self.parse_type_specifier().is_some()
    }
  }

  /// parse argument list, assuming the left paren has been consumed.
  ///
  /// does **NOT** consume the right paren -- caller should check and consume it.
  fn parse_argument_list_inner(&mut self) -> Vec<Expression<'c>> {
    let location = *self.peek_loc();
    let mut arguments = Vec::new();

    while self.peek_lit() != RightParen {
      // parse expression
      let expr = self.next_expression(Operator::EXCOMMA);
      arguments.push(expr);
      if self.peek_lit() == RightParen {
        break;
      }
      self.recoverable_get::<{ Comma }>();
      if self.peek_lit() == RightParen {
        self.add_error(
          ExtraneousComma(
            "Trailing comma in argument list is not allowed in C.",
          ),
          self.eloc(location),
        );
        break;
      }
    }
    arguments
  }

  /// parse argument list in **function call**, not function declaration.
  ///
  /// for function declaration, use [`Parser::parse_function_params`].
  fn _parse_argument_list(&mut self) -> Vec<Expression<'c>> {
    self.must_get_op::<{ LeftParen }>();
    let arguments = self.parse_argument_list_inner();
    self.must_get_op::<{ RightParen }>();
    arguments
  }

  /// common function to parse `(` expr `)`.
  fn parse_paren_expression<const LMIN_PRECEDENCE: u8>(
    &mut self,
  ) -> Expression<'c> {
    if self.peek_lit() != LeftParen {
      self
        .add_error(MissingOpenParen(self.peek_lit().clone()), *self.peek_loc());
      // assume the left paren is missing, continue parsing
    } else {
      self.must_get_op::<{ LeftParen }>();
    }
    let expr = self.next_expression(LMIN_PRECEDENCE);
    if self.peek_lit() != RightParen {
      self.add_error(
        MissingCloseParen(self.peek_lit().clone()),
        *self.peek_loc(),
      );
      self.get(); // get it otherwise infinite loop
    } else {
      self.must_get_op::<{ RightParen }>();
    }
    expr
  }

  fn parse_case_and_default_body(&mut self) -> Vec<Statement<'c>> {
    let mut body = Vec::new();
    while self.peek_lit() != Keyword::Case
      && self.peek_lit() != Keyword::Default
      && self.peek_lit() != RightBrace
    {
      body.push(self.next_statement());
    }
    body
  }

  fn parse_case(&mut self) -> Case<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Case }>();
    let expression = if self.peek_lit() == Colon {
      self.add_error(
        ExprNotConstant(
          "Case label must have a constant expression".to_string(),
        ),
        self.eloc(location),
      );
      self.must_get_op::<{ Colon }>();
      Expression::default()
    } else {
      let expr = self.next_expression(Operator::DEFAULT);
      self.recoverable_get::<{ Colon }>();
      expr
    };
    // if it's a compound statement, we need to extract all statements until the next case/default or right brace
    // else, multiple statements until next case/default
    let body = self.parse_case_and_default_body();
    Case::new(expression, body, self.eloc(location))
  }

  fn parse_default(&mut self) -> Default<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Default }>();
    self.recoverable_get::<{ Colon }>();
    let body = self.parse_case_and_default_body();
    Default::new(body, self.eloc(location))
  }
}
/// declarations
impl<'c> Parser<'c> {
  fn next_initializer(&mut self) -> Initializer<'c> {
    match self.peek_lit() {
      Literal::Operator(LeftBrace) => self.next_initializer_list().into(),
      _ => self.next_expression(Operator::EXCOMMA).into(),
    }
  }

  fn next_initializer_list(&mut self) -> InitializerList<'c> {
    let location = *self.peek_loc();
    self.must_get_op::<{ LeftBrace }>();
    let mut entries = Vec::default();
    while self.peek_lit() != RightBrace {
      entries.push(self.next_initializer_list_entry());
      if self.peek_lit() != RightBrace {
        self.recoverable_get::<{ Comma }>();
      }
    }
    self.silent_get_if::<{ Comma }>();
    self.must_get_op::<{ RightBrace }>();

    InitializerList::new(entries, self.eloc(location))
  }

  fn next_initializer_list_entry(&mut self) -> InitializerListEntry<'c> {
    let mut designators = Vec::default();

    let location = *self.peek_loc();
    loop {
      match self.peek_lit() {
        Literal::Operator(Dot) =>
          designators.push(self.next_field_designator()),
        Literal::Operator(LeftBracket) =>
          designators.push(self.next_index_designator()),
        _ => break,
      }
    }
    if !designators.is_empty() {
      self.recoverable_get::<{ Assign }>();
    }

    let initializer = self.next_initializer();

    if designators.is_empty() {
      initializer.into()
    } else {
      Designated::new(designators, initializer, self.eloc(location)).into()
    }
  }

  fn next_field_designator(&mut self) -> Designator<'c> {
    self.must_get_op::<{ Dot }>();
    let ident = if let Literal::Identifier(ident) = *self.peek_lit() {
      self.get();
      ident
    } else {
      // todo: fix here shall not give it a 'static str
      while self.peek_lit() != Assign {
        self.get();
      }
      "unnamed"
    };
    Designator::Field(ident)
  }

  fn next_index_designator(&mut self) -> Designator<'c> {
    self.must_get_op::<{ LeftBracket }>();
    let expression = self.next_expression(Operator::DEFAULT);
    self.recoverable_get::<{ RightBracket }>();
    Designator::Index(expression)
  }

  fn next_vardef(
    &mut self,
    declspecs: DeclSpecs<'c>,
    declarator: Declarator<'c>,
  ) -> VarDef<'c> {
    let location = *self.peek_loc();
    let initializer = match self.peek_lit() {
      Literal::Operator(Semicolon) => {
        self.must_get_op::<{ Semicolon }>();
        None
      },
      Literal::Operator(Assign) => {
        self.must_get_op::<{ Assign }>();
        let initializer = self.next_initializer();
        self.recoverable_get::<{ Semicolon }>();
        Some(initializer)
      },
      _ => {
        self.add_error(
          VarDeclUnclosed("Expect ';' or '=' after variable name".to_string()),
          *self.peek_loc(),
        );
        self.get();
        None
      },
    };
    VarDef::new(declspecs, declarator, initializer, self.eloc(location))
  }

  fn next_declaration(&mut self) -> Declaration<'c> {
    while matches!(
      self.peek_lit(),
      Literal::Operator(Semicolon) | Literal::Operator(Hash)
    ) {
      if self.peek_lit() == Semicolon {
        // Redundant ';', maybe a warning?
        self.must_get_op::<{ Semicolon }>();
      } else {
        // // skip preprocessor directive
        // let line = self.tokens[self.cursor].location.line;
        // while (!self.is_at_end())
        //   && (self.tokens[self.cursor].location.line == line)
        // {
        //   self.get();
        // }
      }
    }
    let location = *self.peek_loc();
    let mut recovery = false;
    // block definition is not allowed in top
    if self.peek_lit() == LeftBrace {
      self.add_error(InvalidBlockItem, *self.peek_loc());
      self.must_get_op::<{ LeftBrace }>();
      recovery = true;
    }

    let declspecs = self.parse_declspecs();
    let declarator = self.parse_declarator::<{ Maybe }, true>();

    if matches!(declspecs.storage_class, Some(Storage::Typedef)) {
      if let Some(name) = declarator.name {
        self.typedefs.declare(name);
      } else {
        self.add_warning(EmptyTypedef, declarator.span);
      }
      self.must_get_op::<{ Semicolon }>();
      return VarDef::new(declspecs, declarator, None, self.eloc(location))
        .into();
    }
    let declaration =
      if matches!(declarator.modifiers.first(), Some(Modifier::Function(_))) {
        // int(void) is not allowed
        if declarator.name.is_none() {
          self.add_error(MissingFunctionName, *self.peek_loc());
        }
        let (declspecs, declarator, body) =
          self.next_function_body(declspecs, declarator);
        Function::new(declspecs, declarator, body, self.eloc(location)).into()
      } else {
        // `int;` is allowed although useless
        self.next_vardef(declspecs, declarator).into()
      };
    if recovery {
      self.recoverable_get::<{ RightBrace }>();
    }
    declaration
  }
}
/// statements
impl<'c> Parser<'c> {
  fn next_function_body(
    &mut self,
    declspecs: DeclSpecs<'c>,
    declarator: Declarator<'c>,
  ) -> (DeclSpecs<'c>, Declarator<'c>, Option<Compound<'c>>) {
    let body = match self.peek_lit() {
      Literal::Operator(LeftBrace) => Some(self.next_block()),
      _ => {
        self.recoverable_get::<{ Semicolon }>();
        None
      },
    };

    (declspecs, declarator, body)
  }

  fn next_block(&mut self) -> Compound<'c> {
    let location = *self.peek_loc();
    self.must_get_op::<{ LeftBrace }>();
    self.typedefs.push_scope();
    let mut block = Compound::default();

    while self.peek_lit() != RightBrace {
      block.statements.push(self.next_statement());
    }
    self.typedefs.pop_scope();
    self.must_get_op::<{ RightBrace }>();
    Compound::new(block.statements, self.eloc(location))
  }

  fn next_return(&mut self) -> Return<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Return }>();
    let expression = if self.peek_lit() == Semicolon {
      None
    } else {
      Some(self.next_expression(Operator::DEFAULT))
    };

    assert_eq!(self.peek_lit(), Literal::Operator(Semicolon));
    self.must_get_op::<{ Semicolon }>();
    Return::new(expression, self.eloc(location))
  }

  fn next_if(&mut self) -> If<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::If }>();
    let condition = self.parse_paren_expression::<{ Operator::DEFAULT }>();
    let then_branch = self.next_statement();
    self.ios_c_strict_check_for_decl(&then_branch);
    let else_branch = if self.peek_lit() == Keyword::Else {
      self.must_get_key::<{ Keyword::Else }>();
      let body = self.next_statement();
      self.ios_c_strict_check_for_decl(&body);
      Some(body)
    } else {
      None
    };
    If::new(
      condition,
      then_branch.into(),
      else_branch.map(Box::new),
      self.eloc(location),
    )
  }

  fn next_while(&mut self) -> While<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::While }>();
    let condition = self.parse_paren_expression::<{ Operator::DEFAULT }>();

    let body = self.next_statement();
    self.ios_c_strict_check_for_decl(&body);

    While::new(condition, body.into(), self.eloc(location))
  }

  fn next_dowhile(&mut self) -> DoWhile<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Do }>();

    let body = self.next_statement();
    self.ios_c_strict_check_for_decl(&body);
    self.must_get_key::<{ Keyword::While }>();
    let condition = self.parse_paren_expression::<{ Operator::DEFAULT }>();
    assert_eq!(*self.peek_lit(), Literal::Operator(Semicolon));
    self.must_get_op::<{ Semicolon }>();

    DoWhile::new(body.into(), condition, self.eloc(location))
  }

  fn next_for(&mut self) -> For<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::For }>();
    if self.peek_lit() != LeftParen {
      self.add_error(
        MissingOpenParen(self.peek_prev_lit().clone()),
        self.eloc(location),
      );
      panic!() // workaound
    } else {
      self.must_get_op::<{ LeftParen }>();
      // initializer
      let initializer = match self.peek_lit() {
        Literal::Operator(Semicolon) => {
          self.must_get_op::<{ Semicolon }>();
          None
        },
        _ => match self.next_statement() {
          Statement::Declaration(Declaration::Variable(vardef)) => {
            if vardef.initializer.is_none() {
              self.add_warning(
                VariableUninitialized(
                  "Variable declared in for loop without initializer"
                    .to_string(),
                ),
                self.eloc(location),
              );
            }
            Some(Statement::Declaration(vardef.into()))
          },
          Statement::Expression(expr) => Some(expr.into()),
          _ => {
            self.add_error(
              Custom(
                "Expect variable declaration or expression in for initializer"
                  .to_string(),
              ),
              self.eloc(location),
            );
            None
          },
        },
      };
      fn parse_optional_expression<'a, const OP: Operator>(
        parser: &mut Parser<'a>,
      ) -> Option<Expression<'a>> {
        match parser.peek_lit() {
          Literal::Operator(op) if op == OP => {
            parser.must_get_op::<OP>();
            None
          },
          _ => {
            let expr = parser.next_expression(Operator::DEFAULT);
            parser.must_get_op::<OP>();
            Some(expr)
          },
        }
      }
      let condition = parse_optional_expression::<{ Semicolon }>(self);
      let increment = parse_optional_expression::<{ RightParen }>(self);

      let body = self.next_statement();
      self.ios_c_strict_check_for_decl(&body);

      For::new(
        initializer.map(Into::into),
        condition,
        increment,
        body.into(),
        self.eloc(location),
      )
    }
  }

  fn next_switch(&mut self) -> Switch<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Switch }>();
    let condition = self.parse_paren_expression::<{ Operator::EXCOMMA }>();

    self.recoverable_get::<{ LeftBrace }>();
    let mut cases = Vec::new();
    let mut default: Option<Default> = None;
    while self.peek_lit() != RightBrace {
      match self.peek_lit() {
        Literal::Keyword(Keyword::Case) => {
          let case = self.parse_case();
          if default.is_some() {
            self.add_error(CaseLabelAfterDefault, *self.peek_loc());
          } else {
            cases.push(case);
          }
        },
        Literal::Keyword(Keyword::Default) =>
          if default.is_some() {
            self.add_error(MultipleDefaultLabels, *self.peek_loc());
          } else {
            default = Some(self.parse_default());
          },
        _ => {
          self.add_error(MissingLabelInSwitch, *self.peek_loc());
          self.get(); // consume the invalid token
        },
      }
    }

    self.must_get_op::<{ RightBrace }>();

    Switch::new(condition, cases, default, self.eloc(location))
  }

  fn next_statement(&mut self) -> Statement<'c> {
    match *self.peek_lit() {
      Literal::Keyword(Keyword::If) => self.next_if().into(),
      Literal::Keyword(Keyword::For) => self.next_for().into(),
      Literal::Keyword(Keyword::Return) => self.next_return().into(),
      Literal::Keyword(Keyword::While) => self.next_while().into(),
      Literal::Keyword(Keyword::Do) => self.next_dowhile().into(),
      Literal::Keyword(Keyword::Break) => self.next_break().into(),
      Literal::Keyword(Keyword::Continue) => self.next_continue().into(),
      Literal::Keyword(Keyword::Switch) => self.next_switch().into(),
      Literal::Operator(LeftBrace) => self.next_block().into(),
      Literal::Operator(Semicolon) => self.next_emptystmt(),
      Literal::Keyword(Keyword::Case) => {
        self.add_error(LabelNotWithinSwitch(Keyword::Case), *self.peek_loc());
        // attempt to recover
        _ = self.parse_case();
        Statement::default()
      },
      Literal::Keyword(Keyword::Default) => {
        self
          .add_error(LabelNotWithinSwitch(Keyword::Default), *self.peek_loc());
        // ditto
        _ = self.parse_default();
        Statement::default()
      },
      Literal::Keyword(Keyword::Goto) => self.next_gotostmt(),
      Literal::Keyword(_) => self.next_declaration().into(),
      Literal::Identifier(ident) if self.typedefs.contains(ident) =>
        self.next_declaration().into(),
      Literal::Identifier(ident) if self.peek_lit_with_offset(1) == Colon =>
        self.next_labelstmt(ident),

      _ => self.next_exprstmt().into(),
    }
  }

  fn next_labelstmt(&mut self, ident: StrRef<'c>) -> Statement<'c> {
    let location = *self.peek_loc();
    // 1. label at end of compound statement is not allowed until C23
    // 2. label can only jump to statements within the same function, not to mention cross file.
    if self.typedefs.is_top_level() {
      self.add_error(TopLevelLabel, location);
      Statement::default()
    } else {
      self.get(); // consume ident
      self.must_get_op::<{ Colon }>();
      let statement = self.next_statement();
      self.ios_c_strict_check_for_decl(&statement);
      // todo: label validity check, here or in semantic analysis?
      Label::new(ident, statement, self.eloc(location)).into()
    }
  }

  fn next_gotostmt(&mut self) -> Statement<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Goto }>();
    if let Literal::Identifier(ident) = self.peek_lit() {
      let name = *ident;
      self.get(); // consume ident
      self.recoverable_get::<{ Semicolon }>();
      Goto::new(name, self.eloc(location)).into()
    } else {
      self.add_error(MissingLabelAfterGoto, self.eloc(location));
      // assume the label is missing, continue parsing
      self.silent_get_if::<{ Semicolon }>();
      Statement::default()
    }
  }

  fn next_emptystmt(&mut self) -> Statement<'c> {
    self.must_get_op::<{ Semicolon }>();
    Statement::default()
  }

  fn next_exprstmt(&mut self) -> Expression<'c> {
    let expr = self.next_expression(Operator::DEFAULT);
    self.recoverable_get::<{ Semicolon }>();
    expr
  }

  fn next_break(&mut self) -> Break<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Break }>();
    self.recoverable_get::<{ Semicolon }>();
    Break::new(self.eloc(location))
  }

  fn next_continue(&mut self) -> Continue<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Continue }>();

    Continue::new(self.eloc(location))
  }
}
/// expressions
impl<'c> Parser<'c> {
  fn next_keyword_expr(
    &mut self,
    keyword: Keyword,
    location: SourceSpan,
  ) -> Expression<'c> {
    match keyword {
      Keyword::Sizeof => {
        self.cursor -= 1;
        self.next_sizeof()
      },

      kw @ (Keyword::Alignof | Keyword::Alignas | Keyword::Generic) =>
        not_implemented_feature!("not implemented: {kw:#?}"),

      bool_constant @ (Keyword::True | Keyword::False) => Expression::Constant(
        CL::Integral(if self.ast().langopts() >= 17 {
          Integral::from_unsigned(
            bool_constant == Keyword::True,
            self.ast().i8_bool_type().size_bits() as u8,
          )
        } else {
          Integral::from_signed(
            bool_constant == Keyword::True,
            self.ast().int_type().size_bits() as u8,
          )
        }) + self.eloc(location),
      ),
      Keyword::Nullptr => (CL::Nullptr() + self.eloc(location)).into(),
      _ => {
        self.add_error(
          UnexpectedCharacter((keyword.to_string(), None).into()),
          self.eloc(location),
        );
        (CL::Integral(Integral::default()) + self.eloc(location)).into()
      },
    }
  }

  fn next_operator_expr(
    &mut self,
    operator: Operator,
    location: SourceSpan,
  ) -> Expression<'c> {
    match operator {
      op if op.unary() => {
        let (.., r_bp) = op.prefix_binding_power();
        let rhs = self.next_expression(r_bp);
        Unary::prefix(op, rhs, self.eloc(location)).into()
      },
      LeftParen => {
        // self.cursor -= 1;
        // self.must_get_op::<{ LeftParen }>();
        let expr = self.next_expression(Operator::DEFAULT);
        self.recoverable_get::<{ RightParen }>();
        Paren::new(expr, self.eloc(location)).into()
      },
      op => {
        self.add_error(
          UnexpectedCharacter((op.to_string(), None).into()),
          self.eloc(location),
        );

        (CL::Integral(Integral::default()) + self.eloc(location)).into()
      },
    }
  }

  /// primary-expression:
  ///     - identifier
  ///     - constant
  ///     - string-literal
  ///     - ( expression )
  ///     - generic-selection
  fn next_factor(&mut self) -> Expression<'c> {
    let location = *self.peek_loc();
    let literal = self.peek_lit().clone();
    self.get();
    match literal {
      Literal::Operator(operator) =>
        self.next_operator_expr(operator, location),
      Literal::Number(num) => (num.into_with(self.eloc(location))).into(),
      Literal::String(str) => (CL::String(str) + self.eloc(location)).into(),
      Literal::Identifier(ident) =>
        Variable::new(ident, self.eloc(location)).into(),
      Literal::Keyword(keyword) => self.next_keyword_expr(keyword, location),
    }
  }

  /// this should return [`se::SizeOf`].
  fn next_sizeof(&mut self) -> Expression<'c> {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Sizeof }>();
    // maybe type or expression
    if self.peek_lit() == LeftParen {
      self.must_get_op::<{ LeftParen }>();
      match self.parse_type_specifier() {
        Some(_) => {
          // type
          let declspecs = self.parse_declspecs();
          let declarator = self.parse_declarator::<{ Abstract }, false>();
          self.recoverable_get::<{ RightParen }>();
          Expression::SizeOf(
            SizeOfKind::Type(
              UnprocessedType::new(declspecs, declarator).into(),
            )
            .into_with(self.eloc(location)),
          )
        },
        None => {
          // expression
          let expr = self.next_expression(Operator::DEFAULT);
          self.recoverable_get::<{ RightParen }>();
          Expression::SizeOf(
            SizeOfKind::Expression(expr.into()).into_with(self.eloc(location)),
          )
        },
      }
    } else {
      let expr = self.next_expression(Operator::DEFAULT);
      Expression::SizeOf(
        SizeOfKind::Expression(expr.into()).into_with(self.eloc(location)),
      )
    }
  }
}
impl<'c> Parser<'c> {
  /// I refactored the expression parser to use *Pratt Parsing* technique instead of
  /// the previous precedence climbing method.
  ///
  /// In short, Pratt parsing was built
  /// on top precedence climbing and instead of using a single precedence value to
  /// decide whether to continue parsing or not, it uses two binding power values:
  /// **left binding power** and **right binding power**.
  ///
  /// The *left binding power* is much like the precedence value in precedence climbing,
  /// whereas the *right binding power* is used to decide how far to parse the right-hand side,
  /// such as in the case of right-associative operators -- which would be tricky to implement
  /// using precedence climbing. Both methods reduce the complexity of recursive descent parsing.
  ///
  /// rust analyzer also uses Pratt parsing for its expression parser; the relavent part see
  /// [here](https://github.com/rust-lang/rust-analyzer/blob/3cf298f9a92cb4fd0999859821b578bd361d5da2/crates/parser/src/grammar/expressions.rs#L246)
  /// (it's far more complicated than this one, tho).
  ///
  /// For more information, read this excellent
  /// [blog post](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html)
  /// by Matklad.
  fn next_expression(&mut self, min_bp: u8) -> Expression<'c> {
    let location = *self.peek_loc();
    let mut lhs = self.next_factor();

    loop {
      let op = match self.peek_lit() {
        Literal::Operator(Semicolon) | Literal::Operator(EOF) => break,
        Literal::Operator(op) => *op,
        _ => break,
      };

      if let Some((l_bp, ..)) = op.postfix_binding_power() {
        if l_bp < min_bp {
          break;
        }
        self.get();
        lhs = match op {
          LeftBracket => {
            let rhs = self.next_expression(Operator::DEFAULT);
            self.recoverable_get::<{ RightBracket }>();
            ArraySubscript::new(lhs, rhs, self.eloc(location)).into()
          },
          LeftParen => {
            let arguments = self.parse_argument_list_inner();
            self.recoverable_get::<{ RightParen }>();
            Call::new(lhs, arguments, self.eloc(location)).into()
          },
          op => Unary::postfix(op, lhs, self.eloc(location)).into(),
        };

        continue;
      }

      if let Some((l_bp, r_bp)) = op.infix_binding_power() {
        if l_bp < min_bp {
          break;
        }
        self.get();
        lhs = match op {
          Question if self.peek_lit() != Literal::Operator(Colon) => {
            let then_expr = self.next_expression(Operator::DEFAULT);
            self.recoverable_get::<{ Colon }>();

            let else_expr = self.next_expression(r_bp);

            Ternary::new(lhs, then_expr, else_expr, self.eloc(location)).into()
          },
          Question => {
            self.must_get_op::<{ Colon }>();
            let else_expr = self.next_expression(r_bp);
            Ternary::elvis(lhs, else_expr, self.eloc(location)).into()
          },
          Elvis => {
            let else_expr = self.next_expression(r_bp);
            Ternary::elvis(lhs, else_expr, self.eloc(location)).into()
          },
          _ => {
            let rhs = self.next_expression(r_bp);
            Binary::new(op, lhs, rhs, self.eloc(location)).into()
          },
        };
        continue;
      }

      break;
    }

    lhs
  }
}
