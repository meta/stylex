use stylex_swc_plugin::{RuntimeInjectionOption, StyleXTransformOptions, TransformOutput};
use swc_core::{
  common::{FileName, SourceMap, sync::Lrc},
  ecma::{
    ast::*,
    parser::{EsSyntax, Parser, StringInput, Syntax, lexer::Lexer},
    visit::{Visit, VisitWith},
  },
};
#[path = "support/current_babel.rs"]
mod current_babel;

fn transform_source(
  source: &str,
  filename: &str,
  options: &StyleXTransformOptions,
) -> anyhow::Result<TransformOutput> {
  let output = stylex_swc_plugin::transform_source(source, filename, options)?;
  current_babel::assert_babel(source, filename, serde_json::to_value(options)?, &output);
  Ok(output)
}

fn transform(source: &str) -> TransformOutput {
  current_babel::transform(source, "fixture.js", &StyleXTransformOptions::default())
}
fn module(code: &str) -> Module {
  let cm = Lrc::new(SourceMap::default());
  let file = cm.new_source_file(
    FileName::Custom("output.js".to_owned()).into(),
    code.to_owned(),
  );
  let lexer = Lexer::new(
    Syntax::Es(EsSyntax {
      jsx: true,
      ..Default::default()
    }),
    EsVersion::Es2022,
    StringInput::from(&*file),
    None,
  );
  Parser::new_from(lexer)
    .parse_module()
    .expect("valid generated program")
}
fn initializer(output: &TransformOutput, name: &str) -> Box<Expr> {
  for item in module(&output.code).body {
    if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) = item {
      for var in vars.decls {
        if matches!(&var.name,Pat::Ident(binding) if binding.id.sym.as_ref() == name) {
          return var.init.expect("initializer");
        }
      }
    }
  }
  panic!("missing {name}: {}", output.code)
}
fn class(output: &TransformOutput, value: &str) -> String {
  output
    .metadata_stylex
    .iter()
    .find(|rule| rule.1.ltr.contains(value))
    .expect("CSS rule")
    .0
    .clone()
}
fn unparen(expression: &Expr) -> &Expr {
  match expression {
    Expr::Paren(paren) => unparen(&paren.expr),
    _ => expression,
  }
}
fn has_call(
  output: &TransformOutput,
  object: Option<&str>,
  method: &str,
  argument: Option<&str>,
) -> bool {
  struct Calls<'a> {
    object: Option<&'a str>,
    method: &'a str,
    argument: Option<&'a str>,
    found: bool,
  }
  impl Visit for Calls<'_> {
    fn visit_call_expr(&mut self, call: &CallExpr) {
      let matches = match &call.callee {
        Callee::Expr(callee) => match (self.object, unparen(callee)) {
          (None, Expr::Ident(ident)) => ident.sym.as_ref() == self.method,
          (Some(object), Expr::Member(member)) => {
            matches!(unparen(&member.obj),Expr::Ident(ident) if ident.sym.as_ref()==object)
              && matches!(&member.prop,MemberProp::Ident(name) if name.sym.as_ref()==self.method)
          }
          _ => false,
        },
        _ => false,
      };
      if matches
        && self.argument.is_none_or(|wanted| {
          call.args.first().is_some_and(
            |arg| matches!(unparen(&arg.expr),Expr::Ident(ident) if ident.sym.as_ref()==wanted),
          )
        })
      {
        self.found = true;
      }
      call.visit_children_with(self);
    }
  }
  let mut calls = Calls {
    object,
    method,
    argument,
    found: false,
  };
  module(&output.code).visit_with(&mut calls);
  calls.found
}
fn has_jsx_attr(output: &TransformOutput, tag: &str, name: &str) -> bool {
  struct Attributes<'a> {
    tag: &'a str,
    name: &'a str,
    found: bool,
  }
  impl Visit for Attributes<'_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement) {
      if matches!(&element.name,JSXElementName::Ident(tag) if tag.sym.as_ref()==self.tag) && element.attrs.iter().any(|attr|matches!(attr,JSXAttrOrSpread::JSXAttr(attr) if matches!(&attr.name,JSXAttrName::Ident(name) if name.sym.as_ref()==self.name))) { self.found=true; }
      element.visit_children_with(self);
    }
  }
  let mut attrs = Attributes {
    tag,
    name,
    found: false,
  };
  module(&output.code).visit_with(&mut attrs);
  attrs.found
}
fn object_field(expression: &Expr, name: &str) -> Option<String> {
  let Expr::Object(object) = unparen(expression) else {
    panic!("expected object, got {expression:?}")
  };
  object.props.iter().find_map(|prop| {
    let PropOrSpread::Prop(prop) = prop else {
      return None;
    };
    let Prop::KeyValue(kv) = &**prop else {
      return None;
    };
    let key = match &kv.key {
      PropName::Ident(name) => name.sym.as_ref(),
      PropName::Str(name) => name.value.as_str()?,
      _ => return None,
    };
    if key != name {
      return None;
    };
    let Expr::Lit(Lit::Str(value)) = &*kv.value else {
      return None;
    };
    value.value.as_str().map(str::to_owned)
  })
}
fn branches(expression: &Expr) -> Vec<Expr> {
  let Expr::Member(member) = unparen(expression) else {
    panic!("expected conditional lookup")
  };
  let Expr::Object(object) = unparen(&member.obj) else {
    panic!("expected conditional table")
  };
  object
    .props
    .iter()
    .map(|prop| {
      let PropOrSpread::Prop(prop) = prop else {
        panic!()
      };
      let Prop::KeyValue(kv) = &**prop else {
        panic!()
      };
      *kv.value.clone()
    })
    .collect()
}
const STYLES: &str = "import * as stylex from '@stylexjs/stylex'; const styles=stylex.create({red:{color:'red'},blue:{color:'blue'}});";

