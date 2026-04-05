//! Simulation honesty checks.
//!
//! These scenarios expose the sim's known limitations:
//! - Mock yields that violate the rite's declared shape
//! - Fan-out divergence where different nodes yield different values

use frame::AuthorityLevel;
use geas_sim::geas::RiteRef;
use geas_sim::*;

fn main() {
    // =========================================================================
    // SCENARIO 1: Mock yield violates declared shape
    // =========================================================================
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 1: Mock yield violates declared shape            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("check-health declares yields {{healthy: bool, detail: string}}");
    println!("but mock produces {{status: string}} — wrong field names.\n");
    {
        let mut engine = SimEngine::new();

        engine.register_rite(
            Rite::builder("check-health")
                .needs("target::host")
                .needs("resource::health-probe")
                .requires(AuthorityLevel::Pilgrim)
                .yields_field("healthy", ValueType::Bool)
                .yields_field("detail", ValueType::String)
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-01")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );

        // Mock yields a field that doesn't exist in the shape.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![("status".into(), Value::String("ok".into()))],
        );

        let geas = Geas::builder("shape-violation")
            .rite("check-health")
            .build();
        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 2: Mock yield has wrong type
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 2: Mock yield has wrong type                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("check-health declares healthy: bool but mock yields healthy: string.\n");
    {
        let mut engine = SimEngine::new();

        engine.register_rite(
            Rite::builder("check-health")
                .needs("target::host")
                .needs("resource::health-probe")
                .requires(AuthorityLevel::Pilgrim)
                .yields_field("healthy", ValueType::Bool)
                .yields_field("detail", ValueType::String)
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-01")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );

        // Type mismatch: healthy should be Bool, not String.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![
                ("healthy".into(), Value::String("yes".into())),
                ("detail".into(), Value::String("all good".into())),
            ],
        );

        let geas = Geas::builder("type-violation").rite("check-health").build();
        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 3: Valid mock — no warnings
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 3: Valid mock — no warnings                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    {
        let mut engine = SimEngine::new();

        engine.register_rite(
            Rite::builder("check-health")
                .needs("target::host")
                .needs("resource::health-probe")
                .requires(AuthorityLevel::Pilgrim)
                .yields_field("healthy", ValueType::Bool)
                .yields_field("detail", ValueType::String)
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-01")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );

        // Correct shape.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![
                ("healthy".into(), Value::Bool(true)),
                ("detail".into(), Value::String("all good".into())),
            ],
        );

        let geas = Geas::builder("valid-mock").rite("check-health").build();
        print_trace(&engine.submit(&geas));
    }

    // =========================================================================
    // SCENARIO 4: Fan-out divergence — two sentinels yield different values
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 4: Fan-out divergence                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Two sentinels match check-health. One is healthy, one isn't.");
    println!("The branch only sees the first node's yields — this is a gap.\n");
    {
        let mut engine = SimEngine::new();

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
                .build(),
        );

        engine.register_rite(
            Rite::builder("notify-unhealthy")
                .needs("resource::notification")
                .requires(AuthorityLevel::Pilgrim)
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-01")
                .interface("target::host::s01")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );
        engine.add_node(
            SimNode::builder("sentinel-02")
                .interface("target::host::s02")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );
        engine.add_node(
            SimNode::builder("notifier")
                .interface("resource::notification")
                .authority(AuthorityLevel::Pilgrim)
                .build(),
        );

        // sentinel-01 is healthy, sentinel-02 is not.
        engine.mock_yield(
            "sentinel-01",
            "check-health",
            vec![
                ("healthy".into(), Value::Bool(true)),
                ("detail".into(), Value::String("all good".into())),
            ],
        );
        engine.mock_yield(
            "sentinel-02",
            "check-health",
            vec![
                ("healthy".into(), Value::Bool(false)),
                ("detail".into(), Value::String("disk full".into())),
            ],
        );

        // Branch on health — but which sentinel's result does the branch see?
        let geas = Geas::builder("divergent-fanout")
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

        println!("\n  ^ The branch took the healthy path because it only saw");
        println!("    sentinel-01's yields. sentinel-02's unhealthy result");
        println!("    was silently ignored. In a real system, this fan-out");
        println!("    would need per-node branching or result aggregation.");
    }

    // =========================================================================
    // SCENARIO 5: Fan-out agreement — no divergence warning
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 5: Fan-out agreement — no divergence             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Both sentinels yield the same values. No warning.\n");
    {
        let mut engine = SimEngine::new();

        engine.register_rite(
            Rite::builder("check-health")
                .needs("target::host")
                .needs("resource::health-probe")
                .requires(AuthorityLevel::Pilgrim)
                .yields_field("healthy", ValueType::Bool)
                .yields_field("detail", ValueType::String)
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-01")
                .interface("target::host::s01")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );
        engine.add_node(
            SimNode::builder("sentinel-02")
                .interface("target::host::s02")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .build(),
        );

        // Both healthy.
        engine.default_yield(
            "check-health",
            vec![
                ("healthy".into(), Value::Bool(true)),
                ("detail".into(), Value::String("all good".into())),
            ],
        );

        let geas = Geas::builder("agreed-fanout").rite("check-health").build();
        print_trace(&engine.submit(&geas));
    }
}
