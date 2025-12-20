#![allow(internal_features)]
#![allow(unused_variables)]
#![allow(unstable_features)]
#![feature(core_intrinsics)]
#![allow(unreachable_code)]
#![allow(unused_imports)]

use ::rcns::breakpoint;
use ::rcns::lexer::lexer::Lexer;
use ::rcns::parser::parser::Parser;
use ::std::env::args;
use ::std::panic::catch_unwind;
use ::std::process::exit;
use ::std::{fs::File, io::Read};
// use rcns::preprocessor;

fn main() {
  let args = args().collect::<Vec<String>>();

  println!("Args: {:?}", args);

  let (kind, filename) = match args.as_slice() {
    [_] => ("all", "test.c"),
    [_, kind, filename] => (kind.as_str(), filename.as_str()),
    _ => {
      eprintln!("Usage: rcns [all|lex|parse] <filename>");
      exit(1);
    }
  };

  let file = File::open(&filename);
  let mut s = String::new();
  _ = file.and_then(|mut f| f.read_to_string(&mut s));

  let mut lexer = Lexer::new(s);
  let tokens = lexer.lex_all();
  let errors = lexer.errors();
  tokens
    .iter()
    .take(tokens.iter().len() - 1) // last is EOF
    .for_each(|t| println!("{t:?}"));
  if !errors.is_empty() {
    eprintln!("Lex errors:");
    errors.iter().for_each(|e| eprintln!("{e}"));
    exit(1);
  }
  if kind == "--lex" || kind == "lex" {
    println!("Lex succeeded.");
    return;
  }
  let mut parser = Parser::new(tokens);
  let program = parser.parse();
  println!("{}", program);

  let parse_warnings = parser.warnings();
  if !parse_warnings.is_empty() {
    eprintln!("Parse warnings:");
    parse_warnings.iter().for_each(|e| eprintln!("{e}"));
  }

  let parse_errors = parser.errors();
  if !parse_errors.is_empty() {
    eprintln!("Parse errors:");
    parse_errors.iter().for_each(|e| eprintln!("{e}"));
    exit(1);
  }
  println!("Parse succeeded.");
}
