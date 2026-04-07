use ::rcc_ast::{Context as ASTContext, Session as ASTSession};
use ::rcc_ir::{
  Context as IRContext, Emitter as IREmitter, Session as IRSession,
};
use ::rcc_lex::Lexer;
use ::rcc_parse::Parser;
use ::rcc_sema::Sema;
use ::rcc_serialize::{ASTDumper, IRPrinter};
use ::rcc_shared::{Arena, Diagnosis, OpDiag, SourceManager};
use ::rcc_utils::DisplayWith;
enum Stage {
  Lex,
  Parse,
  Analyze,
  Ir,
}
fn main() {
  let args = ::std::env::args().collect::<Vec<String>>();

  println!("Args: {:?}", args);

  let (kind, filename) = match args.as_slice() {
    [_] => ("all", "./tests/test.c"),
    [_, kind] => (kind.as_str(), "./tests/test.c"),
    [_, kind, filename] => (kind.as_str(), filename.as_str()),
    _ => {
      eprintln!("Usage: rcc [all|lex|parse|analyze] <filename>");
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
    "all" | "--all" => Stage::Ir,
    "lex" | "--lex" => Stage::Lex,
    "parse" | "--parse" => Stage::Parse,
    "analyze" | "--analyze" => Stage::Analyze,
    _ => {
      eprintln!("Unknown stage: {}", kind);
      ::std::process::exit(1);
    },
  };

  pipeline(source_manager, stage, false);
}

fn pipeline(manager: SourceManager, stage: Stage, pretty_print: bool) -> i32 {
  let arena = Arena::default();
  let ast_context = arena.alloc(ASTContext::new(&arena));
  let diagnosis = OpDiag::default();
  let ast_session = ASTSession::new(&diagnosis, &manager, ast_context);
  let mut lexer = Lexer::new(&ast_session);
  let tokens = lexer.lex();
  if ast_session.diag().has_errors() {
    eprintln!("Lex errors:");
    ast_session
      .diag()
      .errors()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(ast_session.src())));
    return 1;
  }
  if let Stage::Lex = stage {
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
    println!("Lex succeeded.");
    return 0;
  }
  let mut parser = Parser::new(tokens, &ast_session);
  let program = parser.parse();
  if ast_session.diag().has_errors() {
    eprintln!("Parser errors:");
    ast_session
      .diag()
      .errors()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(ast_session.src())));
    return 1;
  }
  if let Stage::Parse = stage {
    if pretty_print {
      println!("{:#?}", program);
    }
    println!("{program}");
    println!("Parse succeeded.");
    return 0;
  }

  let mut analyzer = Sema::new(program, &ast_session);
  let translation_unit = analyzer.analyze();

  ASTDumper::dump(&translation_unit, &ast_session).unwrap();

  if ast_session.diag().has_warnings() {
    eprintln!("Analyzer warnings:");
    ast_session
      .diag()
      .warnings()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(ast_session.src())));
  }

  if ast_session.diag().has_errors() {
    eprintln!("Analyzer errors:");
    ast_session
      .diag()
      .errors()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(ast_session.src())));
    return 1;
  }
  if let Stage::Analyze = stage {
    if pretty_print {
      println!("{:#?}", translation_unit);
    }
    println!("{translation_unit}");
    println!("Analyze succeeded.");
  }

  if let Stage::Analyze = stage {
    return 0;
  }
  assert!(matches!(stage, Stage::Ir));
  let ir_context = arena.alloc(IRContext::new(&arena, ast_context));
  let session = IRSession::new(
    ast_session.diag(),
    ast_session.src(),
    ast_session.ast(),
    ir_context,
  );
  let builder = IREmitter::new(&session);

  let module = builder.build(translation_unit);
  IRPrinter::print(&module, &session).unwrap();
  0
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
int *normal_ptr;
const int *ptr_to_const;
int const *ptr_to_const2;
int *const const_ptr;
const int *const const_ptr_to_const;

int **ptr_to_ptr;
int *const *ptr_to_const_ptr;
int **const const_ptr_to_ptr;
const int **ptr_to_ptr_to_const;
int *const *const const_ptr_to_const_ptr;
const int **const const_ptr_to_ptr_to_const;
const int *const *ptr_to_const_ptr_to_const;
const int *const *const const_ptr_to_const_ptr_to_const;

// well, if this passed parsing, it might be... ok ig
static const volatile int **const *const
    *volatile volatile_ptr_to_very_const_ptr_to_very_const_ptr;
// func ptr test
extern int j;
static int j = 0;
extern int j;
int j;

"#;
    test_str(s);
  }
  #[test]
  fn t2() {
    let s = r#"
int main(int argc, char **argv) { //
  goto label;
  {
  label:;
    int k = foo(0);
  }
  int f(int, int);
  typedef int const CONST_INT;
  INT x = sizeof(char);
  typedef int const CONST_INT;
  int foo;
  CONST_INT(INT) = (10);
  static int y = sizeof x;
  switch (x) {
  case 3.0 / 5.0: // case expr shall be integral constant expression
  case 2147483647 + 1: // overflow test
    y = y + 1;
    x = x + 1;
    break;
  default:
    y = y + 2;
  }
  for (int i = 0; i < 10; i = i + 1) { // my parser can't handle += and ++
    y = y + i;
    continue;
  }
  const int a = 2.0 / 3;
  return f(2, 3);
}
    "#;
    assert_eq!(test_str(s), 1);
  }
  #[test]
  fn t3() {
    let s = "long int p = 0 && 8 ? 1, 0 : 2;";
    assert_eq!(test_str(s), 0);
  }
  fn test_str(source: &str) -> i32 {
    let mut source_manager = SourceManager::default();
    source_manager.add_string(source.into());

    pipeline(source_manager, Stage::Analyze, true)
  }
}
