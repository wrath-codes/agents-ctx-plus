//! # Spike 0.22: Decision Trace Graph Algorithms
//!
//! Validates graph algorithms from `rustworkx-core` (which re-exports `petgraph`) for
//! decision trace analytics in Zenith.
//!
//! ## What This Spike Validates
//!
//! ### RQ3: Deterministic Graph Analytics
//! - Topological sort on task DAGs is deterministic across runs
//! - Cycle detection correctly identifies cyclic dependencies
//! - Ready-set computation (zero in-degree) for task scheduling
//! - Betweenness centrality ranking is stable with tie-break policy
//! - Shortest explanation path between decisions and hypotheses
//! - Weakly connected component detection on directed evidence graphs
//!
//! ### RQ4: Budget-Enforced Graph Traversal
//! - `max_nodes` budget truncates node addition
//! - `max_edges` budget truncates edge addition
//! - `max_depth` budget truncates BFS expansion
//! - Truncation metadata records reason and actual counts
//! - Output tie-break policy produces identical hashes across runs

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rustworkx_core::centrality::betweenness_centrality;
    use rustworkx_core::connectivity::{connected_components, find_cycle};
    use rustworkx_core::dictmap::{DictMap, InitWithHasher};
    use rustworkx_core::petgraph::algo::toposort;
    use rustworkx_core::petgraph::graph::{DiGraph, NodeIndex};
    use rustworkx_core::petgraph::visit::EdgeRef;
    use rustworkx_core::petgraph::Direction;
    use rustworkx_core::shortest_path::dijkstra;
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::hash::{DefaultHasher, Hash, Hasher};

    // =========================================================================
    // Domain types
    // =========================================================================

    #[allow(clippy::struct_field_names)]
    struct GraphBudget {
        node_limit: usize,
        edge_limit: usize,
        depth_limit: usize,
    }

    impl Default for GraphBudget {
        fn default() -> Self {
            Self {
                node_limit: 500,
                edge_limit: 2000,
                depth_limit: 10,
            }
        }
    }

    struct TruncationInfo {
        truncated: bool,
        truncation_reason: Option<String>,
        actual_nodes: usize,
        actual_edges: usize,
    }

    struct GraphResult<T> {
        data: T,
        truncation: TruncationInfo,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum EntityKind {
        Insight = 0,
        Decision = 1,
        Finding = 2,
        Hypothesis = 3,
        Task = 4,
        Research = 5,
    }

    fn entity_kind_from_id(id: &str) -> EntityKind {
        let prefix = id.split('-').next().unwrap_or("");
        match prefix {
            "ins" => EntityKind::Insight,
            "dec" => EntityKind::Decision,
            "fnd" => EntityKind::Finding,
            "hyp" => EntityKind::Hypothesis,
            "tsk" => EntityKind::Task,
            _ => EntityKind::Research,
        }
    }

    #[derive(Debug, Clone)]
    struct ScoredEntity {
        id: String,
        score: f64,
        kind: EntityKind,
    }

    fn deterministic_sort(entities: &mut [ScoredEntity]) {
        entities.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.kind.cmp(&b.kind))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn build_dag(edges: &[(&str, &str)]) -> (DiGraph<String, ()>, BTreeMap<String, NodeIndex>) {
        let mut graph = DiGraph::new();
        let mut index_map = BTreeMap::new();
        for (src, dst) in edges {
            let src_str = (*src).to_string();
            let dst_str = (*dst).to_string();
            let src_idx = *index_map
                .entry(src_str.clone())
                .or_insert_with(|| graph.add_node(src_str));
            let dst_idx = *index_map
                .entry(dst_str.clone())
                .or_insert_with(|| graph.add_node(dst_str));
            graph.add_edge(src_idx, dst_idx, ());
        }
        (graph, index_map)
    }

    fn ready_set(graph: &DiGraph<String, ()>) -> Vec<NodeIndex> {
        graph
            .node_indices()
            .filter(|&n| {
                graph
                    .neighbors_directed(n, Direction::Incoming)
                    .next()
                    .is_none()
            })
            .collect()
    }

    fn bounded_bfs(
        graph: &DiGraph<String, ()>,
        start: NodeIndex,
        max_depth: usize,
    ) -> Vec<NodeIndex> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((start, 0usize));
        visited.insert(start);
        let mut result = vec![start];

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for neighbor in graph.neighbors(node) {
                if visited.insert(neighbor) {
                    result.push(neighbor);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }
        result
    }

    fn build_budgeted_graph(
        node_ids: &[&str],
        edges: &[(&str, &str)],
        budget: &GraphBudget,
    ) -> GraphResult<(DiGraph<String, ()>, BTreeMap<String, NodeIndex>)> {
        let mut graph = DiGraph::new();
        let mut index_map = BTreeMap::new();
        let mut truncated = false;
        let mut truncation_reason = None;

        for &id in node_ids {
            if graph.node_count() >= budget.node_limit {
                truncated = true;
                truncation_reason =
                    Some(format!("max_nodes ({}) exceeded", budget.node_limit));
                break;
            }
            let s = id.to_string();
            index_map.entry(s.clone()).or_insert_with(|| graph.add_node(s));
        }

        for (src, dst) in edges {
            if graph.edge_count() >= budget.edge_limit {
                truncated = true;
                truncation_reason =
                    Some(format!("max_edges ({}) exceeded", budget.edge_limit));
                break;
            }
            let Some(&src_idx) = index_map.get(*src) else {
                continue;
            };
            let Some(&dst_idx) = index_map.get(*dst) else {
                continue;
            };
            graph.add_edge(src_idx, dst_idx, ());
        }

        let actual_nodes = graph.node_count();
        let actual_edges = graph.edge_count();

        GraphResult {
            data: (graph, index_map),
            truncation: TruncationInfo {
                truncated,
                truncation_reason,
                actual_nodes,
                actual_edges,
            },
        }
    }

    // =========================================================================
    // Test 38: Deterministic topological sort on a task DAG
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_task_dag_toposort_deterministic() {
        let edges = [
            ("tsk-A", "tsk-B"),
            ("tsk-A", "tsk-D"),
            ("tsk-B", "tsk-C"),
            ("tsk-D", "tsk-C"),
        ];

        let mut results: Vec<Vec<String>> = Vec::new();

        for _ in 0..10 {
            let (graph, index_map) = build_dag(&edges);
            let sorted = toposort(&graph, None).expect("DAG should not have a cycle");
            let labels: Vec<String> = sorted.iter().map(|&n| graph[n].clone()).collect();

            let pos = |id: &str| {
                labels
                    .iter()
                    .position(|l| l == id)
                    .unwrap_or_else(|| panic!("{id} not in topo order"))
            };

            assert!(pos("tsk-A") < pos("tsk-B"), "A must precede B");
            assert!(pos("tsk-A") < pos("tsk-D"), "A must precede D");
            assert!(pos("tsk-B") < pos("tsk-C"), "B must precede C");
            assert!(pos("tsk-D") < pos("tsk-C"), "D must precede C");

            let _ = index_map;
            results.push(labels);
        }

        for run in &results[1..] {
            assert_eq!(&results[0], run, "toposort must be deterministic");
        }
    }

    // =========================================================================
    // Test 39: Cycle detection on a cyclic task graph
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_task_dag_cycle_detection() {
        let edges = [("tsk-A", "tsk-B"), ("tsk-B", "tsk-C"), ("tsk-C", "tsk-A")];
        let (graph, index_map) = build_dag(&edges);

        let cycle_edges = find_cycle(&graph, Some(index_map["tsk-A"]));
        assert!(
            !cycle_edges.is_empty(),
            "find_cycle must detect the A->B->C->A cycle"
        );

        let topo_result = toposort(&graph, None);
        assert!(
            topo_result.is_err(),
            "toposort must return Err for a cyclic graph"
        );
    }

    // =========================================================================
    // Test 40: Ready-set (zero in-degree) computation
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_task_dag_ready_set() {
        let edges = [
            ("tsk-A", "tsk-B"),
            ("tsk-A", "tsk-C"),
            ("tsk-B", "tsk-D"),
            ("tsk-C", "tsk-D"),
        ];
        let (mut graph, index_map) = build_dag(&edges);

        let initial_ready: BTreeSet<String> = ready_set(&graph)
            .iter()
            .map(|&n| graph[n].clone())
            .collect();
        assert_eq!(
            initial_ready,
            BTreeSet::from(["tsk-A".to_string()]),
            "Only tsk-A has zero in-degree initially"
        );

        let a_idx = index_map["tsk-A"];
        let outgoing: Vec<_> = graph
            .edges_directed(a_idx, Direction::Outgoing)
            .map(|e| e.id())
            .collect();
        for eid in outgoing {
            graph.remove_edge(eid);
        }

        let after_ready: BTreeSet<String> = ready_set(&graph)
            .iter()
            .map(|&n| graph[n].clone())
            .collect();
        assert!(after_ready.contains("tsk-B"), "tsk-B should be ready");
        assert!(after_ready.contains("tsk-C"), "tsk-C should be ready");
        assert!(
            !after_ready.contains("tsk-D"),
            "tsk-D still has dependencies"
        );
    }

    // =========================================================================
    // Test 41: Betweenness centrality ranking stability
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_evidence_graph_centrality_ranking_stable() {
        let edges = [
            ("dec-001", "fnd-B"),
            ("dec-002", "fnd-B"),
            ("dec-003", "fnd-B"),
            ("fnd-B", "hyp-001"),
            ("fnd-B", "hyp-002"),
            ("dec-001", "fnd-A"),
            ("fnd-A", "hyp-001"),
            ("dec-002", "fnd-C"),
            ("fnd-C", "hyp-002"),
        ];

        let mut all_rankings: Vec<Vec<String>> = Vec::new();

        for _ in 0..5 {
            let (graph, _index_map) = build_dag(&edges);
            let centralities = betweenness_centrality(&graph, false, false, 200);

            let mut entities: Vec<ScoredEntity> = graph
                .node_indices()
                .map(|n| {
                    let id = graph[n].clone();
                    let score = centralities[n.index()].unwrap_or(0.0);
                    let kind = entity_kind_from_id(&id);
                    ScoredEntity { id, score, kind }
                })
                .collect();

            deterministic_sort(&mut entities);

            let ranking: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();
            assert_eq!(
                ranking[0], "fnd-B",
                "fnd-B should have highest betweenness centrality (on paths from 3 decisions to 2 hypotheses)"
            );
            all_rankings.push(ranking);
        }

        for run in &all_rankings[1..] {
            assert_eq!(
                &all_rankings[0], run,
                "centrality ranking must be identical across runs"
            );
        }
    }

    // =========================================================================
    // Test 42: Shortest explanation path (dijkstra)
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_evidence_graph_shortest_explain_path() {
        let edges = [("dec-001", "fnd-A"), ("fnd-A", "hyp-001")];
        let (graph, index_map) = build_dag(&edges);

        let start = index_map["dec-001"];
        let goal = index_map["hyp-001"];

        let mut paths: DictMap<NodeIndex, Vec<NodeIndex>> = DictMap::with_capacity(4);
        let _distances: DictMap<NodeIndex, usize> = dijkstra(
            &graph,
            start,
            Some(goal),
            |_| Ok::<usize, std::convert::Infallible>(1),
            Some(&mut paths),
        )
        .unwrap();

        let path_nodes: Vec<String> = paths[&goal]
            .iter()
            .map(|n: &NodeIndex| graph[*n].clone())
            .collect();
        assert_eq!(
            path_nodes,
            vec!["dec-001", "fnd-A", "hyp-001"],
            "shortest path should traverse dec-001 -> fnd-A -> hyp-001"
        );
    }

    // =========================================================================
    // Test 43: Weakly connected components on a directed evidence graph
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_decision_graph_connected_components() {
        let edges = [
            ("dec-001", "fnd-A"),
            ("fnd-A", "hyp-001"),
            ("dec-002", "fnd-B"),
            ("dec-003", "fnd-B"),
        ];
        let (mut graph, mut index_map) = build_dag(&edges);

        let iso = "dec-004".to_string();
        let iso_idx = graph.add_node(iso.clone());
        index_map.insert(iso, iso_idx);

        let components = connected_components(&graph);
        assert_eq!(components.len(), 3, "should have 3 weakly connected components");

        let cluster_labels: Vec<BTreeSet<String>> = components
            .iter()
            .map(|c| c.iter().map(|idx| graph[*idx].clone()).collect::<BTreeSet<String>>())
            .collect();

        let has_cluster = |members: &[&str]| {
            let target: BTreeSet<String> =
                members.iter().map(|s| (*s).to_string()).collect();
            cluster_labels.contains(&target)
        };

        assert!(
            has_cluster(&["dec-001", "fnd-A", "hyp-001"]),
            "Cluster 1 should contain dec-001, fnd-A, hyp-001"
        );
        assert!(
            has_cluster(&["dec-002", "dec-003", "fnd-B"]),
            "Cluster 2 should contain dec-002, dec-003, fnd-B"
        );
        assert!(
            has_cluster(&["dec-004"]),
            "Cluster 3 should contain isolated dec-004"
        );

        let largest = components.iter().max_by_key(|c| c.len()).unwrap();
        let largest_labels: BTreeSet<String> =
            largest.iter().map(|idx| graph[*idx].clone()).collect();
        assert!(
            largest_labels.contains("dec-001") || largest_labels.contains("dec-002"),
            "Largest component must contain one of the decision clusters"
        );
    }

    // =========================================================================
    // Test 44: max_nodes budget enforcement
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_graph_budget_max_nodes_enforced() {
        let node_ids: Vec<String> = (0..10).map(|i| format!("tsk-{i:03}")).collect();
        let node_refs: Vec<&str> = node_ids.iter().map(String::as_str).collect();

        let budget = GraphBudget {
            node_limit: 5,
            edge_limit: 100,
            depth_limit: 10,
        };

        let result = build_budgeted_graph(&node_refs, &[], &budget);
        assert!(
            result.truncation.truncated,
            "graph should be truncated at max_nodes"
        );
        assert!(
            result
                .truncation
                .truncation_reason
                .as_deref()
                .unwrap_or("")
                .contains("max_nodes"),
            "truncation_reason should mention max_nodes"
        );
        assert_eq!(
            result.truncation.actual_nodes, 5,
            "should have exactly max_nodes nodes"
        );
    }

    // =========================================================================
    // Test 45: max_edges budget enforcement
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_graph_budget_max_edges_enforced() {
        let node_ids = [
            "tsk-000", "tsk-001", "tsk-002", "tsk-003", "tsk-004", "tsk-005", "tsk-006",
            "tsk-007", "tsk-008", "tsk-009",
        ];
        let edges: Vec<(&str, &str)> = (0..10)
            .flat_map(|i| (i + 1..10).map(move |j| (node_ids[i], node_ids[j])))
            .collect();

        let budget = GraphBudget {
            node_limit: 100,
            edge_limit: 3,
            depth_limit: 10,
        };

        let result = build_budgeted_graph(&node_ids, &edges, &budget);
        assert!(
            result.truncation.truncated,
            "graph should be truncated at max_edges"
        );
        assert!(
            result
                .truncation
                .truncation_reason
                .as_deref()
                .unwrap_or("")
                .contains("max_edges"),
            "truncation_reason should mention max_edges"
        );
        assert_eq!(
            result.truncation.actual_edges, 3,
            "should have exactly max_edges edges"
        );
    }

    // =========================================================================
    // Test 46: max_depth budget enforcement via bounded BFS
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_graph_budget_max_depth_enforced() {
        let chain_ids: Vec<String> = (0..8).map(|i| format!("tsk-{i:03}")).collect();
        let chain_edges: Vec<(&str, &str)> = chain_ids
            .windows(2)
            .map(|w| (w[0].as_str(), w[1].as_str()))
            .collect();

        let budget = GraphBudget {
            node_limit: 100,
            edge_limit: 100,
            depth_limit: 3,
        };

        let result = build_budgeted_graph(
            &chain_ids.iter().map(String::as_str).collect::<Vec<_>>(),
            &chain_edges,
            &budget,
        );
        let (graph, index_map) = &result.data;

        let root = index_map["tsk-000"];
        let reachable = bounded_bfs(graph, root, budget.depth_limit);
        let reachable_labels: Vec<String> = reachable.iter().map(|&n| graph[n].clone()).collect();

        assert_eq!(
            reachable_labels.len(),
            4,
            "BFS at depth 3 from root should reach exactly 4 nodes (root + 3 levels)"
        );
        assert!(
            reachable_labels.contains(&"tsk-000".to_string()),
            "root must be included"
        );
        assert!(
            reachable_labels.contains(&"tsk-003".to_string()),
            "depth-3 node must be included"
        );
        assert!(
            !reachable_labels.contains(&"tsk-004".to_string()),
            "depth-4 node must NOT be included"
        );

        let truncated = reachable.len() < graph.node_count();
        let truncation = TruncationInfo {
            truncated,
            truncation_reason: if truncated {
                Some(format!("max_depth ({}) exceeded", budget.depth_limit))
            } else {
                None
            },
            actual_nodes: reachable.len(),
            actual_edges: 0,
        };

        assert!(truncation.truncated, "BFS should be truncated at max_depth");
        assert!(
            truncation
                .truncation_reason
                .as_deref()
                .unwrap_or("")
                .contains("max_depth"),
            "truncation_reason should mention max_depth"
        );
    }

    // =========================================================================
    // Test 47: Output tie-break stability across runs (hash check)
    // =========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn spike_graph_output_tie_break_stability() {
        let edges = [
            ("dec-001", "fnd-A"),
            ("dec-002", "fnd-A"),
            ("dec-001", "fnd-B"),
            ("dec-002", "fnd-B"),
            ("dec-003", "fnd-C"),
            ("dec-004", "fnd-C"),
            ("dec-003", "fnd-D"),
            ("dec-004", "fnd-D"),
        ];

        let mut digests: Vec<u64> = Vec::new();

        for _ in 0..10 {
            let (graph, _) = build_dag(&edges);
            let centralities = betweenness_centrality(&graph, false, false, 200);

            let mut entities: Vec<ScoredEntity> = graph
                .node_indices()
                .map(|n| {
                    let id = graph[n].clone();
                    let score = centralities[n.index()].unwrap_or(0.0);
                    let kind = entity_kind_from_id(&id);
                    ScoredEntity { id, score, kind }
                })
                .collect();

            deterministic_sort(&mut entities);

            let mut state = DefaultHasher::new();
            for e in &entities {
                e.id.hash(&mut state);
            }
            digests.push(state.finish());
        }

        for (i, h) in digests.iter().enumerate().skip(1) {
            assert_eq!(
                digests[0], *h,
                "hash mismatch at run {i}: expected {} got {h}",
                digests[0]
            );
        }
    }
}
