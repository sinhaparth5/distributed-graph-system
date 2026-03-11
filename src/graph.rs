use serde::{ Deserialize, Serialize };
use std::collections::{HashSet, VecDeque, HashMap, BinaryHeap};
use std::ops::Neg;
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: usize,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeFeatures {
    None,
    VectorPerNode(HashMap<usize, Vec<f64>>),
    SparseFeatures(HashMap<usize, HashMap<usize, f64>>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl Neg for OrderedFloat {
    type Output = OrderedFloat;
    fn neg(self) -> Self::Output {
        OrderedFloat(-self.0)
    }
}

pub struct Graph {
    nodes: HashMap<usize, Node>,
    adj_list: HashMap<usize, Vec<(usize, f64)>>,
    node_features: NodeFeatures,
    feature_descriptions: HashMap<usize, String>,
    ego_features: HashMap<usize, Vec<f64>>,
    /// Maps compact sequential index (0..n) → original node ID.
    /// Used by algorithms that need contiguous array indexing.
    compact_to_id: Vec<usize>,
    /// Maps original node ID → compact sequential index.
    id_to_compact: HashMap<usize, usize>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: HashMap::new(),
            adj_list: HashMap::new(),
            node_features: NodeFeatures::None,
            feature_descriptions: HashMap::new(),
            ego_features: HashMap::new(),
            compact_to_id: Vec::new(),
            id_to_compact: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        let node_id = node.id;
        if !self.nodes.contains_key(&node_id) {
            // Assign the next compact index for this new node
            let compact_idx = self.compact_to_id.len();
            self.compact_to_id.push(node_id);
            self.id_to_compact.insert(node_id, compact_idx);
            self.adj_list.entry(node_id).or_insert_with(Vec::new);
        }
        self.nodes.insert(node_id, node);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return;
        }
        self.adj_list
            .entry(edge.from)
            .or_insert_with(Vec::new)
            .push((edge.to, edge.weight));
    }

    pub fn has_node(&self, id: usize) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.adj_list.values().map(|v| v.len()).sum()
    }

    pub fn set_node_features(&mut self, features: NodeFeatures) {
        self.node_features = features;
    }

    pub fn set_feature_descriptions(&mut self, descriptions: HashMap<usize, String>) {
        self.feature_descriptions = descriptions;
    }

    pub fn set_ego_features(&mut self, features: HashMap<usize, Vec<f64>>) {
        self.ego_features = features;
    }

    pub fn merge(&mut self, other: Graph) {
        for (_, node) in other.nodes {
            self.add_node(node);
        }
        for (from, neighbors) in other.adj_list {
            for (to, weight) in neighbors {
                self.add_edge(Edge { from, to, weight });
            }
        }
        match other.node_features {
            NodeFeatures::None => {}
            NodeFeatures::VectorPerNode(features) => {
                match &mut self.node_features {
                    NodeFeatures::None => self.node_features = NodeFeatures::VectorPerNode(features),
                    NodeFeatures::VectorPerNode(existing) => {
                        for (id, vector) in features { existing.insert(id, vector); }
                    }
                    NodeFeatures::SparseFeatures(_) => {
                        println!("Warning: Cannot merge different feature types");
                    }
                }
            }
            NodeFeatures::SparseFeatures(features) => {
                match &mut self.node_features {
                    NodeFeatures::None => self.node_features = NodeFeatures::SparseFeatures(features),
                    NodeFeatures::SparseFeatures(existing) => {
                        for (id, feature_map) in features { existing.insert(id, feature_map); }
                    }
                    NodeFeatures::VectorPerNode(_) => {
                        println!("Warning: Cannot merge different feature types");
                    }
                }
            }
        }
        for (id, desc) in other.feature_descriptions { self.feature_descriptions.insert(id, desc); }
        for (id, features) in other.ego_features { self.ego_features.insert(id, features); }
    }

    pub fn get_node(&self, id: usize) -> Option<&Node> { self.nodes.get(&id) }
    pub fn get_nodes(&self) -> &HashMap<usize, Node> { &self.nodes }
    pub fn get_node_cloned(&self, id: usize) -> Option<Node> { self.nodes.get(&id).cloned() }
    pub fn get_all_nodes_ids(&self) -> Vec<usize> { self.nodes.keys().cloned().collect() }

    pub fn get_all_edges(&self) -> Vec<Edge> {
        let mut edges = Vec::new();
        for (&from, neighbors) in &self.adj_list {
            for &(to, weight) in neighbors {
                edges.push(Edge { from, to, weight });
            }
        }
        edges
    }

    pub fn get_neighbors(&self, id: usize) -> Option<&Vec<(usize, f64)>> {
        self.adj_list.get(&id)
    }

    pub fn get_node_features(&self, id: usize) -> Option<Vec<f64>> {
        match &self.node_features {
            NodeFeatures::None => None,
            NodeFeatures::VectorPerNode(features) => features.get(&id).cloned(),
            NodeFeatures::SparseFeatures(features) => {
                if let Some(sparse) = features.get(&id) {
                    let max_feat = sparse.keys().max().unwrap_or(&0);
                    let mut dense = vec![0.0; max_feat + 1];
                    for (&feat_id, &value) in sparse { dense[feat_id] = value; }
                    Some(dense)
                } else {
                    None
                }
            }
        }
    }

    pub fn get_feature_description(&self, feature_id: usize) -> Option<&String> {
        self.feature_descriptions.get(&feature_id)
    }

    pub fn get_ego_features(&self, ego_id: usize) -> Option<&Vec<f64>> {
        self.ego_features.get(&ego_id)
    }

    // ── Graph Algorithms ───────────────────────────────────────────────────────

    pub fn dfs(&self, start: usize) -> Vec<usize> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        if self.nodes.contains_key(&start) {
            self.dfs_util(start, &mut visited, &mut path);
        }
        path
    }

    fn dfs_util(&self, vertex: usize, visited: &mut HashSet<usize>, path: &mut Vec<usize>) {
        visited.insert(vertex);
        path.push(vertex);
        if let Some(neighbors) = self.adj_list.get(&vertex) {
            for &(next, _) in neighbors {
                if !visited.contains(&next) {
                    self.dfs_util(next, visited, path);
                }
            }
        }
    }

    pub fn bfs(&self, start: usize) -> Vec<usize> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut path = Vec::new();

        if !self.nodes.contains_key(&start) { return path; }

        visited.insert(start);
        queue.push_back(start);

        while let Some(vertex) = queue.pop_front() {
            path.push(vertex);
            if let Some(neighbors) = self.adj_list.get(&vertex) {
                for &(next, _) in neighbors {
                    if !visited.contains(&next) {
                        visited.insert(next);
                        queue.push_back(next);
                    }
                }
            }
        }
        path
    }

    /// Returns (distances, path).
    /// `distances` is a compact Vec indexed 0..num_nodes (safe for any node ID size).
    /// Use `compact_to_original_id` to map index back to original node ID.
    pub fn dijkstra(&self, start: usize) -> (Vec<f64>, Vec<usize>) {
        if !self.nodes.contains_key(&start) {
            return (Vec::new(), Vec::new());
        }

        let mut distances: HashMap<usize, f64> = self.nodes.keys()
            .map(|&k| (k, f64::INFINITY))
            .collect();
        let mut previous: HashMap<usize, Option<usize>> = self.nodes.keys()
            .map(|&k| (k, None))
            .collect();
        let mut heap = BinaryHeap::new();

        distances.insert(start, 0.0);
        heap.push((-OrderedFloat(0.0), start));

        while let Some((cost, current)) = heap.pop() {
            let cost = -cost.0;
            if cost > distances[&current] { continue; }

            if let Some(neighbors) = self.adj_list.get(&current) {
                for &(next, weight) in neighbors {
                    let alt = distances[&current] + weight;
                    if alt < distances[&next] {
                        distances.insert(next, alt);
                        previous.insert(next, Some(current));
                        heap.push((-OrderedFloat(alt), next));
                    }
                }
            }
        }

        // Build compact distances Vec — safe for any node ID magnitude
        let mut dist_vec = vec![f64::INFINITY; self.compact_to_id.len()];
        for (compact_idx, &orig_id) in self.compact_to_id.iter().enumerate() {
            if let Some(&d) = distances.get(&orig_id) {
                dist_vec[compact_idx] = d;
            }
        }

        let path = self.reconstruct_path(&previous, start);
        (dist_vec, path)
    }

    pub fn astar(&self, start: usize, goal: usize) -> Vec<usize> {
        if !self.nodes.contains_key(&start) || !self.nodes.contains_key(&goal) {
            return Vec::new();
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut g_score: HashMap<usize, f64> = self.nodes.keys()
            .map(|&k| (k, f64::INFINITY))
            .collect();
        let mut f_score: HashMap<usize, f64> = self.nodes.keys()
            .map(|&k| (k, f64::INFINITY))
            .collect();

        g_score.insert(start, 0.0);
        f_score.insert(start, self.heuristic(start, goal));
        open_set.push((-OrderedFloat(f_score[&start]), start));

        while let Some((cost, current)) = open_set.pop() {
            let current_f = -cost.0;

            if current == goal {
                return self.reconstruct_path_from_map(&came_from, current);
            }

            if current_f > f_score[&current] { continue; }

            if let Some(neighbors) = self.adj_list.get(&current) {
                for &(next, weight) in neighbors {
                    let tentative_g = g_score[&current] + weight;
                    if tentative_g < g_score[&next] {
                        came_from.insert(next, current);
                        g_score.insert(next, tentative_g);
                        let f = tentative_g + self.heuristic(next, goal);
                        f_score.insert(next, f);
                        open_set.push((-OrderedFloat(f), next));
                    }
                }
            }
        }

        Vec::new()
    }

    fn heuristic(&self, _from: usize, _to: usize) -> f64 {
        1.0
    }

    /// Returns (distances, has_negative_cycle).
    /// `distances` is a compact Vec indexed 0..num_nodes (safe for any node ID size).
    pub fn bellman_ford(&self, start: usize) -> (Vec<f64>, bool) {
        if !self.nodes.contains_key(&start) {
            return (Vec::new(), false);
        }

        let mut distances: HashMap<usize, f64> = self.nodes.keys()
            .map(|&k| (k, f64::INFINITY))
            .collect();
        distances.insert(start, 0.0);

        for _ in 0..self.nodes.len().saturating_sub(1) {
            for (&u, edges) in &self.adj_list {
                let du = distances[&u];
                if du == f64::INFINITY { continue; }
                for &(v, weight) in edges {
                    if du + weight < distances[&v] {
                        distances.insert(v, du + weight);
                    }
                }
            }
        }

        let mut has_negative_cycle = false;
        'outer: for (&u, edges) in &self.adj_list {
            let du = distances[&u];
            if du == f64::INFINITY { continue; }
            for &(v, weight) in edges {
                if du + weight < distances[&v] {
                    has_negative_cycle = true;
                    break 'outer;
                }
            }
        }

        // Build compact distances Vec — safe for any node ID magnitude
        let mut dist_vec = vec![f64::INFINITY; self.compact_to_id.len()];
        for (compact_idx, &orig_id) in self.compact_to_id.iter().enumerate() {
            if let Some(&d) = distances.get(&orig_id) {
                dist_vec[compact_idx] = d;
            }
        }

        (dist_vec, has_negative_cycle)
    }

    /// Returns MST as flat list of original node ID pairs: [u0, v0, u1, v1, ...]
    pub fn kruskal(&self) -> Vec<usize> {
        let mut edges: Vec<(usize, usize, f64)> = Vec::new();
        for (&u, neighbors) in &self.adj_list {
            for &(v, weight) in neighbors {
                edges.push((u, v, weight));
            }
        }
        edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));

        // UnionFind uses compact indices (0..n) to stay memory-safe
        let n = self.compact_to_id.len();
        let mut uf = UnionFind::new(n);
        let mut mst = Vec::new();

        for (u, v, _) in edges {
            let cu = match self.id_to_compact.get(&u) { Some(&c) => c, None => continue };
            let cv = match self.id_to_compact.get(&v) { Some(&c) => c, None => continue };
            if uf.find(cu) != uf.find(cv) {
                uf.union(cu, cv);
                mst.push(u); // original IDs in output
                mst.push(v);
            }
        }

        mst
    }

    /// Returns the ordered list of original node IDs for compact indices 0..n.
    /// Useful for interpreting the distances Vec returned by dijkstra/bellman_ford.
    pub fn compact_to_original_id(&self) -> &Vec<usize> {
        &self.compact_to_id
    }

    fn reconstruct_path(&self, previous: &HashMap<usize, Option<usize>>, start: usize) -> Vec<usize> {
        let mut path = vec![start];
        let mut current = start;
        while let Some(Some(prev)) = previous.get(&current) {
            path.push(*prev);
            current = *prev;
        }
        path.reverse();
        path
    }

    fn reconstruct_path_from_map(&self, came_from: &HashMap<usize, usize>, current: usize) -> Vec<usize> {
        let mut path = vec![current];
        let mut current = current;
        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }
        path.reverse();
        path
    }

    // ── Undirected connected components ────────────────────────────────────
    pub fn connected_components(&self) -> usize {
        let mut visited: HashSet<usize> = HashSet::new();
        let mut count = 0;
        for &start in self.nodes.keys() {
            if visited.contains(&start) { continue; }
            count += 1;
            let mut queue = VecDeque::new();
            visited.insert(start);
            queue.push_back(start);
            while let Some(n) = queue.pop_front() {
                if let Some(neighbors) = self.adj_list.get(&n) {
                    for &(next, _) in neighbors {
                        if !visited.contains(&next) {
                            visited.insert(next);
                            queue.push_back(next);
                        }
                    }
                }
            }
        }
        count
    }

    // ── Top nodes by out-degree ─────────────────────────────────────────────
    pub fn top_hubs(&self, n: usize) -> Vec<(usize, usize)> {
        let mut degrees: Vec<(usize, usize)> = self.adj_list.iter()
            .map(|(&id, neighbors)| (id, neighbors.len()))
            .collect();
        degrees.sort_by(|a, b| b.1.cmp(&a.1));
        degrees.truncate(n);
        degrees
    }

    // ── PageRank (iterative, with dangling-node handling) ───────────────────
    pub fn pagerank(&self, damping: f64, iterations: u32) -> Vec<(usize, f64)> {
        let n = self.nodes.len();
        if n == 0 { return Vec::new(); }

        let init = 1.0 / n as f64;
        let mut rank: HashMap<usize, f64> = self.nodes.keys().map(|&k| (k, init)).collect();

        for _ in 0..iterations {
            let mut new_rank: HashMap<usize, f64> = self.nodes.keys()
                .map(|&k| (k, (1.0 - damping) / n as f64))
                .collect();

            // Dangling nodes spread their rank equally to all nodes
            let dangling: f64 = self.nodes.keys()
                .filter(|&&id| self.adj_list.get(&id).map_or(true, |v| v.is_empty()))
                .map(|&id| rank[&id])
                .sum::<f64>() * damping / n as f64;
            for r in new_rank.values_mut() { *r += dangling; }

            for (&node, neighbors) in &self.adj_list {
                if neighbors.is_empty() { continue; }
                let contribution = damping * rank[&node] / neighbors.len() as f64;
                for &(next, _) in neighbors {
                    if let Some(r) = new_rank.get_mut(&next) { *r += contribution; }
                }
            }
            rank = new_rank;
        }

        let mut sorted: Vec<(usize, f64)> = rank.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        sorted
    }

    // ── Topological Sort (Kahn's algorithm) ─────────────────────────────────
    /// Returns `None` if the graph contains a cycle.
    pub fn topological_sort(&self) -> Option<Vec<usize>> {
        let mut in_degree: HashMap<usize, usize> = self.nodes.keys().map(|&k| (k, 0)).collect();
        for neighbors in self.adj_list.values() {
            for &(next, _) in neighbors {
                *in_degree.entry(next).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<usize> = in_degree.iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(node) = queue.pop_front() {
            order.push(node);
            if let Some(neighbors) = self.adj_list.get(&node) {
                for &(next, _) in neighbors {
                    let deg = in_degree.get_mut(&next).unwrap();
                    *deg -= 1;
                    if *deg == 0 { queue.push_back(next); }
                }
            }
        }

        if order.len() == self.nodes.len() { Some(order) } else { None }
    }

    // ── Strongly Connected Components (Kosaraju — iterative DFS) ────────────
    pub fn scc(&self) -> Vec<Vec<usize>> {
        // Pass 1: iterative DFS, record finish order
        let mut visited: HashSet<usize> = HashSet::new();
        let mut finish: Vec<usize> = Vec::new();
        for &start in self.nodes.keys() {
            if visited.contains(&start) { continue; }
            let mut stk: Vec<(usize, bool)> = vec![(start, false)];
            while let Some((node, post)) = stk.pop() {
                if post { finish.push(node); continue; }
                if visited.contains(&node) { continue; }
                visited.insert(node);
                stk.push((node, true));
                if let Some(neighbors) = self.adj_list.get(&node) {
                    for &(next, _) in neighbors {
                        if !visited.contains(&next) { stk.push((next, false)); }
                    }
                }
            }
        }

        // Build reverse adjacency list
        let mut rev: HashMap<usize, Vec<usize>> = self.nodes.keys()
            .map(|&id| (id, Vec::new()))
            .collect();
        for (&from, neighbors) in &self.adj_list {
            for &(to, _) in neighbors {
                rev.entry(to).or_insert_with(Vec::new).push(from);
            }
        }

        // Pass 2: DFS on reversed graph in reverse-finish order
        let mut visited2: HashSet<usize> = HashSet::new();
        let mut components: Vec<Vec<usize>> = Vec::new();
        while let Some(start) = finish.pop() {
            if visited2.contains(&start) { continue; }
            let mut component: Vec<usize> = Vec::new();
            let mut stk: Vec<usize> = vec![start];
            while let Some(node) = stk.pop() {
                if visited2.contains(&node) { continue; }
                visited2.insert(node);
                component.push(node);
                if let Some(neighbors) = rev.get(&node) {
                    for &next in neighbors {
                        if !visited2.contains(&next) { stk.push(next); }
                    }
                }
            }
            components.push(component);
        }
        components
    }
}

// ── Union-Find (compact indices only) ─────────────────────────────────────────

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        UnionFind {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx != ry {
            match self.rank[rx].cmp(&self.rank[ry]) {
                Ordering::Less    => self.parent[rx] = ry,
                Ordering::Greater => self.parent[ry] = rx,
                Ordering::Equal   => { self.parent[ry] = rx; self.rank[rx] += 1; }
            }
        }
    }
}
