///
/// ```
/// use rcc_utils::interconvert;
/// struct InnerHasLifetime<'a> {
///   a: &'a i32,
/// }
/// struct InnerNoLifetime;
/// enum LifetimeOuter<'a> {
///   InnerHasLifetime(InnerHasLifetime<'a>),
///   InnerNoLifetime(InnerNoLifetime),
/// }
/// interconvert!(InnerNoLifetime, LifetimeOuter<'a>);
/// interconvert!(InnerHasLifetime, LifetimeOuter,'a);
///
/// enum OuterNoLifetime{
///   InnerNoLifetime(InnerNoLifetime),
///   InnerVariantAlias(InnerNoLifetime),
/// }
/// interconvert!(InnerNoLifetime, OuterNoLifetime);
/// ```
#[macro_export]
macro_rules! interconvert {
  // no lifetimes
  ($inner:ident, $outer:ident) => {
    $crate::interconvert!($inner, $outer, $inner);
  };

  // with variant, no lifetimes
  ($inner:ident, $outer:ident, $variant:ident) => {
    $crate::interconvert!(@impl $inner, $outer, $variant, [], []);
  };

  // Both lifetimes
  ($inner:ident, $outer:ident, $outer_lt:lifetime) => {
    $crate::interconvert!(@impl $inner, $outer, $inner, [<$outer_lt>], [<$outer_lt>]);
  };

  // Both lifetimes
  ($inner:ident, $outer:ident, $($outer_lt:lifetime)?) => {
    $crate::interconvert!(@impl $inner, $outer, $inner, [$(<$outer_lt>)?], [$(<$outer_lt>)?]);
  };


  // Both lifetimes
  ($inner:ident, $outer:ident, $outer_lt:lifetime, $variant:ident) => {
    $crate::interconvert!(@impl $inner, $outer, $variant, [<$outer_lt>], [<$outer_lt>]);
  };

  // outer lifetime
  ($inner:ident, $outer:ident<$($outer_lt:lifetime)?>) => {
    $crate::interconvert!(@impl $inner, $outer, $inner, [], [<$($outer_lt)?>]);
  };
    // outer lifetime
  ($inner:ident, $outer:ident<$outer_lt:lifetime>) => {
    $crate::interconvert!(@impl $inner, $outer, $inner, [], [<$outer_lt>]);
  };

  // outer lifetime
  ($inner:ident, $outer:ident<$outer_lt:lifetime>, $variant:ident) => {
    $crate::interconvert!(@impl $inner, $outer, $variant, [], [<$outer_lt>]);
  };


  (@impl $inner:ident, $outer:ident, $variant:ident, [$($inner_lt:tt)*], [$($outer_lt:tt)*]) => {
    impl$($outer_lt)* From<$inner$($inner_lt)*> for $outer$($outer_lt)* {
      #[inline]
      fn from(value: $inner$($inner_lt)*) -> Self {
        $outer::$variant(value)
      }
    }
    impl$($outer_lt)* TryFrom<$outer$($outer_lt)*> for $inner$($inner_lt)* {
      type Error = ();

      #[inline]
      fn try_from(value: $outer$($outer_lt)*) -> Result<Self, Self::Error> {
        match value {
          $outer::$variant(inner) => Ok(inner),
          _ => Err(()),
        }
      }
    }
  };
}
///
/// ```
/// use rcc_utils::make_trio_for;
/// struct InnerHasLifetime<'a> {
///   a: &'a i32,
/// }
/// struct InnerNoLifetime;
/// enum LifetimeOuter<'a> {
///   InnerHasLifetime(InnerHasLifetime<'a>),
///   InnerNoLifetime(InnerNoLifetime),
/// }
/// make_trio_for!(InnerNoLifetime, LifetimeOuter<'a>);
/// make_trio_for!(InnerHasLifetime, LifetimeOuter,'a);
///
/// enum OuterNoLifetime{
///   InnerNoLifetime(InnerNoLifetime),
///   InnerVariantAlias(InnerNoLifetime),
/// }
/// make_trio_for!(InnerNoLifetime, OuterNoLifetime);
/// make_trio_for!(InnerNoLifetime, OuterNoLifetime, InnerVariantAlias);
/// ```
#[macro_export]
macro_rules! make_trio_for {
  // no lifetimes
  ($inner:ident, $main:ident) => {
    $crate::make_trio_for!($inner, $main, $inner);
  };

  // no lifetimes
  ($inner:ident, $main:ident, $variant:ident) => {
    $crate::make_trio_for!(@impl $inner, $main, $variant, [], []);
  };

  // Both lifetimes
  ($inner:ident, $main:ident, $main_lt:lifetime) => {
    $crate::make_trio_for!(@impl $inner, $main, $inner, [<$main_lt>], [<$main_lt>]);
  };

  // Both lifetimes -- only used for macro chaning!
  ($inner:ident, $main:ident, $($main_lt:lifetime)?) => {
    $crate::make_trio_for!(@impl $inner, $main, $inner, [<$($main_lt)?>], [<$($main_lt)?>]);
  };

  // Both lifetimes
  ($inner:ident, $main:ident, $main_lt:lifetime, $variant:ident) => {
    $crate::make_trio_for!(@impl $inner, $main, $variant, [<$main_lt>], [<$main_lt>]);
  };

  // Only main lifetime
  ($inner:ident, $main:ident<$main_lt:lifetime>) => {
    $crate::make_trio_for!(@impl $inner, $main, $inner, [], [<$main_lt>]);
  };

  // Only main lifetime
  ($inner:ident, $main:ident<$main_lt:lifetime>, $variant:ident) => {
    $crate::make_trio_for!(@impl $inner, $main, $variant, [], [<$main_lt>]);
  };

  (@impl $inner:ident, $main:ident, $variant:ident, [$($inner_lt:tt)*], [$($main_lt:tt)*]) => {
    $crate::paste! {
      impl$($main_lt)* $main$($main_lt)* {
        #[inline]
        pub fn [<is_ $variant:lower>](&self) -> bool {
          matches!(self, Self::$variant(_))
        }

        #[inline]
        pub fn [<as_ $variant:lower>](&self) -> Option<&$inner$($inner_lt)*> {
          match self {
            Self::$variant(v) => Some(v),
            _ => None,
          }
        }

        #[inline]
        pub fn [<as_ $variant:lower _unchecked>](&self) -> &$inner$($inner_lt)* {
          match self {
            Self::$variant(v) => v,
            _ => unreachable!()
          }
        }

        #[inline]
        pub fn [<into_ $variant:lower>](self) -> Option<$inner$($inner_lt)*> {
          match self {
            Self::$variant(v) => Some(v),
            _ => None,
          }
        }

        #[inline]
        pub fn [<into_ $variant:lower _unchecked>](self) -> $inner$($inner_lt)* {
          match self {
            Self::$variant(v) => v,
            _ => unreachable!()
          }
        }

        #[inline]
        pub fn [<try_into_ $variant:lower>](self) -> Result<$inner$($inner_lt)*, Self> {
          match self {
            Self::$variant(v) => Ok(v),
            _ => Err(self),
          }
        }

        #[inline]
        pub fn [<as_ $variant:lower _mut>](&mut self) -> Option<&mut $inner$($inner_lt)*> {
          match self {
            Self::$variant(v) => Some(v),
            _ => None,
          }
        }

        #[inline]
        pub fn [<as_ $variant:lower _mut_unchecked>](&mut self) -> &mut $inner$($inner_lt)* {
          match self {
            Self::$variant(v) => v,
            _ => unreachable!()
          }
        }
      }
    }
  };
}

