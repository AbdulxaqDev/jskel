//! Skeleton transformation: replace scalar values with deterministic
//! defaults while preserving structure (objects, arrays, key order).

use crate::json::Value;

#[derive(Clone, Copy, Debug, Default)]
pub enum Strategy {
    /// `string -> ""`, `number -> 0`, `bool -> false`, `null -> null`.
    #[default]
    Default,
    /// Every scalar becomes `null`.
    Nulls,
    /// Every scalar becomes a string naming its type
    /// (`"string"`, `"number"`, `"boolean"`, `"null"`).
    Types,
    /// Like `Default`, but booleans keep their original value.
    PreserveBool,
}

pub fn skeletonize(v: Value, strat: Strategy) -> Value {
    match v {
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .map(|(k, v)| (k, skeletonize(v, strat)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items.into_iter().map(|v| skeletonize(v, strat)).collect(),
        ),
        scalar => match strat {
            Strategy::Default => default_scalar(scalar),
            Strategy::Nulls => Value::Null,
            Strategy::Types => type_scalar(scalar),
            Strategy::PreserveBool => preserve_bool_scalar(scalar),
        },
    }
}

fn default_scalar(v: Value) -> Value {
    match v {
        Value::String(_) => Value::String(String::new()),
        Value::Number(_) => Value::Number("0".into()),
        Value::Bool(_) => Value::Bool(false),
        Value::Null => Value::Null,
        v => v,
    }
}

fn type_scalar(v: Value) -> Value {
    match v {
        Value::String(_) => Value::String("string".into()),
        Value::Number(_) => Value::String("number".into()),
        Value::Bool(_) => Value::String("boolean".into()),
        Value::Null => Value::String("null".into()),
        v => v,
    }
}

fn preserve_bool_scalar(v: Value) -> Value {
    match v {
        Value::Bool(b) => Value::Bool(b),
        other => default_scalar(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn skel(input: &str, strat: Strategy) -> Value {
        skeletonize(parse(input).unwrap(), strat)
    }

    #[test]
    fn default_scalars() {
        assert_eq!(skel(r#""hi""#, Strategy::Default), Value::String("".into()));
        assert_eq!(skel("42", Strategy::Default), Value::Number("0".into()));
        assert_eq!(skel("true", Strategy::Default), Value::Bool(false));
        assert_eq!(skel("null", Strategy::Default), Value::Null);
    }

    #[test]
    fn default_recurses() {
        let v = skel(
            r#"{"name":"Brendan","age":40,"active":true}"#,
            Strategy::Default,
        );
        assert_eq!(
            v,
            Value::Object(vec![
                ("name".into(), Value::String("".into())),
                ("age".into(), Value::Number("0".into())),
                ("active".into(), Value::Bool(false)),
            ])
        );
    }

    #[test]
    fn nulls_strategy() {
        let v = skel(r#"{"a":1,"b":[true,"x"]}"#, Strategy::Nulls);
        assert_eq!(
            v,
            Value::Object(vec![
                ("a".into(), Value::Null),
                (
                    "b".into(),
                    Value::Array(vec![Value::Null, Value::Null])
                ),
            ])
        );
    }

    #[test]
    fn types_strategy() {
        let v = skel(r#"{"a":1,"b":"x","c":true,"d":null}"#, Strategy::Types);
        assert_eq!(
            v,
            Value::Object(vec![
                ("a".into(), Value::String("number".into())),
                ("b".into(), Value::String("string".into())),
                ("c".into(), Value::String("boolean".into())),
                ("d".into(), Value::String("null".into())),
            ])
        );
    }

    #[test]
    fn preserve_bool_strategy() {
        let v = skel(
            r#"{"flag":true,"other":false,"x":42}"#,
            Strategy::PreserveBool,
        );
        assert_eq!(
            v,
            Value::Object(vec![
                ("flag".into(), Value::Bool(true)),
                ("other".into(), Value::Bool(false)),
                ("x".into(), Value::Number("0".into())),
            ])
        );
    }

    #[test]
    fn preserves_structure_through_nesting() {
        let v = skel(r#"[{"a":[1,2]},{"b":"x"}]"#, Strategy::Default);
        assert_eq!(
            v,
            Value::Array(vec![
                Value::Object(vec![(
                    "a".into(),
                    Value::Array(vec![
                        Value::Number("0".into()),
                        Value::Number("0".into())
                    ])
                )]),
                Value::Object(vec![("b".into(), Value::String("".into()))]),
            ])
        );
    }
}
