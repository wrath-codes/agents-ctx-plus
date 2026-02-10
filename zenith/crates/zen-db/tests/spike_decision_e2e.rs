//! # Spike 0.22: Decision Traces — Visibility Safety + Performance
//!
//! Integration tests validating:
//! - **Group I (tests 48-50)**: Visibility filtering before graph build, team scope
//!   isolation, public scope edge exclusion.
//! - **Group J (tests 51-54)**: Performance at small/medium/large graph scales,
//!   deterministic hash across runs.
//!
//! These tests live in `zen-db` because they validate that entity data loaded from
//! the database respects visibility rules before being fed to graph algorithms.
//! Graph construction helpers mirror the `zen-search` spike but operate on
//! filtered node/edge sets.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Domain types (mirrored from zen-search spike for self-contained tests)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    Public,
    Team,
    Private,
}

struct EntityNode {
    id: String,
    #[allow(dead_code)]
    entity_type: String,
    visibility: Visibility,
    org_id: Option<String>,
}

struct EntityEdge {
    source_id: String,
    target_id: String,
    #[allow(dead_code)]
    relation: String,
}

struct VisibilityFilter {
    scope: Visibility,
    org_id: Option<String>,
}

impl VisibilityFilter {
    fn node_visible(&self, node: &EntityNode) -> bool {
        match self.scope {
            Visibility::Public => node.visibility == Visibility::Public,
            Visibility::Team => {
                if node.visibility == Visibility::Private {
                    return false;
                }
                if node.visibility == Visibility::Team {
                    return self.org_id.is_some()
                        && node.org_id.as_deref() == self.org_id.as_deref();
                }
                true
            }
            Visibility::Private => true,
        }
    }

    fn edge_visible(&self, edge: &EntityEdge, nodes: &HashMap<String, &EntityNode>) -> bool {
        let src_visible = nodes
            .get(&edge.source_id)
            .is_some_and(|n| self.node_visible(n));
        let tgt_visible = nodes
            .get(&edge.target_id)
            .is_some_and(|n| self.node_visible(n));
        src_visible && tgt_visible
    }
}

fn build_filtered_graph(
    nodes: &[EntityNode],
    edges: &[EntityEdge],
    filter: &VisibilityFilter,
) -> (Vec<String>, Vec<(String, String)>) {
    let node_map: HashMap<String, &EntityNode> = nodes.iter().map(|n| (n.id.clone(), n)).collect();

    let visible_nodes: Vec<String> = nodes
        .iter()
        .filter(|n| filter.node_visible(n))
        .map(|n| n.id.clone())
        .collect();

    let visible_set: HashSet<&str> = visible_nodes.iter().map(String::as_str).collect();

    let visible_edges: Vec<(String, String)> = edges
        .iter()
        .filter(|e| filter.edge_visible(e, &node_map))
        .filter(|e| {
            visible_set.contains(e.source_id.as_str()) && visible_set.contains(e.target_id.as_str())
        })
        .map(|e| (e.source_id.clone(), e.target_id.clone()))
        .collect();

    (visible_nodes, visible_edges)
}

