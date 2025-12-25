//! Move tree data structure for representing chess games with variations.
//!
//! This is a pure domain module with no GPUI dependencies.

use shakmaty::Chess;

/// Unique identifier for a node in the move tree
pub type MoveNodeId = usize;

/// A node in the move tree representing a position after a move
#[derive(Clone, Debug)]
pub struct MoveNode {
    /// Unique identifier for this node
    pub id: MoveNodeId,
    /// Parent node ID (None for root)
    pub parent_id: Option<MoveNodeId>,
    /// The chess position at this node
    pub position: Chess,
    /// The SAN notation of the move that led to this position (None for root)
    pub san: Option<String>,
    /// Child node IDs - first child is the main line continuation, rest are variations
    pub children: Vec<MoveNodeId>,
}

impl MoveNode {
    /// Create a new root node with the starting position
    pub fn root() -> Self {
        Self {
            id: 0,
            parent_id: None,
            position: Chess::default(),
            san: None,
            children: Vec::new(),
        }
    }

    /// Create a new node from a move
    pub fn new(id: MoveNodeId, parent_id: MoveNodeId, position: Chess, san: String) -> Self {
        Self {
            id,
            parent_id: Some(parent_id),
            position,
            san: Some(san),
            children: Vec::new(),
        }
    }

    /// Check if this is the root node
    #[allow(dead_code)]
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this node has any children
    #[allow(dead_code)]
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Check if this node has variations (more than one child)
    pub fn has_variations(&self) -> bool {
        self.children.len() > 1
    }

    /// Get the main line continuation (first child), if any
    pub fn main_line_child(&self) -> Option<MoveNodeId> {
        self.children.first().copied()
    }

    /// Get variation children (all children except the first)
    pub fn variation_children(&self) -> &[MoveNodeId] {
        if self.children.len() > 1 {
            &self.children[1..]
        } else {
            &[]
        }
    }

    /// Calculate the half-move number (ply) for this node
    /// Root is ply 0, first move is ply 1, etc.
    pub fn ply(&self, tree: &MoveTree) -> usize {
        let mut count = 0;
        let mut current_id = self.id;
        while let Some(parent_id) = tree.get(current_id).and_then(|n| n.parent_id) {
            count += 1;
            current_id = parent_id;
        }
        count
    }

    /// Get the move number (1-based, for display)
    /// Returns (move_number, is_black_move)
    pub fn move_number(&self, tree: &MoveTree) -> (usize, bool) {
        let ply = self.ply(tree);
        if ply == 0 {
            (0, false) // Root has no move number
        } else {
            let move_num = (ply + 1) / 2;
            let is_black = ply % 2 == 0; // ply 2, 4, 6... are black's moves
            (move_num, is_black)
        }
    }
}

/// A tree structure representing a chess game with variations
#[derive(Clone, Debug)]
pub struct MoveTree {
    /// All nodes in the tree, indexed by their ID
    nodes: Vec<MoveNode>,
    /// The currently viewed node
    current_id: MoveNodeId,
}

impl MoveTree {
    /// Create a new tree with just the root (starting position)
    pub fn new() -> Self {
        Self {
            nodes: vec![MoveNode::root()],
            current_id: 0,
        }
    }

    /// Get a node by ID
    pub fn get(&self, id: MoveNodeId) -> Option<&MoveNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node by ID
    #[allow(dead_code)]
    pub fn get_mut(&mut self, id: MoveNodeId) -> Option<&mut MoveNode> {
        self.nodes.get_mut(id)
    }

    /// Get the root node
    #[allow(dead_code)]
    pub fn root(&self) -> &MoveNode {
        &self.nodes[0]
    }

    /// Get the currently viewed node
    pub fn current(&self) -> &MoveNode {
        &self.nodes[self.current_id]
    }

    /// Get the current node ID
    pub fn current_id(&self) -> MoveNodeId {
        self.current_id
    }

    /// Navigate to a specific node
    pub fn go_to(&mut self, id: MoveNodeId) -> bool {
        if id < self.nodes.len() {
            self.current_id = id;
            true
        } else {
            false
        }
    }

    /// Navigate to the root
    pub fn go_to_root(&mut self) {
        self.current_id = 0;
    }

    /// Navigate to parent (go back one move)
    pub fn go_back(&mut self) -> bool {
        if let Some(parent_id) = self.current().parent_id {
            self.current_id = parent_id;
            true
        } else {
            false
        }
    }

    /// Navigate to main line child (go forward one move)
    pub fn go_forward(&mut self) -> bool {
        if let Some(child_id) = self.current().main_line_child() {
            self.current_id = child_id;
            true
        } else {
            false
        }
    }

