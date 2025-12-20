use ::std::str::FromStr;

use crate::{
  breakpoint,
  common::{
    keyword::Keyword,
    operator::Operator,
    token::{Literal, Token},
  },
  parser::{
    declaration::{
      DeclSpecs, Declaration, Declarator, Function, FunctionSignature, Initializer, Modifier,
      Parameter, Specifier, TranslationUnit, VarDef,
    },
    expression::{Binary, Call, Constant, Expression, Ternary, Unary, Variable},
    statement::{
      Compound, DoWhile, For, If, Return, SingleLabel, Statement, While, new_loop_dummy_identifier,
    },
  },
};
#[cfg(test)]
use pretty_assertions::assert_eq;

pub struct Parser {
  tokens: Vec<Token>,
  cursor: usize,
  errors: Vec<String>,
  warnings: Vec<String>,
  loop_labels: Vec<String>,
}

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
    }
  }
  pub fn warnings(&self) -> &[String] {
    &self.warnings
  }
  pub fn errors(&self) -> &[String] {
    &self.errors
  }
  pub fn parse(&mut self) -> TranslationUnit {
    let mut program = TranslationUnit::new();
    while !self.is_at_end() {
      program.declarations.push(self.next_declaration());
    }

    program
  }
  fn is_at_end(&self) -> bool {
    self.tokens.len() <= self.cursor + 1
  }
  fn recoverable_get<const OP: Operator>(&mut self) {
    if *self.peek(0) != Literal::Operator(OP) {
      self.errors.push(format!("Expect '{}'.", OP));
    } else {
      self.must_get_op::<OP>();
    }
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
        self
          .errors
          .push("Trailing comma in argument list is not allowed in C.".to_string());
        break;
      }
    }
    self.must_get_op::<{ Operator::RightParen }>();
    arguments
  }
  fn next_vardef(&mut self) -> VarDef {
    let var_type = self.parse_type();

    let name_idx = self.get();

    let name = if let Literal::Identifier(ref ident) = self.tokens[name_idx].literal {
      ident.to_string()
    } else {
      self
        .errors
        .push("Expect identifier as variable name".to_string());
      "unnamed".to_string()
    };

    let mut declspec = DeclSpecs::new();
    declspec.specifiers.push(var_type);
    let declarator = Declarator::new(name);

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
        self
          .errors
          .push("Expect ';' or '=' after variable name".to_string());
        None
      }
    };
    VarDef::new(
      declspec,
      declarator,
      initializer.map(|init_expr| Initializer::Expression(Box::new(init_expr))),
    )
  }
  fn next_declaration(&mut self) -> Declaration {
    match self.peek(0) {
      Literal::Keyword(keyword) => match keyword.to_type() {
        Some(_) => match self.peek(1) {
          Literal::Identifier(_) => match self.peek(2) {
            Literal::Operator(Operator::LeftParen) => Declaration::Function(self.next_function()),
            _ => Declaration::Variable(self.next_vardef()),
          },
          _ => {
            breakpoint!("others");
            panic!()
          }
        },
        None => {
          breakpoint!();
          panic!()
        }
      },
      Literal::Identifier(_identifier) => {
        // maybe a userdefined type, now ignore
        while (!self.is_at_end()) && (*self.peek(0) != Literal::Operator(Operator::Semicolon)) {
          self.get();
        }
        self.must_get_op::<{ Operator::Semicolon }>();
        self
          .errors
          .push("Userdefined types are not supported yet".to_string());
        Declaration::Variable(VarDef::new(
          DeclSpecs::new(),
          Declarator::new("unnamed".to_string()),
          None,
        ))
        // let declspec = DeclSpecs::new();
        // let declarator = Declarator::new("".to_string());
        // self.get();
        // Declaration::Variable(VarDef::new(declspec, declarator, None))
      }
      // below is just workaround
      Literal::Operator(Operator::Hash) => {
        // skip preprocessor directive
        let line = self.tokens[self.cursor].location.line;
        while (!self.is_at_end()) && (self.tokens[self.cursor].location.line == line) {
          self.get();
        }

        self.next_declaration()
      }
      Literal::String(value) | Literal::Number(value) => {
        self
          .errors
          .push(format!("Unexpected value literal {} in declaration", value));
        while (!self.is_at_end()) && (*self.peek(0) != Literal::Operator(Operator::Semicolon)) {
          self.get();
        }
        self.get(); // skip ';'
        self.next_declaration()
      }
      Literal::Operator(op) => {
        self
          .errors
          .push(format!("Unexpected operator {} in declaration", op));
        while (!self.is_at_end()) && (*self.peek(0) != Literal::Operator(Operator::Semicolon)) {
          self.get();
        }
        self.get(); // skip ';'
        self.next_declaration()
      }
    }
  }
  fn next_function(&mut self) -> Function {
    let return_type = self.parse_type();
    let name_idx = self.get();
    self.recoverable_get::<{ Operator::LeftParen }>();

    let parameters = self.parse_function_params();

    self.recoverable_get::<{ Operator::RightParen }>();

    let body = match self.tokens[self.cursor].literal {
      Literal::Operator(Operator::LeftBrace) => Some(self.next_block()),
      _ => {
        self.recoverable_get::<{ Operator::Semicolon }>();
        None
      }
    };
    let name = self.tokens[name_idx].to_owned_string();

    let mut declspec = DeclSpecs::new();

    declspec.specifiers.push(return_type);

    let mut declarator = Declarator::new(name.clone());

    declarator.modifiers.push(Modifier::Function(parameters));

    Function::new(declspec, declarator, body)
  }
  fn next_block(&mut self) -> Compound {
    self.must_get_op::<{ Operator::LeftBrace }>();
    let mut block = Compound::new();

    while *self.peek(0) != Literal::Operator(Operator::RightBrace) {
      block.statements.push(self.next_statement());
    }

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
    Return::new(expression)
  }
  fn next_if(&mut self) -> If {
    self.must_get_key::<{ Keyword::If }>();
    if *self.peek(0) != Literal::Operator(Operator::LeftParen) {
      self.errors.push("Expect '(' after 'if'".to_string());
      panic!() // workaound
    } else {
      self.must_get_op::<{ Operator::LeftParen }>();
      let condition = self.next_expression(0);
      if *self.peek(0) != Literal::Operator(Operator::RightParen) {
        self
          .errors
          .push("Expect ')' after if condition".to_string());
        panic!() // workaround
      } else {
        self.must_get_op::<{ Operator::RightParen }>();
        let if_branch = self.next_statement();
        let else_branch = if *self.peek(0) == Literal::Keyword(Keyword::Else) {
          self.must_get_key::<{ Keyword::Else }>();
          Some(self.next_statement())
        } else {
          None
        };
        If::new(condition, if_branch, else_branch)
      }
    }
  }
  fn next_while(&mut self) -> While {
    self.must_get_key::<{ Keyword::While }>();
    if *self.peek(0) != Literal::Operator(Operator::LeftParen) {
      self.errors.push("Expect '(' after 'while'".to_string());
      panic!() // workaound
    } else {
      self.must_get_op::<{ Operator::LeftParen }>();
      let condition = self.next_expression(0);
      if *self.peek(0) != Literal::Operator(Operator::RightParen) {
        self
          .errors
          .push("Expect ')' after while condition".to_string());
        panic!() // workaround
      } else {
        self.must_get_op::<{ Operator::RightParen }>();
        self.loop_labels.push(new_loop_dummy_identifier("while"));
        let body = self.next_statement();
        let while_stmt = While::new(condition, body, self.loop_labels.last().unwrap().clone());
        self.loop_labels.pop();
        while_stmt
      }
    }
  }
  fn next_dowhile(&mut self) -> DoWhile {
    self.must_get_key::<{ Keyword::Do }>();
    self.loop_labels.push(new_loop_dummy_identifier("do_while"));
    let body = self.next_statement();
    self.must_get_key::<{ Keyword::While }>();
    if *self.peek(0) != Literal::Operator(Operator::LeftParen) {
      self.errors.push("Expect '(' after 'while'".to_string());
      panic!() // workaound
    } else {
      self.must_get_op::<{ Operator::LeftParen }>();
      let condition = self.next_expression(0);
      if *self.peek(0) != Literal::Operator(Operator::RightParen) {
        self
          .errors
          .push("Expect ')' after while condition".to_string());
        panic!() // workaround
      } else {
        self.must_get_op::<{ Operator::RightParen }>();
        assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
        self.must_get_op::<{ Operator::Semicolon }>();
        let dowhile_stmt = DoWhile::new(body, condition, self.loop_labels.last().unwrap().clone());
        self.loop_labels.pop();
        dowhile_stmt
      }
    }
  }
  fn next_for(&mut self) -> For {
    self.must_get_key::<{ Keyword::For }>();
    if *self.peek(0) != Literal::Operator(Operator::LeftParen) {
      self.errors.push("Expect '(' after 'for'".to_string());
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
                self
                  .warnings
                  .push("Expect initializer in for loop variable declaration".to_string());
              }
              Some(_) => {}
            }
            Some(Statement::Declaration(Declaration::Variable(vardef)))
          }
          Statement::Expression(expr) => Some(Statement::Expression(expr)),
          _ => {
            self
              .errors
              .push("Expect variable declaration or expression in for initializer".to_string());
            None
          }
        },
      };
      // condition
      let condition = match self.peek(0) {
        Literal::Operator(Operator::Semicolon) => {
          self.must_get_op::<{ Operator::Semicolon }>();
          None
        }
        _ => {
          let cond_expr = self.next_expression(0);
          self.must_get_op::<{ Operator::Semicolon }>();
          Some(cond_expr)
        }
      };
      // increment
      let increment = match self.peek(0) {
        Literal::Operator(Operator::RightParen) => {
          self.must_get_op::<{ Operator::RightParen }>();
          None
        }
        _ => {
          let incr_expr = self.next_expression(0);
          self.must_get_op::<{ Operator::RightParen }>();
          Some(incr_expr)
        }
      };
      self.loop_labels.push(new_loop_dummy_identifier("for"));
      let body = self.next_statement();
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
  fn next_statement(&mut self) -> Statement {
    match *self.peek(0) {
      Literal::Keyword(Keyword::Return) => {
        let statement = Statement::Return(self.next_return());
        assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
        self.must_get_op::<{ Operator::Semicolon }>();
        statement
      }
      Literal::Operator(Operator::LeftBrace) => Statement::Compound(self.next_block()),
      Literal::Operator(Operator::Semicolon) => {
        self.must_get_op::<{ Operator::Semicolon }>();
        self.warnings.push(format!(
          "Redundant ';' at {}:{}",
          self.tokens[self.cursor - 1].location.line,
          self.tokens[self.cursor - 1].location.column
        ));
        Statement::Empty()
      }
      Literal::Keyword(Keyword::If) => Statement::If(self.next_if()),
      Literal::Keyword(Keyword::While) => Statement::While(self.next_while()),
      Literal::Keyword(Keyword::Do) => Statement::DoWhile(self.next_dowhile()),
      Literal::Keyword(Keyword::For) => Statement::For(self.next_for()),
      Literal::Keyword(Keyword::Break) => {
        self.must_get_key::<{ Keyword::Break }>();
        self.recoverable_get::<{ Operator::Semicolon }>();
        match self.loop_labels.last() {
          Some(ref label) => Statement::Break(SingleLabel::new(label.to_string())),
          None => {
            self
              .errors
              .push("Break statement not within a loop".to_string());
            Statement::Break(SingleLabel::new("invalid_loop".to_string()))
          }
        }
      }
      Literal::Keyword(Keyword::Continue) => {
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
          Some(label) => Statement::Continue(SingleLabel::new(label)),
          None => {
            self
              .errors
              .push("Continue statement not within a loop".to_string());
            Statement::Continue(SingleLabel::new("invalid_loop".to_string()))
          }
        }
      }
      // if it's primitive type, it's a declaration
      Literal::Keyword(ref keyword) if keyword.to_type().is_some() => {
        // self.peek(1) should be ident
        match *self.peek(2) {
          Literal::Operator(Operator::LeftParen) => {
            let mut funcdecl = self.next_function();
            if funcdecl.body.is_some() {
              self
                .errors
                .push("Function definition not allowed here".to_string());
              funcdecl.body = None;
            }
            Statement::Declaration(Declaration::Function(funcdecl))
          }
          _ => Statement::Declaration(Declaration::Variable(self.next_vardef())),
        }
      }
      _ => {
        let exprstmt = Statement::Expression(self.next_expression(0));
        self.recoverable_get::<{ Operator::Semicolon }>();
        exprstmt

        // breakpoint!();
        // panic!()
      }
    }
  }
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
            self.errors.push("Expect '}'".to_string());
          }
          expr
        } else {
          self
            .errors
            .push(format!("Unexpected operator {op} in factor, assuming int",));
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
        // currently just take it as the `a2` in `a = a2 + 1`
        // Expression::Variable(Variable::new(ident.to_string()))
        // breakpoint!();
        // panic!()
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
        self
          .errors
          .push("Expect ':' in tenary expression".to_string());
        panic!()
      } else {
        self.must_get_op::<{ Operator::Colon }>();
        let false_expr = self.next_expression(0);
        current = Expression::Ternary(Ternary::new(current, true_expr, false_expr));
      }
    }
    current
  }
  fn peek(&self, offset: usize) -> &Literal {
    if self.is_at_end() {
      breakpoint!(
        "check your code! cursor: {}, current token: {:} ",
        self.cursor,
        self.tokens[self.cursor - 1]
      );
      panic!();
    }
    &self.tokens[self.cursor + offset].literal
  }
  fn must_get_key<const KEY: Keyword>(&mut self) -> usize {
    let index = self.get();
    if matches!(&self.tokens[index].literal, Literal::Keyword(kw) if *kw != KEY) {
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

  fn parse_function_params(&mut self) -> FunctionSignature {
    // C17: a function declaration without a parameter list
    //  or function body provides no information about that function’s parameters
    // but I won't support that obselete feature :(
    if let Literal::Keyword(Keyword::Void) = self.tokens[self.cursor].literal {
      // single void parameter
      self.must_get_key::<{ Keyword::Void }>();
      if *self.peek(0) != Literal::Operator(Operator::RightParen) {
        self
          .errors
          .push("Unexpected token after 'void' in parameter list".to_string());
        while *self.peek(0) != Literal::Operator(Operator::RightParen) {
          self.get();
        }
      }
      FunctionSignature::default()
    } else {
      let mut parameters = Vec::new();
      loop {
        let param_type = self.parse_type();
        let param_name = match self.peek(0) {
          Literal::Identifier(_) => {
            let name_idx = self.get();
            self.tokens[name_idx].to_owned_string()
          }
          _ => "unnamed".to_string(),
        };
        let mut declspec = DeclSpecs::new();
        declspec.specifiers.push(param_type);
        let declarator = Declarator::new(param_name);
        parameters.push(Parameter::new(declspec, declarator));

        if *self.peek(0) == Literal::Operator(Operator::RightParen) {
          break;
        }
        self.recoverable_get::<{ Operator::Comma }>();
        if *self.peek(0) == Literal::Operator(Operator::RightParen) {
          self
            .errors
            .push("Trailing comma in parameter list is not allowed in C.".to_string());
          break;
        }
      }
      FunctionSignature::new(parameters, false)
    }
  }

  fn parse_type(&mut self) -> Specifier {
    // assume type are primitive, and only one keyword
    let type_idx = self.get();
    let param_type = Specifier::from_str(&self.tokens[type_idx].to_owned_string())
      .ok()
      .or_else(|| {
        self.errors.push("Unknown type".to_string());
        Some(Specifier::Int)
      })
      .unwrap();
    param_type
  }
}
