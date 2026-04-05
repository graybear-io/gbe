//! Branching and failure-as-yield-shape scenarios.
//!
//! Explores: branch on yield values, failure as just another shape,
//! nested branches, wildcard fallback, no-arm-matched.

use frame::AuthorityLevel;
use geas_sim::geas::RiteRef;
use geas_sim::*;

fn setup(engine: &mut SimEngine) {
    // --- Rites ---

    engine.register_rite(
        Rite::builder("check-health")
            .needs("target::host")
            .needs("resource::health-probe")
            .requires(AuthorityLevel::Pilgrim)
            .yields_field("healthy", ValueType::Bool)
            .yields_field("detail", ValueType::String)
            .build(),
    );

    engine.register_rite(
        Rite::builder("deploy-image")
            .needs("target::host")
            .needs("resource::deploy")
            .requires(AuthorityLevel::Consul)
            .param("image", ValueType::String)
            .yields_field("deployed", ValueType::Bool)
            .yields_field("deploy_id", ValueType::String)
            // Failure is part of the yield shape — not a separate mechanism.
            .yields_field("failed", ValueType::Bool)
            .yields_field("reason", ValueType::String)
            .build(),
    );

    engine.register_rite(
        Rite::builder("notify-unhealthy")
            .needs("resource::notification")
            .requires(AuthorityLevel::Pilgrim)
            .build(),
    );

    engine.register_rite(
        Rite::builder("notify-success")
            .needs("resource::notification")
            .requires(AuthorityLevel::Pilgrim)
            .build(),
    );

    engine.register_rite(
        Rite::builder("notify-failure")
            .needs("resource::notification")
            .requires(AuthorityLevel::Pilgrim)
            .build(),
    );

    engine.register_rite(
        Rite::builder("rollback")
            .needs("target::host")
            .needs("resource::deploy")
            .requires(AuthorityLevel::Consul)
            .build(),
    );

    engine.register_rite(
        Rite::builder("record-deploy")
            .needs("resource::job-router")
            .requires(AuthorityLevel::Pilgrim)
            .build(),
    );

    // --- Nodes ---

    engine.add_node(
        SimNode::builder("sentinel-01")
            .interface("target::host::sentinel-01")
            .interface("resource::health-probe")
            .interface("resource::deploy")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    engine.add_node(
        SimNode::builder("oracle")
            .interface("resource::job-router")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    engine.add_node(
        SimNode::builder("notifier")
            .interface("resource::notification")
            .authority(AuthorityLevel::Pilgrim)
            .domain("gbe")
            .build(),
    );
}

fn main() {
    // =========================================================================
    // SCENARIO 1: Branch on health — healthy path
    // =========================================================================
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 1: Branch on health — healthy path               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        // Sentinel reports healthy.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![
                ("healthy".into(), Value::Bool(true)),
                ("detail".into(), Value::String("all good".into())),
            ],
        );

        let geas = Geas::builder("deploy-if-healthy")
            .requires(AuthorityLevel::Consul)
            .rite("check-health")
            .branch(vec![
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("healthy".into(), Value::Bool(true))]),
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "deploy-image".into(),
                        bindings: vec![],
                    })],
                },
                BranchArm {
                    pattern: YieldPattern::Wildcard,
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "notify-unhealthy".into(),
                        bindings: vec![],
                    })],
                },
            ])
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 2: Branch on health — unhealthy path
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 2: Branch on health — unhealthy path             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        // Sentinel reports unhealthy.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![
                ("healthy".into(), Value::Bool(false)),
                ("detail".into(), Value::String("disk full".into())),
            ],
        );

        let geas = Geas::builder("deploy-if-healthy")
            .requires(AuthorityLevel::Consul)
            .rite("check-health")
            .branch(vec![
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("healthy".into(), Value::Bool(true))]),
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "deploy-image".into(),
                        bindings: vec![],
                    })],
                },
                BranchArm {
                    pattern: YieldPattern::Wildcard,
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "notify-unhealthy".into(),
                        bindings: vec![],
                    })],
                },
            ])
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 3: Failure as yield shape
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 3: Failure as yield shape — deploy fails         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Deploy yields {{failed: true}}. The branch matches on that shape");
    println!("and routes to rollback + notify-failure. No special error mechanism.\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        // Deploy fails — yields failure shape.
        engine.mock_yield(
            "sentinel-01",
            "deploy-image",
            vec![
                ("deployed".into(), Value::Bool(false)),
                ("failed".into(), Value::Bool(true)),
                (
                    "reason".into(),
                    Value::String("image checksum mismatch".into()),
                ),
            ],
        );

        let geas = Geas::builder("deploy-with-recovery")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .branch(vec![
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("failed".into(), Value::Bool(true))]),
                    steps: vec![
                        ChainStep::Rite(RiteRef {
                            rite_name: "rollback".into(),
                            bindings: vec![],
                        }),
                        ChainStep::Rite(RiteRef {
                            rite_name: "notify-failure".into(),
                            bindings: vec![],
                        }),
                    ],
                },
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("deployed".into(), Value::Bool(true))]),
                    steps: vec![
                        ChainStep::Rite(RiteRef {
                            rite_name: "record-deploy".into(),
                            bindings: vec![],
                        }),
                        ChainStep::Rite(RiteRef {
                            rite_name: "notify-success".into(),
                            bindings: vec![],
                        }),
                    ],
                },
            ])
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 4: Failure as yield shape — deploy succeeds
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 4: Deploy succeeds — happy path                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        // Deploy succeeds.
        engine.mock_yield(
            "sentinel-01",
            "deploy-image",
            vec![
                ("deployed".into(), Value::Bool(true)),
                ("deploy_id".into(), Value::String("deploy-42".into())),
                ("failed".into(), Value::Bool(false)),
                ("reason".into(), Value::String("".into())),
            ],
        );

        let geas = Geas::builder("deploy-with-recovery")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .branch(vec![
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("failed".into(), Value::Bool(true))]),
                    steps: vec![
                        ChainStep::Rite(RiteRef {
                            rite_name: "rollback".into(),
                            bindings: vec![],
                        }),
                        ChainStep::Rite(RiteRef {
                            rite_name: "notify-failure".into(),
                            bindings: vec![],
                        }),
                    ],
                },
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("deployed".into(), Value::Bool(true))]),
                    steps: vec![
                        ChainStep::Rite(RiteRef {
                            rite_name: "record-deploy".into(),
                            bindings: vec![],
                        }),
                        ChainStep::Rite(RiteRef {
                            rite_name: "notify-success".into(),
                            bindings: vec![],
                        }),
                    ],
                },
            ])
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 5: No arm matches — unexpected yield
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 5: No arm matches — missing wildcard             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("The geas branches on health but the node yields something");
    println!("unexpected and there's no wildcard. The branch halts.\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        // Sentinel yields something unexpected — neither true nor false for healthy,
        // but a string (simulating a shape mismatch or unexpected value).
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![("healthy".into(), Value::String("degraded".into()))],
        );

        // No wildcard arm.
        let geas = Geas::builder("strict-branch")
            .rite("check-health")
            .branch(vec![
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("healthy".into(), Value::Bool(true))]),
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "deploy-image".into(),
                        bindings: vec![],
                    })],
                },
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("healthy".into(), Value::Bool(false))]),
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "notify-unhealthy".into(),
                        bindings: vec![],
                    })],
                },
            ])
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 6: No mock yields — branch has no values to match
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 6: No yield values — wildcard catches            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Rite completes but no mock yield configured. Wildcard matches.\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        // No mock yields — sentinel completes but produces no values.
        let geas = Geas::builder("no-values")
            .rite("check-health")
            .branch(vec![
                BranchArm {
                    pattern: YieldPattern::Fields(vec![("healthy".into(), Value::Bool(true))]),
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "deploy-image".into(),
                        bindings: vec![],
                    })],
                },
                BranchArm {
                    pattern: YieldPattern::Wildcard,
                    steps: vec![ChainStep::Rite(RiteRef {
                        rite_name: "notify-unhealthy".into(),
                        bindings: vec![],
                    })],
                },
            ])
            .build();

        print_trace(&engine.submit(&geas));
    }
}
