use ::std::ffi::c_void;

use crate::ensure_is_pod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opaque {
  ptr: *mut c_void,
  // #[cfg(debug_assertions)]
  // type_id: TypeId, //< T': static makes the struct with lifetime specs unusable here.
  #[cfg(debug_assertions)]
  type_name: &'static str,
}

ensure_is_pod!(Opaque);

#[cfg(debug_assertions)]
use ::std::any::type_name;

impl Opaque {
  pub const fn new<T>(data: &T) -> Self {
    Self {
      ptr: data as *const T as *mut c_void,
      // #[cfg(debug_assertions)]
      // type_id: TypeId::of::<T>(),
      #[cfg(debug_assertions)]
      type_name: type_name::<T>(),
    }
  }

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

  pub const fn get_ref<T: 'static>(self) -> &'static T {
    unsafe { &*self.get_ptr::<T>() }
  }

  pub const fn get_ref_mut<T: 'static>(self) -> &'static mut T {
    unsafe { &mut *self.get_ptr::<T>() }
  }

  #[cfg(debug_assertions)]
  const fn type_id_name(&self) -> &'static str {
    self.type_name
  }
}

impl ::std::fmt::Display for Opaque {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
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
  #[should_panic]
  const fn test_invalid_get() {
    const value: i32 = 42;
    const opaque: Opaque = Opaque::new(&value);
    let _retrieved_value: i64 = unsafe { *opaque.get_ptr::<i64>() };
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
