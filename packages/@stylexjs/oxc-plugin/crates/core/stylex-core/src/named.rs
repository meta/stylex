//! `keyframes`, `positionTry` and `viewTransitionClass`: one object in, a generated name and
//! one rule out. Ported from #2008's `stylex_keyframes.rs`, `stylex_position_try.rs` and
//! `stylex_view_transition_class.rs`.

use std::{borrow::Cow, fmt::Write};

use indexmap::IndexMap;
use stylex_css::css::{generate_ltr::generate_ltr, generate_rtl::generate_rtl};
use stylex_structures::{
  order_pair::OrderPair,
  pair::{Pair, PairCow},
  pre_rule_value::PreRuleValue,
  raw_value::TRawValue,
  stylex_state_options::with_default_options,
};
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_utils::{hash::create_hash, string::dashify};
use stylex_value::{Object, Value};

use crate::{
  StyleXCore,
  css_value::{flat_map_expanded_shorthands, preprocess_object_properties},
};

type KeyframeSteps = IndexMap<String, Vec<Pair>>;

impl StyleXCore {
  pub(crate) fn keyframes_impl(&mut self, frames: &Object) -> (String, InjectableStyle) {
    let mut steps = KeyframeSteps::with_capacity(frames.len());
    for (step, frame) in frames {
      let entries = self.expand_frame_shorthands(frame);
      let pairs = self.transform_then_map_keys(&entries, |key| dashify(key).into_owned());
      steps.insert(step.clone(), pairs);
    }

    let options = &self.options;
    let ltr_string = construct_keyframes_obj(&steps, |pair| generate_ltr(pair, options));
    let stable_string = with_default_options(|stable| {
      construct_keyframes_obj(&steps, |pair| generate_ltr(pair, stable))
    });
    let rtl_string = construct_keyframes_obj(&steps, |pair| {
      generate_rtl(pair, options).unwrap_or_else(|| PairCow::borrowed(pair))
    });

    let animation_name = format!(
      "{}{}-B",
      options.class_name_prefix,
      create_hash(&format!("<>{}", stable_string))
    );
    let ltr = format!("@keyframes {}{{{}}}", animation_name, ltr_string);
    let rtl = (ltr_string != rtl_string)
      .then(|| format!("@keyframes {}{{{}}}", animation_name, rtl_string));
    (
      animation_name,
      InjectableStyle {
        ltr,
        rtl,
        priority: Some(0.0),
      },
    )
  }

  fn expand_frame_shorthands(&self, frame: &Value) -> IndexMap<String, TRawValue> {
    let Value::Object(frame) = frame else {
      stylex_macros::stylex_panic!("Object expected")
    };
    let mut out = IndexMap::new();
    for (key, value) in frame {
      let value = match value {
        Value::Number(n) => PreRuleValue::number(*n),
        Value::String(s) => PreRuleValue::string(s.clone()),
        _ => continue,
      };
      for OrderPair(property, value) in
        flat_map_expanded_shorthands((Cow::Owned(key.clone()), value), &self.options)
      {
        if let Some(value) = value {
          out.insert(property.into_owned(), value);
        }
      }
    }
    out
  }

  pub(crate) fn position_try_impl(&mut self, styles: &Object) -> (String, InjectableStyle) {
    let entries = preprocess_object_properties(styles, &self.options);
    let declarations = self.map_keys_then_transform(&entries, |key| dashify(key).into_owned());
    let options = &self.options;
    let ltr_string = construct_position_try_obj(&declarations, |pair| generate_ltr(pair, options));
    let rtl_string = construct_position_try_obj(&declarations, |pair| {
      generate_rtl(pair, options).unwrap_or_else(|| PairCow::borrowed(pair))
    });
    let name = format!("--{}{}", options.class_name_prefix, create_hash(&ltr_string));
    let ltr = format!("@position-try {} {{{}}}", name, ltr_string);
    let rtl =
      (ltr_string != rtl_string).then(|| format!("@position-try {} {{{}}}", name, rtl_string));
    (
      name,
      InjectableStyle {
        ltr,
        rtl,
        priority: Some(0.0),
      },
    )
  }

  pub(crate) fn view_transition_class_impl(&mut self, styles: &Object) -> (String, InjectableStyle) {
    let mut parts: IndexMap<String, String> = IndexMap::with_capacity(styles.len());
    for (part, value) in styles {
      let Value::Object(value) = value else {
        stylex_macros::stylex_panic!("Object expected")
      };
      let entries = preprocess_object_properties(value, &self.options);
      let declarations = self.transform_then_map_keys(&entries, |key| dashify(key).into_owned());
      let css_text: String = declarations.iter().filter_map(Pair::as_css_text).collect();
      parts.insert(format!("::view-transition-{}", dashify(part)), css_text);
    }

    let mut to_hash = String::new();
    for (part, css_text) in &parts {
      let _ = write!(to_hash, "{}:{};", part, css_text);
    }
    let class_name = format!("{}{}", self.options.class_name_prefix, create_hash(&to_hash));
    let mut style = String::new();
    for (part, css_text) in &parts {
      let _ = write!(style, "{}(*.{}){{{}}}", part, class_name, css_text);
    }
    (
      class_name,
      InjectableStyle {
        ltr: style,
        rtl: None,
        priority: Some(1.0),
      },
    )
  }
}

fn construct_keyframes_obj(
  steps: &KeyframeSteps,
  resolve: impl for<'a> Fn(&'a Pair) -> PairCow<'a>,
) -> String {
  let mut result = String::new();
  for (step, pairs) in steps {
    result.push_str(step);
    result.push('{');
    for css_text in pairs.iter().filter_map(|pair| resolve(pair).as_css_text()) {
      result.push_str(&css_text);
    }
    result.push('}');
  }
  result
}

fn construct_position_try_obj(
  declarations: &[Pair],
  resolve: impl for<'a> Fn(&'a Pair) -> PairCow<'a>,
) -> String {
  let mut mapped = IndexMap::with_capacity(declarations.len());
  for declaration in declarations {
    let pair = resolve(declaration);
    mapped.insert(pair.key.into_owned(), pair.value.into_owned());
  }
  let mut sorted = mapped.into_iter().collect::<Vec<_>>();
  sorted.sort_unstable_by(|(l, _), (r, _)| l.encode_utf16().cmp(r.encode_utf16()));
  let mut output = String::new();
  for (key, value) in sorted {
    if let Some(css_text) = Pair::new(key, value).as_css_text() {
      output.push_str(&css_text);
    }
  }
  output
}
