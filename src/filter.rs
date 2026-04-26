//! Top-level key filtering. For an array of objects, the filter applies to
//! each item — that's the common shape (e.g. a JSON list endpoint) and the
//! intuitive thing to do. We don't recurse into nested objects.

use crate::json::Value;

pub fn pick(v: Value, keys: &[String]) -> Value {
    match v {
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .filter(|(k, _)| keys.iter().any(|target| target == k))
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items.into_iter().map(|v| pick(v, keys)).collect(),
        ),
        v => v,
    }
}

pub fn omit(v: Value, keys: &[String]) -> Value {
    match v {
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .filter(|(k, _)| !keys.iter().any(|target| target == k))
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items.into_iter().map(|v| omit(v, keys)).collect(),
        ),
        v => v,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn pick_keeps_only_listed() {
        let v = parse(r#"{"a":1,"b":2,"c":3}"#).unwrap();
        let v = pick(v, &["a".into(), "c".into()]);
        assert_eq!(
            v,
            Value::Object(vec![
                ("a".into(), Value::Number("1".into())),
                ("c".into(), Value::Number("3".into())),
            ])
        );
    }

    #[test]
    fn omit_drops_listed() {
        let v = parse(r#"{"a":1,"b":2,"c":3}"#).unwrap();
        let v = omit(v, &["b".into()]);
        assert_eq!(
            v,
            Value::Object(vec![
                ("a".into(), Value::Number("1".into())),
                ("c".into(), Value::Number("3".into())),
            ])
        );
    }

    #[test]
    fn pick_in_array_of_objects() {
        let v = parse(r#"[{"a":1,"b":2},{"a":3,"b":4}]"#).unwrap();
        let v = pick(v, &["a".into()]);
        assert_eq!(
            v,
            Value::Array(vec![
                Value::Object(vec![("a".into(), Value::Number("1".into()))]),
                Value::Object(vec![("a".into(), Value::Number("3".into()))]),
            ])
        );
    }

    #[test]
    fn pick_does_not_recurse_into_nested_objects() {
        let v = parse(r#"{"user":{"a":1,"b":2}}"#).unwrap();
        let v = pick(v, &["user".into()]);
        assert_eq!(
            v,
            Value::Object(vec![(
                "user".into(),
                Value::Object(vec![
                    ("a".into(), Value::Number("1".into())),
                    ("b".into(), Value::Number("2".into())),
                ])
            )])
        );
    }
}
