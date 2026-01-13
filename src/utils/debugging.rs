#[macro_export]
macro_rules! breakpoint {
  () => {{
    use std::io::{Write, stderr, stdout};
    _ = stdout().flush();
    _ = stderr().flush();
    eprintln!("Breakpoint at {}:{}", file!(), line!());
    _ = stdout().flush();
    _ = stderr().flush();
    std::intrinsics::breakpoint();
  }};
  ($($arg:tt)*) => {{
    use std::io::{Write, stderr, stdout};
    eprintln!("Fatal error at {}:{}:", file!(), line!());
    eprintln!($($arg)*);
    _ = stdout().flush();
    _ = stderr().flush();
    std::intrinsics::breakpoint();
    _ = stdout().flush();
    _ = stderr().flush();
  }};
}
