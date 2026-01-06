#![allow(internal_features)]
#![allow(unstable_features)]
#![allow(unreachable_code)]
#![allow(unused_imports)]

use ::std::{env::args, fs::File, io::Read, path::PathBuf, process::exit};
use rcns::{analyzer::Analyzer, lexer::Lexer, parser::Parser};
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

  let mut lexer = Lexer::new(
    s,
    std::path::absolute(filename).unwrap_or(PathBuf::from("<invalid path>")),
  );
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
  println!("{:}", program);

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
  if kind == "--parse" || kind == "parse" {
    println!("Parse succeeded.");
    return;
  }
  let mut analyzer = Analyzer::new(program);
  let translation_unit = analyzer.analyze();

  let analyze_warnings = analyzer.warnings();
  if !analyze_warnings.is_empty() {
    eprintln!("Analyze warnings:");
    analyze_warnings.iter().for_each(|e| eprintln!("{e}"));
  }
  let analyze_errors = analyzer.errors();
  if !analyze_errors.is_empty() {
    eprintln!("Analyze errors:");
    analyze_errors.iter().for_each(|e| eprintln!("{e}"));
    exit(1);
  }
  println!("{:}", translation_unit.unwrap());
  println!("Analyze succeeded.");
}
