//! Decision graph analytics over zen-db `entity_links`.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::Infallible;

use rustworkx_core::centrality::betweenness_centrality;
use rustworkx_core::connectivity::connected_components;
use rustworkx_core::dictmap::{DictMap, InitWithHasher};
use rustworkx_core::petgraph::algo::toposort;
use rustworkx_core::petgraph::graph::{DiGraph, NodeIndex};
use rustworkx_core::shortest_path::dijkstra;
use zen_db::service::ZenService;

use crate::error::SearchError;

/// Node payload for decision graph entities.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub entity_type: String,
    pub entity_id: String,
    pub label: String,
}

/// Edge payload for decision graph relations.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub relation: String,
    pub weight: f64,
}

/// Analysis summary over a decision graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphAnalysis {
    pub node_count: usize,
    pub edge_count: usize,
    pub components: usize,
    pub has_cycles: bool,
    pub topological_order: Option<Vec<String>>,
    pub centrality: Vec<(String, f64)>,
}

/// Directed graph built from `entity_links`.
pub struct DecisionGraph {
    graph: DiGraph<GraphNode, GraphEdge>,
    id_to_index: HashMap<String, NodeIndex>,
}

impl DecisionGraph {
    /// Build a graph from zen-db `entity_links` rows.
    ///
    /// # Errors
    ///
    /// Returns a database-backed [`SearchError`] if reading links fails.
    pub async fn from_service(service: &ZenService) -> Result<Self, SearchError> {
        let mut rows = service
            .db()
            .conn()
            .query(
                "SELECT source_type, source_id, target_type, target_id, relation FROM entity_links",
                (),
            )
            .await
            .map_err(|e| {
                SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
            })?;

        let mut graph = DiGraph::new();
        let mut id_to_index = HashMap::new();

        while let Some(row) = rows.next().await.map_err(|e| {
            SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
        })? {
            let source_type = row.get::<String>(0).map_err(|e| {
                SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
            })?;
            let source_id = row.get::<String>(1).map_err(|e| {
                SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
            })?;
            let target_type = row.get::<String>(2).map_err(|e| {
                SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
            })?;
            let target_id = row.get::<String>(3).map_err(|e| {
                SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
            })?;
            let relation = row.get::<String>(4).map_err(|e| {
                SearchError::Database(zen_db::error::DatabaseError::Query(e.to_string()))
            })?;

            let src_key = node_key(&source_type, &source_id);
            let dst_key = node_key(&target_type, &target_id);

            let src_idx = *id_to_index.entry(src_key.clone()).or_insert_with(|| {
                graph.add_node(GraphNode {
                    entity_type: source_type.clone(),
                    entity_id: source_id.clone(),
                    label: src_key,
                })
            });

            let dst_idx = *id_to_index.entry(dst_key.clone()).or_insert_with(|| {
                graph.add_node(GraphNode {
                    entity_type: target_type.clone(),
                    entity_id: target_id.clone(),
                    label: dst_key,
                })
            });

            graph.add_edge(
                src_idx,
                dst_idx,
                GraphEdge {
                    relation,
                    weight: 1.0,
                },
            );
        }

        Ok(Self { graph, id_to_index })
    }

    /// Topological ordering for DAGs.
    #[must_use]
    pub fn toposort(&self) -> Option<Vec<String>> {
        let sorted = toposort(&self.graph, None).ok()?;
        Some(
            sorted
                .into_iter()
                .map(|idx| self.graph[idx].label.clone())
                .collect(),
        )
    }

    /// Betweenness centrality ranked descending.
    #[must_use]
    pub fn centrality(&self) -> Vec<(String, f64)> {
        let centralities = betweenness_centrality(&self.graph, false, false, 200);
        let mut values: Vec<(String, f64)> = self
            .graph
            .node_indices()
            .map(|idx| {
                (
                    self.graph[idx].label.clone(),
                    centralities[idx.index()].unwrap_or(0.0),
                )
            })
            .collect();

        values.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        values
    }

    /// Shortest path between two node labels (`"type:id"`).
    #[must_use]
    pub fn shortest_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        let start = *self.id_to_index.get(from)?;
        let goal = *self.id_to_index.get(to)?;

