use crate::error::GraphError;
use crate::types::{GraphType, NodeId};
use parking_lot::RwLock;
use petgraph::graphmap::DiGraphMap;
use petgraph::algo::toposort;
use petgraph::Direction;

#[derive(Debug)]
pub struct Dag {
    graph_type: GraphType,
    inner: RwLock<DiGraphMap<NodeId, ()>>,
    frozen: RwLock<Vec<NodeId>>,
    deactivated: RwLock<Vec<NodeId>>,
}

impl Dag {
    pub fn new(graph_type: GraphType) -> Self {
        Self {
            graph_type,
            inner: RwLock::new(DiGraphMap::new()),
            frozen: RwLock::new(Vec::new()),
            deactivated: RwLock::new(Vec::new()),
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

    pub fn node_count(&self) -> usize {
        self.inner.read().node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.inner.read().edge_count()
    }

    pub fn freeze_node(&self, node_id: NodeId) -> Result<(), GraphError> {
        let g = self.inner.read();
        if !g.contains_node(node_id) {
            return Err(GraphError::NodeNotFound);
        }
        drop(g);
        
        let mut frozen = self.frozen.write();
        if !frozen.contains(&node_id) {
            frozen.push(node_id);
        }
        Ok(())
    }

    pub fn is_frozen(&self, node_id: NodeId) -> bool {
        self.frozen.read().contains(&node_id)
    }

    pub fn deactivate_node(&self, node_id: NodeId) -> Result<(), GraphError> {
        let g = self.inner.read();
        if !g.contains_node(node_id) {
            return Err(GraphError::NodeNotFound);
        }
        drop(g);
        
        let mut deactivated = self.deactivated.write();
        if !deactivated.contains(&node_id) {
            deactivated.push(node_id);
        }
        Ok(())
    }

    pub fn is_deactivated(&self, node_id: NodeId) -> bool {
        self.deactivated.read().contains(&node_id)
    }

    /// Validate the entire graph structure
    pub fn validate(&self) -> Result<(), GraphError> {
        let g = self.inner.read();
        
        // Check for cycles in production DAG
        if matches!(self.graph_type, GraphType::ProductionDAG) {
            if petgraph::algo::is_cyclic_directed(&*g) {
                return Err(GraphError::CycleDetected);
            }
        }
        
        Ok(())
    }

    /// Get topological sort of nodes (for scheduling)
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, GraphError> {
        let g = self.inner.read();
        match toposort(&*g, None) {
            Ok(order) => Ok(order),
            Err(_) => Err(GraphError::CycleDetected),
        }
    }

    /// Get nodes with no predecessors (entry points)
    pub fn entry_nodes(&self) -> Vec<NodeId> {
        let g = self.inner.read();
        g.nodes()
            .filter(|n| g.neighbors_directed(*n, Direction::Incoming).next().is_none())
            .collect()
    }

    /// Get nodes with no successors (exit points)
    pub fn exit_nodes(&self) -> Vec<NodeId> {
        let g = self.inner.read();
        g.nodes()
            .filter(|n| g.neighbors_directed(*n, Direction::Outgoing).next().is_none())
            .collect()
    }
}
