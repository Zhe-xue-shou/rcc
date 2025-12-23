use crate::{
  breakpoint,
  common::{
    environment::UnitScope,
    keyword::Keyword,
    operator::Operator,
    token::{Literal, Token},
    types::Qualifiers,
  },
  parser::{
    declaration::{
      DeclSpecs, Declaration, Declarator, DeclaratorType, Function, FunctionSignature,
      FunctionSpecifier, Initializer, Modifier, Parameter, Program, Storage, TypeSpecifier, VarDef,
    },
    expression::{Binary, Call, Constant, Expression, Ternary, Unary, Variable},
    statement::{
      Break, Case, Compound, Continue, Default, DoWhile, For, Goto, If, Label, Return, Statement,
      Switch, While, new_loop_dummy_identifier,
    },
  },
};
#[cfg(test)]
use pretty_assertions::assert_eq;

use crate::parser::Parser;

/// utility functions
impl Parser {
  pub fn new(tokens: Vec<Token>) -> Self {
    assert_eq!(
      tokens.last().map(|t| &t.literal),
      Some(&Literal::Operator(Operator::EOF))
    );
    Self {
      tokens,
      cursor: 0,
      errors: Vec::new(),
      warnings: Vec::new(),
      loop_labels: Vec::new(),
      typedefs: UnitScope::new(),
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

    program
  }
  fn is_at_end(&self) -> bool {
    self.tokens.len() <= self.cursor + 1
  }
  fn peek(&self, offset: usize) -> &Literal {
    if self.is_at_end() {
      // breakpoint!(
      //   "check your code! cursor: {}, current token: {:} ",
      //   self.cursor,
      //   self.tokens[self.cursor - 1]
      // );
      // panic!();
      &self.tokens[self.cursor].literal
    } else {
      &self.tokens[self.cursor + offset].literal
    }
  }
  fn must_get_key<const KEY: Keyword>(&mut self) -> usize {
    let index = self.get();
    if matches!(&self.tokens[index].literal, Literal::Keyword(kw) if kw != &KEY) {
      breakpoint!(
        "check your code! expected: {:?}, found: {:?}",
        KEY,
        self.tokens[index].literal
      );
      panic!()
    }
    index
  }
  fn must_get_op<const OP: Operator>(&mut self) -> usize {
    let index = self.get();
    if matches!(&self.tokens[index].literal, Literal::Operator(op) if *op != OP) {
      breakpoint!(
        "check your code! expected: {:?}, found: {:?}",
        OP,
        self.tokens[index].literal
      );
      panic!()
    }
    index
  }
  fn get(&mut self) -> usize {
    self.get_with_offset(1)
  }
  fn get_with_offset(&mut self, offset: usize) -> usize {
    assert!(self.cursor < self.tokens.len());
    let index = self.cursor;
    self.cursor += offset;
    index
  }
  fn recoverable_get<const OP: Operator>(&mut self) {
    if *self.peek(0) != Literal::Operator(OP) {
      self.add_error(format!("Expect '{}' ", OP));
    } else {
      self.must_get_op::<OP>();
    }
  }
  fn silent_get_if<const OP: Operator>(&mut self) {
    if *self.peek(0) == Literal::Operator(OP) {
      self.must_get_op::<OP>();
    }
  }
  // fn recoverable_consume<const OP: Operator>(&mut self) {
  //   if *self.peek(0) == Literal::Operator(OP) {
  //     self.must_get_op::<OP>();
  //   } else {
  //     self.errors.push(format!(
  //       "Expect '{}' at {}:{}.",
  //       OP, self.tokens[self.cursor].location.line, self.tokens[self.cursor].location.column
  //     ));
  //     self.get();
  //   }
  // }
}
/// diagnostic functions
impl Parser {
  pub fn errors(&self) -> &[String] {
    &self.errors
  }
  pub fn warnings(&self) -> &[String] {
    &self.warnings
  }
  fn add_error(&mut self, message: String) {
    let token = &self.tokens[self.cursor];
    self.errors.push(format!(
      "In file {}:{}:{}: {}",
      token.location.file.to_str().unwrap_or("<invalid utf8>"),
      token.location.line,
      token.location.column,
      message
    ));
  }
  fn add_warning(&mut self, message: String) {
    let token = &self.tokens[self.cursor];
    self.warnings.push(format!(
      "In file {}:{}:{}: {}",
      token.location.file.to_str().unwrap_or("<invalid utf8>"),
      token.location.line,
      token.location.column,
      message
    ));
  }
}
/// opt checks
impl Parser {
  fn ios_c_strict_check_for_decl(&mut self, statement: &Statement) {
    if matches!(statement, Statement::Declaration(_)) {
      self.add_error(
        "C standard pre C23 does not allow declaration in 'if', 'while', 'for' statements. If it's intended, please use surrounding braces '{}' to form a block."
          .to_string(),
      );
    }
  }
}
/// meta
impl Parser {
  fn parse_type_specifier(&mut self) -> Option<TypeSpecifier> {
    match self.peek(0) {
      Literal::Keyword(Keyword::Struct) => todo!(),
      Literal::Keyword(Keyword::Union) => todo!(),
      Literal::Keyword(Keyword::Enum) => todo!(),
      Literal::Keyword(keyword) => TypeSpecifier::try_from(keyword).ok(),
      Literal::Identifier(ident) => {
        if self.typedefs.contains(ident) {
          Some(TypeSpecifier::Typedef(ident.to_string()))
        } else {
          None
        }
      }
      _ => None,
    }
  }
  fn parse_function_specifier(&mut self) -> Option<FunctionSpecifier> {
    match self.peek(0) {
      Literal::Keyword(kw) => FunctionSpecifier::try_from(kw).ok(),
      _ => None,
    }
  }
  fn parse_declspecs(&mut self) -> DeclSpecs {
    let mut declspecs = DeclSpecs::default();

    loop {
      if self.peek(0).is_qualifier() {
        let qualifier = Qualifiers::from(self.peek(0));
        // qualifiers is a bitfield
        if declspecs.qualifiers & qualifier != Qualifiers::empty() {
          self.add_warning(format!("Redundant qualifier '{}'.", qualifier));
        } else {
          declspecs.qualifiers |= qualifier;
        }
        self.get(); // get the qualifier
      } else if self.peek(0).is_storage_class() {
        let storage_class = Storage::from(self.peek(0));
        match declspecs.storage_class {
          Some(ref existing_storage) if existing_storage == &storage_class => {
            self.add_warning(format!(
              "Redundant storage class specifier '{}'.",
              storage_class
            ));
          }
          Some(ref existing_storage) => {
            self.add_error(format!(
              "Cannot combine '{}' with '{}'.",
              storage_class, existing_storage
            ));
          }
          None => {
            declspecs.storage_class = Some(storage_class);
          }
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
        declspecs.function_specifiers.push(kw);
        self.get();
      } else {
        break;
      }
    }

    if declspecs.type_specifiers.is_empty() {
      self.add_error("Expect type specifier in declaration, default to int".to_string());
      declspecs.type_specifiers.push(TypeSpecifier::Int);
    }

    declspecs
  }
  fn parse_declarator<const TYPE: DeclaratorType, const AGGRESSIVE: bool>(&mut self) -> Declarator {
    let name = if TYPE != DeclaratorType::Abstract {
      if let Literal::Identifier(_) = self.peek(0) {
        let name_idx = self.get(); // consume the ident
        Some(self.tokens[name_idx].to_owned_string())
      } else {
        if TYPE == DeclaratorType::Named {
          self.add_error("Expect identifier in declarator".to_string());
          if AGGRESSIVE {
            self.get();
          }
        }
        None
      }
    } else {
      None
    };
    let mut declarator = Declarator::new(name);
    // if the next token is '(', it's a function declarator
    if *self.peek(0) == Literal::Operator(Operator::LeftParen) {
      self.must_get_op::<{ Operator::LeftParen }>();
      let parameters = self.parse_function_params();
      self.recoverable_get::<{ Operator::RightParen }>();
      declarator.modifiers.push(Modifier::Function(parameters));
    }
    declarator
  }
  fn parse_argument_list(&mut self) -> Vec<Expression> {
    self.must_get_op::<{ Operator::LeftParen }>();
    let mut arguments = Vec::new();

    while *self.peek(0) != Literal::Operator(Operator::RightParen) {
      // parse expression
      let expr = self.next_expression(0);
      arguments.push(expr);
      if *self.peek(0) == Literal::Operator(Operator::RightParen) {
        break;
      }
      self.recoverable_get::<{ Operator::Comma }>();
      if *self.peek(0) == Literal::Operator(Operator::RightParen) {
        self.add_error("Trailing comma in argument list is not allowed in C.".to_string());
        break;
      }
    }
    self.must_get_op::<{ Operator::RightParen }>();
    arguments
  }

  fn parse_function_params(&mut self) -> FunctionSignature {
    // C17: a function declaration without a parameter list
    //  or function body provides no information about that function’s parameters
    // but I won't support that obselete feature :(
    if let Literal::Keyword(Keyword::Void) = self.tokens[self.cursor].literal {
      // single void parameter
      self.must_get_key::<{ Keyword::Void }>();
      if *self.peek(0) != Literal::Operator(Operator::RightParen) {
        self.add_error("Unexpected token after 'void' in parameter list".to_string());
        while *self.peek(0) != Literal::Operator(Operator::RightParen) {
          self.get();
        }
      }
      FunctionSignature::default()
    } else {
      let mut parameters = Vec::new();
      loop {
        let mut declspecs = self.parse_declspecs();
        let declarator = self.parse_declarator::<{ DeclaratorType::Maybe }, false>();
        if declspecs.storage_class.is_some() {
          self.add_error(
            "Storage class specifier is not allowed in parameter declaration".to_string(),
          );
          declspecs.storage_class = None;
        }
        parameters.push(Parameter::new(declspecs, declarator));

        match self.peek(0) {
          Literal::Operator(Operator::RightParen) => break,
          Literal::Operator(Operator::Comma) => {
            self.must_get_op::<{ Operator::Comma }>();
            if self.peek(0) == &Literal::Operator(Operator::RightParen) {
              self.add_error("Trailing comma in parameter list is not allowed in C.".to_string());
              break;
            }
          }
          _ => {
            if self.parse_type_specifier().is_none() {
              self.add_error("Expect ',', ')', or type specifier in parameter list".to_string());
              break;
            }
            // continuing parsing
          }
        }
      }
      FunctionSignature::new(parameters, false)
    }
  }
  /// common function to parse `(` expr `)`.
  fn parse_paren_expression(&mut self) -> Expression {
    if self.peek(0) != &Literal::Operator(Operator::LeftParen) {
      self.add_error(format!("Expcet '(' after {}", self.tokens[self.cursor - 1]));
      // assume the left paren is missing, continue parsing
    } else {
      self.must_get_op::<{ Operator::LeftParen }>();
    }
    let expr = self.next_expression(0);
    if self.peek(0) != &Literal::Operator(Operator::RightParen) {
      self.add_error("Expect ')'".to_string());
      self.get(); // get it otherwise infinite loop
    } else {
      self.must_get_op::<{ Operator::RightParen }>();
    }
    expr
  }
  fn parse_case_and_default_body(&mut self) -> Vec<Statement> {
    let mut body = Vec::new();
    while self.peek(0) != &Literal::Keyword(Keyword::Case)
      && self.peek(0) != &Literal::Keyword(Keyword::Default)
      && self.peek(0) != &Literal::Operator(Operator::RightBrace)
    {
      body.push(self.next_statement());
    }
    body
  }
  fn parse_case(&mut self) -> Case {
    self.must_get_key::<{ Keyword::Case }>();
    let expression = if self.peek(0) == &Literal::Operator(Operator::Colon) {
      self.add_error("Expect constant expression after 'case'".to_string());
      self.must_get_op::<{ Operator::Colon }>();
      Expression::Empty
    } else {
      let expr = self.next_expression(0);
      self.recoverable_get::<{ Operator::Colon }>();
      expr
    };
    // if it's a compound statement, we need to extract all statements until the next case/default or right brace
    // else, multiple statements until next case/default
    let body = self.parse_case_and_default_body();
    Case::new(expression, body)
  }
  fn parse_default(&mut self) -> Default {
    self.must_get_key::<{ Keyword::Default }>();
    self.recoverable_get::<{ Operator::Colon }>();
    let body = self.parse_case_and_default_body();
    Default::new(body)
  }
}
/// declarations
impl Parser {
  fn next_vardef(&mut self, declspecs: DeclSpecs, declarator: Declarator) -> VarDef {
    let initializer = match self.peek(0) {
      Literal::Operator(Operator::Semicolon) => {
        self.must_get_op::<{ Operator::Semicolon }>();
        None
      }
      Literal::Operator(Operator::Assign) => {
        self.must_get_op::<{ Operator::Assign }>();
        let initializer = self.next_expression(0);
        assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
        self.must_get_op::<{ Operator::Semicolon }>();
        Some(initializer)
      }
      _ => {
        self.add_error("Expect ';' or '=' after variable name".to_string());
        self.get();
        None
      }
    };
    VarDef::new(
      declspecs,
      declarator,
      initializer.map(|init_expr| Initializer::Expression(Box::new(init_expr))),
    )
  }
  fn next_declaration(&mut self) -> Declaration {
    while matches!(
      self.peek(0),
      Literal::Operator(Operator::Semicolon) | Literal::Operator(Operator::Hash)
    ) {
      if *self.peek(0) == Literal::Operator(Operator::Semicolon) {
        self.add_warning("Redundant ';'".to_string());
        self.must_get_op::<{ Operator::Semicolon }>();
      } else {
        // skip preprocessor directive
        let line = self.tokens[self.cursor].location.line;
        while (!self.is_at_end()) && (self.tokens[self.cursor].location.line == line) {
          self.get();
        }
      }
    }

    let mut recovery = false;
    // block definition is not allowed in top
    if *self.peek(0) == Literal::Operator(Operator::LeftBrace) {
      self.add_error("Block definition is not allowed here.".to_string());
      self.must_get_op::<{ Operator::LeftBrace }>();
      recovery = true;
    }

    let declspecs = self.parse_declspecs();
    let declarator = self.parse_declarator::<{ DeclaratorType::Maybe }, true>();

    if matches!(declspecs.storage_class, Some(Storage::Typedef)) {
      if let Some(ref name) = declarator.name {
        self.typedefs.declare(name.clone());
      } else {
        self.add_warning("Typedef defines nothing.".to_string());
      }
      self.must_get_op::<{ Operator::Semicolon }>();
      return Declaration::Variable(VarDef::new(declspecs, declarator, None));
    }
    let declaration = if declarator
      .modifiers
      .iter()
      .any(|m| matches!(m, Modifier::Function(_)))
    {
      // int(void) is not allowed
      if declarator.name.is_none() {
        self.add_error("Expcet a function name.".to_string());
      }
      Declaration::Function(self.next_function_body(declspecs, declarator))
    } else {
      // `int;` is allowed although useless
      Declaration::Variable(self.next_vardef(declspecs, declarator))
    };
    if recovery {
      self.recoverable_get::<{ Operator::RightBrace }>();
    }
    declaration
  }
}
/// statements
impl Parser {
  fn next_function_body(&mut self, declspec: DeclSpecs, declarator: Declarator) -> Function {
    let body = match self.tokens[self.cursor].literal {
      Literal::Operator(Operator::LeftBrace) => Some(self.next_block()),
      _ => {
        self.recoverable_get::<{ Operator::Semicolon }>();
        None
      }
    };

    Function::new(declspec, declarator, body)
  }
  fn next_block(&mut self) -> Compound {
    self.must_get_op::<{ Operator::LeftBrace }>();
    self.typedefs.push_scope();
    let mut block = Compound::new();

    while *self.peek(0) != Literal::Operator(Operator::RightBrace) {
      block.statements.push(self.next_statement());
    }
    self.typedefs.pop_scope();
    self.must_get_op::<{ Operator::RightBrace }>();
    block
  }
  fn next_return(&mut self) -> Return {
    self.must_get_key::<{ Keyword::Return }>();
    let expression = if *self.peek(0) == Literal::Operator(Operator::Semicolon) {
      None
    } else {
      Some(self.next_expression(0))
    };

    assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
    self.must_get_op::<{ Operator::Semicolon }>();
    Return::new(expression)
  }
  fn next_if(&mut self) -> If {
    self.must_get_key::<{ Keyword::If }>();
    let condition = self.parse_paren_expression();
    let if_branch = self.next_statement();
    self.ios_c_strict_check_for_decl(&if_branch);
    let else_branch = if self.peek(0) == &Literal::Keyword(Keyword::Else) {
      self.must_get_key::<{ Keyword::Else }>();
      let body = self.next_statement();
      self.ios_c_strict_check_for_decl(&body);
      Some(body)
    } else {
      None
    };
    If::new(condition, if_branch, else_branch)
  }
  fn next_while(&mut self) -> While {
    self.must_get_key::<{ Keyword::While }>();
    let condition = self.parse_paren_expression();
    self.loop_labels.push(new_loop_dummy_identifier("while"));
    let body = self.next_statement();
    self.ios_c_strict_check_for_decl(&body);
    let while_stmt = While::new(condition, body, self.loop_labels.last().unwrap().clone());
    self.loop_labels.pop();
    while_stmt
  }
  fn next_dowhile(&mut self) -> DoWhile {
    self.must_get_key::<{ Keyword::Do }>();
    self.loop_labels.push(new_loop_dummy_identifier("do_while"));
    let body = self.next_statement();
    self.ios_c_strict_check_for_decl(&body);
    self.must_get_key::<{ Keyword::While }>();
    let condition = self.parse_paren_expression();
    assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
    self.must_get_op::<{ Operator::Semicolon }>();
    let dowhile_stmt = DoWhile::new(body, condition, self.loop_labels.last().unwrap().clone());
    self.loop_labels.pop();
    dowhile_stmt
  }
  fn next_for(&mut self) -> For {
    self.must_get_key::<{ Keyword::For }>();
    if *self.peek(0) != Literal::Operator(Operator::LeftParen) {
      self.add_error("Expect '(' after 'for'".to_string());
      panic!() // workaound
    } else {
      self.must_get_op::<{ Operator::LeftParen }>();
      // initializer
      let initializer = match self.peek(0) {
        Literal::Operator(Operator::Semicolon) => {
          self.must_get_op::<{ Operator::Semicolon }>();
          None
        }
        _ => match self.next_statement() {
          Statement::Declaration(Declaration::Variable(vardef)) => {
            match vardef.initializer {
              None => {
                self.add_warning("Expect initializer in for loop variable declaration".to_string());
              }
              Some(_) => {}
            }
            Some(Statement::Declaration(Declaration::Variable(vardef)))
          }
          Statement::Expression(expr) => Some(Statement::Expression(expr)),
          _ => {
            self.add_error(
              "Expect variable declaration or expression in for initializer".to_string(),
            );
            None
          }
        },
      };
      fn parse_optional_expression<const OP: Operator>(parser: &mut Parser) -> Option<Expression> {
        match parser.peek(0) {
          Literal::Operator(op) if op == &OP => {
            parser.must_get_op::<OP>();
            None
          }
          _ => {
            let expr = parser.next_expression(0);
            parser.must_get_op::<OP>();
            Some(expr)
          }
        }
      }
      let condition = parse_optional_expression::<{ Operator::Semicolon }>(self);
      let increment = parse_optional_expression::<{ Operator::RightParen }>(self);
      self.loop_labels.push(new_loop_dummy_identifier("for"));
      let body = self.next_statement();
      self.ios_c_strict_check_for_decl(&body);
      let for_stmt = For::new(
        initializer,
        condition,
        increment,
        body,
        self.loop_labels.last().unwrap().clone(),
      );
      self.loop_labels.pop();
      for_stmt
    }
  }
  fn next_switch(&mut self) -> Switch {
    self.must_get_key::<{ Keyword::Switch }>();
    let condition = self.parse_paren_expression();
    self.loop_labels.push(new_loop_dummy_identifier("switch"));
    self.recoverable_get::<{ Operator::LeftBrace }>();
    let mut cases = Vec::new();
    let mut default: Option<Default> = None;
    while *self.peek(0) != Literal::Operator(Operator::RightBrace) {
      match self.peek(0) {
        Literal::Keyword(Keyword::Case) => {
          let case = self.parse_case();
          if default.is_some() {
            self.add_error("Case label after default label in switch; case ignored".to_string());
          } else {
            cases.push(case);
          }
        }
        Literal::Keyword(Keyword::Default) => {
          if default.is_some() {
            self.add_error("Multiple default labels in one switch; ignoring latter".to_string());
          } else {
            default = Some(self.parse_default());
          }
        }
        _ => {
          self.add_error("Expect 'case' or 'default' in switch body".to_string());
          self.get(); // consume the invalid token
        }
      }
    }

    self.must_get_op::<{ Operator::RightBrace }>();
    self.loop_labels.pop();
    Switch::new(condition, cases, default)
  }
  fn next_statement(&mut self) -> Statement {
    match *self.peek(0) {
      Literal::Keyword(Keyword::If) => Statement::If(self.next_if()),
      Literal::Keyword(Keyword::For) => Statement::For(self.next_for()),
      Literal::Keyword(Keyword::Return) => Statement::Return(self.next_return()),
      Literal::Keyword(Keyword::While) => Statement::While(self.next_while()),
      Literal::Keyword(Keyword::Do) => Statement::DoWhile(self.next_dowhile()),
      Literal::Keyword(Keyword::Break) => Statement::Break(self.next_break()),
      Literal::Keyword(Keyword::Continue) => Statement::Continue(self.next_continue()),
      Literal::Keyword(Keyword::Switch) => Statement::Switch(self.next_switch()),
      Literal::Operator(Operator::LeftBrace) => Statement::Compound(self.next_block()),
      Literal::Operator(Operator::Semicolon) => self.next_emptystmt(),
      Literal::Keyword(Keyword::Case) => {
        self.add_error("Case label not within switch statement".to_string());
        // attempt to recover
        _ = self.parse_case();
        Statement::Empty()
      }
      Literal::Keyword(Keyword::Default) => {
        self.add_error("Default label not within switch statement".to_string());
        // ditto
        _ = self.parse_default();
        Statement::Empty()
      }
      Literal::Keyword(Keyword::Goto) => self.next_gotostmt(),
      Literal::Keyword(_) => Statement::Declaration(self.next_declaration()),
      Literal::Identifier(ref ident) if self.typedefs.contains(&ident) => {
        Statement::Declaration(self.next_declaration())
      }
      Literal::Identifier(ref ident) if self.peek(1) == &Literal::Operator(Operator::Colon) => {
        self.next_labelstmt(ident.to_string())
      }

      _ => Statement::Expression(self.next_exprstmt()),
    }
  }

