use crate::error::GraphError;
use crate::types::{GraphType, NodeId};
use parking_lot::RwLock;
use petgraph::graphmap::DiGraphMap;

#[derive(Debug)]
pub struct Dag {
    graph_type: GraphType,
    inner: RwLock<DiGraphMap<NodeId, ()>>,
}

impl Dag {
    pub fn new(graph_type: GraphType) -> Self {
        Self {
            graph_type,
            inner: RwLock::new(DiGraphMap::new()),
        }
    }

    pub fn add_node(&self, node_id: NodeId) {
        self.inner.write().add_node(node_id);
    }

    pub fn add_edge(&self, from: NodeId, to: NodeId) -> Result<(), GraphError> {
        if from == to {
            return Err(GraphError::SelfLoop);
        }

        let mut g = self.inner.write();
        g.add_node(from);
        g.add_node(to);
        g.add_edge(from, to, ());

        if matches!(self.graph_type, GraphType::ProductionDAG)
            && petgraph::algo::is_cyclic_directed(&*g)
        {
            g.remove_edge(from, to);
            return Err(GraphError::CycleDetected);
        }

        Ok(())
    }
}
