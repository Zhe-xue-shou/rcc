use ::rc_utils::{IntoWith, contract_assert};

use crate::{
  common::{
    Error,
    ErrorData::*,
    Keyword, Literal,
    Operator::{self, *},
    OperatorCategory, SourceSpan, Storage, Token, UnitScope, Warning,
    WarningData::*,
  },
  parser::{
    declaration::{
      DeclSpecs, Declaration, Declarator, DeclaratorType, Function,
      FunctionSignature, Initializer, Modifier, Parameter, Program,
      TypeSpecifier, VarDef,
    },
    expression::{
      Binary, Call, ConstantLiteral, Expression, Paren, SizeOfKind, Ternary,
      Unary, UnprocessedType, Variable,
    },
    statement::{
      Break, Case, Compound, Continue, Default, DoWhile, For, Goto, If, Label,
      Return, Statement, Switch, While,
    },
  },
  types::{FunctionSpecifier, Qualifiers},
};
#[derive(Debug, Default)]
pub struct Parser {
  tokens: Vec<Token>,
  cursor: usize,
  errors: Vec<Error>,
  warnings: Vec<Warning>,
  loop_labels: Vec<String>,
  // contest-sensitive part - needed to parse `T * x`.
  typedefs: UnitScope,
}

/// utility functions -- allow unused to suppress those annoying warnings.
#[allow(unused)]
impl Parser {
  pub fn new(tokens: Vec<Token>) -> Self {
    assert_eq!(
      tokens.last().map(|t| &t.literal),
      Some(&Literal::Operator(EOF))
    );
    Self {
      tokens,
      ..::std::default::Default::default()
    }
  }

