//! Geas — latent instructions encoded in the system.
//!
//! A geas is a named template for orchestrated action across multiple nodes.
//! It doesn't execute on its own — it needs a trigger (a writ from a human)
//! and an interpreter (Oracle). Oracle reads the geas, resolves capability
//! references to specific nodes, and produces a mandate (a concrete DAG).
//!
//! Named after the Forerunner concept from Halo — a genetic command implanted
//! by the Librarian that activates when conditions are met.

use serde::{Deserialize, Serialize};

use crate::authority::AuthorityLevel;

/// A latent instruction — a named, composable recipe of capability
/// references that Oracle translates into a DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geas {
    /// Unique name: "deploy-to-staging", "notify-on-failure".
    pub name: String,

    /// Human-readable description of what this geas does.
    pub description: String,

    /// Schema version for this geas definition.
    pub version: u32,

    /// Standing or invoked.
    pub lifecycle: GeasLifecycle,

    /// Minimum authority level required to invoke this geas.
    pub authority_required: AuthorityLevel,

    /// Parameters the human must (or may) provide when invoking.
    pub params: Vec<GeasParam>,

    /// The ordered steps that compose this geas.
    pub steps: Vec<GeasStep>,
}

/// Whether a geas defines a node's identity or a one-shot action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeasLifecycle {
    /// The DNA of a role. Runs for the lifetime of the node.
    /// Defines what the node *is*. Capabilities are derived from this.
    Standing,

    /// A one-shot action triggered by a writ. Runs, completes, done.
    Invoked,
}

/// A parameter the human provides when invoking a geas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeasParam {
    /// Parameter name.
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// Whether this parameter is required.
    pub required: bool,
}

/// A single step in a geas — a reference to a capability on a role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeasStep {
    /// Human-readable name for this step.
    pub name: String,

    /// The role that performs this step (e.g., "sentinel", "oracle").
    /// Not a specific node — any node with this role that offers the
    /// named capability.
    pub role: String,

    /// The capability to invoke on that role.
    pub capability: String,

    /// How to bind inputs to this step's parameters.
    /// Keys are the capability's param names, values are source references.
    pub input: Vec<StepBinding>,

    /// Steps that must complete before this one can run.
    pub depends_on: Vec<String>,
}

/// Binds a step parameter to a source — either a geas-level param
/// from the human, or an output from a previous step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case")]
pub enum StepBinding {
    /// Value comes from the geas invocation params (the human provided it).
    Param {
        /// The geas-level param name.
        name: String,
        /// The step's capability param to bind to.
        target: String,
    },

    /// Value comes from a previous step's output.
    Step {
        /// The step name to read from.
        step: String,
        /// The output field to extract.
        field: String,
        /// The step's capability param to bind to.
        target: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geas_round_trip() {
        let geas = Geas {
            name: "restart-failed-task".to_string(),
            description: "Retry a failed task by ID".to_string(),
            version: 1,
            lifecycle: GeasLifecycle::Invoked,
            authority_required: AuthorityLevel::Pilgrim,
            params: vec![GeasParam {
                name: "task_id".to_string(),
                description: "The task to restart".to_string(),
                required: true,
            }],
            steps: vec![
                GeasStep {
                    name: "get-status".to_string(),
                    role: "oracle".to_string(),
                    capability: "job-status".to_string(),
                    input: vec![StepBinding::Param {
                        name: "task_id".to_string(),
                        target: "job_id".to_string(),
                    }],
                    depends_on: vec![],
                },
                GeasStep {
                    name: "resubmit".to_string(),
                    role: "oracle".to_string(),
                    capability: "create-job".to_string(),
                    input: vec![StepBinding::Step {
                        step: "get-status".to_string(),
                        field: "definition".to_string(),
                        target: "definition".to_string(),
                    }],
                    depends_on: vec!["get-status".to_string()],
                },
            ],
        };

        let json = serde_json::to_string_pretty(&geas).unwrap();
        let back: Geas = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "restart-failed-task");
        assert_eq!(back.steps.len(), 2);
        assert_eq!(back.steps[1].depends_on, vec!["get-status"]);
    }

    #[test]
    fn geas_yaml_round_trip() {
        let yaml = r#"
name: notify-on-failure
description: Watch for a job failure and send a notification
version: 1
lifecycle: invoked
authority_required: pilgrim
params:
  - name: job_type
    description: The job type to watch
    required: true
steps:
  - name: watch
    role: watcher
    capability: trigger-sweep
    input: []
    depends_on: []
"#;
        let geas: Geas = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(geas.name, "notify-on-failure");
        assert_eq!(geas.steps[0].role, "watcher");
    }
}
