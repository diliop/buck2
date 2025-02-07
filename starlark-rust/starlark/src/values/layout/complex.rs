/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::marker::PhantomData;

use dupe::Clone_;
use dupe::Copy_;
use dupe::Dupe_;
use either::Either;

use crate::typing::Ty;
use crate::values::type_repr::StarlarkTypeRepr;
use crate::values::AllocValue;
use crate::values::ComplexValue;
use crate::values::StarlarkValue;
use crate::values::UnpackValue;
use crate::values::Value;
use crate::values::ValueLike;

/// Value which is either a complex mutable value or a frozen value.
#[derive(Copy_, Clone_, Dupe_)]
pub struct ValueOfComplex<'v, T>(Value<'v>, PhantomData<T>)
where
    T: ComplexValue<'v>,
    T::Frozen: StarlarkValue<'static>;

impl<'v, T> ValueOfComplex<'v, T>
where
    T: ComplexValue<'v>,
    T::Frozen: StarlarkValue<'static>,
{
    /// Get the value back.
    #[inline]
    pub fn to_value(self) -> Value<'v> {
        self.0
    }

    /// Unpack the mutable or frozen value.
    #[inline]
    pub fn unpack(self) -> Either<&'v T, &'v T::Frozen> {
        if let Some(v) = self.0.downcast_ref::<T>() {
            Either::Left(v)
        } else if let Some(v) =
            unsafe { self.0.to_value().cast_lifetime() }.downcast_ref::<T::Frozen>()
        {
            Either::Right(v)
        } else {
            unreachable!("validated at construction")
        }
    }
}

impl<'v, T> StarlarkTypeRepr for ValueOfComplex<'v, T>
where
    T: ComplexValue<'v>,
    T::Frozen: StarlarkValue<'static>,
{
    fn starlark_type_repr() -> Ty {
        T::starlark_type_repr()
    }
}

impl<'v, T> AllocValue<'v> for ValueOfComplex<'v, T>
where
    T: ComplexValue<'v>,
    T::Frozen: StarlarkValue<'static>,
{
    #[inline]
    fn alloc_value(self, _heap: &'v crate::values::Heap) -> Value<'v> {
        self.0
    }
}

impl<'v, T> UnpackValue<'v> for ValueOfComplex<'v, T>
where
    T: ComplexValue<'v>,
    T::Frozen: StarlarkValue<'static>,
{
    fn unpack_value(value: Value<'v>) -> Option<Self> {
        if value.downcast_ref::<T>().is_some()
            || unsafe { value.cast_lifetime() }
                .downcast_ref::<T::Frozen>()
                .is_some()
        {
            Some(ValueOfComplex(value, PhantomData))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use allocative::Allocative;
    use anyhow::Context;
    use either::Either;
    use starlark_derive::starlark_module;
    use starlark_derive::Freeze;
    use starlark_derive::NoSerialize;
    use starlark_derive::Trace;

    use crate as starlark;
    use crate::any::ProvidesStaticType;
    use crate::assert::Assert;
    use crate::const_frozen_string;
    use crate::environment::GlobalsBuilder;
    use crate::values::layout::complex::ValueOfComplex;
    use crate::values::starlark_value;
    use crate::values::StarlarkValue;
    use crate::values::Value;
    use crate::values::ValueLike;

    #[derive(
        Trace,
        Freeze,
        Debug,
        derive_more::Display,
        Allocative,
        ProvidesStaticType,
        NoSerialize
    )]
    struct TestValueOfComplex<V>(V);

    #[starlark_value(type = "test_value_of_complex")]
    impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for TestValueOfComplex<V> where
        Self: ProvidesStaticType<'v>
    {
    }

    #[starlark_module]
    fn test_module(globals: &mut GlobalsBuilder) {
        fn test_unpack<'v>(
            v: ValueOfComplex<'v, TestValueOfComplex<Value<'v>>>,
        ) -> anyhow::Result<&'v str> {
            Ok(match v.unpack() {
                Either::Left(v) => v.0.unpack_str().context("not a string")?,
                Either::Right(v) => v.0.to_value().unpack_str().context("not a string")?,
            })
        }
    }

    #[test]
    fn test_unpack() {
        let mut a = Assert::new();
        a.globals_add(test_module);
        a.setup_eval(|eval| {
            let s = eval.heap().alloc("test1");
            let x = eval.heap().alloc_complex(TestValueOfComplex(s));
            let y = eval.frozen_heap().alloc_simple(TestValueOfComplex(
                const_frozen_string!("test2").to_frozen_value(),
            ));
            eval.module().set("x", x);
            eval.module().set("y", y.to_value());
        });
        a.eq("'test1'", "test_unpack(x)");
        a.eq("'test2'", "test_unpack(y)");
    }
}
