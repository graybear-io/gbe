use frame::{AuthorityLevel, ValueType};
use serde::{Deserialize, Serialize};

/// A composed geas — rites chained by interface dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geas {
    pub name: String,
    pub requires: AuthorityLevel,
    pub params: Vec<GeasParam>,
    pub chain: Vec<ChainStep>,
}

/// A parameter the human provides when invoking a geas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeasParam {
    pub name: String,
    pub kind: ValueType,
}

/// A step in a geas chain — either a rite invocation, a branch
/// on the previous rite's yield values, or a sub-geas invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainStep {
    /// Invoke a rite.
    Rite(RiteRef),
    /// Branch on the previous rite's yield values.
    /// Each arm has a pattern and a sub-chain.
    Branch { arms: Vec<BranchArm> },
    /// Invoke another geas as a step. The sub-geas executes its full
    /// chain and its final yields become this step's yields.
    /// A geas *is* a rite — it has needs (unmet) and yields (final).
    SubGeas { geas_name: String },
}

/// A reference to a rite within a geas, with argument bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiteRef {
    pub rite_name: String,
    pub bindings: Vec<Binding>,
}

/// Where a rite's parameter value comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Binding {
    /// From the geas invocation params (the human provided it).
    Param {
        /// Geas-level param name.
        source: String,
        /// Rite param to bind to.
        target: String,
    },
    /// From a previous rite's yields.
    Yield {
        /// The rite name to read from.
        rite: String,
        /// The yield field to extract.
        field: String,
        /// Rite param to bind to.
        target: String,
    },
}

/// A branch arm — a pattern to match against yield values, and
/// a sub-chain to execute if the pattern matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchArm {
    pub pattern: frame::YieldPattern,
    pub steps: Vec<ChainStep>,
}