  pub fn parse(&mut self) -> Program {
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
  fn peek(&self) -> &Token {
    &self.tokens[self.cursor]
  }

  #[inline]
  fn peek_with_offset(&self, offset: usize) -> &Token {
    &self.tokens[self.cursor + offset]
  }

  #[inline]
  fn peek_lit(&self) -> &Literal {
    &self.tokens[self.cursor].literal
  }

  #[inline]
  fn peek_lit_with_offset(&self, offset: usize) -> &Literal {
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
  fn peek_prev_lit(&self) -> &Literal {
    &self.tokens[self.cursor - 1].literal
  }

  #[inline]
  fn peek_prev_loc(&self) -> &SourceSpan {
    &self.tokens[self.cursor - 1].location
  }

  #[inline]
  fn peek_backward_loc(&self, offset: isize) -> &SourceSpan {
    contract_assert!(
      (offset as isize) < (self.cursor as isize),
      "peek_backward_loc: offset out of bounds"
    );
    contract_assert!(
      offset.is_negative(),
      "peek_backward_loc: offset must be negative"
    );
    &self.tokens[(self.cursor as isize + offset) as usize].location
  }

  #[inline]
  fn peek_backward_lit_with_offset(&self, offset: isize) -> &Literal {
    contract_assert!(
      (offset as isize) < (self.cursor as isize),
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
        UnexpectedCharacter(self.peek_lit().clone(), Some(OP.into()))
          .into_with(*self.peek_loc()),
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
}
/// diagnostic functions
impl Parser {
  pub fn errors(&self) -> &[Error] {
    &self.errors
  }

  pub fn warnings(&self) -> &[Warning] {
    &self.warnings
  }

  fn add_error(&mut self, error: Error) {
    self.errors.push(error);
  }

  fn add_warning(&mut self, warning: Warning) {
    self.warnings.push(warning);
  }
}
/// opt checks
impl Parser {
  fn ios_c_strict_check_for_decl(&mut self, statement: &Statement) {
    if matches!(statement, Statement::Declaration(_)) {
      self.add_warning(DeprecatedStmtDeclCvt.into_with(*self.peek_loc()));
    }
  }
}
/// meta parse
impl Parser {
  fn parse_type_specifier(&mut self) -> Option<TypeSpecifier> {
    match self.peek_lit() {
      Literal::Keyword(Keyword::Struct) => todo!(),
      Literal::Keyword(Keyword::Union) => todo!(),
      Literal::Keyword(Keyword::Enum) => todo!(),
      Literal::Keyword(keyword) => TypeSpecifier::try_from(keyword).ok(),
      Literal::Identifier(ident) =>
        if self.typedefs.contains(ident) {
          Some(TypeSpecifier::Typedef(ident.to_string()))
        } else {
          None
        },
      _ => None,
    }
  }

  fn parse_function_specifier(&mut self) -> Option<FunctionSpecifier> {
    match self.peek_lit() {
      Literal::Keyword(kw) => FunctionSpecifier::try_from(kw).ok(),
      _ => None,
    }
  }

  fn parse_declspecs(&mut self) -> DeclSpecs {
    let location = *self.peek_loc();
    let mut declspecs = DeclSpecs::default();

    loop {
      if self.peek_lit().is_qualifier() {
        let qualifier = Qualifiers::from(self.peek_lit());
        // qualifiers is a bitfield
        if declspecs.qualifiers & qualifier != Qualifiers::empty() {
          self.add_warning(RedundantQualifier(qualifier).into_with(
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          ));
        } else {
          declspecs.qualifiers |= qualifier;
        }
        self.get(); // get the qualifier
      } else if self.peek_lit().is_storage_class() {
        let storage_class = Storage::from(self.peek_lit());
        match declspecs.storage_class {
          Some(ref existing_storage) if existing_storage == &storage_class => {
            self.add_warning(RedundantStorageSpecs(storage_class).into_with(
              SourceSpan {
                end: self.peek_loc().end,
                ..location
              },
            ));
          },
          Some(ref existing_storage) => {
            self.add_error(
              StorageSpecsUnmergeable(existing_storage.clone(), storage_class)
                .into_with(SourceSpan {
                  end: self.peek_loc().end,
                  ..location
                }),
            );
          },
          None => {
            declspecs.storage_class = Some(storage_class);
          },
        }
        self.get(); // get the storage class
      // 1. it's a keyword type specifier
      // 2. it's an identifier and we already have some type specifier -- break
      } else if let Some(specifier) = self.parse_type_specifier() {
        if !declspecs.type_specifiers.is_empty() {
          // already have some type specifier
          break;
        }
        declspecs.type_specifiers.push(specifier);
        self.get();
      } else if let Some(kw) = self.parse_function_specifier() {
        declspecs.function_specifiers |= kw;
        self.get();
      } else {
        break;
      }
    }

    if declspecs.type_specifiers.is_empty() {
      self.add_error(MissingTypeSpecifier.into_with(SourceSpan {
        end: self.peek_loc().end,
        ..location
      }));
      declspecs.type_specifiers.push(TypeSpecifier::Int);
    }

    declspecs
  }

  /// `TYPE`: Named: must have a name; Maybe: may have a name; Abstract: no name.
  /// `AGGRESSIVE`: if true, will try to recover from missing identifier by consuming the next token.
  fn parse_declarator<const TYPE: DeclaratorType, const AGGRESSIVE: bool>(
    &mut self,
  ) -> Declarator {
    let location = *self.peek_loc();

    let mut pointer_qualifiers = Vec::new();
    while self.peek_lit() == Star {
      self.must_get_op::<{ Star }>();

      let mut qualifier = Qualifiers::empty();
      while self.peek_lit().is_qualifier() {
        let q = Qualifiers::from(self.peek_lit());
        self.get();
        if qualifier.contains(q) {
          self.add_warning(RedundantQualifier(q).into_with(SourceSpan {
            end: self.peek_loc().end,
            ..location
          }));
        }
        qualifier |= q;
      }
      pointer_qualifiers.push(qualifier);
    }

    let name = if TYPE != DeclaratorType::Abstract {
      if let Literal::Identifier(_) = self.peek_lit() {
        let name_idx = self.get(); // consume the ident
        Some(self.tokens[name_idx].to_owned_string())
      } else {
        if TYPE == DeclaratorType::Named {
          self.add_error(
            MissingIdentifier("Expect identifier in declarator".to_string())
              .into_with(SourceSpan {
                end: self.peek_loc().end,
                ..location
              }),
          );
          if AGGRESSIVE {
            self.get();
          }
        }
        None
      }
    } else {
      None
    };
    let mut modifiers = Vec::new();
    // if the next token is '(', it's a function declarator
    if self.peek_lit() == LeftParen {
      self.must_get_op::<{ LeftParen }>();
      let parameters = self.parse_function_params();
      self.recoverable_get::<{ RightParen }>();
      modifiers.push(Modifier::Function(parameters));
    }
    for qualifiers in pointer_qualifiers.into_iter().rev() {
      modifiers.push(Modifier::Pointer(qualifiers));
    }
    Declarator::new(
      name,
      modifiers,
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }

  fn parse_argument_list(&mut self) -> Vec<Expression> {
    let location = *self.peek_loc();
    self.must_get_op::<{ LeftParen }>();
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
          )
          .into_with(SourceSpan {
            end: self.peek_loc().end,
            ..location
          }),
        );
        break;
      }
    }
    self.must_get_op::<{ RightParen }>();
    arguments
  }

  fn parse_function_params(&mut self) -> FunctionSignature {
    // C17: a function declaration without a parameter list
    //  or function body provides no information about that function’s parameters
    // but I won't support that obselete feature :(
    if self.peek_lit() == Keyword::Void {
      // single void parameter
      self.must_get_key::<{ Keyword::Void }>();
      if self.peek_lit() != RightParen {
        self.add_error(
          VoidVariableDecl(
            "Unexpected token after 'void' in parameter list".to_string(),
          )
          .into_with(*self.peek_loc()),
        );
        while self.peek_lit() != RightParen {
          self.get();
        }
      }
      FunctionSignature::default()
    } else if self.peek_lit() == RightParen {
      // empty parameter list -- assuming no parameters
      FunctionSignature::default()
    } else {
      let mut parameters = Vec::new();
      loop {
        let location = *self.peek_loc();
        let mut declspecs = self.parse_declspecs();
        let declarator =
          self.parse_declarator::<{ DeclaratorType::Maybe }, false>();
        if let Some(storage) = &declspecs.storage_class
          && storage != Storage::Register
        {
          self.add_error(ExtraneousStorageSpecs(*storage).into_with(
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          ));
          declspecs.storage_class = None;
        }
        parameters.push(Parameter::new(
          declspecs,
          declarator,
          SourceSpan {
            end: self.peek_loc().end,
            ..location
          },
        ));

        match self.peek_lit() {
          Literal::Operator(RightParen) => break,
          Literal::Operator(Comma) => {
            self.must_get_op::<{ Comma }>();
            if self.peek_lit() == RightParen {
              self.add_error(
                ExtraneousComma(
                  "Trailing comma in parameter list is not allowed in C.",
                )
                .into_with(*self.peek_loc()),
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
                )
                .into_with(*self.peek_loc()),
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

  /// common function to parse `(` expr `)`.
  fn parse_paren_expression<const LMIN_PRECEDENCE: u8>(
    &mut self,
  ) -> Expression {
    if self.peek_lit() != LeftParen {
      self.add_error(
        MissingOpenParen(self.peek_lit().clone()).into_with(*self.peek_loc()),
      );
      // assume the left paren is missing, continue parsing
    } else {
      self.must_get_op::<{ LeftParen }>();
    }
    let expr = self.next_expression(LMIN_PRECEDENCE);
    if self.peek_lit() != RightParen {
      self.add_error(
        MissingCloseParen(self.peek_lit().clone()).into_with(*self.peek_loc()),
      );
      self.get(); // get it otherwise infinite loop
    } else {
      self.must_get_op::<{ RightParen }>();
    }
    expr
  }

  fn parse_case_and_default_body(&mut self) -> Vec<Statement> {
    let mut body = Vec::new();
    while self.peek_lit() != Keyword::Case
      && self.peek_lit() != Keyword::Default
      && self.peek_lit() != RightBrace
    {
      body.push(self.next_statement());
    }
    body
  }

  fn parse_case(&mut self) -> Case {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Case }>();
    let expression = if self.peek_lit() == Colon {
      self.add_error(
        ExprNotConstant(
          "Case label must have a constant expression".to_string(),
        )
        .into_with(SourceSpan {
          end: self.peek_loc().end,
          ..location
        }),
      );
      self.must_get_op::<{ Colon }>();
      Expression::Empty
    } else {
      let expr = self.next_expression(Operator::DEFAULT);
      self.recoverable_get::<{ Colon }>();
      expr
    };
    // if it's a compound statement, we need to extract all statements until the next case/default or right brace
    // else, multiple statements until next case/default
    let body = self.parse_case_and_default_body();
    Case::new(
      expression,
      body,
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }

  fn parse_default(&mut self) -> Default {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Default }>();
    self.recoverable_get::<{ Colon }>();
    let body = self.parse_case_and_default_body();
    Default::new(
      body,
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }
}
/// declarations
impl Parser {
  fn next_vardef(
    &mut self,
    declspecs: DeclSpecs,
    declarator: Declarator,
  ) -> VarDef {
    let location = *self.peek_loc();
    let initializer = match self.peek_lit() {
      Literal::Operator(Semicolon) => {
        self.must_get_op::<{ Semicolon }>();
        None
      },
      Literal::Operator(Assign) => {
        self.must_get_op::<{ Assign }>();
        let initializer = self.next_expression(Operator::DEFAULT);
        assert_eq!(*self.peek_lit(), Literal::Operator(Semicolon));
        self.must_get_op::<{ Semicolon }>();
        Some(initializer)
      },
      _ => {
        self.add_error(
          VarDeclUnclosed("Expect ';' or '=' after variable name".to_string())
            .into_with(*self.peek_loc()),
        );
        self.get();
        None
      },
    };
    VarDef::new(
      declspecs,
      declarator,
      initializer.map(|init_expr| Initializer::Expression(init_expr.into())),
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }

  fn next_declaration(&mut self) -> Declaration {
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
      self.add_error(InvalidBlockItem.into_with(*self.peek_loc()));
      self.must_get_op::<{ LeftBrace }>();
      recovery = true;
    }

    let declspecs = self.parse_declspecs();
    let declarator = self.parse_declarator::<{ DeclaratorType::Maybe }, true>();

    if matches!(declspecs.storage_class, Some(Storage::Typedef)) {
      if let Some(name) = &declarator.name {
        self.typedefs.declare(name.clone());
      } else {
        self.add_warning(EmptyTypedef.into_with(declarator.span));
      }
      self.must_get_op::<{ Semicolon }>();
      return VarDef::new(
        declspecs,
        declarator,
        None,
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      )
      .into();
    }
    let declaration = if declarator
      .modifiers
      .iter()
      .any(|m| matches!(m, Modifier::Function(_)))
    {
      // int(void) is not allowed
      if declarator.name.is_none() {
        self.add_error(MissingFunctionName.into_with(*self.peek_loc()));
      }
      let (declspecs, declarator, body) =
        self.next_function_body(declspecs, declarator);
      Function::new(
        declspecs,
        declarator,
        body,
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      )
      .into()
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
impl Parser {
  fn next_function_body(
    &mut self,
    declspecs: DeclSpecs,
    declarator: Declarator,
  ) -> (DeclSpecs, Declarator, Option<Compound>) {
    let body = match self.peek_lit() {
      Literal::Operator(LeftBrace) => Some(self.next_block()),
      _ => {
        self.recoverable_get::<{ Semicolon }>();
        None
      },
    };

    (declspecs, declarator, body)
  }

  fn next_block(&mut self) -> Compound {
    let location = *self.peek_loc();
    self.must_get_op::<{ LeftBrace }>();
    self.typedefs.push_scope();
    let mut block = Compound::default();

    while self.peek_lit() != RightBrace {
      block.statements.push(self.next_statement());
    }
    self.typedefs.pop_scope();
    self.must_get_op::<{ RightBrace }>();
    Compound::new(
      block.statements,
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }

  fn next_return(&mut self) -> Return {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Return }>();
    let expression = if self.peek_lit() == Semicolon {
      None
    } else {
      Some(self.next_expression(Operator::DEFAULT))
    };

    assert_eq!(*self.peek_lit(), Literal::Operator(Semicolon));
    self.must_get_op::<{ Semicolon }>();
    Return::new(
      expression,
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }

  fn next_if(&mut self) -> If {
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
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    )
  }

  fn next_while(&mut self) -> While {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::While }>();
    let condition = self.parse_paren_expression::<{ Operator::DEFAULT }>();
    self
      .loop_labels
      .push(Statement::new_loop_dummy_identifier("while"));
    let body = self.next_statement();
    self.ios_c_strict_check_for_decl(&body);
    let while_stmt = While::new(
      condition,
      body.into(),
      self.loop_labels.last().unwrap().clone(),
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    );
    self.loop_labels.pop();
    while_stmt
  }

  fn next_dowhile(&mut self) -> DoWhile {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Do }>();
    self
      .loop_labels
      .push(Statement::new_loop_dummy_identifier("do_while"));
    let body = self.next_statement();
    self.ios_c_strict_check_for_decl(&body);
    self.must_get_key::<{ Keyword::While }>();
    let condition = self.parse_paren_expression::<{ Operator::DEFAULT }>();
    assert_eq!(*self.peek_lit(), Literal::Operator(Semicolon));
    self.must_get_op::<{ Semicolon }>();
    let dowhile_stmt = DoWhile::new(
      Box::new(body),
      condition,
      self.loop_labels.last().unwrap().clone(),
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    );
    self.loop_labels.pop();
    dowhile_stmt
  }

  fn next_for(&mut self) -> For {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::For }>();
    if self.peek_lit() != LeftParen {
      self.add_error(MissingOpenParen(self.peek_prev_lit().clone()).into_with(
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      ));
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
                )
                .into_with(SourceSpan {
                  end: self.peek_loc().end,
                  ..location
                }),
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
              )
              .into_with(SourceSpan {
                end: self.peek_loc().end,
                ..location
              }),
            );
            None
          },
        },
      };
      fn parse_optional_expression<const OP: Operator>(
        parser: &mut Parser,
      ) -> Option<Expression> {
        match parser.peek_lit() {
          Literal::Operator(op) if op == &OP => {
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
      self
        .loop_labels
        .push(Statement::new_loop_dummy_identifier("for"));
      let body = self.next_statement();
      self.ios_c_strict_check_for_decl(&body);
      let for_stmt = For::new(
        initializer.map(Box::new),
        condition,
        increment,
        body.into(),
        self
          .loop_labels
          .last()
          .expect("invariant: loop_labels should not be empty")
          .clone(),
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      );
      self.loop_labels.pop();
      for_stmt
    }
  }

  fn next_switch(&mut self) -> Switch {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Switch }>();
    let condition = self.parse_paren_expression::<{ Operator::EXCOMMA }>();
    self
      .loop_labels
      .push(Statement::new_loop_dummy_identifier("switch"));
    self.recoverable_get::<{ LeftBrace }>();
    let mut cases = Vec::new();
    let mut default: Option<Default> = None;
    while self.peek_lit() != RightBrace {
      match self.peek_lit() {
        Literal::Keyword(Keyword::Case) => {
          let case = self.parse_case();
          if default.is_some() {
            self.add_error(CaseLabelAfterDefault.into_with(*self.peek_loc()));
          } else {
            cases.push(case);
          }
        },
        Literal::Keyword(Keyword::Default) =>
          if default.is_some() {
            self.add_error(MultipleDefaultLabels.into_with(*self.peek_loc()));
          } else {
            default = Some(self.parse_default());
          },
        _ => {
          self.add_error(MissingLabelInSwitch.into_with(*self.peek_loc()));
          self.get(); // consume the invalid token
        },
      }
    }

    self.must_get_op::<{ RightBrace }>();
    let switch_stmt = Switch::new(
      condition,
      cases,
      default,
      self
        .loop_labels
        .last()
        .expect("invariant: loop_labels should not be empty")
        .clone(),
      SourceSpan {
        end: self.peek_loc().end,
        ..location
      },
    );
    self.loop_labels.pop();
    switch_stmt
  }

  fn next_statement(&mut self) -> Statement {
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
        self.add_error(
          LabelNotWithinSwitch(Keyword::Case).into_with(*self.peek_loc()),
        );
        // attempt to recover
        _ = self.parse_case();
        Statement::Empty()
      },
      Literal::Keyword(Keyword::Default) => {
        self.add_error(
          LabelNotWithinSwitch(Keyword::Default).into_with(*self.peek_loc()),
        );
        // ditto
        _ = self.parse_default();
        Statement::Empty()
      },
      Literal::Keyword(Keyword::Goto) => self.next_gotostmt(),
      Literal::Keyword(_) => self.next_declaration().into(),
      Literal::Identifier(ref ident) if self.typedefs.contains(ident) =>
        self.next_declaration().into(),
      Literal::Identifier(ref ident)
        if self.peek_lit_with_offset(1) == Colon =>
        self.next_labelstmt(ident.to_string()),

      _ => self.next_exprstmt().into(),
    }
  }

  fn next_labelstmt(&mut self, ident: String) -> Statement {
    let location = *self.peek_loc();
    // 1. label at end of compound statement is not allowed until C23
    // 2. label can only jump to statements within the same function, not to mention cross file.
    if self.typedefs.is_top_level() {
      self.add_error(TopLevelLabel.into_with(location));
      Statement::Empty()
    } else {
      self.get(); // consume ident
      self.must_get_op::<{ Colon }>();
      let statement = self.next_statement();
      self.ios_c_strict_check_for_decl(&statement);
      // todo: label validity check, here or in semantic analysis?
      Label::new(
        ident,
        statement,
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      )
      .into()
    }
  }

  fn next_gotostmt(&mut self) -> Statement {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Goto }>();
    if let Literal::Identifier(ident) = self.peek_lit() {
      let name = ident.to_string();
      self.get(); // consume ident
      self.recoverable_get::<{ Semicolon }>();
      Goto::new(
        name,
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      )
      .into()
    } else {
      self.add_error(MissingLabelAfterGoto.into_with(SourceSpan {
        end: self.peek_loc().end,
        ..location
      }));
      // assume the label is missing, continue parsing
      self.silent_get_if::<{ Semicolon }>();
      Statement::Empty()
    }
  }

