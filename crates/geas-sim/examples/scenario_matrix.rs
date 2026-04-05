//! Comprehensive scenario matrix for exploring geas behavior.
//!
//! Covers: discovery, matching, authority, condition,
//! yield chain validation, barriers (absorb/forward/translate),
//! and standing geas (imprints).

use frame::AuthorityLevel;
use geas_sim::geas::Binding;
use geas_sim::imprint::Imprint;
use geas_sim::*;

fn register_rites(engine: &mut SimEngine) {
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
            .build(),
    );

    engine.register_rite(
        Rite::builder("run-task")
            .needs("target::task")
            .needs("resource::execute")
            .requires(AuthorityLevel::Pilgrim)
            .param("task_type", ValueType::String)
            .yields_field("result", ValueType::String)
            .build(),
    );

    engine.register_rite(
        Rite::builder("sweep")
            .needs("resource::sweep")
            .requires(AuthorityLevel::Pilgrim)
            .yields_field("swept", ValueType::Integer)
            .build(),
    );

    engine.register_rite(
        Rite::builder("record-deploy")
            .needs("resource::job-router")
            .requires(AuthorityLevel::Pilgrim)
            .param("deploy_id", ValueType::String)
            .yields_field("recorded", ValueType::Bool)
            .build(),
    );

    engine.register_rite(
        Rite::builder("notify")
            .needs("resource::notification")
            .requires(AuthorityLevel::Pilgrim)
            .param("message", ValueType::String)
            .yields_field("sent", ValueType::Bool)
            .build(),
    );

    engine.register_rite(
        Rite::builder("drain-host")
            .needs("target::host")
            .needs("resource::drain")
            .requires(AuthorityLevel::Consul)
            .yields_field("drained", ValueType::Bool)
            .build(),
    );

    // Inner-only rites (operatives understand these, outer world doesn't).
    engine.register_rite(
        Rite::builder("prepare-filesystem")
            .needs("target::filesystem")
            .needs("resource::prepare")
            .requires(AuthorityLevel::Pilgrim)
            .yields_field("prepared", ValueType::Bool)
            .build(),
    );

    engine.register_rite(
        Rite::builder("start-process")
            .needs("target::process")
            .needs("resource::lifecycle")
            .requires(AuthorityLevel::Pilgrim)
            .yields_field("pid", ValueType::Integer)
            .build(),
    );
}

fn register_imprints(engine: &mut SimEngine) {
    engine.register_imprint(
        Imprint::builder("sentinel")
            .rite("check-health")
            .rite("deploy-image")
            .rite("drain-host")
            .authority(AuthorityLevel::Consul)
            .build(),
    );

    engine.register_imprint(
        Imprint::builder("oracle")
            .rite("record-deploy")
            .authority(AuthorityLevel::Consul)
            .build(),
    );

    engine.register_imprint(
        Imprint::builder("watcher")
            .rite("sweep")
            .authority(AuthorityLevel::Pilgrim)
            .build(),
    );

    engine.register_imprint(Imprint::builder("operative").rite("run-task").build());

    engine.register_imprint(
        Imprint::builder("operative-full")
            .rite("run-task")
            .rite("prepare-filesystem")
            .rite("start-process")
            .build(),
    );
}

