//! Reproducer for a glibc tcache abort observed in gage's tool dispatcher.
//!
//! When an [`Object`] is built natively by inserting a key
//! ([`rune::alloc::String`]) and a value whose payload is also a
//! `rune::alloc::String` wrapped via [`Value::new`], then passed into a Rune
//! function whose first parameter destructures via an anonymous-object pattern
//! (`#{ text }`), the run completes functionally — but at thread exit glibc
//! reports `tcache_thread_shutdown(): unaligned tcache chunk detected`.
//!
//! These tests intentionally run their VM work on a worker thread so the
//! tcache shutdown actually executes and any corruption surfaces.

use std::thread;

use rune::alloc;
use rune::runtime::{Object, Value};
use rune::sync::Arc;
use rune::{Context, Vm};

/// Mirror gage-runtime's `json_to_object` for a single string field. The
/// value payload is constructed exactly as gage did it: an `alloc::String`
/// boxed through [`Value::new`].
fn build_object_string_value(key: &str, text: &str) -> Object {
    let mut obj = Object::new();
    let k = alloc::String::try_from(key).unwrap();
    let rs = alloc::String::try_from(text).unwrap();
    let v = Value::new(rs).unwrap();
    obj.insert(k, v).unwrap();
    obj
}

/// Compile a one-fn scanner-like unit and run `fn_name(args_obj)` on it.
/// Runs entirely on the calling thread so the caller can place this inside
/// `thread::spawn` and observe shutdown effects on join.
fn run_fn(source: &str, fn_name: &str, args_obj: Object) -> Value {
    let context = Context::with_default_modules().unwrap();
    let runtime = Arc::try_new(context.runtime().unwrap()).unwrap();

    let mut sources = rune::Sources::new();
    sources
        .insert(rune::Source::memory(source).unwrap())
        .unwrap();
    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .build()
        .unwrap();
    let mut vm = Vm::new(runtime, Arc::try_new(unit).unwrap());
    vm.call([fn_name], (args_obj,)).unwrap()
}

/// Baseline: indexed access on the same object — known clean in the
/// gage bisection. Included so a failure in *this* test points at the
/// shared scaffolding (Object construction / Vm setup) rather than the
/// destructure-specific path.
#[test]
fn indexed_access_clean() {
    let handle = thread::spawn(|| {
        let obj = build_object_string_value("text", "Hello, Joe");
        let _ = run_fn(
            r#"pub fn entry(args) { args["text"] }"#,
            "entry",
            obj,
        );
    });
    handle.join().unwrap();
}

/// The reproducer: anonymous-object destructure pattern on the
/// natively-constructed object. In the gage build this aborts on thread
/// shutdown with `unaligned tcache chunk detected`.
#[test]
fn destructure_pattern_corrupts_tcache() {
    let handle = thread::spawn(|| {
        let obj = build_object_string_value("text", "Hello, Joe");
        let _ = run_fn(
            r#"pub fn entry(#{ text }) { text }"#,
            "entry",
            obj,
        );
    });
    handle.join().unwrap();
}

/// Destructure pattern but discard the bound name and return a constant.
/// Isolates the destructure-bind step from the bound-value-return step.
#[test]
fn destructure_pattern_discard_binding() {
    let handle = thread::spawn(|| {
        let obj = build_object_string_value("text", "Hello, Joe");
        let _ = run_fn(
            r#"pub fn entry(#{ text }) { 42 }"#,
            "entry",
            obj,
        );
    });
    handle.join().unwrap();
}

/// Value payload is a plain integer rather than a `Value::new(alloc::String)`
/// any-wrapped string. Isolates whether the trigger is the destructure op
/// itself or the specific value type stored in the object.
#[test]
fn destructure_pattern_integer_value() {
    let handle = thread::spawn(|| {
        let mut obj = Object::new();
        let k = alloc::String::try_from("text").unwrap();
        obj.insert(k, rune::to_value(7i64).unwrap()).unwrap();
        let _ = run_fn(
            r#"pub fn entry(#{ text }) { text }"#,
            "entry",
            obj,
        );
    });
    handle.join().unwrap();
}

/// Same destructure, but the value is constructed as a proper Rune
/// `runtime::String` (via `rune::to_value`) instead of an any-wrapped
/// `alloc::String`. If this is clean, the trigger is specifically the
/// `Value::new(alloc::String)` wrapping form.
#[test]
fn destructure_pattern_rune_string_value() {
    let handle = thread::spawn(|| {
        let mut obj = Object::new();
        let k = alloc::String::try_from("text").unwrap();
        obj.insert(k, rune::to_value("Hello, Joe").unwrap()).unwrap();
        let _ = run_fn(
            r#"pub fn entry(#{ text }) { text }"#,
            "entry",
            obj,
        );
    });
    handle.join().unwrap();
}

/// Native build but using `Object::with_capacity(1)` instead of
/// `Object::new()`. Tests whether the trigger is the zero-capacity
/// `HashMap::new()` start-state used by `Object::new()`.
#[test]
fn destructure_pattern_with_capacity() {
    let handle = thread::spawn(|| {
        let mut obj = Object::with_capacity(1).unwrap();
        let k = alloc::String::try_from("text").unwrap();
        obj.insert(k, rune::to_value(7i64).unwrap()).unwrap();
        let _ = run_fn(r#"pub fn entry(#{ text }) { text }"#, "entry", obj);
    });
    handle.join().unwrap();
}

/// Native-built Object passed as a plain-name argument, destructured
/// inside the body via `let #{ text } = args;`. Tests whether the
/// trigger is specifically *function-signature* destructure vs *body*
/// destructure.
#[test]
fn destructure_pattern_in_body() {
    let handle = thread::spawn(|| {
        let obj = build_object_string_value("text", "Hello, Joe");
        let _ = run_fn(
            r#"pub fn entry(args) { let #{ text } = args; text }"#,
            "entry",
            obj,
        );
    });
    handle.join().unwrap();
}

/// Destructure against an Object built entirely by Rune literal — no
/// native construction. If this is clean and the others corrupt, the
/// trigger is something native-side construction does that Rune literal
/// construction does not.
#[test]
fn destructure_pattern_rune_literal_object() {
    let handle = thread::spawn(|| {
        let context = Context::with_default_modules().unwrap();
        let runtime = Arc::try_new(context.runtime().unwrap()).unwrap();
        let mut sources = rune::Sources::new();
        sources
            .insert(
                rune::Source::memory(
                    r#"pub fn entry() {
                        let obj = #{ text: "Hello, Joe" };
                        let #{ text } = obj;
                        text
                    }"#,
                )
                .unwrap(),
            )
            .unwrap();
        let unit = rune::prepare(&mut sources)
            .with_context(&context)
            .build()
            .unwrap();
        let mut vm = Vm::new(runtime, Arc::try_new(unit).unwrap());
        let _ = vm.call(["entry"], ()).unwrap();
    });
    handle.join().unwrap();
}