  fn next_emptystmt(&mut self) -> Statement {
    self.must_get_op::<{ Semicolon }>();
    Statement::Empty()
  }

  fn next_exprstmt(&mut self) -> Expression {
    let expr = self.next_expression(Operator::DEFAULT);
    self.recoverable_get::<{ Semicolon }>();
    expr
  }

  fn next_break(&mut self) -> Break {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Break }>();
    self.recoverable_get::<{ Semicolon }>();
    let newloc = SourceSpan {
      end: self.peek_loc().end,
      ..location
    };
    match self.loop_labels.last() {
      Some(label) => Break::new(label.to_string(), newloc),
      None => {
        self.add_error(
          InvalidControlFlowStmt(
            "Break statement not within a loop".to_string(),
          )
          .into_with(newloc),
        );
        Break::new("invalid_loop".to_string(), newloc)
      },
    }
  }

  fn next_continue(&mut self) -> Continue {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Continue }>();

    self.recoverable_get::<{ Semicolon }>();
    // we need to handle continus differently; since the continue cannot be used to `continue` a switch.
    // search reversely for the nearest loop label which does not start with 'switch_'
    let mut found_label: Option<String> = None;
    for label in self.loop_labels.iter().rev() {
      if !label.starts_with("switch_") {
        found_label = Some(label.to_string());
        break;
      }
    }
    let newloc = SourceSpan {
      end: self.peek_loc().end,
      ..location
    };
    match found_label {
      Some(label) => Continue::new(label, newloc),
      None => {
        self.add_error(
          InvalidControlFlowStmt(
            "Continue statement not within a loop".to_string(),
          )
          .into_with(newloc),
        );
        Continue::new("invalid_loop".to_string(), newloc)
      },
    }
  }
}
/// expressions
impl Parser {
  fn next_factor(&mut self) -> Expression {
    let location = *self.peek_loc();
    self.get();
    let literal = self.peek_prev_lit();
    match literal {
      Literal::Number(num) =>
        Expression::Constant(num.clone().into_with(SourceSpan {
          end: self.peek_loc().end,
          ..location
        })),
      Literal::String(str) => Expression::Constant(
        ConstantLiteral::StringLiteral(str.clone()).into_with(SourceSpan {
          end: self.peek_loc().end,
          ..location
        }),
      ),
      Literal::Operator(op) =>
        if op.unary() {
          Unary::new(
            op.clone(),
            self.next_expression(Operator::DEFAULT),
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          )
          .into()
        } else if op == LeftParen {
          let expr = self.next_expression(Operator::DEFAULT);
          if self.peek_lit() == RightParen {
            self.get();
          } else {
            self.add_error(
              MissingCloseParen(self.peek_lit().clone()).into_with(
                SourceSpan {
                  end: self.peek_loc().end,
                  ..location
                },
              ),
            );
          }
          Paren::new(
            expr,
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          )
          .into()
        } else {
          self.add_error(
            UnexpectedCharacter(op.clone().into(), None).into_with(
              SourceSpan {
                end: self.peek_loc().end,
                ..location
              },
            ),
          );
          self.get();
          Expression::Constant(ConstantLiteral::Int(0).into_with(SourceSpan {
            end: self.peek_loc().end,
            ..location
          }))
        },
      Literal::Identifier(ident) => {
        let ident_expr = Variable::new(
          ident.to_string(),
          SourceSpan {
            end: self.peek_loc().end,
            ..location
          },
        )
        .into();
        if self.peek_lit() == LeftParen {
          let arguments = self.parse_argument_list();
          Call::new(
            ident_expr,
            arguments,
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          )
          .into()
        } else {
          ident_expr
        }
      },
      Literal::Keyword(keyword) => match keyword {
        Keyword::Sizeof => self.next_sizeof(),
        Keyword::Alignof => todo!(),
        Keyword::Alignas => todo!(),

        _ => {
          self.add_error(
            UnexpectedCharacter(keyword.clone().into(), None).into_with(
              SourceSpan {
                end: self.peek_loc().end,
                ..location
              },
            ),
          );
          Expression::Constant(ConstantLiteral::Int(0).into_with(SourceSpan {
            end: self.peek_loc().end,
            ..location
          }))
        },
      },
    }
  }

