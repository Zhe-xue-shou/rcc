use ::rcc_core::{
  common::SourceManager,
  diagnosis::Diagnosis,
  ir::{Context as IRContext, Emitter as IREmitter, IRDumper},
  lexer::Lexer,
  parse::Parser,
  sema::{ASTDumper, Sema},
  session::{Session, SessionRef},
  storage::{Arena, Storage},
  types::Context as ASTContext,
};
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
    [_] => ("all", "test.c"),
    [_, kind] => (kind.as_str(), "test.c"),
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
  let arena = Arena::default();
  let storage = arena.alloc(Storage::new(&arena));
  let ast_context = arena.alloc(ASTContext::new(storage));
  let ir_context = arena.alloc(IRContext::new(storage));
  let session =
    arena.alloc(Session::new(&source_manager, ast_context, ir_context));
  pipeline(session, stage, false);
}

fn pipeline(session: SessionRef, stage: Stage, pretty_print: bool) -> i32 {
  // let content = session.manager.files.first().unwrap().source;
  let mut lexer = Lexer::new(session);
  let tokens = lexer.lex();
  if session.diagnosis.has_errors() {
    eprintln!("Lex errors:");
    session
      .diagnosis
      .errors()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(session.manager)));
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
  let mut parser = Parser::new(tokens, session);
  let program = parser.parse();
  if session.diagnosis.has_errors() {
    eprintln!("Parser errors:");
    session
      .diagnosis
      .errors()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(session.manager)));
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

  let mut analyzer = Sema::new(program, session);
  let translation_unit = analyzer.analyze();
  if session.diagnosis.has_errors() {
    eprintln!("Analyzer errors:");
    session
      .diagnosis
      .errors()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(session.manager)));
    return 1;
  }
  if let Stage::Analyze = stage {
    if pretty_print {
      println!("{:#?}", translation_unit);
    }
    println!("{translation_unit}");
    println!("Analyze succeeded.");
  }

  if session.diagnosis.has_warnings() {
    eprintln!("Analyzer warnings:");
    session
      .diagnosis
      .warnings()
      .iter()
      .for_each(|e| eprintln!("{}", e.display_with(session.manager)));
  }
  ASTDumper::dump(&translation_unit, session).unwrap();
  if let Stage::Analyze = stage {
    return 0;
  }
  assert!(matches!(stage, Stage::Ir));
  let builder = IREmitter::new(session);

  let m = builder.build(translation_unit);
  println!("{m:#?}");
  IRDumper::dump(&m, session).unwrap();
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
  fn test_str(_source: &str) -> i32 {
    0
    // let mut source_manager = SourceManager::default();
    // source_manager.add_string(source.into());
    // let arena = Default::default();
    // let ast_type_interner = Default::default();
    // let ast_str_interner = Default::default();
    // let ast_context =
    //   ASTContext::new(&arena, &ast_type_interner, &ast_str_interner);
    // let ir_type_interner = Default::default();
    // let slotmap = Default::default();
    // let ir_context = IRContext::new(&arena, &ir_type_interner, &slotmap);
    // let session = Session::new(&source_manager, &ast_context, &ir_context);
    // pipeline(session, Stage::Analyze, true)
  }
  #[test]
  fn t4() {
    use ::std::io::Write;
    use termcolor::*;
    // Create a stream for Standard Output (Stdout)
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // 1. Create a Color Specification
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Cyan)).set_bold(true);

    // 2. Apply the spec to the stream
    stdout.set_color(&spec).unwrap();

    // 3. Write your text
    write!(&mut stdout, "BinaryExpr").unwrap();

    // 4. Reset to default colors
    stdout.reset().unwrap();
    writeln!(&mut stdout, ": +").unwrap();
  }
}
