use crate::alloc;
use crate::compile::meta;

use super::{FromValue, MaybeTypeOf, RuntimeError, Value, VmError};

/// An owning iterator.
#[derive(Debug)]
pub struct Iterator {
    iter: Value,
}

impl Iterator {
    pub(crate) fn new(iter: Value) -> Self {
        Self { iter }
    }

    /// Get the size hint of the iterator by calling the `SIZE_HINT` protocol of
    /// the underlying value.
    #[inline]
    pub fn size_hint(&self) -> Result<(usize, Option<usize>), VmError> {
        self.iter.protocol_size_hint()
    }

    /// Advance the iterator by calling the `NEXT` protocol of the underlying
    /// value, returning the next element or `None` once it is exhausted.
    ///
    /// This lets native code consume any Rune iterable. Combined with the
    /// [`FromValue`] implementation, which applies the `INTO_ITER` protocol, a
    /// function argument typed as [`Iterator`] accepts vectors, objects, ranges
    /// and lazy iterator chains alike.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Context, Module, Vm};
    /// use rune::runtime::{Iterator, VmError};
    /// use rune::sync::Arc;
    ///
    /// fn sum(mut it: Iterator) -> Result<i64, VmError> {
    ///     let mut total = 0;
    ///     while let Some(value) = it.next()? {
    ///         total += rune::from_value::<i64>(value)?;
    ///     }
    ///     Ok(total)
    /// }
    ///
    /// let mut module = Module::new();
    /// module.function("sum", sum).build()?;
    ///
    /// let mut context = Context::with_default_modules()?;
    /// context.install(module)?;
    /// let runtime = Arc::try_new(context.runtime()?)?;
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         pub fn main() {
    ///             sum([1, 2, 3].iter().map(|n| n * 2))
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).with_context(&context).build()?;
    /// let mut vm = Vm::new(runtime, Arc::try_new(unit)?);
    ///
    /// let output = vm.call(["main"], ())?;
    /// assert_eq!(rune::from_value::<i64>(output)?, 12);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.protocol_next()
    }
}

impl FromValue for Iterator {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_iter().map_err(RuntimeError::from)
    }
}

impl MaybeTypeOf for Iterator {
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        Ok(meta::DocType::empty())
    }
}