  fn next_sizeof(&mut self) -> Expression {
    let location = *self.peek_loc();
    self.must_get_key::<{ Keyword::Sizeof }>();
    // maybe type or expression, assume expression for now
    // let expr = self.parse_paren_expression();
    // Expression::SizeOf(SizeOf::Expression(Box::new(expr)))
    self.cursor -= 1;
    if self.peek_lit() == LeftParen {
      self.must_get_op::<{ LeftParen }>();
      match self.parse_type_specifier() {
        Some(_) => {
          // type
          let declspecs = self.parse_declspecs();
          let declarator =
            self.parse_declarator::<{ DeclaratorType::Abstract }, false>();
          self.recoverable_get::<{ RightParen }>();
          Expression::SizeOf(
            SizeOfKind::Type(UnprocessedType::new(declspecs, declarator))
              .into_with(SourceSpan {
                end: self.peek_loc().end,
                ..location
              }),
          )
        },
        None => {
          // expression
          let expr = self.next_expression(Operator::DEFAULT);
          self.recoverable_get::<{ RightParen }>();
          Expression::SizeOf(SizeOfKind::Expression(expr.into()).into_with(
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          ))
        },
      }
    } else {
      let expr = self.next_expression(Operator::DEFAULT);
      Expression::SizeOf(SizeOfKind::Expression(expr.into()).into_with(
        SourceSpan {
          end: self.peek_loc().end,
          ..location
        },
      ))
    }
  }

  fn next_expression(&mut self, lmin_precedence: u8) -> Expression {
    let location = *self.peek_loc();
    let mut current = self.next_factor();
    loop {
      if let Literal::Operator(op) = self.peek_lit() {
        if op.binary() && op.precedence() >= lmin_precedence {
          let operator = op.clone();
          self.get(); // operator
          let right = self.next_expression(
            if operator.category() == OperatorCategory::Assignment {
              operator.precedence()
            } else {
              operator.precedence() + 1
            },
          );
          current = Binary::from_operator_unchecked(
            operator,
            current,
            right,
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          )
          .into();
          continue;
        } else if op == Question {
          self.must_get_op::<{ Question }>();
          let then_branch = self.next_expression(Operator::DEFAULT);
          self.recoverable_get::<{ Colon }>();
          let else_branch = self.next_expression(Operator::TERNARY);
          current = Ternary::new(
            current,
            then_branch,
            else_branch,
            SourceSpan {
              end: self.peek_loc().end,
              ..location
            },
          )
          .into();
          continue;
        }
      }
      break;
    }
    current
  }
}