#[macro_export]
macro_rules! make_trio_for_unit_tuple {
  ($inner:ident, $main:ident<$main_lt:lifetime>) => {
    $crate::make_trio_for_unit_tuple!(@impl $inner, $main, [<$main_lt>]);
  };

  (@impl $variant:ident, $main:ident, [$($main_lt:tt)*]) => {
    $crate::paste! {
      impl$($main_lt)* $main$($main_lt)* {
        #[inline]
        pub fn [<is_ $variant:lower>](&self) -> bool {
          matches!(self, Self::$variant())
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
macro_rules! static_assert_eq {
  ($lhs:expr, $right:expr, $($arg:tt)+) => {
    #[allow(clippy::bool_comparison)]
    const _: () = {
      assert!($lhs == $right, $($arg)+);
    };
  };
  ($lhs:expr, $right:expr) => {
    #[allow(clippy::bool_comparison)]
    const _: () = {
      assert!($lhs == $right);
    };
  };
}
#[macro_export]
macro_rules! ensure_is_pod {
  ($ty:ty) => {
    $crate::static_assert!(
      ::std::mem::needs_drop::<$ty>() == false,
      concat!("Type ", stringify!($ty), " is not POD")
    );
  };
}
#[macro_export]
macro_rules! static_dispatch {
    (
        $enum_type:ident:      // The Enum
        $target:expr,       // The thing to match
        |$v:ident| $body:expr => // The binder call expr
        $($variant:ident)* // List of variants
    ) => {
        match $target {
            $(
                $enum_type::$variant($v) => $body,
            )*
        }
    };

    // wrapper for `Self`
    (
        $target:expr,
        |$v:ident| $body:expr =>
        $($variant:ident)*
    ) => {
        $crate::static_dispatch!(Self: $target, |$v| $body => $($variant)*)
    };
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

/// [`assert`] in constant evaluation, [`debug_assert`] in runtime evaluation.
#[macro_export]
macro_rules! const_assert {
  ($cond:expr) => {
    ::core::intrinsics::const_eval_select(
      ($cond, stringify!($cond)),
      $crate::static_assert,
      $crate::debug_assertion,
    )
  };
  ($cond:expr, $msg:expr) => {
    ::core::intrinsics::const_eval_select(
      ($cond, $msg),
      $crate::static_assert,
      $crate::debug_assertion,
    )
  };
}
#[macro_export]
macro_rules! const_assert_eq {
  ($left:expr, $right:expr) => {
    $crate::const_assert!($left == $right)
  };
  ($left:expr, $right:expr, $msg:expr) => {
    $crate::const_assert!($left == $right, $msg)
  };
}

mod tests {}
