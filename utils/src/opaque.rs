use crate::ensure_is_pod;

/// An opaque handle that can store a pointer to any type, with optional debug information for type checking in debug mode.
/// This is served as a workaround to avoid both [`Box<dyn Any>`] and cyclic dependencies between modules.
///
/// You should always know what type you stored in the opaque handle, and retrieve it with the correct type,
/// otherwise it would cause undefined behavior. If the type stored is not clear, then this structure is probably the wrong choice
/// and [`Box<dyn Any>`] should be used instead. If the stored types are limited and known,
/// consider wrapping those inside an discriminated union (i.e., enum) and store the enum instead.
///
/// In debug mode, it would *panic* if the wrong type is retrieved to help catch potential bugs.
///
/// The struct and it's methods are all `const`-compatible, so it can be used in const contexts as well.
#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opaque {
  ptr: *mut c_void,
  type_name: &'static str,
  // type_id: TypeId, //< T': static makes the struct with lifetime specs unusable here.
}

#[cfg(not(debug_assertions))]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opaque {
  ptr: *mut c_void,
}

ensure_is_pod!(Opaque);

#[cfg(debug_assertions)]
use ::std::any::type_name;
use ::std::ffi::c_void;

impl Opaque {
  /// Creates a new opaque handle from a reference to any type.
  /// The caller must ensure that the reference outlives the opaque handle.
  /// Making a wrapper with [`PhantomData`](::std::marker::PhantomData) is recommended.
  #[must_use]
  #[allow(clippy::unnecessary_cast)]
  pub const fn new<T>(data: &T) -> Self {
    Self {
      ptr: &raw const *data as *const T as *mut c_void,
      #[cfg(debug_assertions)]
      type_name: type_name::<T>(),
      // #[cfg(debug_assertions)]
      // type_id: TypeId::of::<T>(),
    }
  }

  /// Retrieves a raw pointer to the stored data, with an optional type check in debug mode.
  ///
  /// Usually this is not the preferred way to retrieve the data, and [`Self::get_ref`] or [`Self::get_ref_mut`] should be used instead.
  ///
  /// ```rust
  /// let opaque = Opaque::new(&value);
  /// let ptr = opaque.get_ptr::<i32>();
  /// let value_ref = unsafe { &*ptr };
  /// let mut value_ref_mut = unsafe { &mut *ptr };
  /// ```
  ///
  /// ## Safety
  /// The caller must ensure that the type `T` matches the actual type of the stored data,
  /// otherwise it is *undefined behavior*.
  #[must_use]
  pub const fn get_ptr<T>(self) -> *mut T {
    #[cfg(debug_assertions)]
    {
      fn do_panic<T>(this: Opaque) -> ! {
        panic!(
          "Type mismatch in Opaque handle!\nStored: {}\nRequested: {}",
          this.type_id_name(),
          type_name::<T>()
        );
      }
      #[allow(clippy::extra_unused_type_parameters)]
      const fn static_assertion<T>(_this: Opaque) -> ! {
        panic!("type mismatch!")
      }

      if self.type_name != type_name::<T>() {
        use ::core::intrinsics::const_eval_select;

        const_eval_select((self,), static_assertion::<T>, do_panic::<T>)
      }
    }

    self.ptr as *mut T
  }

  /// Retrieves a reference to the stored data.
  ///
  /// This is the preferred way to retrieve the data, as you don't need to use `unsafe` block on the caller side.
  ///
  /// ```rust
  /// let opaque = Opaque::new(&value);
  /// let value_ref: &i32 = opaque.get_ref::<i32>();
  /// ```
  #[must_use]
  #[inline]
  pub const fn get_ref<T>(&self) -> &T {
    unsafe { &*self.get_ptr::<T>() }
  }

  /// Retrieves a mutable reference to the stored data.
  ///
  /// ```
  /// let opaque = Opaque::new(&value);
  /// let value_ref_mut: &mut i32 = opaque.get_ref_mut::<i32>();
  /// ```
  #[must_use]
  #[inline]
  pub const fn get_ref_mut<T>(&mut self) -> &mut T {
    unsafe { &mut *self.get_ptr::<T>() }
  }