  fn next_labelstmt(&mut self, ident: String) -> Statement {
    // 1. label at end of compound statement is not allowed until C23
    // 2. label can only jump to statements within the same function, not to mention cross file.
    if self.typedefs.is_top_level() {
      self.add_error("Label statement is not allowed in top level.".to_string());
      Statement::Empty()
    } else {
      self.get(); // consume ident
      self.must_get_op::<{ Operator::Colon }>();
      let statement = self.next_statement();
      self.ios_c_strict_check_for_decl(&statement);
      // todo: label validity check, here or in semantic analysis?
      Statement::Label(Label::new(ident, statement))
    }
  }

  fn next_gotostmt(&mut self) -> Statement {
    self.must_get_key::<{ Keyword::Goto }>();
    if let Literal::Identifier(ident) = self.peek(0) {
      let name = ident.to_string();
      self.get(); // consume ident
      self.recoverable_get::<{ Operator::Semicolon }>();
      Statement::Goto(Goto::new(name))
    } else {
      self.add_error("Expect label identifier after 'goto'".to_string());
      // assume the label is missing, continue parsing
      self.silent_get_if::<{ Operator::Semicolon }>();
      Statement::Empty()
    }
  }

  fn next_emptystmt(&mut self) -> Statement {
    self.must_get_op::<{ Operator::Semicolon }>();
    self.add_warning("Redundant ';'".to_string());
    Statement::Empty()
  }

