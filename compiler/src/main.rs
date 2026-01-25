#![allow(internal_features)]
#![allow(unstable_features)]
#![allow(unreachable_code)]
#![allow(unused_imports)]

use ::rc_core::{
  analyzer::Analyzer,
  common::{SourceDisplay, SourceManager},
  lexer::Lexer,
  parser::Parser,
};
use ::rc_utils::DisplayWith;
use ::std::{env::args, fs::File, io::Read, path::PathBuf, process::exit};

fn main() {
  let args = args().collect::<Vec<String>>();

  println!("Args: {:?}", args);

  let (kind, filename) = match args.as_slice() {
    [_] => ("all", "test.c"),
    [_, kind, filename] => (kind.as_str(), filename.as_str()),
    _ => {
      eprintln!("Usage: rcns [all|lex|parse] <filename>");
      exit(1);
    },
  };
  let mut source_manager = SourceManager::default();

  let _id = source_manager.try_add_file(filename.into());

  let mut lexer = Lexer::new(&source_manager.files[0].source);
  let tokens = lexer.lex_all();
  let errors = lexer.errors();
  tokens
    .iter()
    .take(tokens.iter().len() - 1) // last is EOF
    .for_each(|t| println!("{t:?}"));
  if !errors.is_empty() {
    eprintln!("Lex errors:");
    errors
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(&source_manager)));
    exit(1);
  }
  if kind == "--lex" || kind == "lex" {
    println!("Lex succeeded.");
    return;
  }
  let mut parser = Parser::new(tokens, &source_manager);
  let program = parser.parse();
  println!("{:}", program);

  let parse_warnings = parser.warnings();
  if !parse_warnings.is_empty() {
    eprintln!("Parse warnings:");
    parse_warnings
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(&source_manager)));
  }

  let parse_errors = parser.errors();
  if !parse_errors.is_empty() {
    eprintln!("Parse errors:");
    parse_errors.iter().for_each(|e| eprintln!("{e}"));
    exit(1);
  }
  if kind == "--parse" || kind == "parse" {
    println!("Parse succeeded.");
    return;
  }
  let mut analyzer = Analyzer::new(program);
  let translation_unit = analyzer.analyze();

  let analyze_warnings = analyzer.warnings();
  if !analyze_warnings.is_empty() {
    eprintln!("Analyze warnings:");
    analyze_warnings
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(&source_manager)));
  }
  let analyze_errors = analyzer.errors();
  if !analyze_errors.is_empty() {
    eprintln!("Analyze errors:");
    analyze_errors
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(&source_manager)));
    exit(1);
  }
  println!("{:}", translation_unit);
  println!("Analyze succeeded.");
}