  /// Retrieves the type name of the stored data in debug mode, or a placeholder string in release mode.
  #[cfg(debug_assertions)]
  #[must_use]
  #[inline]
  const fn type_id_name(&self) -> &'static str {
    self.type_name
  }

  #[cfg(not(debug_assertions))]
  #[must_use]
  #[inline]
  const fn type_id_name(&self) -> &'static str {
    "<typeinfo not available>"
  }
}

impl ::std::fmt::Display for Opaque {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    <*mut _ as ::std::fmt::Pointer>::fmt(&self.ptr, f)
  }
}

#[cfg(test)]
#[allow(clippy::unnecessary_cast)]
#[allow(non_upper_case_globals)]
mod tests {
  use ::std::marker::PhantomData;

  use super::*;
  use crate::const_assert_eq;

  #[test]
  const fn test_opaque() {
    {
      const value: i32 = 42;
      const opaque: Opaque = Opaque::new(&value);
      const retrieved_value: i32 = unsafe { *opaque.get_ptr::<i32>() };
      const_assert_eq!(opaque.type_id_name(), "i32");
      const_assert_eq!(value, retrieved_value);
    }
    {
      const value: &str = "Hello, world!";
      const opaque: Opaque = Opaque::new(&value);
      const retrieved_value: &str = opaque.get_ref::<&str>();
      const_assert_eq!(opaque.type_id_name(), "&str");
      const_assert_eq!(value, retrieved_value);
    }
  }
  #[test]
  const fn test_mutable() {
    let value: i32 = 42;
    let mut opaque: Opaque = Opaque::new(&value);
    let retrieved_value: &mut i32 = opaque.get_ref_mut::<i32>();
    *retrieved_value += 1;
    const_assert_eq!(value, 43);
  }
  #[test]
  #[should_panic]
  const fn test_invalid_get() {
    const value: i32 = 42;
    const opaque: Opaque = Opaque::new(&value);
    let _panicked: i64 = unsafe { *opaque.get_ptr::<i64>() };
  }
  trait DynamicTrait {
    fn get_value(&self) -> i32;
  }
  struct DynamicStruct {
    value: i32,
  }

  impl DynamicStruct {
    const fn new(value: i32) -> Self {
      Self { value }
    }
  }
  impl DynamicTrait for DynamicStruct {
    fn get_value(&self) -> i32 {
      self.value
    }
  }
  #[test]
  fn dyn_currect() {
    {
      const value: DynamicStruct = DynamicStruct::new(123);
      const opaque: Opaque = Opaque::new(&value);
      let retrieved_value: &DynamicStruct =
        unsafe { &*opaque.get_ptr::<DynamicStruct>() };
      debug_assert_eq!(retrieved_value.get_value(), 123);
    }
    {
      let boxed: Box<dyn DynamicTrait> = Box::new(DynamicStruct::new(456));
      let opaque: Opaque = Opaque::new(&boxed);
      let retrieved_box = opaque.get_ref::<Box<dyn DynamicTrait>>();
      debug_assert_eq!(retrieved_box.get_value(), 456);
    }
  }
  #[test]
  #[should_panic]
  fn dyn_fail() {
    {
      let boxed: Box<dyn DynamicTrait> = Box::new(DynamicStruct::new(456));
      let opaque: Opaque = Opaque::new(&boxed);
      let dynamic_cast = opaque.get_ref::<Box<DynamicStruct>>();
      debug_assert_eq!(dynamic_cast.get_value(), 456);
    }
  }

  struct Lifetime<'c> {
    value: String,
    data: PhantomData<&'c str>,
  }

  impl<'c> Lifetime<'c> {
    fn new(value: String) -> Self {
      Self {
        value,
        data: PhantomData,
      }
    }
  }

  #[test]
  #[allow(
    clippy::extra_unused_lifetimes,
    reason = "Explicit lifetime is required to test"
  )]
  fn lifetime_test<'c>() {
    let lifetime = Lifetime::<'c>::new("Hello, lifetime!".to_string());
    let opaque = Opaque::new(&lifetime);
    let retrieved_lifetime: &Lifetime =
      unsafe { &*opaque.get_ptr::<Lifetime>() };
    assert_eq!(retrieved_lifetime.value, "Hello, lifetime!");
  }
}
