//! Minimal `${{ expression }}` template engine for workflow step inputs.
//!
//! Supported expressions:
//!
//! - `steps.<step_name>.output.<dot.path>` — value from a previous step's
//!   handler output (preserving JSON type when the expression is the whole
//!   string).
//! - `steps.<step_name>.run_id` — the persisted run id of a previous step.
//! - `workflow.inputs.<key>` / `inputs.<key>` — value from `workflow.inputs`
//!   merged with the top-level inputs passed when running the workflow.
//!
//! When a string is exactly `${{ <expr> }}` (after trim, no embedding), the
//! resolved value's JSON type is preserved. When the marker is embedded in
//! other text, the value is coerced to a string and substituted.

use color_eyre::eyre::{Result, bail, eyre};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct TemplateContext {
    pub workflow_inputs: BTreeMap<String, Value>,
    pub step_outputs: BTreeMap<String, Value>,
    pub step_run_ids: BTreeMap<String, String>,
}

impl TemplateContext {
    pub fn record_step(&mut self, name: &str, run_id: String, output: Option<Value>) {
        self.step_run_ids.insert(name.to_string(), run_id);
        if let Some(value) = output {
            self.step_outputs.insert(name.to_string(), value);
        }
    }
}

pub fn expand_value(value: &Value, ctx: &TemplateContext) -> Result<Value> {
    match value {
        Value::String(text) => expand_string(text, ctx),
        Value::Array(items) => {
            let mut expanded = Vec::with_capacity(items.len());
            for item in items {
                expanded.push(expand_value(item, ctx)?);
            }
            Ok(Value::Array(expanded))
        }
        Value::Object(map) => {
            let mut expanded = Map::with_capacity(map.len());
            for (key, item) in map {
                expanded.insert(key.clone(), expand_value(item, ctx)?);
            }
            Ok(Value::Object(expanded))
        }
        other => Ok(other.clone()),
    }
}

pub fn expand_inputs(
    inputs: &BTreeMap<String, Value>,
    ctx: &TemplateContext,
) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for (key, value) in inputs {
        out.insert(key.clone(), expand_value(value, ctx)?);
    }
    Ok(out)
}

const OPEN: &str = "${{";
const CLOSE: &str = "}}";

fn expand_string(text: &str, ctx: &TemplateContext) -> Result<Value> {
    let trimmed = text.trim();
    if let Some(expr) = exact_single_expression(trimmed) {
        return resolve_expression(expr, ctx);
    }

    let mut out = String::new();
    let mut remaining = text;
    while let Some(start) = remaining.find(OPEN) {
        out.push_str(&remaining[..start]);
        let after_open = &remaining[start + OPEN.len()..];
        let end = after_open.find(CLOSE).ok_or_else(|| {
            eyre!(
                "template '{}' has an unterminated `${{{{ ... }}}}` block",
                text
            )
        })?;
        let expr = after_open[..end].trim();
        let value = resolve_expression(expr, ctx)?;
        out.push_str(&value_to_string(&value));
        remaining = &after_open[end + CLOSE.len()..];
    }
    out.push_str(remaining);
    Ok(Value::String(out))
}

/// If the whole (trimmed) string is exactly one `${{ ... }}` block with no
/// surrounding text, return the inner expression.
fn exact_single_expression(trimmed: &str) -> Option<&str> {
    if !trimmed.starts_with(OPEN) || !trimmed.ends_with(CLOSE) {
        return None;
    }
    let inner = &trimmed[OPEN.len()..trimmed.len() - CLOSE.len()];
    if inner.contains(OPEN) || inner.contains(CLOSE) {
        return None;
    }
    Some(inner.trim())
}

fn resolve_expression(expr: &str, ctx: &TemplateContext) -> Result<Value> {
    let mut parts = expr.split('.');
    let head = parts.next().ok_or_else(|| eyre!("empty expression"))?;
    let rest: Vec<&str> = parts.collect();

    match head {
        "steps" => resolve_steps(&rest, ctx),
        "workflow" => match rest.split_first() {
            Some((&"inputs", tail)) => resolve_path(
                &Value::Object(map_from_btreemap(&ctx.workflow_inputs)),
                tail,
                expr,
            ),
            _ => bail!(
                "unsupported workflow expression '{}'; expected `workflow.inputs.<key>`",
                expr
            ),
        },
        "inputs" => resolve_path(
            &Value::Object(map_from_btreemap(&ctx.workflow_inputs)),
            &rest,
            expr,
        ),
        other => bail!(
            "unsupported expression '{}': expected `steps.<name>.output.<...>`, `steps.<name>.run_id`, or `workflow.inputs.<key>`",
            other
        ),
    }
}

