use ::rc_core::{
  analyzer::Analyzer, common::SourceManager, lexer::Lexer, parser::Parser,
};
use ::rc_utils::DisplayWith;
enum Stage {
  Lex,
  Parse,
  Analyze,
}
fn main() {
  let args = ::std::env::args().collect::<Vec<String>>();

  println!("Args: {:?}", args);

  let (kind, filename) = match args.as_slice() {
    [_] => ("all", "test.c"),
    [_, kind, filename] => (kind.as_str(), filename.as_str()),
    _ => {
      eprintln!("Usage: rc [all|lex|parse] <filename>");
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

  let stage = match kind {
    "all" | "--all" => Stage::Analyze,
    "lex" | "--lex" => Stage::Lex,
    "parse" | "--parse" => Stage::Parse,
    _ => {
      eprintln!("Unknown stage: {}", kind);
      ::std::process::exit(1);
    },
  };
  pipeline(&mut source_manager, stage, false);
}

fn pipeline(
  source_manager: &mut SourceManager,
  stage: Stage,
  pretty_print: bool,
) {
  let content = &source_manager.files.get(0).unwrap().source;
  let mut lexer = Lexer::new(content);
  let tokens = lexer.lex();
  let errors = lexer.errors();
  tokens
    .iter()
    .take(tokens.iter().len() - 1) // last is EOF
    .for_each(|t| {
      if pretty_print {
        println!("{t}");
      } else {
        println!("{} ", t);
      }
    });
  if !errors.is_empty() {
    eprintln!("Lex errors:");
    errors
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(&source_manager)));
    ::std::process::exit(1);
  }
  if let Stage::Lex = stage {
    println!("Lex succeeded.");
    return;
  }
  let mut parser = Parser::new(tokens);
  let program = parser.parse();
  println!("{program}");
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
  if let Stage::Parse = stage {
    if pretty_print {
      println!("{:#?}", program);
    }
    println!("Parse succeeded.");
    return;
  }
  assert!(matches!(stage, Stage::Analyze));
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
  if pretty_print {
    println!("{:#?}", translation_unit);
  }
  println!("{translation_unit}");

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

  #[test]
  fn t1() {
    let s = r#"
static const volatile int **const *const* volatile 
    volatile_ptr_to_very_const_ptr_to_very_const_ptr;
    int main(int argc, char **);
"#;
    test_str(s);
  }
  #[test]
  fn t2() {
    let s = "int (*func_ptr)(int, int);";
    test_str(s);
  }
  fn test_str(source: &str) {
    let mut source_manager = SourceManager::default();
    source_manager.add_string(source.into());
    pipeline(&mut source_manager, Stage::Analyze, true);
  }
}
