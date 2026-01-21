#[macro_export]
macro_rules! interconvert {
  ($inner:ident, $outer:ident) => {
    ::rc_utils::interconvert!($inner, $outer, $inner);
  };

  ($inner:ident, $outer:ident, $variant:ident) => {
    impl From<$inner> for $outer {
      fn from(value: $inner) -> Self {
        $outer::$variant(value)
      }
    }
    impl TryFrom<$outer> for $inner {
      type Error = ();

      fn try_from(value: $outer) -> Result<Self, Self::Error> {
        match value {
          $outer::$variant(inner) => Ok(inner),
          _ => Err(()),
        }
      }
    }
  };
}

#[macro_export]
macro_rules! make_trio_for {
  ($variant:ident,$main:ident) => {
    make_trio_for!($variant, $variant, $main);
  };
  // We use :ident because we are working with names, not complex types
  ($variant:ident, $inner:ident, $main:ident) => {
    ::paste::paste! {
        impl $main {
            #[inline]
            pub fn [<is_ $variant:lower>](&self) -> bool {
                matches!(self, Self::$variant(_))
            }

            #[inline]
            pub fn [<as_ $variant:lower>](&self) -> Option<&$inner> {
                match self {
                    Self::$variant(v) => Some(v),
                    _ => None,
                }
            }

            #[inline]
            pub fn [<as_ $variant:lower _unchecked>](&self) -> &$inner {
                match self {
                    Self::$variant(v) => v,
                    _ => {
                        ::rc_utils::breakpoint!();
                        unreachable!()
                    }
                }
            }

            #[inline]
            pub fn [<into_ $variant:lower>](self) -> Option<$inner> {
                match self {
                    Self::$variant(v) => Some(v),
                    _ => None,
                }
            }
        }
    }
  };
}

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

pub type SmallString = compact_str::CompactString;