/// Build the standard test network.
fn build_network(engine: &mut SimEngine) {
    engine.add_node(
        SimNode::builder("sentinel-01")
            .interface("target::host::sentinel-01")
            .interface("resource::health-probe")
            .interface("resource::deploy")
            .interface("resource::drain")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    engine.add_node(
        SimNode::builder("sentinel-02")
            .interface("target::host::sentinel-02")
            .interface("resource::health-probe")
            .interface("resource::drain")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    engine.add_node(
        SimNode::builder("oracle")
            .interface("resource::job-router")
            .interface("resource::dag-planner")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    engine.add_node(
        SimNode::builder("watcher")
            .interface("resource::sweep")
            .interface("resource::dead-letter")
            .authority(AuthorityLevel::Pilgrim)
            .domain("gbe")
            .build(),
    );

    // Operatives behind sentinel-01.
    engine.add_node(
        SimNode::builder("operative-01")
            .interface("target::task")
            .interface("resource::execute")
            .interface("target::filesystem")
            .interface("resource::prepare")
            .interface("target::process")
            .interface("resource::lifecycle")
            .authority(AuthorityLevel::Pilgrim)
            .domain("vm-sentinel-01")
            .build(),
    );
    engine.add_node(
        SimNode::builder("operative-02")
            .interface("target::task")
            .interface("resource::execute")
            .authority(AuthorityLevel::Pilgrim)
            .domain("vm-sentinel-01")
            .build(),
    );

    // Sentinel-01 as barrier with crossing rules.
    engine.add_barrier(
        Barrier::builder("sentinel-01")
            .outer_domain("gbe")
            .inner_domain("vm-sentinel-01")
            .authority(AuthorityLevel::Consul)
            .forwards("run-task")
            .translates("deploy-image", vec!["prepare-filesystem", "start-process"])
            .absorbs("check-health")
            .absorbs("drain-host")
            .build(),
    );
}

fn main() {
    // =========================================================================
    // SECTION 1: DISCOVERY
    // =========================================================================
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  SECTION 1: DISCOVERY                                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // 1a: Rite finds nodes by interface shape.
    println!("\n=== 1a: Basic discovery — rite finds matching nodes ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);

        let g = Geas::builder("discover-hosts").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 1b: Empty network.
    println!("\n=== 1b: Empty network — nothing to discover ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        let g = Geas::builder("discover-nothing")
            .rite("check-health")
            .build();
        print_trace(&engine.submit(&g));
    }

    // 1c: Unknown rite.
    println!("\n=== 1c: Unknown rite — not in registry ===");
    {
        let mut engine = SimEngine::new();
        build_network(&mut engine);

        let g = Geas::builder("missing-rite")
            .rite("nonexistent-rite")
            .build();
        print_trace(&engine.submit(&g));
    }

    // 1d: Partial interface coverage.
    println!("\n=== 1d: Partial match — node has some but not all interfaces ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        engine.add_node(
            SimNode::builder("oracle")
                .interface("resource::dag-planner")
                .interface("resource::job-router")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );

        let g = Geas::builder("partial-test").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // =========================================================================
    // SECTION 2: MATCHING
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SECTION 2: MATCHING                                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // 2a: Exact match.
    println!("\n=== 2a: Exact match ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(
            SimNode::builder("precise-node")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );
        let g = Geas::builder("exact").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 2b: Prefix match.
    println!("\n=== 2b: Prefix match — node more specific than need ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(
            SimNode::builder("sentinel-07")
                .interface("target::host::rack-3::sentinel-07")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );
        let g = Geas::builder("prefix").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 2c: Fan-out.
    println!("\n=== 2c: Fan-out — multiple nodes match ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);
        let g = Geas::builder("fan-out").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 2d: Selective match.
    println!("\n=== 2d: Selective — only nodes with resource::deploy match ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);
        let g = Geas::builder("selective")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        print_trace(&engine.submit(&g));
    }

    // =========================================================================
    // SECTION 3: AUTHORITY
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SECTION 3: AUTHORITY                                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // 3a: Precept within scope.
    println!("\n=== 3a: Precept — pilgrim issues check-health (within scope) ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);
        let g = Geas::builder("precept-health")
            .requires(AuthorityLevel::Pilgrim)
            .rite("check-health")
            .build();
        print_trace(&engine.submit(&g));
    }

    // 3b: Precept exceeds scope.
    println!("\n=== 3b: Precept exceeds scope — pilgrim tries deploy (Consul required) ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(
            SimNode::builder("pilgrim-sentinel")
                .interface("target::host")
                .interface("resource::deploy")
                .authority(AuthorityLevel::Pilgrim)
                .domain("gbe")
                .build(),
        );
        let g = Geas::builder("precept-deploy").rite("deploy-image").build();
        print_trace(&engine.submit(&g));
    }

    // 3c: Dictum flows through.
    println!("\n=== 3c: Dictum — consul issues deploy ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);
        let g = Geas::builder("dictum-deploy")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        print_trace(&engine.submit(&g));
    }

    // 3d: Authority at barrier — consul outside, pilgrim inside.
    println!("\n=== 3d: Authority at barrier — forward to pilgrim operatives ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        engine.add_node(
            SimNode::builder("operative-inner")
                .interface("target::task")
                .interface("resource::execute")
                .authority(AuthorityLevel::Pilgrim)
                .domain("vm-inner")
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-barrier")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );

        engine.add_barrier(
            Barrier::builder("sentinel-barrier")
                .outer_domain("gbe")
                .inner_domain("vm-inner")
                .authority(AuthorityLevel::Consul)
                .forwards("run-task")
                .build(),
        );

        let g = Geas::builder("barrier-authority").rite("run-task").build();
        print_trace(&engine.submit(&g));
    }

    // 3e: Consul rite blocked at inner boundary.
    println!("\n=== 3e: Consul rite blocked inside — pilgrim operatives can't act ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        engine.add_node(
            SimNode::builder("operative-low")
                .interface("target::host")
                .interface("resource::deploy")
                .authority(AuthorityLevel::Pilgrim)
                .domain("vm-locked")
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-locked")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );

        engine.add_barrier(
            Barrier::builder("sentinel-locked")
                .outer_domain("gbe")
                .inner_domain("vm-locked")
                .authority(AuthorityLevel::Consul)
                .forwards("deploy-image") // Forwarding consul rite to pilgrim nodes.
                .build(),
        );

        let g = Geas::builder("inner-authority-fail")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        print_trace(&engine.submit(&g));
    }

    // =========================================================================
    // SECTION 4: MATCH-BUT-WRONG
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SECTION 4: MATCH-BUT-WRONG                                ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // 4a: Draining.
    println!("\n=== 4a: Node draining — interfaces match, condition blocks ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(
            SimNode::builder("sentinel-draining")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .condition(NodeCondition::Draining)
                .build(),
        );
        let g = Geas::builder("drain-test").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 4b: Offline.
    println!("\n=== 4b: Node offline ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(
            SimNode::builder("sentinel-offline")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .condition(NodeCondition::Offline)
                .build(),
        );
        let g = Geas::builder("offline-test").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 4c: Degraded still acts.
    println!("\n=== 4c: Degraded — matches and acts ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(
            SimNode::builder("sentinel-degraded")
                .interface("target::host")
                .interface("resource::health-probe")
                .authority(AuthorityLevel::Consul)
                .condition(NodeCondition::Degraded)
                .build(),
        );
        let g = Geas::builder("degraded-test").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 4d: Barrier offline.
    println!("\n=== 4d: Barrier offline — inner nodes unreachable ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        engine.add_node(
            SimNode::builder("operative-stranded")
                .interface("target::task")
                .interface("resource::execute")
                .authority(AuthorityLevel::Pilgrim)
                .domain("vm-dead")
                .build(),
        );

        engine.add_node(
            SimNode::builder("sentinel-dead")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );

        engine.add_barrier(
            Barrier::builder("sentinel-dead")
                .outer_domain("gbe")
                .inner_domain("vm-dead")
                .authority(AuthorityLevel::Consul)
                .forwards("run-task")
                .condition(NodeCondition::Offline)
                .build(),
        );

        let g = Geas::builder("barrier-dead").rite("run-task").build();
        print_trace(&engine.submit(&g));
    }

    // 4e: Yield chain break.
    println!("\n=== 4e: Yield chain break — deploy yields lack 'message' for notify ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);

        engine.add_node(
            SimNode::builder("notifier")
                .interface("resource::notification")
                .authority(AuthorityLevel::Pilgrim)
                .domain("gbe")
                .build(),
        );

        let g = Geas::builder("broken-chain")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .rite_with(
                "notify",
                vec![Binding::Yield {
                    rite: "deploy-image".into(),
                    field: "message".into(), // doesn't exist!
                    target: "message".into(),
                }],
            )
            .build();
        print_trace(&engine.submit(&g));
    }

    // 4f: Yield chain valid.
    println!("\n=== 4f: Yield chain valid — deploy -> record-deploy ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);

        let g = Geas::builder("valid-chain")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .rite_with(
                "record-deploy",
                vec![Binding::Yield {
                    rite: "deploy-image".into(),
                    field: "deploy_id".into(),
                    target: "deploy_id".into(),
                }],
            )
            .build();
        print_trace(&engine.submit(&g));
    }

    // =========================================================================
    // SECTION 5: BARRIER CROSSING MODES
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SECTION 5: BARRIER CROSSING MODES                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // 5a: Absorb — sentinel handles check-health itself.
    println!("\n=== 5a: Absorb — sentinel handles check-health, nothing goes inward ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);

        let g = Geas::builder("absorb-test").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 5b: Forward — sentinel passes run-task to operatives.
    println!("\n=== 5b: Forward — sentinel passes run-task unchanged to operatives ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);

        let g = Geas::builder("forward-test").rite("run-task").build();
        print_trace(&engine.submit(&g));
    }

    // 5c: Translate — sentinel turns deploy-image into inner rites.
    println!("\n=== 5c: Translate — deploy-image becomes prepare-filesystem + start-process ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        build_network(&mut engine);

        let g = Geas::builder("translate-test")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        print_trace(&engine.submit(&g));
    }

    // 5d: No crossing rule — default absorb.
    println!("\n=== 5d: No crossing rule for drain-host — default absorb ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        engine.add_node(
            SimNode::builder("sentinel-minimal")
                .interface("target::host")
                .interface("resource::drain")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );

        engine.add_node(
            SimNode::builder("op")
                .interface("target::host")
                .interface("resource::drain")
                .authority(AuthorityLevel::Pilgrim)
                .domain("vm-min")
                .build(),
        );

        engine.add_barrier(
            Barrier::builder("sentinel-minimal")
                .outer_domain("gbe")
                .inner_domain("vm-min")
                .authority(AuthorityLevel::Consul)
                // No crossing rules at all.
                .build(),
        );

        let g = Geas::builder("default-absorb")
            .requires(AuthorityLevel::Consul)
            .rite("drain-host")
            .build();
        print_trace(&engine.submit(&g));
    }

    // 5e: Translate to unknown inner rite.
    println!("\n=== 5e: Translate references unknown inner rite ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);

        engine.add_node(
            SimNode::builder("sentinel-bad")
                .interface("target::host")
                .interface("resource::deploy")
                .authority(AuthorityLevel::Consul)
                .domain("gbe")
                .build(),
        );

        engine.add_barrier(
            Barrier::builder("sentinel-bad")
                .outer_domain("gbe")
                .inner_domain("vm-bad")
                .authority(AuthorityLevel::Consul)
                .translates("deploy-image", vec!["nonexistent-inner-rite"])
                .build(),
        );

        let g = Geas::builder("bad-translate")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        print_trace(&engine.submit(&g));
    }

    // =========================================================================
    // SECTION 6: STANDING GEAS / IMPRINTS
    // =========================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SECTION 6: STANDING GEAS / IMPRINTS                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // 6a: Bare node.
    println!("\n=== 6a: Bare node — no imprint, no match ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        engine.add_node(SimNode::builder("bare-node").domain("gbe").build());
        let g = Geas::builder("bare-test").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 6b: Imprint makes node visible.
    println!("\n=== 6b: Imprint bare node as sentinel — interfaces derived ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        register_imprints(&mut engine);

        engine.add_node(SimNode::builder("bare-node").domain("gbe").build());

        println!("  Before imprint:");
        let g = Geas::builder("before").rite("check-health").build();
        print_trace(&engine.submit(&g));

        println!("\n  Imprinting...");
        print_trace(&engine.imprint_node("bare-node", "sentinel"));

        println!("\n  After imprint:");
        let g = Geas::builder("after").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 6c: Same imprint, fan-out.
    println!("\n=== 6c: Same imprint on two nodes — fan-out ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        register_imprints(&mut engine);

        engine.add_node(SimNode::builder("node-a").domain("gbe").build());
        engine.add_node(SimNode::builder("node-b").domain("gbe").build());
        print_trace(&engine.imprint_node("node-a", "sentinel"));
        print_trace(&engine.imprint_node("node-b", "sentinel"));

        let g = Geas::builder("twin-sentinels").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 6d: Different imprints, different shapes.
    println!("\n=== 6d: Different imprints — each matches only its rites ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        register_imprints(&mut engine);

        engine.add_node(SimNode::builder("node-s").domain("gbe").build());
        engine.add_node(SimNode::builder("node-o").domain("gbe").build());
        engine.add_node(SimNode::builder("node-w").domain("gbe").build());

        print_trace(&engine.imprint_node("node-s", "sentinel"));
        print_trace(&engine.imprint_node("node-o", "oracle"));
        print_trace(&engine.imprint_node("node-w", "watcher"));

        println!("\n  check-health:");
        let g = Geas::builder("who-health").rite("check-health").build();
        print_trace(&engine.submit(&g));

        println!("\n  sweep:");
        let g = Geas::builder("who-sweeps").rite("sweep").build();
        print_trace(&engine.submit(&g));

        println!("\n  record-deploy:");
        let g = Geas::builder("who-records").rite("record-deploy").build();
        print_trace(&engine.submit(&g));
    }

    // 6e: Re-imprint.
    println!("\n=== 6e: Re-imprint — watcher becomes sentinel ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        register_imprints(&mut engine);

        engine.add_node(SimNode::builder("flex").domain("gbe").build());

        print_trace(&engine.imprint_node("flex", "watcher"));
        let g = Geas::builder("as-watcher").rite("sweep").build();
        print_trace(&engine.submit(&g));

        println!("\n  Re-imprint as sentinel:");
        print_trace(&engine.imprint_node("flex", "sentinel"));

        let g = Geas::builder("lost-sweep").rite("sweep").build();
        print_trace(&engine.submit(&g));

        let g = Geas::builder("gained-health").rite("check-health").build();
        print_trace(&engine.submit(&g));
    }

    // 6f: Operative imprinted behind barrier with crossing rules.
    println!("\n=== 6f: Operative behind barrier — forward and translate ===");
    {
        let mut engine = SimEngine::new();
        register_rites(&mut engine);
        register_imprints(&mut engine);

        engine.add_node(SimNode::builder("bare-operative").domain("vm-sx").build());
        print_trace(&engine.imprint_node("bare-operative", "operative-full"));

        engine.add_node(SimNode::builder("sentinel-x").domain("gbe").build());
        print_trace(&engine.imprint_node("sentinel-x", "sentinel"));

        engine.add_barrier(
            Barrier::builder("sentinel-x")
                .outer_domain("gbe")
                .inner_domain("vm-sx")
                .authority(AuthorityLevel::Consul)
                .forwards("run-task")
                .translates("deploy-image", vec!["prepare-filesystem", "start-process"])
                .absorbs("check-health")
                .build(),
        );

        println!("\n  run-task (forward):");
        let g = Geas::builder("forward-to-op").rite("run-task").build();
        print_trace(&engine.submit(&g));

        println!("\n  deploy-image (translate):");
        let g = Geas::builder("translate-for-op")
            .requires(AuthorityLevel::Consul)
            .rite("deploy-image")
            .build();
        print_trace(&engine.submit(&g));

        println!("\n  check-health (absorb — sentinel handles it):");
        let g = Geas::builder("absorb-at-sentinel")
            .rite("check-health")
            .build();
        print_trace(&engine.submit(&g));
    }
}
