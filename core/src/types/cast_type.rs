use ::strum_macros::Display;

#[derive(Debug, Display)]
pub enum CastType {
  Noop, // don't use this for implicit casts - in that case no cast is needed; only used for explicit casts like (int)x where x is already int
  ToVoid, // (void)expr

  LValueToRValue,         // Read value from a variable (6.3.2.1)
  ArrayToPointerDecay,    // int[10] -> int*
  FunctionToPointerDecay, // void f() -> void(*)()
  NullptrToPointer,       // nullptr -> ptr

  IntegralCast, // int -> long, unsigned -> int - bit widening/narrowing
  IntegralToFloating, // int -> float
  IntegralToBoolean, // int -> bool (val != 0)

  FloatingCast,       // float -> double
  FloatingToIntegral, // float -> int
  FloatingToBoolean,  // float -> bool (val != 0.0)

  IntegralToPointer, // int -> ptr (addr 0 is null)
  PointerToIntegral,
  PointerToBoolean, // ptr -> bool (ptr != 0)
  BitCast, // pesudo cast; no actual conversion, just reinterpret the bits

  // ^^^ those exist in Clang's frontend too
  // vvv custom casts
  NullptrToIntegral, // nullptr -> int
  NullptrToBoolean,  // nullptr -> bool
}