  fn next_exprstmt(&mut self) -> Expression {
    let expr = self.next_expression(0);
    self.recoverable_get::<{ Operator::Semicolon }>();
    expr
  }
  fn next_break(&mut self) -> Break {
    self.must_get_key::<{ Keyword::Break }>();
    self.recoverable_get::<{ Operator::Semicolon }>();
    match self.loop_labels.last() {
      Some(label) => Break::new(label.to_string()),
      None => {
        self.add_error("Break statement not within a loop".to_string());
        Break::new("invalid_loop".to_string())
      }
    }
  }

  fn next_continue(&mut self) -> Continue {
    self.must_get_key::<{ Keyword::Continue }>();

    self.recoverable_get::<{ Operator::Semicolon }>();
    // we need to handle continus differently; since the continue cannot be used to `continue` a switch.
    // search reversely for the nearest loop label which does not start with 'switch_'
    let mut found_label: Option<String> = None;
    for label in self.loop_labels.iter().rev() {
      if !label.starts_with("switch_") {
        found_label = Some(label.to_string());
        break;
      }
    }
    match found_label {
      Some(label) => Continue::new(label),
      None => {
        self.add_error("Continue statement not within a loop".to_string());
        Continue::new("invalid_loop".to_string())
      }
    }
  }
}
/// expressions
impl Parser {
  fn next_factor(&mut self) -> Expression {
    self.get();
    let literal = &self
      .tokens[self.cursor - 1]
      .literal
    // rust forces me to clone, but here it's guranteed not UB. :(
    ;
    match literal {
      Literal::Number(num_str) => Expression::Constant(Constant::from_str(&num_str)),
      Literal::String(str) => Expression::Constant(Constant::String(str.to_string())),
      Literal::Operator(op) => {
        if op.unary() {
          Expression::Unary(Unary::new(op.clone(), self.next_expression(0)))
        } else if *op == Operator::LeftParen {
          let expr = self.next_expression(0);
          if *self.peek(0) == Literal::Operator(Operator::RightParen) {
            self.get();
          } else {
            self.add_error("Expect '}'".to_string());
          }
          expr
        } else {
          self.add_error(format!("Unexpected operator {op} in factor, assuming int",));
          self.get();
          Expression::Constant(Constant::Int32(0))
        }
      }
      Literal::Identifier(ident) => {
        let ident_expr = Expression::Variable(Variable::new(ident.to_string()));
        if *self.peek(0) == Literal::Operator(Operator::LeftParen) {
          let arguments = self.parse_argument_list();
          Expression::Call(Call::new(ident_expr, arguments))
        } else {
          ident_expr
        }
      }
      Literal::Keyword(_keyword) => {
        breakpoint!();
        panic!()
      }
    }
  }
  fn next_expression(&mut self, lmin_precedence: u8) -> Expression {
    let mut current = self.next_factor();
    let mut lookahead = self.peek(0).clone();
    while matches!(lookahead, Literal::Operator(ref op) if op.binary() && op.precedence() >= lmin_precedence)
    {
      let op = match lookahead {
        Literal::Operator(ref op) => op.clone(),
        _ => unreachable!(),
      };
      self.get(); // operator
      let right = self.next_expression(op.precedence() + 1);
      current = Expression::Binary(Binary::from_operator(op, current, right).unwrap());
      lookahead = self.peek(0).clone();
    }
    // tenary
    if lookahead == Literal::Operator(Operator::Question) {
      self.must_get_op::<{ Operator::Question }>();

      let true_expr = self.next_expression(0);
      if self.peek(0) != &Literal::Operator(Operator::Colon) {
        self.add_error("Expect ':' in tenary expression".to_string());
        panic!()
      } else {
        self.must_get_op::<{ Operator::Colon }>();
        let false_expr = self.next_expression(0);
        current = Expression::Ternary(Ternary::new(current, true_expr, false_expr));
      }
    }
    current
  }
}
