use super::{Context, Value, output_limit, value::*};
use swc_core::common::DUMMY_SP;
/// String-pattern replacement only. RegExp objects, Symbol hooks and host callbacks
/// are not admitted into Value and therefore cannot reach this dispatch.
pub(super) fn evaluate(
  v: &Value,
  name: &str,
  args: &[Value],
  ctx: &mut Context<'_>,
) -> Option<Value> {
  let input = v.string_data()?;
  let needle = args
    .first()
    .cloned()
    .unwrap_or(Value::Undefined)
    .to_units()?;
  let replacement = args.get(1).cloned().unwrap_or(Value::Undefined);
  let mut out = vec![];
  let mut last = 0;
  let mut i = 0;
  while i <= input.len() {
    ctx.budget.spend(needle.len().max(1))?;
    if input[i..].starts_with(&needle) {
      out.extend_from_slice(&input[last..i]);
      let matched = &input[i..i + needle.len()];
      if replacement.is_function() {
        let result = ctx.call_value(
          &replacement,
          vec![
            string_units(matched.to_vec()),
            Value::Number(i as f64),
            v.clone(),
          ],
          ctx.callsite.unwrap_or(DUMMY_SP),
        )?;
        out.extend(result.to_units()?);
      } else {
        let text = replacement.to_units()?;
        let mut j = 0;
        while j < text.len() {
          if text[j] == 36 {
            match text.get(j + 1).copied() {
              Some(36) => {
                out.push(36);
                j += 2;
                continue;
              }
              Some(38) => {
                out.extend_from_slice(matched);
                j += 2;
                continue;
              }
              Some(96) => {
                out.extend_from_slice(&input[..i]);
                j += 2;
                continue;
              }
              Some(39) => {
                out.extend_from_slice(&input[i + needle.len()..]);
                j += 2;
                continue;
              }
              _ => {}
            }
          }
          out.push(text[j]);
          j += 1;
        }
      }
      if out.len() > output_limit() {
        return None;
      }
      last = i + needle.len();
      if name == "replace" {
        break;
      }
      i += needle.len().max(1);
    } else {
      i += 1;
    }
  }
  out.extend_from_slice(&input[last..]);
  if out.len() > output_limit() {
    return None;
  }
  Some(string_units(out))
}
