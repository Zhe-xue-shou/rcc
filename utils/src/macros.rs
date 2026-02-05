#[macro_export]
macro_rules! interconvert {
  ($inner:ident, $outer:ident) => {
    $crate::interconvert!($inner, $outer, $inner);
  };

  ($inner:ident, $outer:ident, $variant:ident) => {
    impl From<$inner> for $outer {
      #[inline]
      fn from(value: $inner) -> Self {
        $outer::$variant(value)
      }
    }
    impl TryFrom<$outer> for $inner {
      type Error = ();

      #[inline]
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
  ($variant:ident, $main:ident) => {
    $crate::make_trio_for!($variant, $variant, $main);
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
                        $crate::breakpoint!();
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

            #[inline]
            pub fn [<into_ $variant:lower _unchecked>](self) -> $inner {
                match self {
                    Self::$variant(v) => v,
                    _ => {
                        $crate::breakpoint!();
                        unreachable!()
                    }
                }
            }

            #[inline]
            pub fn [<try_into_ $variant:lower>](self) -> Result<$inner, Self> {
                match self {
                    Self::$variant(v) => Ok(v),
                    _ => Err(self),
                }
            }
        }
    }
  };
}

#[macro_export]
#[cfg(debug_assertions)]
macro_rules! breakpoint {
  () => {
    $crate::breakpoint!("");
  };
  ($($arg:tt)*) => {{
    use ::std::io::{Write, stderr, stdout};
    eprintln!("Fatal error at {}:{}:", file!(), line!());
    eprintln!($($arg)*);
    _ = stdout().flush();
    _ = stderr().flush();
    ::core::hint::black_box(());
    ::std::intrinsics::breakpoint();
    ::core::hint::black_box(());
    _ = stdout().flush();
    _ = stderr().flush();
  }};
}

#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! breakpoint {
  () => {{}};
  ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! static_assert {
  ($condition:expr $(,)?) => {
    #[allow(clippy::bool_comparison)]
    const _: () = {
      assert!($condition);
    };
  };
  ($condition:expr, $($arg:tt)+) => {
    #[allow(clippy::bool_comparison)]
    const _: () = {
      assert!($condition, $($arg)+);
    };
  };
}
#[macro_export]
macro_rules! static_dispatch {
  ($this:ident.$func:ident $args:tt, $($variant:ident)*) => {
    match $this {
      $(
        Self::$variant(v) => v.$func $args,
      )*
    }
  }
}

/// This macro acts like `assert!`,
/// but with more clarification that it's the program's error, not the user's.
#[macro_export]
macro_rules! contract_assert {
  ($condition:expr) => {{
    if !$condition {
      eprintln!(
        "invariant at {}.
        This is a program internal error, please fix it!",
        ::std::panic::Location::caller()
      );
      panic!();
    }
  }};
  ($condition:expr, $($arg:tt)+) => {{
    if !$condition {
      eprintln!(
        "invariant: {} at {}.
        This is a program internal error, please fix it!",
        format!($($arg)+),
        ::std::panic::Location::caller()
      );
      panic!();
    }
  }};
}
/// This macro unconditionally signals a contract violation.
/// It acts like `panic!`,
/// but with more clarification that it's the program's error, not the user's.
#[macro_export]
macro_rules! contract_violation {
  () => {{
    eprintln!(
      "invariant at {}.
      This is a program internal error, please fix it!",
      ::std::panic::Location::caller()
    );
    panic!();
  }};
  ($($arg:tt)+) => {{
    eprintln!(
      "invariant at {}: {}.
      This is a program internal error, please fix it!",
      ::std::panic::Location::caller(),
      format!($($arg)+)
    );
    panic!();
  }};
}

/// like `todo!` or `unimplemented!`, but indicates a not implemented feature.
#[macro_export]
macro_rules! not_implemented_feature {
  () => {{
    panic!(
      "not implemented feature at {}",
      ::std::panic::Location::caller(),
    );
  }};
  ($($arg:tt)+) => {{
    panic!(
      "not implemented feature '{}' at {}",
      format!($($arg)+),
      ::std::panic::Location::caller(),
    );
  }};
}