    /// Navigate to the end of the main line from current position
    pub fn go_to_end(&mut self) {
        while self.go_forward() {}
    }

    /// Check if we're at the root
    pub fn is_at_root(&self) -> bool {
        self.current_id == 0
    }

    /// Check if we're at a leaf node (no children)
    pub fn is_at_leaf(&self) -> bool {
        self.current().children.is_empty()
    }

    /// Add a new move from the current position
    /// Returns the ID of the new or existing node
    ///
    /// If a child with the same SAN exists, navigates to it.
    /// Otherwise, creates a new node and navigates to it.
    pub fn add_move(&mut self, position: Chess, san: String) -> MoveNodeId {
        // Check if this move already exists as a child
        let current = &self.nodes[self.current_id];
        for &child_id in &current.children {
            if let Some(child) = self.nodes.get(child_id) {
                if child.san.as_ref() == Some(&san) {
                    // Move already exists, navigate to it
                    self.current_id = child_id;
                    return child_id;
                }
            }
        }

        // Create new node
        let new_id = self.nodes.len();
        let new_node = MoveNode::new(new_id, self.current_id, position, san);
        self.nodes.push(new_node);

        // Add as child of current node
        self.nodes[self.current_id].children.push(new_id);

        // Navigate to new node
        self.current_id = new_id;
        new_id
    }

    /// Get the main line as a sequence of node IDs (from root to end)
    pub fn main_line(&self) -> Vec<MoveNodeId> {
        let mut line = vec![0]; // Start with root
        let mut current = &self.nodes[0];
        while let Some(child_id) = current.main_line_child() {
            line.push(child_id);
            current = &self.nodes[child_id];
        }
        line
    }

    /// Get the path from root to current position
    #[allow(dead_code)]
    pub fn path_to_current(&self) -> Vec<MoveNodeId> {
        let mut path = Vec::new();
        let mut id = self.current_id;
        loop {
            path.push(id);
            if let Some(parent_id) = self.nodes[id].parent_id {
                id = parent_id;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    /// Check if a node is on the main line
    #[allow(dead_code)]
    pub fn is_on_main_line(&self, id: MoveNodeId) -> bool {
        self.main_line().contains(&id)
    }

    /// Get all nodes that have variations (useful for UI)
    #[allow(dead_code)]
    pub fn nodes_with_variations(&self) -> Vec<MoveNodeId> {
        self.nodes
            .iter()
            .filter(|n| n.has_variations())
            .map(|n| n.id)
            .collect()
    }

    /// Get the total number of nodes
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if tree is empty (only root)
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.nodes.len() == 1
    }
}

impl Default for MoveTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tree() {
        let tree = MoveTree::new();
        assert_eq!(tree.len(), 1);
        assert!(tree.is_at_root());
        assert!(tree.is_at_leaf());
    }

    #[test]
    fn test_add_move() {
        let mut tree = MoveTree::new();
        let pos = Chess::default(); // simplified - in real use this would be the position after the move

        let id1 = tree.add_move(pos.clone(), "e4".to_string());
        assert_eq!(id1, 1);
        assert_eq!(tree.current_id(), 1);

        let id2 = tree.add_move(pos.clone(), "e5".to_string());
        assert_eq!(id2, 2);
        assert_eq!(tree.current_id(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        tree.add_move(pos.clone(), "e4".to_string());
        tree.add_move(pos.clone(), "e5".to_string());

        assert!(tree.go_back());
        assert_eq!(tree.current_id(), 1);

        assert!(tree.go_back());
        assert_eq!(tree.current_id(), 0);

        assert!(!tree.go_back()); // Can't go back from root

        assert!(tree.go_forward());
        assert_eq!(tree.current_id(), 1);
    }

    #[test]
    fn test_existing_move_navigation() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        tree.add_move(pos.clone(), "e4".to_string());
        tree.go_to_root();

        // Adding same move should navigate to existing node
        let id = tree.add_move(pos.clone(), "e4".to_string());
        assert_eq!(id, 1);
        assert_eq!(tree.current_id(), 1);
    }

    #[test]
    fn test_variations() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        // Main line: e4
        tree.add_move(pos.clone(), "e4".to_string());
        tree.go_to_root();

        // Variation: d4
        tree.add_move(pos.clone(), "d4".to_string());

        assert!(tree.root().has_variations());
        assert_eq!(tree.root().children.len(), 2);
        assert_eq!(tree.root().main_line_child(), Some(1)); // e4 is main line
        assert_eq!(tree.root().variation_children(), &[2]); // d4 is variation
    }
}
