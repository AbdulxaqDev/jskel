//! JSON value tree.
//!
//! Numbers are kept as their original lexeme so precision is preserved
//! end-to-end (we never round-trip through f64). Objects use Vec to keep
//! insertion order, which matters for deterministic output.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}