fn resolve_steps(rest: &[&str], ctx: &TemplateContext) -> Result<Value> {
    let (step_name, tail) = rest
        .split_first()
        .ok_or_else(|| eyre!("expression `steps` requires a step name"))?;
    let step_name = *step_name;

    match tail.split_first() {
        Some((&"run_id", remainder)) if remainder.is_empty() => ctx
            .step_run_ids
            .get(step_name)
            .map(|id| Value::String(id.clone()))
            .ok_or_else(|| eyre!("step '{}' has not run yet", step_name)),
        Some((&"output", remainder)) => {
            let output = ctx.step_outputs.get(step_name).ok_or_else(|| {
                eyre!(
                    "step '{}' produced no output (it failed or was skipped)",
                    step_name
                )
            })?;
            resolve_path(output, remainder, step_name)
        }
        _ => bail!(
            "unsupported `steps.{}` expression; expected `.output.<path>` or `.run_id`",
            step_name
        ),
    }
}

fn resolve_path(value: &Value, path: &[&str], source: &str) -> Result<Value> {
    let mut current = value.clone();
    for segment in path {
        current = current
            .get(*segment)
            .cloned()
            .ok_or_else(|| eyre!("path segment '{}' not found in '{}'", segment, source))?;
    }
    Ok(current)
}

fn map_from_btreemap(input: &BTreeMap<String, Value>) -> Map<String, Value> {
    let mut map = Map::with_capacity(input.len());
    for (k, v) in input {
        map.insert(k.clone(), v.clone());
    }
    map
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> TemplateContext {
        let mut c = TemplateContext::default();
        c.workflow_inputs
            .insert("env".into(), Value::String("prod".into()));
        c.workflow_inputs.insert("retries".into(), json!(3));
        c.step_outputs.insert(
            "first".into(),
            json!({ "id": 42, "name": "alpha", "nested": { "ok": true } }),
        );
        c.step_run_ids
            .insert("first".into(), "run-12345".to_string());
        c
    }

    #[test]
    fn exact_match_preserves_int() {
        let v = expand_value(&Value::String("${{ steps.first.output.id }}".into()), &ctx()).unwrap();
        assert_eq!(v, json!(42));
    }

    #[test]
    fn exact_match_preserves_object() {
        let v = expand_value(
            &Value::String("${{ steps.first.output.nested }}".into()),
            &ctx(),
        )
        .unwrap();
        assert_eq!(v, json!({ "ok": true }));
    }

    #[test]
    fn embedded_does_string_substitution() {
        let v = expand_value(
            &Value::String("issue-${{ steps.first.output.id }}-${{ workflow.inputs.env }}".into()),
            &ctx(),
        )
        .unwrap();
        assert_eq!(v, Value::String("issue-42-prod".into()));
    }

    #[test]
    fn inputs_shortcut_works() {
        let v = expand_value(
            &Value::String("${{ inputs.retries }}".into()),
            &ctx(),
        )
        .unwrap();
        assert_eq!(v, json!(3));
    }

    #[test]
    fn run_id_resolution() {
        let v = expand_value(
            &Value::String("${{ steps.first.run_id }}".into()),
            &ctx(),
        )
        .unwrap();
        assert_eq!(v, Value::String("run-12345".into()));
    }

    #[test]
    fn unknown_step_errors() {
        let err = expand_value(
            &Value::String("${{ steps.missing.output.id }}".into()),
            &ctx(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("step 'missing' produced no output"));
    }

    #[test]
    fn missing_path_segment_errors() {
        let err = expand_value(
            &Value::String("${{ steps.first.output.does_not_exist }}".into()),
            &ctx(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("does_not_exist"));
    }

    #[test]
    fn unterminated_template_errors() {
        let err = expand_value(&Value::String("oops ${{ broken".into()), &ctx()).unwrap_err();
        assert!(err.to_string().contains("unterminated"));
    }

    #[test]
    fn non_string_values_pass_through() {
        let original = json!({ "a": [1, 2, "${{ workflow.inputs.env }}"], "b": false });
        let expanded = expand_value(&original, &ctx()).unwrap();
        assert_eq!(expanded, json!({ "a": [1, 2, "prod"], "b": false }));
    }
}
