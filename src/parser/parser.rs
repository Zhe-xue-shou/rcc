use crate::{
  breakpoint,
  common::{
    keyword::Keyword,
    operator::Operator,
    token::{Literal, Token},
  },
  parser::{
    ast::{Block, Declaration, FunctionDef, Program},
    expression::{Binary, Constant, Expression, Unary, Variable},
    statement::{Return, Statement, VarDef},
    types::{Primitive, Type},
  },
};
#[cfg(test)]
use pretty_assertions::assert_eq;

pub struct Parser {
  tokens: Vec<Token>,
  cursor: usize,
  errors: Vec<String>,
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
    }
  }
  pub fn errors(self) -> Vec<String> {
    self.errors
  }
  pub fn parse(&mut self) -> Program {
    let mut program = Program::new();
    while !self.is_at_end() {
      program.declarations.push(self.next_declaration());
    }

    program
  }
  fn is_at_end(&self) -> bool {
    self.tokens.len() <= self.cursor + 1
  }
  fn next_vardef(&mut self) -> VarDef {
    let type_idx = self.get();
    let var_type = Primitive::new(self.tokens[type_idx].to_owned_string()).as_type();

    let name_idx = self.get();

    let name = if let Literal::Identifier(ref ident) = self.tokens[name_idx].literal {
      ident.to_string()
    } else {
      self
        .errors
        .push("Expect identifier as variable name".to_string());
      "unnamed".to_string()
    };

    match self.peek(0) {
      Literal::Operator(Operator::Semicolon) => {
        self.get(); // skip ';'
        VarDef::new(name, None)
      }
      Literal::Operator(Operator::Assign) => {
        self.get(); // skip '='
        let initializer = self.next_expression(0);
        assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
        self.get(); // skip ';'
        VarDef::new(name, Some(initializer))
      }
      _ => {
        self
          .errors
          .push("Expect ';' or '=' after variable name".to_string());
        VarDef::new(name, None)
      }
    }
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
        self
          .errors
          .push("Userdefined types are not supported yet".to_string());
        // self.get(); // skip type
        // get until ';'
        while (!self.is_at_end()) && (*self.peek(0) != Literal::Operator(Operator::Semicolon)) {
          self.get();
        }
        Declaration::Variable(VarDef::new("".to_string(), None))
      }
      _ => {
        breakpoint!();
        panic!()
      }
    }
  }
  fn next_function(&mut self) -> FunctionDef {
    let return_type_idx = self.get();
    let name_idx = self.get();
    self.get(); // skip '('

    let parameters = match self.tokens[self.cursor].literal {
      Literal::Operator(Operator::RightParen) => {
        self.get(); // skip ')'
        Vec::new() // zero params
      }
      _ => self.parse_function_params(),
    };

    let body = match self.tokens[self.cursor].literal {
      Literal::Operator(Operator::LeftBrace) => self.next_block(),
      _ => {
        breakpoint!("{:?}", self.tokens[self.cursor]);
        panic!()
      }
    };
    let name = self.tokens[name_idx].to_owned_string();
    let return_type = Primitive::new(self.tokens[return_type_idx].to_owned_string()).as_type();

    FunctionDef::new(name, parameters, body, return_type)
  }
  fn next_block(&mut self) -> Block {
    self.get(); // skip '{'
    let mut block = Block::new();

    while *self.peek(0) != Literal::Operator(Operator::RightBrace) {
      block.statements.push(self.next_statement());
    }

    self.get();

    block
  }
  fn next_statement(&mut self) -> Statement {
    while *self.peek(0) == Literal::Operator(Operator::Semicolon) {
      self.get(); // skip extra ';'
    }
    match *self.peek(0) {
      Literal::Keyword(Keyword::Return) => {
        self.get(); // return
        let statement = Statement::Return(Return::new(
          if *self.peek(0) == Literal::Operator(Operator::Semicolon) {
            None
          } else {
            Some(self.next_expression(0))
          },
        ));
        assert_eq!(*self.peek(0), Literal::Operator(Operator::Semicolon));
        self.get(); // ;
        statement
      }
      // Literal::Operator(Operator::Semicolon) => {
      //   self.get(); // skip ';'
      //   Statement::Empty
      // }
      // if it's primitive type, it's a declaration
      Literal::Keyword(ref keyword) if keyword.to_type().is_some() => {
        Statement::Declaration(self.next_vardef())
      }
      _ => {
        Statement::Expression(self.next_expression(0))

        // breakpoint!();
        // panic!()
      }
    }
  }
  fn next_factor(&mut self) -> Expression {
    self.get();
    let literal = &self
      .tokens[self.cursor - 1].literal
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
        // currently just take it as the `a2` in `a = a2 + 1`
        Expression::Variable(Variable::new(ident.to_string()))
        // breakpoint!();
        // panic!()
      }
      Literal::Keyword(keyword) => {
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
    current
  }
  fn peek(&self, offset: usize) -> &Literal {
    assert_eq!(self.is_at_end(), false);
    &self.tokens[self.cursor + offset].literal
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

  fn parse_function_params(&mut self) -> Vec<(String, Type)> {
    // type (name)* (, type (name)*)*
    let mut params = Vec::new();
    loop {
      // let param_type = self.token_at(self.get());
      let type_idx = self.get();
      let param_type = Primitive::new(self.tokens[type_idx].to_owned_string()).as_type();
      // todo: find that it's a valid type
      match self.peek(0) {
        Literal::Identifier(_) => {
          let name_idx = self.get();
          let param_name = self.tokens[name_idx].to_owned_string();
          params.push((param_name, param_type));
        }
        Literal::Operator(Operator::Comma) => {
          self.get(); // skip ','
        }
        Literal::Operator(Operator::RightParen) => {
          self.get(); // skip ')'
          break;
        }
        _ => {
          self.errors.push("Expect parameter name".to_string());
          break;
        }
      }
    }
    params
  }
}