#[test]
fn conditionals_preserve_argument_precedence_for_props_and_legacy() {
  let output = transform(&format!(
    "{STYLES} const p=stylex.props(flag && styles.red,styles.blue); const c=stylex(flag && styles.red,styles.blue);"
  ));
  let blue = class(&output, "color:blue");
  for branch in branches(&initializer(&output, "p")) {
    assert_eq!(object_field(&branch, "className"), Some(blue.clone()));
  }
  for branch in branches(&initializer(&output, "c")) {
    assert!(
      matches!(&branch,Expr::Lit(Lit::Str(value)) if value.value.as_str() == Some(blue.as_str()))
    );
  }
}
#[test]
fn array_ternary_branches_are_not_reversed() {
  // Babel leaves arrays in legacy calls for the runtime. Use a callable default
  // import and verify the original consequent/alternate survive unchanged.
  let styles = STYLES.replacen("import * as stylex", "import stylex", 1);
  let output = transform(&format!(
    "{styles} const c=stylex([flag?styles.red:styles.blue]);"
  ));
  let init = initializer(&output, "c");
  let Expr::Call(call) = unparen(&init) else {
    panic!("legacy array must remain a runtime call")
  };
  let Expr::Array(array) = unparen(&call.args[0].expr) else {
    panic!("array")
  };
  let Expr::Cond(conditional) = unparen(&array.elems[0].as_ref().unwrap().expr) else {
    panic!("ternary")
  };
  assert!(
    matches!(unparen(&conditional.cons),Expr::Member(member) if matches!(&member.prop,MemberProp::Ident(prop) if prop.sym=="red"))
  );
  assert!(
    matches!(unparen(&conditional.alt),Expr::Member(member) if matches!(&member.prop,MemberProp::Ident(prop) if prop.sym=="blue"))
  );
}
#[test]
fn fifth_conditional_keeps_runtime_call_instead_of_exponential_table() {
  let output = transform(&format!(
    "{STYLES} const p=stylex.props(a&&styles.red,b&&styles.red,c&&styles.red,d&&styles.red,e&&styles.red);"
  ));
  assert!(matches!(&*initializer(&output, "p"), Expr::Call(_)));
}
#[test]
fn shadowed_styles_and_imports_are_not_inlined() {
  let source = format!(
    "{STYLES} function render(styles){{return stylex.props(styles.red)}} function unrelated(stylex){{return stylex.props(styles.red)}}"
  );
  let output =
    stylex_swc_plugin::transform_source(&source, "fixture.js", &StyleXTransformOptions::default())
      .unwrap();
  // Babel currently compiles both shadowed names unsafely. Pin the requested
  // Id-safe output through Babel's syntax-only printer, not that compiler bug.
  current_babel::assert_authored(
    "import * as stylex from '@stylexjs/stylex';const styles={red:{kMwMTN:\"x1e2nbdu\",$$css:true}};function render(styles){return stylex.props(styles.red)}function unrelated(stylex){return stylex.props(styles.red)}",
    "fixture.js",
    &output,
  );
  struct Calls(usize);
  impl Visit for Calls {
    fn visit_call_expr(&mut self, call: &CallExpr) {
      self.0 += 1;
      call.visit_children_with(self);
    }
  }
  let mut calls = Calls(0);
  module(&output.code).visit_with(&mut calls);
  assert_eq!(calls.0, 2);
}
#[test]
fn mutated_array_retains_runtime_semantics() {
  let output = transform(&format!(
    "{STYLES} const list=[styles.red];list.push(styles.blue);const p=stylex.props(list);"
  ));
  assert!(matches!(&*initializer(&output, "p"), Expr::Call(_)));
}
#[test]
fn array_bindings_do_not_leak_across_functions() {
  let output = transform(&format!(
    "{STYLES} function a(){{const list=[styles.red];return stylex.props(list)}}function b(list){{return stylex.props(list)}}"
  ));
  assert!(output.code.contains("return stylex.props(list)"));
}
#[test]
fn runtime_import_reuse_and_collision_avoidance() {
  let options = StyleXTransformOptions {
    runtime_injection: RuntimeInjectionOption::Bool(true),
    ..Default::default()
  };
  let output=transform_source(&format!("import inject from '@stylexjs/stylex/lib/stylex-inject';{STYLES}const p=stylex.props(styles.red);"),"fixture.js",&options).unwrap();
  assert!(has_call(&output, None, "_inject2", None));
  assert!(output.code.contains("default as _inject"));
  let output = transform_source(
    &format!("const _inject=1,_inject2=2;{STYLES}const p=stylex.props(styles.red);"),
    "fixture.js",
    &options,
  )
  .unwrap();
  assert!(output.code.contains("import _inject3 from"));
}
#[test]
fn named_runtime_import_and_directive_prologue() {
  let options: StyleXTransformOptions = serde_json::from_value(
    serde_json::json!({"runtimeInjection":{"from":"custom-inject","as":"inject"}}),
  )
  .unwrap();
  let output = transform_source(
    &format!("'use client';{STYLES}const p=stylex.props(styles.red);"),
    "fixture.js",
    &options,
  )
  .unwrap();
  let parsed = module(&output.code);
  assert!(
    matches!(&parsed.body[0],ModuleItem::Stmt(Stmt::Expr(stmt)) if matches!(&*stmt.expr,Expr::Lit(Lit::Str(value)) if value.value.as_str()==Some("use client")))
  );
  assert!(parsed.body.iter().any(|item| matches!(item,ModuleItem::ModuleDecl(ModuleDecl::Import(import)) if import.src.value.as_str()==Some("custom-inject") && matches!(&import.specifiers[0],ImportSpecifier::Named(named) if matches!(&named.imported, Some(ModuleExportName::Ident(name)) if name.sym=="inject")))));
}
#[test]
fn sx_is_intrinsic_only_and_false_disables_it() {
  let output = transform(&format!(
    "{STYLES} const a=<div sx={{styles.red}}/>;const b=<Component sx={{styles.red}}/>;const c=<div sx={{external}}/>;"
  ));
  assert!(has_jsx_attr(&output, "div", "className"));
  assert!(has_jsx_attr(&output, "Component", "sx"));
  assert!(has_call(&output, Some("stylex"), "props", Some("external")));
  let options: StyleXTransformOptions =
    serde_json::from_value(serde_json::json!({"sxPropName":false})).unwrap();
  let output = transform_source(
    &format!("{STYLES} const a=<div sx={{styles.red}}/>;"),
    "fixture.js",
    &options,
  )
  .unwrap();
  assert!(output.code.contains("sx={styles.red}"));
}
#[test]
fn sx_creates_a_hygienic_visible_runtime_import_when_namespace_is_shadowed() {
  let output = transform(
    "import * as stylex from '@stylexjs/stylex';function render(stylex){return <div sx={external}/>}",
  );
  assert!(output.code.contains("import * as _stylex from"));
  assert!(has_call(
    &output,
    Some("_stylex"),
    "props",
    Some("external")
  ));
}
#[test]
fn custom_sx_name_and_named_only_runtime_import() {
  let options: StyleXTransformOptions =
    serde_json::from_value(serde_json::json!({"sxPropName":"css"})).unwrap();
  let output=transform_source("import {create} from '@stylexjs/stylex';const a=<div css={external}/>;const b=<div sx={external}/>;","fixture.js",&options).unwrap();
  assert!(has_call(
    &output,
    Some("_stylex"),
    "props",
    Some("external")
  ));
  assert!(output.code.contains("sx={external}"));
}
#[test]
fn unrelated_default_marker_is_not_treated_as_stylex() {
  let output = transform(&format!(
    "{STYLES}const p=stylex.props(other.defaultMarker());"
  ));
  assert!(matches!(&*initializer(&output, "p"), Expr::Call(_)));
}
#[test]
fn atoms_static_and_dynamic_paths_use_native_core() {
  let output = transform(
    "import * as css from '@stylexjs/atoms';import * as stylex from '@stylexjs/stylex';const p=stylex.props(css.display.flex);const q=stylex.props(css.color(value));",
  );
  assert!(object_field(&initializer(&output, "p"), "className").is_some());
  assert!(
    output
      .metadata_stylex
      .iter()
      .any(|rule| rule.1.ltr.contains("display:flex"))
  );
  assert!(
    output
      .metadata_stylex
      .iter()
      .any(|rule| rule.1.ltr.starts_with("@property --x-color"))
  );
  assert!(output.code.contains("value"));
}
#[test]
fn theme_result_const_is_inlined_but_never_pruned() {
  let options: StyleXTransformOptions =
    serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste"}}))
      .unwrap();
  let output=transform_source("import * as stylex from '@stylexjs/stylex';export const vars=stylex.defineVars({color:'red'});const theme=stylex.createTheme(vars,{color:'blue'});const styles=stylex.create({a:{color:'green'}});const p=stylex.props(styles.a,theme);","fixture.stylex.js",&options).unwrap();
  let theme = initializer(&output, "theme");
  let Expr::Object(object) = unparen(&theme) else {
    panic!()
  };
  let classes = object
    .props
    .iter()
    .find_map(|prop| {
      let PropOrSpread::Prop(prop) = prop else {
        return None;
      };
      let Prop::KeyValue(kv) = &**prop else {
        return None;
      };
      match &*kv.value {
        Expr::Lit(Lit::Str(value)) => value.value.as_str().map(str::to_owned),
        _ => None,
      }
    })
    .unwrap();
  assert_eq!(
    object_field(&initializer(&output, "p"), "className"),
    Some(format!("{} {classes}", class(&output, "color:green")))
  );
  assert!(!module(&output.code).body.iter().any(|item|matches!(item,ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars)))if vars.decls.iter().any(|var|matches!(&var.name,Pat::Ident(binding)if binding.id.sym=="styles")))));
}
#[test]
fn babel_uid_sequence_is_preserved_for_thirteen_production_hoists() {
  let mut source =
    "import * as css from '@stylexjs/atoms';import * as stylex from '@stylexjs/stylex';".to_owned();
  for index in 0..13 {
    source.push_str(&format!("stylex.props(css.color(value{index}));"));
  }
  let output = transform(&source);
  let names = module(&output.code)
    .body
    .iter()
    .filter_map(|item| match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) => Some(vars),
      _ => None,
    })
    .flat_map(|vars| vars.decls.iter())
    .filter_map(|var| match &var.name {
      Pat::Ident(binding) if binding.id.sym.starts_with("_temp") => {
        Some(binding.id.sym.to_string())
      }
      _ => None,
    })
    .collect::<Vec<_>>();
  assert_eq!(
    names,
    vec![
      "_temp", "_temp2", "_temp3", "_temp4", "_temp5", "_temp6", "_temp7", "_temp8", "_temp9",
      "_temp0", "_temp1", "_temp10", "_temp11"
    ]
  );
}
#[test]
fn runtime_null_slots_follow_ordered_prefixes_and_real_escapes() {
  let options: StyleXTransformOptions =
    serde_json::from_value(serde_json::json!({"styleResolution":"application-order"})).unwrap();
  let source =
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({a:{padding:4}});";
  let count = |suffix: &str| {
    let output = transform_source(&format!("{source}{suffix}"), "fixture.js", &options).unwrap();
    let init = initializer(&output, "styles");
    let Expr::Object(object) = unparen(&init) else {
      panic!()
    };
    let namespace = object
      .props
      .iter()
      .find_map(|property| {
        let PropOrSpread::Prop(property) = property else {
          return None;
        };
        let Prop::KeyValue(kv) = &**property else {
          return None;
        };
        match &kv.key {
          PropName::Ident(key) if key.sym == "a" => Some(&*kv.value),
          PropName::Str(key) if key.value.as_str() == Some("a") => Some(&*kv.value),
          _ => None,
        }
      })
      .unwrap();
    let Expr::Object(object) = unparen(namespace) else {
      panic!()
    };
    object.props.len()
  };
  assert_eq!(count("const p=stylex.props(styles.a,external);"), 2);
  assert!(count("const p=stylex.props(external,styles.a);") > 2);
  assert!(count("const p=stylex.props(styles.a,external);console.log(styles.a);") > 2);
}
#[test]
fn define_vars_evaluates_current_static_spreads_and_shorthand() {
  let options: StyleXTransformOptions =
    serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste"}}))
      .unwrap();
  let output=transform_source("import * as stylex from '@stylexjs/stylex';const color='red';const base={accent:'blue'};export const tokens=stylex.defineVars({...base,color,space:10});","fixture.stylex.js",&options).unwrap();
  let rules = output
    .metadata_stylex
    .iter()
    .map(|rule| rule.1.ltr.as_str())
    .collect::<String>();
  assert!(rules.contains(":blue;"));
  assert!(rules.contains(":red;"));
  assert!(rules.contains(":10;"));
}
#[test]
fn define_vars_rejects_methods_and_getters_on_production_path() {
  let options: StyleXTransformOptions =
    serde_json::from_value(serde_json::json!({"unstable_moduleResolution":{"type":"haste"}}))
      .unwrap();
  for value in ["{get color(){return 'red'}}", "{color(){return 'red'}}"] {
    let error=transform_source(&format!("import * as stylex from '@stylexjs/stylex';const color='red';export const tokens=stylex.defineVars({value});"),"fixture.stylex.js",&options).expect_err("invalid shape");
    assert!(error.to_string().contains("Only static values"));
  }
}