        let mut paths: DictMap<NodeIndex, Vec<NodeIndex>> = DictMap::with_capacity(16);
        let distances: DictMap<NodeIndex, usize> = dijkstra(
            &self.graph,
            start,
            Some(goal),
            |_| Ok::<usize, Infallible>(1),
            Some(&mut paths),
        )
        .ok()?;
        let _ = distances;

        let nodes = paths.get(&goal)?;
        Some(
            nodes
                .iter()
                .map(|idx| self.graph[*idx].label.clone())
                .collect(),
        )
    }

    /// Weakly connected component count.
    #[must_use]
    pub fn connected_components(&self) -> usize {
        connected_components(&self.graph).len()
    }

    /// Whether the graph has any cycle.
    #[must_use]
    pub fn has_cycles(&self) -> bool {
        toposort(&self.graph, None).is_err()
    }

    /// Aggregate analysis with optional centrality budget.
    #[must_use]
    pub fn analyze(&self, max_nodes_for_centrality: usize) -> GraphAnalysis {
        GraphAnalysis {
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            components: self.connected_components(),
            has_cycles: self.has_cycles(),
            topological_order: self.toposort(),
            centrality: if self.graph.node_count() <= max_nodes_for_centrality {
                self.centrality()
            } else {
                Vec::new()
            },
        }
    }
}

fn node_key(entity_type: &str, entity_id: &str) -> String {
    format!("{entity_type}:{entity_id}")
}

#[cfg(test)]
mod tests {
    use zen_db::service::ZenService;

    use super::*;

    async fn make_service() -> ZenService {
        ZenService::new_local(":memory:", None).await.unwrap()
    }

    async fn insert_link(
        service: &ZenService,
        id: &str,
        source_type: &str,
        source_id: &str,
        target_type: &str,
        target_id: &str,
        relation: &str,
    ) {
        service
            .db()
            .conn()
            .execute(
                "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (id, source_type, source_id, target_type, target_id, relation),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn graph_builds_from_entity_links() {
        let service = make_service().await;
        insert_link(
            &service, "lnk-1", "decision", "dec-1", "finding", "fnd-1", "supports",
        )
        .await;
        insert_link(
            &service,
            "lnk-2",
            "finding",
            "fnd-1",
            "hypothesis",
            "hyp-1",
            "informs",
        )
        .await;

        let graph = DecisionGraph::from_service(&service).await.unwrap();
        let analysis = graph.analyze(1_000);

        assert_eq!(analysis.node_count, 3);
        assert_eq!(analysis.edge_count, 2);
        assert_eq!(analysis.components, 1);
        assert!(!analysis.has_cycles);
        assert!(analysis.topological_order.is_some());
        assert!(!analysis.centrality.is_empty());
    }

    #[tokio::test]
    async fn shortest_path_returns_expected_chain() {
        let service = make_service().await;
        insert_link(
            &service, "lnk-1", "decision", "dec-1", "finding", "fnd-1", "supports",
        )
        .await;
        insert_link(
            &service,
            "lnk-2",
            "finding",
            "fnd-1",
            "hypothesis",
            "hyp-1",
            "informs",
        )
        .await;

        let graph = DecisionGraph::from_service(&service).await.unwrap();
        let path = graph
            .shortest_path("decision:dec-1", "hypothesis:hyp-1")
            .unwrap();

        assert_eq!(
            path,
            vec![
                "decision:dec-1".to_string(),
                "finding:fnd-1".to_string(),
                "hypothesis:hyp-1".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn cycle_detection_disables_toposort() {
        let service = make_service().await;
        insert_link(
            &service,
            "lnk-1",
            "task",
            "tsk-1",
            "task",
            "tsk-2",
            "depends_on",
        )
        .await;
        insert_link(
            &service,
            "lnk-2",
            "task",
            "tsk-2",
            "task",
            "tsk-1",
            "depends_on",
        )
        .await;

        let graph = DecisionGraph::from_service(&service).await.unwrap();
        let analysis = graph.analyze(1_000);

        assert!(analysis.has_cycles);
        assert!(analysis.topological_order.is_none());
    }
}
