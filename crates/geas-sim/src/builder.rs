use frame::{AuthorityLevel, Interface, NodeCondition, ValueType, iface};

use crate::geas::{Binding, BranchArm, ChainStep, Geas, GeasParam, RiteRef};
use crate::imprint::Imprint;
use crate::node::{Barrier, CrossingMode, CrossingRule, SimNode};

// --- SimNode builder ---

pub struct SimNodeBuilder {
    name: String,
    interfaces: Vec<Interface>,
    authority: AuthorityLevel,
    domain: String,
    condition: NodeCondition,
}

impl SimNode {
    pub fn builder(name: &str) -> SimNodeBuilder {
        SimNodeBuilder {
            name: name.to_string(),
            interfaces: Vec::new(),
            authority: AuthorityLevel::Pilgrim,
            domain: "gbe".to_string(),
            condition: NodeCondition::Ready,
        }
    }
}

impl SimNodeBuilder {
    pub fn interface(mut self, path: &str) -> Self {
        self.interfaces.push(iface(path));
        self
    }

    pub fn authority(mut self, level: AuthorityLevel) -> Self {
        self.authority = level;
        self
    }

    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }

    pub fn condition(mut self, condition: NodeCondition) -> Self {
        self.condition = condition;
        self
    }

    pub fn build(self) -> SimNode {
        SimNode {
            name: self.name,
            interfaces: self.interfaces,
            authority: self.authority,
            domain: self.domain,
            imprint_name: None,
            condition: self.condition,
        }
    }
}

// --- Barrier builder ---

pub struct BarrierBuilder {
    name: String,
    outer_domain: String,
    inner_domain: String,
    authority: AuthorityLevel,
    crossing_rules: Vec<CrossingRule>,
    condition: NodeCondition,
}

impl Barrier {
    pub fn builder(name: &str) -> BarrierBuilder {
        BarrierBuilder {
            name: name.to_string(),
            outer_domain: String::new(),
            inner_domain: String::new(),
            authority: AuthorityLevel::Consul,
            crossing_rules: Vec::new(),
            condition: NodeCondition::Ready,
        }
    }
}

impl BarrierBuilder {
    pub fn outer_domain(mut self, domain: &str) -> Self {
        self.outer_domain = domain.to_string();
        self
    }

    pub fn inner_domain(mut self, domain: &str) -> Self {
        self.inner_domain = domain.to_string();
        self
    }

    pub fn authority(mut self, level: AuthorityLevel) -> Self {
        self.authority = level;
        self
    }

    /// Add a crossing rule: absorb (handle locally, nothing inward).
    pub fn absorbs(mut self, outer_rite: &str) -> Self {
        self.crossing_rules.push(CrossingRule {
            outer_rite: outer_rite.to_string(),
            mode: CrossingMode::Absorb,
        });
        self
    }

    /// Add a crossing rule: forward (pass rite unchanged to inner domain).
    pub fn forwards(mut self, outer_rite: &str) -> Self {
        self.crossing_rules.push(CrossingRule {
            outer_rite: outer_rite.to_string(),
            mode: CrossingMode::Forward,
        });
        self
    }

    /// Add a crossing rule: translate (outer rite becomes different inner rites).
    pub fn translates(mut self, outer_rite: &str, inner_rites: Vec<&str>) -> Self {
        self.crossing_rules.push(CrossingRule {
            outer_rite: outer_rite.to_string(),
            mode: CrossingMode::Translate {
                inner_rites: inner_rites.into_iter().map(|s| s.to_string()).collect(),
            },
        });
        self
    }

    pub fn condition(mut self, condition: NodeCondition) -> Self {
        self.condition = condition;
        self
    }

    pub fn build(self) -> Barrier {
        Barrier {
            name: self.name,
            outer_domain: self.outer_domain,
            inner_domain: self.inner_domain,
            authority: self.authority,
            crossing_rules: self.crossing_rules,
            condition: self.condition,
        }
    }
}

// --- Imprint builder ---

pub struct ImprintBuilder {
    name: String,
    rites: Vec<String>,
    authority: AuthorityLevel,
}

impl Imprint {
    pub fn builder(name: &str) -> ImprintBuilder {
        ImprintBuilder {
            name: name.to_string(),
            rites: Vec::new(),
            authority: AuthorityLevel::Pilgrim,
        }
    }
}

impl ImprintBuilder {
    pub fn rite(mut self, name: &str) -> Self {
        self.rites.push(name.to_string());
        self
    }

    pub fn authority(mut self, level: AuthorityLevel) -> Self {
        self.authority = level;
        self
    }

    pub fn build(self) -> Imprint {
        Imprint {
            name: self.name,
            rites: self.rites,
            authority: self.authority,
        }
    }
}

// --- Geas builder ---

pub struct GeasBuilder {
    name: String,
    requires: AuthorityLevel,
    params: Vec<GeasParam>,
    chain: Vec<ChainStep>,
}

impl Geas {
    pub fn builder(name: &str) -> GeasBuilder {
        GeasBuilder {
            name: name.to_string(),
            requires: AuthorityLevel::Pilgrim,
            params: Vec::new(),
            chain: Vec::new(),
        }
    }
}

impl GeasBuilder {
    pub fn requires(mut self, level: AuthorityLevel) -> Self {
        self.requires = level;
        self
    }

    pub fn param(mut self, name: &str, kind: ValueType) -> Self {
        self.params.push(GeasParam {
            name: name.to_string(),
            kind,
        });
        self
    }

    pub fn rite(mut self, name: &str) -> Self {
        self.chain.push(ChainStep::Rite(RiteRef {
            rite_name: name.to_string(),
            bindings: vec![],
        }));
        self
    }

    pub fn rite_with(mut self, name: &str, bindings: Vec<Binding>) -> Self {
        self.chain.push(ChainStep::Rite(RiteRef {
            rite_name: name.to_string(),
            bindings,
        }));
        self
    }

    pub fn branch(mut self, arms: Vec<BranchArm>) -> Self {
        self.chain.push(ChainStep::Branch { arms });
        self
    }

    pub fn sub_geas(mut self, name: &str) -> Self {
        self.chain.push(ChainStep::SubGeas {
            geas_name: name.to_string(),
        });
        self
    }

    pub fn build(self) -> Geas {
        Geas {
            name: self.name,
            requires: self.requires,
            params: self.params,
            chain: self.chain,
        }
    }
}
