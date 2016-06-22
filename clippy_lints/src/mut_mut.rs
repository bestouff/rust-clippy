use rustc::hir::intravisit;
use rustc::hir;
use rustc::lint::*;
use rustc::ty::{TypeAndMut, TyRef};
use utils::{in_external_macro, recover_for_loop, span_lint};

/// **What it does:** This lint checks for instances of `mut mut` references.
///
/// **Why is this bad?** Multiple `mut`s don't add anything meaningful to the source.
///
/// **Known problems:** None
///
/// **Example:** `let x = &mut &mut y;`
declare_lint! {
    pub MUT_MUT,
    Allow,
    "usage of double-mut refs, e.g. `&mut &mut ...` (either copy'n'paste error, \
     or shows a fundamental misunderstanding of references)"
}

#[derive(Copy,Clone)]
pub struct MutMut;

impl LintPass for MutMut {
    fn get_lints(&self) -> LintArray {
        lint_array!(MUT_MUT)
    }
}

impl LateLintPass for MutMut {
    fn check_crate(&mut self, cx: &LateContext, krate: &hir::Crate) {
        krate.visit_all_items(&mut MutVisitor { cx: cx });
    }
}

pub struct MutVisitor<'a, 'tcx: 'a> {
    cx: &'a LateContext<'a, 'tcx>,
}

impl<'a, 'tcx, 'v> intravisit::Visitor<'v> for MutVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'v hir::Expr) {
        if in_external_macro(self.cx, expr.span) {
            return;
        }

        if let Some((_, arg, body)) = recover_for_loop(expr) {
            // A `for` loop lowers to:
            // ```rust
            // match ::std::iter::Iterator::next(&mut iter) {
            // //                                ^^^^
            // ```
            // Let's ignore the generated code.
            intravisit::walk_expr(self, arg);
            intravisit::walk_expr(self, body);
            return;
        } else if let hir::ExprAddrOf(hir::MutMutable, ref e) = expr.node {
            if let hir::ExprAddrOf(hir::MutMutable, _) = e.node {
                span_lint(self.cx, MUT_MUT, expr.span, "generally you want to avoid `&mut &mut _` if possible");
            } else if let TyRef(_, TypeAndMut { mutbl: hir::MutMutable, .. }) = self.cx.tcx.expr_ty(e).sty {
                span_lint(self.cx,
                          MUT_MUT,
                          expr.span,
                          "this expression mutably borrows a mutable reference. Consider reborrowing");
            }
        }

        intravisit::walk_expr(self, expr);
    }

    fn visit_ty(&mut self, ty: &hir::Ty) {
        if let hir::TyRptr(_, hir::MutTy { ty: ref pty, mutbl: hir::MutMutable }) = ty.node {
            if let hir::TyRptr(_, hir::MutTy { mutbl: hir::MutMutable, .. }) = pty.node {
                span_lint(self.cx, MUT_MUT, ty.span, "generally you want to avoid `&mut &mut _` if possible");
            }

        }

        intravisit::walk_ty(self, ty);
    }
}
