#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

/// Flags exempt from the lint: already the clear, canonical single-dash spelling with no GNU
/// long form, so there is nothing to convert. `-NoProfile`/`-Command` (PowerShell) and `-plugin`
/// (micro) are each tool-unique, so a global exempt is safe. The short-flag conversion effort
/// grows this list as it meets more such flags (e.g. an unconvertible `sh -c`).
const ALLOWLIST: &[&str] = &["-y", "-NoProfile", "-Command", "-plugin"];

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Flags short CLI flags (e.g. `-y`, `-Syu`, `-i`) passed to `.arg`/`.args`
    /// on a command-builder receiver: topgrade's own `Executor` (any module) or
    /// `std::process::Command`.
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

/// True only when the receiver is a command builder. `std::process::Command` matches by full
/// path (external crate, stable). topgrade's own `Executor` matches by crate-local identity plus
/// name: `is_local` scopes it to the crate under lint -- a dependency's same-named `Executor`
/// stays silent -- while surviving a module move (unlike a hardcoded `executor::Executor` path, a
/// rename of `src/executor.rs` keeps the arm live). topgrade has exactly one local `Executor`, so
/// the bare-name match is unambiguous.
fn receiver_is_builder<'tcx>(cx: &LateContext<'tcx>, receiver: &Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(receiver).peel_refs();
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    let did = adt.did();
    (did.is_local() && cx.tcx.item_name(did).as_str() == "Executor")
        || cx.tcx.def_path_str(did) == "std::process::Command"
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
