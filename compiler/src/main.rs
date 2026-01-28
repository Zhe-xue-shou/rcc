use ::rc_core::{
  analyzer::Analyzer, common::SourceManager, lexer::Lexer, parser::Parser,
};
use ::rc_utils::DisplayWith;

fn main() {
  let args = ::std::env::args().collect::<Vec<String>>();

  println!("Args: {:?}", args);

  let (kind, filename) = match args.as_slice() {
    [_] => ("all", "test.c"),
    [_, kind, filename] => (kind.as_str(), filename.as_str()),
    _ => {
      eprintln!("Usage: rcns [all|lex|parse] <filename>");
      ::std::process::exit(1);
    },
  };
  let mut source_manager = SourceManager::default();

  let _id = source_manager
    .try_add_file(filename.into())
    .unwrap_or_else(|e| {
      eprintln!("Error reading file {}: {}", filename, e);
      ::std::process::exit(1);
    });

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
    ::std::process::exit(1);
  }
  if kind == "--lex" || kind == "lex" {
    println!("Lex succeeded.");
    return;
  }
  let mut parser = Parser::new(tokens);
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
    parse_errors
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(&source_manager)));
    ::std::process::exit(1);
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
    ::std::process::exit(1);
  }
  println!("{:}", translation_unit);
  println!("Analyze succeeded.");
}

#[cfg(test)]
mod test {
  use ::std::{backtrace::Backtrace, panic::Location};

  #[allow(unused_imports)]
  use super::*;
  #[test]
  fn caller() {
    dummy();
  }
  #[track_caller]
  fn dummy() {
    println!("{}", Location::caller());
    println!("{}", Backtrace::capture())
  }
}
