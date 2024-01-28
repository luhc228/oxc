use oxc_ast::{ast::Statement, AstKind};
use oxc_macros::declare_oxc_lint;
use oxc_span::{GetSpan, Span};
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
        let mut any_expressions = false;

        let Some(root) = ctx.nodes().iter().next() else { return };
        let AstKind::Program(program) = root.kind() else { return };

        let directive = program.directives.iter().next();
        let mut directive_span: Option<Span> = None;
        if let Some(directive) = directive {
            directive_span = Some(directive.span());
        }

        for statement in &program.body {
            if !any_expressions && directive_span.is_some() && directive_span.unwrap().end >= statement.span().start  {
                break;
            }

            any_expressions = true;

            if let Statement::ModuleDeclaration(module_decl) = statement {
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
            } else {
                no_import_count += 1;
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

            "'use directive';\
            import { x } from 'foo';",

            "import { x } from 'foo'; import { y } from './bar'",

            "import { x } from './foo'; import { y } from 'bar'",
        ];
        let fail = vec![
            "import { x } from './foo';\
            export { x };\
            import { y } from './bar';",

            "import { x } from './foo';\
            export { x };\
            import { y } from './bar';\
            import { z } from './baz';",

            // directive
            "import { x } from 'foo';\
              'use directive';\
              import { y } from 'bar';",

            //   reference
            "var a = 1;\
              import { y } from './bar';\
              if (true) { x() };\
              import { x } from './foo';\
              import { z } from './baz';",

            "if (true) { console.log(1) }import a from 'b'",
        ];

        Tester::new(First::NAME, pass, fail)
        .change_rule_path("index.js")
        .with_import_plugin(true)
        .test()
    }
}