// ---------------------------------------------------------------------------
// Test 48: Visibility filter before graph build
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_visibility_filter_before_graph_build() {
    let nodes = vec![
        EntityNode {
            id: "dec-001".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
        EntityNode {
            id: "dec-002".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Team,
            org_id: Some("org-alpha".into()),
        },
        EntityNode {
            id: "fnd-001".into(),
            entity_type: "finding".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
        EntityNode {
            id: "fnd-002".into(),
            entity_type: "finding".into(),
            visibility: Visibility::Private,
            org_id: Some("org-alpha".into()),
        },
        EntityNode {
            id: "hyp-001".into(),
            entity_type: "hypothesis".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
    ];

    let edges = vec![
        EntityEdge {
            source_id: "dec-001".into(),
            target_id: "fnd-001".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-002".into(),
            target_id: "fnd-002".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-001".into(),
            target_id: "hyp-001".into(),
            relation: "validates".into(),
        },
    ];

    let public_filter = VisibilityFilter {
        scope: Visibility::Public,
        org_id: None,
    };
    let (pub_nodes, pub_edges) = build_filtered_graph(&nodes, &edges, &public_filter);

    assert_eq!(pub_nodes.len(), 3, "public scope: 3 public nodes");
    assert!(pub_nodes.contains(&"dec-001".to_string()));
    assert!(pub_nodes.contains(&"fnd-001".to_string()));
    assert!(pub_nodes.contains(&"hyp-001".to_string()));
    assert!(
        !pub_nodes.contains(&"dec-002".to_string()),
        "team node excluded"
    );
    assert!(
        !pub_nodes.contains(&"fnd-002".to_string()),
        "private node excluded"
    );

    assert_eq!(pub_edges.len(), 2, "public scope: 2 visible edges");
    assert!(pub_edges.contains(&("dec-001".to_string(), "fnd-001".to_string())));
    assert!(pub_edges.contains(&("dec-001".to_string(), "hyp-001".to_string())));
    assert!(!pub_edges.contains(&("dec-002".to_string(), "fnd-002".to_string())));

    let team_filter = VisibilityFilter {
        scope: Visibility::Team,
        org_id: Some("org-alpha".into()),
    };
    let (team_nodes, team_edges) = build_filtered_graph(&nodes, &edges, &team_filter);

    assert_eq!(team_nodes.len(), 4, "team scope: public + team nodes");
    assert!(
        team_nodes.contains(&"dec-002".to_string()),
        "team node included for same org"
    );
    assert!(
        !team_nodes.contains(&"fnd-002".to_string()),
        "private node excluded from team scope"
    );

    assert_eq!(
        team_edges.len(),
        2,
        "team scope: edges between visible nodes only"
    );
}

// ---------------------------------------------------------------------------
// Test 49: Team scope does not leak private decisions
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_team_scope_does_not_leak_private_decisions() {
    let nodes = vec![
        EntityNode {
            id: "dec-pub".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
        EntityNode {
            id: "dec-team-alpha".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Team,
            org_id: Some("org-alpha".into()),
        },
        EntityNode {
            id: "dec-team-beta".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Team,
            org_id: Some("org-beta".into()),
        },
        EntityNode {
            id: "dec-private".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Private,
            org_id: Some("org-alpha".into()),
        },
        EntityNode {
            id: "fnd-shared".into(),
            entity_type: "finding".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
    ];

    let edges = vec![
        EntityEdge {
            source_id: "dec-pub".into(),
            target_id: "fnd-shared".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-team-alpha".into(),
            target_id: "fnd-shared".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-team-beta".into(),
            target_id: "fnd-shared".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-private".into(),
            target_id: "fnd-shared".into(),
            relation: "derived_from".into(),
        },
    ];

    let alpha_filter = VisibilityFilter {
        scope: Visibility::Team,
        org_id: Some("org-alpha".into()),
    };
    let (alpha_nodes, alpha_edges) = build_filtered_graph(&nodes, &edges, &alpha_filter);

    assert!(
        alpha_nodes.contains(&"dec-pub".to_string()),
        "public visible to team"
    );
    assert!(
        alpha_nodes.contains(&"dec-team-alpha".to_string()),
        "own team visible"
    );
    assert!(
        !alpha_nodes.contains(&"dec-team-beta".to_string()),
        "other team's decisions must NOT leak"
    );
    assert!(
        !alpha_nodes.contains(&"dec-private".to_string()),
        "private decisions must NOT leak to team scope"
    );

    assert_eq!(alpha_edges.len(), 2, "only edges touching visible nodes");
    assert!(alpha_edges.contains(&("dec-pub".to_string(), "fnd-shared".to_string())));
    assert!(alpha_edges.contains(&("dec-team-alpha".to_string(), "fnd-shared".to_string())));
}

// ---------------------------------------------------------------------------
// Test 50: Public scope excludes team-private edges
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_public_scope_excludes_team_private_edges() {
    let nodes = vec![
        EntityNode {
            id: "dec-001".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
        EntityNode {
            id: "dec-002".into(),
            entity_type: "decision".into(),
            visibility: Visibility::Team,
            org_id: Some("org-alpha".into()),
        },
        EntityNode {
            id: "fnd-001".into(),
            entity_type: "finding".into(),
            visibility: Visibility::Public,
            org_id: None,
        },
    ];

    let edges = vec![
        EntityEdge {
            source_id: "dec-001".into(),
            target_id: "fnd-001".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-002".into(),
            target_id: "fnd-001".into(),
            relation: "derived_from".into(),
        },
        EntityEdge {
            source_id: "dec-001".into(),
            target_id: "dec-002".into(),
            relation: "follows_precedent".into(),
        },
    ];

    let public_filter = VisibilityFilter {
        scope: Visibility::Public,
        org_id: None,
    };
    let (pub_nodes, pub_edges) = build_filtered_graph(&nodes, &edges, &public_filter);

    assert_eq!(pub_nodes.len(), 2, "only public nodes");
    assert!(!pub_nodes.contains(&"dec-002".to_string()));

    assert_eq!(pub_edges.len(), 1, "only edges between public nodes");
    assert_eq!(pub_edges[0], ("dec-001".to_string(), "fnd-001".to_string()));

    for (src, tgt) in &pub_edges {
        assert!(
            pub_nodes.contains(src),
            "edge source {src} must be in visible nodes"
        );
        assert!(
            pub_nodes.contains(tgt),
            "edge target {tgt} must be in visible nodes"
        );
    }
}

// ---------------------------------------------------------------------------
// Performance helpers
// ---------------------------------------------------------------------------

fn generate_scale_graph(
    num_nodes: usize,
    num_edges: usize,
) -> (Vec<String>, Vec<(String, String)>) {
    let nodes: Vec<String> = (0..num_nodes).map(|i| format!("n-{i:08}")).collect();

    let mut edges = Vec::with_capacity(num_edges);
    for i in 0..num_edges {
        let src = i % num_nodes;
        let dst = (i * 7 + 13) % num_nodes;
        if src != dst {
            edges.push((nodes[src].clone(), nodes[dst].clone()));
        }
    }

    (nodes, edges)
}

fn build_graph_from_vecs(
    nodes: &[String],
    edges: &[(String, String)],
) -> (
    rustworkx_core::petgraph::graph::DiGraph<String, ()>,
    BTreeMap<String, rustworkx_core::petgraph::graph::NodeIndex>,
) {
    use rustworkx_core::petgraph::graph::DiGraph;

    let mut graph = DiGraph::new();
    let mut index_map = BTreeMap::new();

    for n in nodes {
        let idx = graph.add_node(n.clone());
        index_map.insert(n.clone(), idx);
    }

    for (src, dst) in edges {
        if let (Some(&si), Some(&di)) = (index_map.get(src), index_map.get(dst)) {
            graph.add_edge(si, di, ());
        }
    }

    (graph, index_map)
}

// ---------------------------------------------------------------------------
// Test 51: Performance — small graph (500 nodes / 2k edges)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_perf_small_graph() {
    let (nodes, edges) = generate_scale_graph(500, 2_000);
    let start = Instant::now();
    let (graph, _) = build_graph_from_vecs(&nodes, &edges);
    let build_ms = start.elapsed().as_millis();

    let start = Instant::now();
    let _centrality = rustworkx_core::centrality::betweenness_centrality(&graph, false, false, 200);
    let centrality_ms = start.elapsed().as_millis();

    assert!(
        build_ms < 5_000,
        "small graph build should be < 5s, was {build_ms}ms"
    );
    assert!(
        centrality_ms < 30_000,
        "small graph centrality should be < 30s, was {centrality_ms}ms"
    );

    assert_eq!(graph.node_count(), 500);
}

// ---------------------------------------------------------------------------
// Test 52: Performance — medium graph (5k nodes / 20k edges)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_perf_medium_graph() {
    let (nodes, edges) = generate_scale_graph(5_000, 20_000);
    let start = Instant::now();
    let (graph, _) = build_graph_from_vecs(&nodes, &edges);
    let build_ms = start.elapsed().as_millis();

    let start = Instant::now();
    let components = rustworkx_core::connectivity::connected_components(&graph);
    let comp_ms = start.elapsed().as_millis();

    assert!(
        build_ms < 10_000,
        "medium graph build should be < 10s, was {build_ms}ms"
    );
    assert!(
        comp_ms < 30_000,
        "medium graph components should be < 30s, was {comp_ms}ms"
    );

    assert_eq!(graph.node_count(), 5_000);
    assert!(!components.is_empty(), "should find at least 1 component");
}

// ---------------------------------------------------------------------------
// Test 53: Performance — large graph (20k nodes / 100k edges)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_perf_large_graph() {
    let (nodes, edges) = generate_scale_graph(20_000, 100_000);
    let start = Instant::now();
    let (graph, _) = build_graph_from_vecs(&nodes, &edges);
    let build_ms = start.elapsed().as_millis();

    let start = Instant::now();
    let components = rustworkx_core::connectivity::connected_components(&graph);
    let comp_ms = start.elapsed().as_millis();

    assert!(
        build_ms < 30_000,
        "large graph build should be < 30s, was {build_ms}ms"
    );
    assert!(
        comp_ms < 60_000,
        "large graph components should be < 60s, was {comp_ms}ms"
    );

    assert_eq!(graph.node_count(), 20_000);
    assert!(!components.is_empty(), "should find at least 1 component");
}

// ---------------------------------------------------------------------------
// Test 54: Deterministic hash across runs
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_perf_deterministic_hash_across_runs() {
    let (nodes, edges) = generate_scale_graph(500, 2_000);

    let mut digests: Vec<u64> = Vec::new();

    for _ in 0..5 {
        let (graph, _) = build_graph_from_vecs(&nodes, &edges);
        let centralities =
            rustworkx_core::centrality::betweenness_centrality(&graph, false, false, 200);

        let mut scored: Vec<(String, u64)> = graph
            .node_indices()
            .map(|n| {
                let id = graph[n].clone();
                let raw = centralities[n.index()].unwrap_or(0.0);
                let bits = raw.to_bits();
                (id, bits)
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut h = DefaultHasher::new();
        for (id, bits) in &scored {
            id.hash(&mut h);
            bits.hash(&mut h);
        }
        digests.push(h.finish());
    }

    for (i, d) in digests.iter().enumerate().skip(1) {
        assert_eq!(
            digests[0], *d,
            "hash mismatch at run {i}: expected {} got {d}",
            digests[0]
        );
    }
}
