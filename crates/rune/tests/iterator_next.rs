//! Native code can consume any Rune iterable through the public
//! [`Iterator::next`] method. The [`FromValue`] implementation for
//! [`Iterator`] applies the `INTO_ITER` protocol, so a function argument typed
//! as [`Iterator`] accepts vectors, objects, ranges and lazy iterator chains
//! alike — the host then drives them with `next` without knowing the concrete
//! type.
//!
//! [`Iterator`]: rune::runtime::Iterator
//! [`Iterator::next`]: rune::runtime::Iterator::next
//! [`FromValue`]: rune::FromValue

use rune::runtime::{Iterator, VmError};
use rune::sync::Arc;
use rune::{Context, ContextError, Module, Vm};

/// A native function that sums an arbitrary Rune iterable by repeatedly calling
/// [`Iterator::next`].
fn sum(mut it: Iterator) -> Result<i64, VmError> {
    let mut total = 0;

    while let Some(value) = it.next()? {
        total += rune::from_value::<i64>(value)?;
    }

    Ok(total)
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::new();
    m.function("sum", sum).build()?;
    Ok(m)
}

/// Compile and run `pub fn main() { <expr> }` against a context that has our
/// native `sum` installed, returning its `i64` result.
fn eval_sum(expr: &str) -> i64 {
    let mut context = Context::with_default_modules().expect("default modules to build");
    context
        .install(module().expect("native module to build"))
        .expect("native module to install");

    let runtime = Arc::try_new(context.runtime().expect("runtime to build")).expect("arc runtime");

    let mut sources = rune::Sources::new();
    sources
        .insert(rune::Source::memory(format!("pub fn main() {{ {expr} }}")).expect("source"))
        .expect("source to insert");

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .build()
        .expect("unit to build");
    let mut vm = Vm::new(runtime, Arc::try_new(unit).expect("arc unit"));

    let output = vm.call(["main"], ()).expect("main to run");
    rune::from_value::<i64>(output).expect("an i64 result")
}

#[test]
fn consumes_a_vec() {
    assert_eq!(eval_sum("sum([1, 2, 3])"), 6);
}

#[test]
fn consumes_object_values() {
    assert_eq!(eval_sum("sum(#{ a: 1, b: 2, c: 3 }.values())"), 6);
}

#[test]
fn consumes_a_range() {
    assert_eq!(eval_sum("sum((1..=4).iter())"), 10);
}

#[test]
fn consumes_a_lazy_iterator_chain() {
    assert_eq!(
        eval_sum("sum([1, 2, 3, 4].iter().filter(|n| n % 2 == 0).map(|n| n * 10))"),
        60,
    );
}

#[test]
fn consumes_an_empty_iterable() {
    assert_eq!(eval_sum("sum([])"), 0);
}
