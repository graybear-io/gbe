//! Rite — the composable unit of geas.
//!
//! A rite declares what interfaces it needs, what authority it requires,
//! what parameters it accepts, and what it yields. It does not name nodes —
//! it names interface shapes. Nodes match when their published interfaces
//! satisfy the rite's needs.

use serde::{Deserialize, Serialize};

use crate::authority::AuthorityLevel;
use crate::interface::Interface;

/// The type of a value in a rite's yield or parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Bool,
    String,
    Integer,
}

/// A single field in a yield shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldField {
    pub name: String,
    pub kind: ValueType,
}

/// The shape of what a rite produces — named, typed fields.
///
/// Shape contracts use structural subtyping: a rite that yields
/// `{healthy: bool, detail: string, uptime: integer}` satisfies
/// a consumer expecting `{healthy: bool, detail: string}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldShape {
    pub fields: Vec<YieldField>,
}

impl YieldShape {
    pub fn empty() -> Self {
        Self { fields: vec![] }
    }

    /// Does this shape satisfy the expectations of `expected`?
    /// Structural subtyping: we must have at least every field the consumer expects.
    pub fn satisfies(&self, expected: &YieldShape) -> bool {
        expected.fields.iter().all(|exp| {
            self.fields
                .iter()
                .any(|f| f.name == exp.name && f.kind == exp.kind)
        })
    }
}

/// A parameter a rite accepts from the geas composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiteParam {
    pub name: String,
    pub kind: ValueType,
}

/// A rite — the composable unit of geas.
///
/// Declares what interfaces it needs, what authority it requires,
/// and what it yields. Does not name nodes — names interface shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rite {
    pub name: String,
    pub needs: Vec<Interface>,
    pub requires: AuthorityLevel,
    pub params: Vec<RiteParam>,
    pub yields: YieldShape,
}

impl Rite {
    /// Create a builder for constructing a Rite.
    pub fn builder(name: &str) -> RiteBuilder {
        RiteBuilder {
            name: name.to_string(),
            needs: Vec::new(),
            requires: AuthorityLevel::Pilgrim,
            params: Vec::new(),
            yields: Vec::new(),
        }
    }
}

/// Builder for constructing a Rite.
pub struct RiteBuilder {
    name: String,
    needs: Vec<Interface>,
    requires: AuthorityLevel,
    params: Vec<RiteParam>,
    yields: Vec<YieldField>,
}

impl RiteBuilder {
    pub fn needs(mut self, path: &str) -> Self {
        self.needs.push(crate::interface::iface(path));
        self
    }

    pub fn requires(mut self, level: AuthorityLevel) -> Self {
        self.requires = level;
        self
    }

    pub fn param(mut self, name: &str, kind: ValueType) -> Self {
        self.params.push(RiteParam {
            name: name.to_string(),
            kind,
        });
        self
    }

    pub fn yields_field(mut self, name: &str, kind: ValueType) -> Self {
        self.yields.push(YieldField {
            name: name.to_string(),
            kind,
        });
        self
    }

    pub fn build(self) -> Rite {
        Rite {
            name: self.name,
            needs: self.needs,
            requires: self.requires,
            params: self.params,
            yields: YieldShape {
                fields: self.yields,
            },
        }
    }
}

/// A concrete value produced by a rite at runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    String(String),
    Integer(i64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Integer(i) => write!(f, "{i}"),
        }
    }
}

/// A set of yield values produced by a rite execution.
pub type YieldValues = Vec<(String, Value)>;

/// Validate that yield values conform to a rite's declared yield shape.
/// Returns a list of violations (empty = valid).
pub fn validate_yields(values: &YieldValues, shape: &YieldShape) -> Vec<String> {
    let mut violations = Vec::new();

    for (name, val) in values {
        match shape.fields.iter().find(|f| f.name == *name) {
            None => {
                violations.push(format!("field '{name}' not declared in yield shape"));
            }
            Some(field) => {
                let actual_type = match val {
                    Value::Bool(_) => ValueType::Bool,
                    Value::String(_) => ValueType::String,
                    Value::Integer(_) => ValueType::Integer,
                };
                if actual_type != field.kind {
                    violations.push(format!(
                        "field '{name}' type mismatch: declared {:?}, got {:?}",
                        field.kind, actual_type
                    ));
                }
            }
        }
    }

    violations
}

/// Check if a pattern matches against yield values.
pub fn pattern_matches(pattern: &YieldPattern, values: &YieldValues) -> bool {
    match pattern {
        YieldPattern::Wildcard => true,
        YieldPattern::Fields(expected) => expected
            .iter()
            .all(|(name, val)| values.iter().any(|(n, v)| n == name && v == val)),
    }
}

/// A pattern for matching against yield values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum YieldPattern {
    /// Match specific field values. All specified fields must match.
    /// Fields not mentioned in the pattern are ignored (structural).
    Fields(Vec<(String, Value)>),
    /// Wildcard — matches anything. The default/fallback arm.
    Wildcard,
}
