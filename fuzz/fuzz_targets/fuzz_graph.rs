#![no_main]
use libfuzzer_sys::fuzz_target;

// Adversarial component-graph shapes (#689): building and querying a
// `ComponentGraph` over arbitrary bytes — malformed DIRM directories, dangling
// or cyclic INCL edges, self-including components, duplicate identities — must
// never panic, hang, or overflow the stack. Every traversal is resource-bounded
// (visit + depth caps), so this target proves those bounds hold on hostile input.
fuzz_target!(|data: &[u8]| {
    let Ok(graph) = djvu_rs::ComponentGraph::parse(data) else {
        return;
    };

    // Validation walks the INCL graph with bounded cycle/depth detection.
    let _ = graph.validate();
    // Reachability from pages is a bounded traversal.
    let _ = graph.unreachable_components();

    // Per-node edge queries must stay in bounds and never index out of range.
    for node in graph.nodes() {
        let _ = graph.includes(&node.id);
        let _ = graph.included_by(&node.id);
    }

    // Transitive closure with every node as a root exercises the widest
    // fan-out; it is capped by the same visit budget.
    let roots: Vec<&str> = graph.nodes().iter().map(|node| node.id.as_str()).collect();
    let _ = graph.transitive_closure(&roots);
});
