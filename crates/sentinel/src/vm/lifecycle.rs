/// VM lifecycle state machine.
///
/// ```text
/// IDLE → PROVISIONING → RUNNING → COLLECTING → TEARDOWN → IDLE
///              │             │           │
///              ▼             ▼           ▼
///           FAILED        TIMEOUT     FAILED
///              │             │           │
///              └──── all ────┴→ TEARDOWN ┘
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmState {
    Idle,
    Provisioning,
    Running,
    Collecting,
    Teardown,
    Failed(String),
    Timeout,
}

pub struct VmLifecycle {
    pub state: VmState,
    pub task_id: Option<String>,
}

impl Default for VmLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl VmLifecycle {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: VmState::Idle,
            task_id: None,
        }
    }

    pub fn transition(&mut self, next: VmState) {
        tracing::info!(from = ?self.state, to = ?next, task = ?self.task_id, "vm state transition");
        self.state = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_idle() {
        let vm = VmLifecycle::new();
        assert_eq!(vm.state, VmState::Idle);
        assert!(vm.task_id.is_none());
    }

    #[test]
    fn default_trait_matches_new() {
        let vm = VmLifecycle::default();
        assert_eq!(vm.state, VmState::Idle);
    }

    #[test]
    fn happy_path_lifecycle() {
        let mut vm = VmLifecycle::new();
        vm.task_id = Some("task-1".into());
        vm.transition(VmState::Provisioning);
        assert_eq!(vm.state, VmState::Provisioning);
        vm.transition(VmState::Running);
        assert_eq!(vm.state, VmState::Running);
        vm.transition(VmState::Collecting);
        assert_eq!(vm.state, VmState::Collecting);
        vm.transition(VmState::Teardown);
        assert_eq!(vm.state, VmState::Teardown);
        vm.transition(VmState::Idle);
        assert_eq!(vm.state, VmState::Idle);
    }

    #[test]
    fn failure_during_provisioning() {
        let mut vm = VmLifecycle::new();
        vm.transition(VmState::Provisioning);
        vm.transition(VmState::Failed("disk full".into()));
        assert_eq!(vm.state, VmState::Failed("disk full".into()));
        vm.transition(VmState::Teardown);
        assert_eq!(vm.state, VmState::Teardown);
    }

    #[test]
    fn timeout_during_running() {
        let mut vm = VmLifecycle::new();
        vm.transition(VmState::Running);
        vm.transition(VmState::Timeout);
        assert_eq!(vm.state, VmState::Timeout);
        vm.transition(VmState::Teardown);
        assert_eq!(vm.state, VmState::Teardown);
    }

    #[test]
    fn task_id_persists_through_transitions() {
        let mut vm = VmLifecycle::new();
        vm.task_id = Some("task-42".into());
        vm.transition(VmState::Provisioning);
        vm.transition(VmState::Running);
        assert_eq!(vm.task_id.as_deref(), Some("task-42"));
    }

    #[test]
    fn vm_state_equality() {
        assert_eq!(VmState::Idle, VmState::Idle);
        assert_ne!(VmState::Idle, VmState::Running);
        assert_eq!(VmState::Failed("a".into()), VmState::Failed("a".into()));
        assert_ne!(VmState::Failed("a".into()), VmState::Failed("b".into()));
    }
}
