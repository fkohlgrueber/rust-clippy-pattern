//! Checks for if expressions that contain only an if expression.
//!
//! For example, the lint would catch:
//!
//! ```rust,ignore
//! if x {
//!     if y {
//!         println!("Hello world");
//!     }
//! }
//! ```
//!
//! This lint is **warn** by default

use rustc::lint::{EarlyContext, EarlyLintPass, LintArray, LintPass};
use rustc::{declare_tool_lint, lint_array};
use syntax::ast;

use crate::utils::sugg::Sugg;
use crate::utils::{in_macro, snippet_block, snippet_block_with_applicability, span_lint_and_sugg, span_lint_and_then};
use rustc_errors::Applicability;

use pattern::pattern;

declare_clippy_lint! {
    /// **What it does:** Checks for nested `if` statements which can be collapsed
    /// by `&&`-combining their conditions and for `else { if ... }` expressions
    /// that
    /// can be collapsed to `else if ...`.
    ///
    /// **Why is this bad?** Each `if`-statement adds one level of nesting, which
    /// makes code look more complex than it really is.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// if x {
    ///     if y {
    ///         …
    ///     }
    /// }
    ///
    /// // or
    ///
    /// if x {
    ///     …
    /// } else {
    ///     if y {
    ///         …
    ///     }
    /// }
    /// ```
    ///
    /// Should be written:
    ///
    /// ```rust.ignore
    /// if x && y {
    ///     …
    /// }
    ///
    /// // or
    ///
    /// if x {
    ///     …
    /// } else if y {
    ///     …
    /// }
    /// ```
    pub COLLAPSIBLE_IF,
    style,
    "`if`s that can be collapsed (e.g. `if x { if y { ... } }` and `else { if x { ... } }`)"
}

#[derive(Copy, Clone)]
pub struct CollapsibleIf;

impl LintPass for CollapsibleIf {
    fn get_lints(&self) -> LintArray {
        lint_array!(COLLAPSIBLE_IF)
    }

    fn name(&self) -> &'static str {
        "CollapsibleIf"
    }
}

pattern!{
    pat_if_without_else: Expr = 
        If(
            _#check,
            Block(
                Expr( If(_#check_inner, _#content, ())#inner )
                | Semi( If(_#check_inner, _#content, ())#inner ) 
            )#then, 
            ()
        )
}

pattern!{
    pat_if_else: Expr = 
        If(
            _, 
            _, 
            Block_(
                Block(
                    Expr((If(_, _, _?) | IfLet(_, _?))#else_) | 
                    Semi((If(_, _, _?) | IfLet(_, _?))#else_)
                )#block_inner
            )#block
        ) |
        IfLet(
            _, 
            Block_(
                Block(
                    Expr((If(_, _, _?) | IfLet(_, _?))#else_) | 
                    Semi((If(_, _, _?) | IfLet(_, _?))#else_)
                )#block_inner
            )#block
        )
}

impl EarlyLintPass for CollapsibleIf {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &ast::Expr) {
        if in_macro(expr.span) {
            return;
        }

        if let Some(result) = pat_if_without_else(expr) {
            if !block_starts_with_comment(cx, result.then) && expr.span.ctxt() == result.inner.span.ctxt() {
                span_lint_and_then(cx, COLLAPSIBLE_IF, expr.span, "this if statement can be collapsed", |db| {
                    let lhs = Sugg::ast(cx, result.check, "..");
                    let rhs = Sugg::ast(cx, result.check_inner, "..");
                    db.span_suggestion(
                        expr.span,
                        "try",
                        format!(
                            "if {} {}",
                            lhs.and(&rhs),
                            snippet_block(cx, result.content.span, ".."),
                        ),
                        Applicability::MachineApplicable, // snippet
                    );
                });
            }
        }
        
        if let Some(result) = pat_if_else(expr) {
            if !block_starts_with_comment(cx, result.block_inner) && !in_macro(result.else_.span){
                let mut applicability = Applicability::MachineApplicable;
                span_lint_and_sugg(
                    cx,
                    COLLAPSIBLE_IF,
                    result.block.span,
                    "this `else { if .. }` block can be collapsed",
                    "try",
                    snippet_block_with_applicability(cx, result.else_.span, "..", &mut applicability).into_owned(),
                    applicability,
                );
            }
        }
    }
}

fn block_starts_with_comment(cx: &EarlyContext<'_>, expr: &ast::Block) -> bool {
    // We trim all opening braces and whitespaces and then check if the next string is a comment.
    let trimmed_block_text = snippet_block(cx, expr.span, "..")
        .trim_start_matches(|c: char| c.is_whitespace() || c == '{')
        .to_owned();
    trimmed_block_text.starts_with("//") || trimmed_block_text.starts_with("/*")
}
