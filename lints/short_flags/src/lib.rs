#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

/// Short flags that are intentionally kept (never converted to a long form).
///
/// Minimal on purpose: the short-flag conversion effort grows this list as it
/// meets flags with no long-form equivalent (e.g. `sh -c`). Until then a `Warn`
/// level plus this allowlist keeps the ~131 unconverted call sites building.
const ALLOWLIST: &[&str] = &["-y"];

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Flags short CLI flags (e.g. `-y`, `-Syu`, `-i`) passed to `.arg`/`.args`
    /// on a command-builder receiver, matched by the receiver's qualified type
    /// path (`executor::Executor` or `std::process::Command`).
    ///
    /// ### Why is this bad?
    /// Short flags are cryptic and collide across tools; long forms read clearly.
    ///
    /// ### Example
    /// ```rust
    /// # use std::process::Command;
    /// Command::new("pacman").arg("-Syu");
    /// ```
    /// Use instead:
    /// ```rust
    /// # use std::process::Command;
    /// Command::new("pacman").args(["--sync", "--refresh", "--sysupgrade"]);
    /// ```
    pub SHORT_FLAGS,
    Warn,
    "short CLI flag passed to a command builder"
}

/// `-` followed by an ASCII-alphanumeric char: `-y`, `-Syu` match; `--yes`, `-`, `--` do not.
fn is_short_flag(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 2 && b[0] == b'-' && b[1].is_ascii_alphanumeric()
}

/// True only when the receiver's type resolves to one of the command builders by qualified
/// path. `def_path_str` omits the crate name for the crate under lint, so topgrade's own
/// `Executor` prints as `executor::Executor` (the ui suite reproduces this exact path with a
/// `mod executor { .. }` fixture). A bare `Executor` -- a crate-root or unrelated local type
/// whose path is just `Executor` -- never matches, so the lint is path-based, not name-based.
fn receiver_is_builder<'tcx>(cx: &LateContext<'tcx>, receiver: &Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(receiver).peel_refs();
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    let path = cx.tcx.def_path_str(adt.did());
    path == "executor::Executor" || path == "std::process::Command"
}

fn check_str_lit(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Str(sym, _) = lit.node
    {
        let s = sym.as_str();
        if is_short_flag(&s) && !ALLOWLIST.contains(&s.as_ref()) {
            span_lint(
                cx,
                SHORT_FLAGS,
                lit.span,
                format!("short flag `{s}`; use the long form"),
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ShortFlags {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(segment, receiver, args, _) = expr.kind else {
            return;
        };
        let method = segment.ident.name.as_str();
        let (is_arg, is_args) = (method == "arg", method == "args");
        if !is_arg && !is_args {
            return;
        }
        if !receiver_is_builder(cx, receiver) {
            return;
        }
        match (is_arg, args) {
            (true, [single]) => check_str_lit(cx, single),
            (false, [single]) => {
                if let ExprKind::Array(elems) = single.kind {
                    for e in elems {
                        check_str_lit(cx, e);
                    }
                }
            }
            _ => {}
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
