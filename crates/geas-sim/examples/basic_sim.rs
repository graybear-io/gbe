use frame::AuthorityLevel;
use geas_sim::*;

fn main() {
    let mut engine = SimEngine::new();

    // --- Register rites ---

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

    // --- Build the network ---

    // Two sentinels on different hosts.
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
        SimNode::builder("sentinel-02")
            .interface("target::host::sentinel-02")
            .interface("resource::health-probe")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    // Oracle.
    engine.add_node(
        SimNode::builder("oracle")
            .interface("resource::job-router")
            .interface("resource::dag-planner")
            .authority(AuthorityLevel::Consul)
            .domain("gbe")
            .build(),
    );

    // Watcher.
    engine.add_node(
        SimNode::builder("watcher")
            .interface("resource::sweep")
            .interface("resource::dead-letter")
            .authority(AuthorityLevel::Pilgrim)
            .domain("gbe")
            .build(),
    );

    // Operative behind sentinel-01's barrier.
    engine.add_node(
        SimNode::builder("operative-01")
            .interface("target::task")
            .interface("resource::execute")
            .authority(AuthorityLevel::Pilgrim)
            .domain("vm-sentinel-01")
            .build(),
    );

    // Sentinel-01 is a barrier: forwards run-task, absorbs everything else.
    engine.add_barrier(
        Barrier::builder("sentinel-01")
            .outer_domain("gbe")
            .inner_domain("vm-sentinel-01")
            .authority(AuthorityLevel::Consul)
            .forwards("run-task")
            .build(),
    );

    // --- Scenarios ---

    println!("=== Scenario 1: Health check (fan-out) ===");
    println!("Sentinel absorbs — it handles health checks itself.\n");
    let g = Geas::builder("health-sweep")
        .requires(AuthorityLevel::Pilgrim)
        .rite("check-health")
        .build();
    print_trace(&engine.submit(&g));

    println!("\n=== Scenario 2: Deploy image (selective match) ===");
    println!("Only sentinel-01 has resource::deploy.\n");
    let g = Geas::builder("deploy-staging")
        .requires(AuthorityLevel::Consul)
        .rite("deploy-image")
        .build();
    print_trace(&engine.submit(&g));

    println!("\n=== Scenario 3: Run task (barrier forwards) ===");
    println!("Sentinel forwards run-task to operatives behind it.\n");
    let g = Geas::builder("run-job")
        .requires(AuthorityLevel::Pilgrim)
        .rite("run-task")
        .build();
    print_trace(&engine.submit(&g));

    println!("\n=== Scenario 4: Sweep (single match) ===");
    println!("Only watcher has resource::sweep.\n");
    let g = Geas::builder("maintenance-sweep")
        .requires(AuthorityLevel::Pilgrim)
        .rite("sweep")
        .build();
    print_trace(&engine.submit(&g));
}
