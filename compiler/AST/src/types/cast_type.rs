//! Revised v1: removed custom conversion.

#[derive(Debug, Clone, Copy, PartialEq, Eq, ::strum_macros::Display)]
pub enum CastType {
  /// don't use this for implicit casts - in that case no cast is needed;
  /// only used for explicit casts like `(int)x` where `x` is already `int`
  Noop,
  /// `(void)expr`
  ToVoid,
  /// pesudo cast; no actual conversion, just reinterpret the bits, like
  /// ```c
  /// int a = 42;
  /// double b = *(double *)&a; // copied
  /// ```
  /// or just
  /// ```cpp
  /// auto  a = 42;
  /// auto& b = reinterpret_cast<double&>(a); // no copy
  /// ```
  /// we got a fancy name for that in rust:
  /// ```rust
  /// let a = 42;
  /// let b = unsafe { ::std::mem::transmute::<i64, f64>(a) };
  /// ```
  BitCast,

  /// Read value from a variable (item 6.3.2.1).
  LValueToRValue,
  /// `int[10]` -> `int*`
  ArrayToPointerDecay,
  /// `void f()` -> `void(*)()`
  FunctionToPointerDecay,
  /// `nullptr` -> ptr
  NullptrToPointer,

  /// `int` -> `long`
  IntegralCast,
  /// `int` -> `float`
  IntegralToFloating,
  /// `int` -> `bool`/`_Bool`
  ///
  /// Only exists in explicit cast (in C).
  IntegralToBoolean,

  /// `float` -> `double`
  FloatingCast,
  /// `float` -> `int`
  FloatingToIntegral,
  /// `float` -> `bool`/`_Bool`
  ///
  /// this is *not* correct for conditional checks like `if (x)` where `x` is a float, but for explicit casts like `(bool)x`.
  ///
  /// Only exists in explicit cast (in C).
  FloatingToBoolean,

  /// `int` -> ptr
  IntegralToPointer,
  PointerToIntegral,
  /// ptr -> `bool`/`_Bool`
  ///
  /// Only exists in explicit cast (in C).
  PointerToBoolean,
}
