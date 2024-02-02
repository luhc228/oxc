use oxc_ast::{ast::{ImportDeclarationSpecifier, ModuleDeclaration, Statement}, AstKind};
use oxc_macros::declare_oxc_lint;
use oxc_span::{GetSpan, Span};
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::Error,
};
use crate::{context::LintContext, fixer::Fix, rule::Rule};

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
        let mut source_text = String::from(ctx.source_text());
        let mut non_import_count = 0;
        let mut any_expressions = false;
        let mut should_fix = true;
        let mut last_import_statement_end: usize = 0;

        let Some(root) = ctx.nodes().iter().next() else { return };
        let AstKind::Program(program) = root.kind() else { return };

        // directive is like `use 'directive';` at the top of the file
        let directive = program.directives.iter().next();
        let mut directive_span: Option<Span> = None;
        if let Some(directive) = directive {
            directive_span = Some(directive.span());
        }

        for (index, statement) in program.body.iter().enumerate() {
            if !any_expressions && directive_span.is_some() && directive_span.unwrap().end >= statement.span().start  {
                break;
            }

            any_expressions = true;
            
            if let Statement::ModuleDeclaration(module_decl) = statement {
                if let ModuleDeclaration::ImportDeclaration(import_decl) = &**module_decl {
                    let symbol_table = ctx.semantic().symbols();

                    if non_import_count > 0 {
                        let local_binding_identifiers =  import_decl.specifiers
                            .iter()
                            .flatten()
                            .map(|import_declaration_specifier| {
                                let local = match import_declaration_specifier {
                                    ImportDeclarationSpecifier::ImportDefaultSpecifier(import_default_specifier) => {
                                        import_default_specifier.local.clone()
                                    },
                                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(import_namespace_specifier) => {
                                        import_namespace_specifier.local.clone()
                                    },
                                    ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                                        import_specifier.local.clone()
                                    },
                                };
                                local
                            })
                            .collect::<Vec<_>>();
                        println!("====> {:?}", local_binding_identifiers);
                        for local_binding_identifier in local_binding_identifiers.iter() {
                            if !should_fix {
                                break;
                            }
                            let symbol_id = local_binding_identifier.symbol_id.get();
                            if symbol_id.is_none() {
                                continue;
                            }
                            for reference in symbol_table.get_resolved_references(symbol_id.unwrap()) {
                                let reference_span = reference.span();

                                if reference_span.start < directive_span.unwrap().end {
                                    should_fix = false;
                                    break;
                                }
                            }
                        }

                        if should_fix {
                            ctx.diagnostic_with_fix(
                                FirstDiagnostic(import_decl.span), 
                                || {
                                    let fixed_content = build_code(
                                        &source_text, 
                                        last_import_statement_end, 
                                        import_decl.span,
                                    );
                                    source_text = fixed_content.clone();
                                    let len = fixed_content.len();
                                    println!("====> fixed_content {:?}", fixed_content);
                                    println!("====> len {:?}", len);
                                    Fix::new(
                                        fixed_content,
                                        Span { 
                                            start: 0,
                                            end: len as u32,
                                        }
                                    )
                                 }
                            );
                        } else {
                            ctx.diagnostic(FirstDiagnostic(import_decl.span));
                        }
                    }

                    last_import_statement_end += import_decl.span.end as usize - import_decl.span.start as usize;

                    let pre = &source_text[..last_import_statement_end as usize];
                    let post = &source_text[last_import_statement_end as usize..];

                    println!("====> pre {:?}", pre);
                    println!("====> post {:?}", post);
                } else {
                    non_import_count += 1;
                }
            } else {
                non_import_count += 1;
            }
        }
    }
}

fn build_code(
    source_text: &str, 
    last_import_statement_span_end: usize, 
    import_decl_span: Span
) -> String {
    let prefix_content = &source_text[..last_import_statement_span_end];
    let suffix_content = &source_text[import_decl_span.end as usize..];
    // swap
    let current_import_content = &source_text[import_decl_span.start as usize..import_decl_span.end as usize];
    let last_content = &source_text[last_import_statement_span_end..import_decl_span.start as usize];
    let swaped_content = format!("{}{}", current_import_content, last_content);

    let fixed_code = prefix_content.to_string() + &swaped_content + suffix_content;
    fixed_code
}

#[test]
fn test() {
    use crate::tester::Tester;

    {
        let pass: Vec<&str> = vec![
            // "import { x } from './foo'; 
            // import { y } from './bar';
            // export { x, y };",

            // "'use directive';\
            // import { x } from 'foo';",

            // "import { x } from 'foo'; import { y } from './bar'",

            // "import { x } from './foo'; import { y } from 'bar'",
        ];
        let fail: Vec<&str> = vec![
            // "import { x } from './foo';\
            // export { x };\
            // import { y } from './bar';",

            // "import { x } from './foo';\
            // export { x };\
            // import { y } from './bar';\
            // import { z } from './baz';",

            // // directive
            // "import { x } from 'foo';\
            //   'use directive';\
            //   import { y } from 'bar';",

            //   reference
            // "var a = 1;\
            //   import { y } from './bar';\
            //   if (true) { abcd() };\
            //   import { abcd } from './foo';\
            //   import { z } from './baz';",

            // "if (true) { console.log(1) }import a from 'b'",
        ];
        let fix = vec![
            // (
            //     // input
            //     "import { x } from './foo';\
            //     export { x };\
            //     import { y } from './bar';",
            //     // output
            //     "import { x } from './foo';\
            //     import { y } from './bar';\
            //     export { x };",
            //     None,
            // ),
            (
                // input
                "import { x } from './foo';\
                export { x };\
                import { y } from './bar';\
                import { z } from './baz';",
                // output
                "import { x } from './foo';\
                import { y } from './bar';\
                import { z } from './baz';\
                export { x };",
                None,
            ),
            // (
            //     // input
            //     "import { x } from 'foo';\
            //     'use directive';\
            //     import { y } from 'bar';",
            //     // output
            //     "import { x } from 'foo';\
            //     import { y } from 'bar';\
            //     'use directive';",
            //     None,
            // ),
            // (
            //     // input
            //     "var a = 1;\
            //     import { y } from './bar';\
            //     if (true) { x() };\
            //     import { x } from './foo';\
            //     import { z } from './baz';",
            //     // output
            //     "import { y } from './bar';\
            //     var a = 1;\
            //     if (true) { x() };\
            //     import { x } from './foo';\
            //     import { z } from './baz';",
            //     None,
            // ),
            // (
            //     // input
            //     "if (true) { console.log(1) }import a from 'b'",
            //     // output
            //     "import a from 'b'\nif (true) { console.log(1) }",
            //     None,
            // ),
        ];
        Tester::new(First::NAME, pass, fail)
        .change_rule_path("index.js")
        .with_import_plugin(true)
        .expect_fix(fix)
        .test()
    }
}
