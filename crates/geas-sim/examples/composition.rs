//! Sub-geas composition scenarios.
//!
//! Proves: a geas IS a rite. It can be invoked as a step inside another geas.
//! The sub-geas runs its full chain in the same network context.

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
            .build(),
    );

    engine.register_rite(
        Rite::builder("deploy-image")
            .needs("target::host")
            .needs("resource::deploy")
            .requires(AuthorityLevel::Consul)
            .yields_field("deployed", ValueType::Bool)
            .yields_field("deploy_id", ValueType::String)
            .build(),
    );

    engine.register_rite(
        Rite::builder("verify-deploy")
            .needs("target::host")
            .needs("resource::health-probe")
            .requires(AuthorityLevel::Pilgrim)
            .yields_field("verified", ValueType::Bool)
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

    engine.register_rite(
        Rite::builder("audit-log")
            .needs("resource::audit")
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

    engine.add_node(
        SimNode::builder("auditor")
            .interface("resource::audit")
            .authority(AuthorityLevel::Pilgrim)
            .domain("gbe")
            .build(),
    );
}

fn main() {
    // =========================================================================
    // SCENARIO 1: Simple sub-geas — deploy-safe used inside full-deploy
    // =========================================================================
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 1: Sub-geas — deploy-safe inside full-deploy     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("deploy-safe = check-health -> deploy-image -> verify-deploy");
    println!("full-deploy = deploy-safe -> record-deploy -> notify-success\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        engine.default_yield(
            "check-health",
            vec![("healthy".into(), Value::Bool(true))],
        );
        engine.default_yield(
            "deploy-image",
            vec![
                ("deployed".into(), Value::Bool(true)),
                ("deploy_id".into(), Value::String("deploy-99".into())),
            ],
        );
        engine.default_yield(
            "verify-deploy",
            vec![("verified".into(), Value::Bool(true))],
        );

        // Register deploy-safe as a reusable geas.
        engine.register_geas(
            Geas::builder("deploy-safe")
                .requires(AuthorityLevel::Consul)
                .rite("check-health")
                .rite("deploy-image")
                .rite("verify-deploy")
                .build(),
        );

        // full-deploy invokes deploy-safe, then continues.
        let geas = Geas::builder("full-deploy")
            .requires(AuthorityLevel::Consul)
            .sub_geas("deploy-safe")
            .rite("record-deploy")
            .rite("notify-success")
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 2: Sub-geas with branching inside
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 2: Sub-geas with internal branching              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("deploy-with-recovery = deploy-image -> match {{");
    println!("  {{deployed: true}} -> verify-deploy");
    println!("  _                -> rollback");
    println!("}}\n");
    println!("full-pipeline = check-health -> deploy-with-recovery -> audit-log\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        engine.default_yield(
            "check-health",
            vec![("healthy".into(), Value::Bool(true))],
        );
        engine.default_yield(
            "deploy-image",
            vec![
                ("deployed".into(), Value::Bool(true)),
                ("deploy_id".into(), Value::String("deploy-100".into())),
            ],
        );

        // Sub-geas with internal branching.
        engine.register_geas(Geas {
            name: "deploy-with-recovery".into(),
            requires: AuthorityLevel::Consul,
            params: vec![],
            chain: vec![
                ChainStep::Rite(RiteRef {
                    rite_name: "deploy-image".into(),
                    bindings: vec![],
                }),
                ChainStep::Branch {
                    arms: vec![
                        BranchArm {
                            pattern: YieldPattern::Fields(vec![(
                                "deployed".into(),
                                Value::Bool(true),
                            )]),
                            steps: vec![ChainStep::Rite(RiteRef {
                                rite_name: "verify-deploy".into(),
                                bindings: vec![],
                            })],
                        },
                        BranchArm {
                            pattern: YieldPattern::Wildcard,
                            steps: vec![ChainStep::Rite(RiteRef {
                                rite_name: "rollback".into(),
                                bindings: vec![],
                            })],
                        },
                    ],
                },
            ],
        });

        // Parent geas invokes the branching sub-geas.
        let geas = Geas::builder("full-pipeline")
            .requires(AuthorityLevel::Consul)
            .rite("check-health")
            .sub_geas("deploy-with-recovery")
            .rite("audit-log")
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 3: Sub-geas with failure — rollback branch taken
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 3: Sub-geas failure path — rollback taken        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Same deploy-with-recovery, but deploy-image fails.\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        engine.default_yield(
            "check-health",
            vec![("healthy".into(), Value::Bool(true))],
        );
        // Deploy fails this time.
        engine.default_yield(
            "deploy-image",
            vec![("deployed".into(), Value::Bool(false))],
        );

        engine.register_geas(Geas {
            name: "deploy-with-recovery".into(),
            requires: AuthorityLevel::Consul,
            params: vec![],
            chain: vec![
                ChainStep::Rite(RiteRef {
                    rite_name: "deploy-image".into(),
                    bindings: vec![],
                }),
                ChainStep::Branch {
                    arms: vec![
                        BranchArm {
                            pattern: YieldPattern::Fields(vec![(
                                "deployed".into(),
                                Value::Bool(true),
                            )]),
                            steps: vec![ChainStep::Rite(RiteRef {
                                rite_name: "verify-deploy".into(),
                                bindings: vec![],
                            })],
                        },
                        BranchArm {
                            pattern: YieldPattern::Wildcard,
                            steps: vec![ChainStep::Rite(RiteRef {
                                rite_name: "rollback".into(),
                                bindings: vec![],
                            })],
                        },
                    ],
                },
            ],
        });

        let geas = Geas::builder("full-pipeline")
            .requires(AuthorityLevel::Consul)
            .rite("check-health")
            .sub_geas("deploy-with-recovery")
            .rite("audit-log")
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 4: Nested sub-geas — geas inside geas inside geas
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 4: Nested — geas inside geas inside geas         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("inner = check-health -> verify-deploy");
    println!("middle = inner -> deploy-image");
    println!("outer = middle -> record-deploy -> notify-success\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        engine.default_yield(
            "check-health",
            vec![("healthy".into(), Value::Bool(true))],
        );
        engine.default_yield(
            "deploy-image",
            vec![("deployed".into(), Value::Bool(true))],
        );
        engine.default_yield(
            "verify-deploy",
            vec![("verified".into(), Value::Bool(true))],
        );

        engine.register_geas(
            Geas::builder("inner")
                .rite("check-health")
                .rite("verify-deploy")
                .build(),
        );

        engine.register_geas(
            Geas::builder("middle")
                .requires(AuthorityLevel::Consul)
                .sub_geas("inner")
                .rite("deploy-image")
                .build(),
        );

        let geas = Geas::builder("outer")
            .requires(AuthorityLevel::Consul)
            .sub_geas("middle")
            .rite("record-deploy")
            .rite("notify-success")
            .build();

        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 5: Unknown sub-geas
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 5: Unknown sub-geas — not in registry            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    {
        let mut engine = SimEngine::new();
        setup(&mut engine);

        let geas = Geas::builder("broken")
            .rite("check-health")
            .sub_geas("nonexistent-geas")
            .rite("notify-success") // should not execute
            .build();

        print_trace(&engine.submit(&geas));
    }
}
