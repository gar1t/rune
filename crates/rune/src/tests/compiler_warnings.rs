prelude!();

use diagnostics::WarningDiagnosticKind::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        "let [0, 1, 3] = [];",
        span!(4, 13), LetPatternMightPanic { context: Some(span!(0, 19)), .. }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        "`Hello World`",
        span!(0, 13), TemplateWithoutExpansions { context: Some(span!(0, 13)), .. }
    };
}

#[test]
fn test_unused_const_spans_name_only() {
    assert_warnings! {
        "const FOO = 10;",
        span!(6, 9), NotUsed { .. }
    };
}

#[test]
fn test_pub_fn_no_unused_warning() {
    let mut diagnostics = Default::default();
    let _ = crate::tests::compile_helper("pub fn foo() {}", &mut diagnostics)
        .expect("source should compile");
    assert!(!diagnostics.has_warning(), "pub fn should not produce unused warning");
}

#[test]
fn test_pub_const_no_unused_warning() {
    let mut diagnostics = Default::default();
    let _ = crate::tests::compile_helper("pub const FOO = 10;", &mut diagnostics)
        .expect("source should compile");
    assert!(!diagnostics.has_warning(), "pub const should not produce unused warning");
}
