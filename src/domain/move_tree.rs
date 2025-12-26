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
            let move_num = ply.div_ceil(2);
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

    /// Delete a node and all its descendants.
    /// If the current position is within the deleted subtree, navigates to the parent.
    /// Returns true on success, false if node_id is root or invalid.
    pub fn delete_node(&mut self, node_id: MoveNodeId) -> bool {
        // Can't delete root
        if node_id == 0 || node_id >= self.nodes.len() {
            return false;
        }

        // Get parent_id before we modify anything
        let parent_id = match self.nodes[node_id].parent_id {
            Some(pid) => pid,
            None => return false, // Root node, shouldn't happen given check above
        };

        // Check if current position is in the subtree being deleted
        if self.is_descendant_of(self.current_id, node_id) {
            self.current_id = parent_id;
        }

        // Remove from parent's children
        self.nodes[parent_id].children.retain(|&id| id != node_id);

        // Note: We don't actually remove nodes from the vec (would invalidate IDs).
        // The nodes become orphaned but that's fine for our use case.

        true
    }

    /// Check if `node_id` is equal to `ancestor_id` or is a descendant of it
    fn is_descendant_of(&self, node_id: MoveNodeId, ancestor_id: MoveNodeId) -> bool {
        let mut current = node_id;
        loop {
            if current == ancestor_id {
                return true;
            }
            match self.nodes.get(current).and_then(|n| n.parent_id) {
                Some(parent) => current = parent,
                None => return false,
            }
        }
    }

    /// Promote a variation to be the main line at its branch point.
    /// This finds the first ancestor that is not the main line continuation
    /// (i.e., not the first child of its parent) and promotes that.
    /// Returns true on success.
    pub fn promote_variation(&mut self, node_id: MoveNodeId) -> bool {
        if node_id == 0 || node_id >= self.nodes.len() {
            return false;
        }

        // Find the first ancestor that is a variation (not first child of parent)
        let branch_node_id = self.find_variation_branch_point(node_id);

        if branch_node_id == 0 {
            // Already on main line, nothing to promote
            return true;
        }

        // Now promote that branch node
        self.promote_node(branch_node_id)
    }

    /// Find the first ancestor (including self) that is not the first child of its parent.
    /// Returns 0 if already on main line.
    fn find_variation_branch_point(&self, node_id: MoveNodeId) -> MoveNodeId {
        let mut current = node_id;

        while current != 0 {
            let parent_id = match self.nodes[current].parent_id {
                Some(pid) => pid,
                None => return 0, // at root
            };

            let children = &self.nodes[parent_id].children;
            if children.first() != Some(&current) {
                // This node is not the first child - it's where the variation branches
                return current;
            }

            current = parent_id;
        }

        0 // On main line
    }

    /// Promote a specific node to be the first child of its parent.
    fn promote_node(&mut self, node_id: MoveNodeId) -> bool {
        let parent_id = match self.nodes[node_id].parent_id {
            Some(pid) => pid,
            None => return false,
        };

        let children = &mut self.nodes[parent_id].children;

        // Find position of node_id in children
        let pos = match children.iter().position(|&id| id == node_id) {
            Some(p) => p,
            None => return false,
        };

        // Already first? Nothing to do
        if pos == 0 {
            return true;
        }

        // Move to front
        let id = children.remove(pos);
        children.insert(0, id);

        true
    }

    /// Promote a variation to be the global main line.
    /// This promotes at every branch point from this node up to the root.
    /// Returns true on success.
    pub fn promote_to_main_line(&mut self, node_id: MoveNodeId) -> bool {
        if node_id == 0 || node_id >= self.nodes.len() {
            return false;
        }

        // Collect path from node to root
        let mut path = Vec::new();
        let mut current = node_id;
        while current != 0 {
            path.push(current);
            match self.nodes[current].parent_id {
                Some(parent) => current = parent,
                None => break,
            }
        }

        // Promote each node in the path (from root towards the node)
        for &nid in path.iter().rev() {
            self.promote_variation(nid);
        }

        true
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

        let root = tree.get(0).unwrap();
        assert!(!root.variation_children().is_empty());
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.main_line_child(), Some(1)); // e4 is main line
        assert_eq!(root.variation_children(), &[2]); // d4 is variation
    }

    #[test]
    fn test_delete_node() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        // Build: 1.e4 e5 2.Nf3
        tree.add_move(pos.clone(), "e4".to_string()); // id=1
        tree.add_move(pos.clone(), "e5".to_string()); // id=2
        tree.add_move(pos.clone(), "Nf3".to_string()); // id=3

        // Delete e5 (and its descendants)
        assert!(tree.delete_node(2));

        // e4 should now have no children
        let e4 = tree.get(1).unwrap();
        assert!(e4.children.is_empty());

        // Current should be e4 (parent of deleted)
        assert_eq!(tree.current_id(), 1);
    }

    #[test]
    fn test_delete_node_not_current() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        // Build: 1.e4 (1.d4)
        tree.add_move(pos.clone(), "e4".to_string()); // id=1
        tree.go_to_root();
        tree.add_move(pos.clone(), "d4".to_string()); // id=2

        // We're on d4, delete e4
        assert!(tree.delete_node(1));

        // Should still be on d4
        assert_eq!(tree.current_id(), 2);

        // Root should only have d4 as child now
        let root = tree.get(0).unwrap();
        assert_eq!(root.children, vec![2]);
    }

    #[test]
    fn test_cannot_delete_root() {
        let mut tree = MoveTree::new();
        assert!(!tree.delete_node(0));
    }

    #[test]
    fn test_promote_variation() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        // Build: 1.e4 (1.d4, 1.c4)
        tree.add_move(pos.clone(), "e4".to_string()); // id=1
        tree.go_to_root();
        tree.add_move(pos.clone(), "d4".to_string()); // id=2
        tree.go_to_root();
        tree.add_move(pos.clone(), "c4".to_string()); // id=3

        let root = tree.get(0).unwrap();
        assert_eq!(root.children, vec![1, 2, 3]); // e4 is main line

        // Promote d4
        assert!(tree.promote_variation(2));

        let root = tree.get(0).unwrap();
        assert_eq!(root.children, vec![2, 1, 3]); // d4 is now main line
    }

    #[test]
    fn test_promote_variation_from_middle_of_variation() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        // Build: 1.e4 e5 (1...c5 2.Nf3)
        //                      ^-- promote from here should promote the whole c5 variation
        tree.add_move(pos.clone(), "e4".to_string()); // id=1
        tree.add_move(pos.clone(), "e5".to_string()); // id=2

        // Go back to e4, add c5 variation
        tree.go_to(1);
        tree.add_move(pos.clone(), "c5".to_string()); // id=3
        tree.add_move(pos.clone(), "Nf3".to_string()); // id=4

        // e4's children are [e5, c5]
        let e4 = tree.get(1).unwrap();
        assert_eq!(e4.children, vec![2, 3]);

        // Promote from Nf3 (id=4) - should find branch point at c5 (id=3) and promote that
        assert!(tree.promote_variation(4));

        // Now e4's children should be [c5, e5]
        let e4 = tree.get(1).unwrap();
        assert_eq!(e4.children, vec![3, 2]);
    }

    #[test]
    fn test_promote_to_main_line() {
        let mut tree = MoveTree::new();
        let pos = Chess::default();

        // Build: 1.e4 e5 (1...c5) 2.Nf3
        //                  ^-- we want to promote this deeply nested variation
        tree.add_move(pos.clone(), "e4".to_string()); // id=1
        tree.add_move(pos.clone(), "e5".to_string()); // id=2
        tree.add_move(pos.clone(), "Nf3".to_string()); // id=3

        // Go back to e4, add c5 as variation
        tree.go_to(1);
        tree.add_move(pos.clone(), "c5".to_string()); // id=4

        // Verify structure: e4's children are [e5, c5]
        let e4 = tree.get(1).unwrap();
        assert_eq!(e4.children, vec![2, 4]);

        // Promote c5 to main line
        assert!(tree.promote_to_main_line(4));

        // Now e4's children should be [c5, e5]
        let e4 = tree.get(1).unwrap();
        assert_eq!(e4.children, vec![4, 2]);
    }
}
