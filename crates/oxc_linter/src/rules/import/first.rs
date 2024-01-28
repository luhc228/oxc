use oxc_ast::AstKind;
use oxc_macros::declare_oxc_lint;
use oxc_span::{Atom, GetSpan, Span};
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::Error,
};
use crate::{rule::Rule, context::LintContext};

#[derive(Debug, Error, Diagnostic)]
#[error("eslint-plugin-import(import/first): Import in body of module; reorder to top.")]
#[diagnostic(severity(warning))]
struct FirstDiagnostic(#[label] pub Span);

#[derive(Debug, Default, Clone)]
pub struct First;

declare_oxc_lint!(
    /// ### What it does
    /// 
    /// This rule reports any imports that come after non-import statements.
    /// 
    /// ### Example
    /// 
    /// ```javascript
    /// import foo from './foo'
    /// 
    /// // some module-level initializer
    /// initWith(foo)
    /// 
    /// import bar from './bar' // <- reported
    /// ```
    First,
    nursery
);

impl Rule for First {
    fn run_once(&self, ctx: &LintContext) {
        let mut no_import_count  = 0;

        for ast_node in ctx.semantic().nodes().iter() {
            match ast_node.kind() {
                AstKind::ModuleDeclaration(module_decl) => {
                    if module_decl.is_import() {
                        // TODO: support absolute first option

                        if no_import_count > 0 {
                            ctx.diagnostic(FirstDiagnostic(
                                module_decl.span(),
                            ));
                        }
                    } else {
                        no_import_count += 1;
                    }
                },
                _ => {
                    println!("we are here {:#?}", ast_node.kind());
                    no_import_count += 1;
                }
            }
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    {
        let pass = vec![
            "import { x } from './foo'; 
            import { y } from './bar';
            export { x, y };",
        ];
        let fail: Vec<&str> = vec![];

        Tester::new(First::NAME, pass, fail)
        .change_rule_path("index.js")
        .with_import_plugin(true)
        .test()
    }
}