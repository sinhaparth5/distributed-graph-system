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
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: HashMap::new(),
            adj_list: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        let node_id = node.id;
        self.nodes.insert(node_id, node);
        self.adj_list.entry(node_id).or_insert(Vec::new());
    }

    pub fn add_edge(&mut self, edge: Edge) {
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return; // Silently ignore edges with non-existent nodes
        }
        
        self.adj_list
            .entry(edge.from)
            .or_insert(Vec::new())
            .push((edge.to, edge.weight));
    }

    pub fn get_node(&self, id: usize) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn get_neighbors(&self, id: usize) -> Option<&Vec<(usize, f64)>> {
        self.adj_list.get(&id)
    }

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

        if !self.nodes.contains_key(&start) {
            return path;
        }

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
        
        // Use BinaryHeap instead of PriorityQueue
        let mut heap = BinaryHeap::new();

        distances.insert(start, 0.0);
        heap.push((-OrderedFloat(0.0), start));

        while let Some((cost, current)) = heap.pop() {
            let cost = -cost.0; // Convert back to actual cost

            if cost > distances[&current] {
                continue;
            }

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

        let mut dist_vec = vec![f64::INFINITY; self.nodes.len()];
        for (&node, &dist) in distances.iter() {
            if node < dist_vec.len() {
                dist_vec[node] = dist;
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

            if current_f > f_score[&current] {
                continue;
            }

            if let Some(neighbors) = self.adj_list.get(&current) {
                for &(next, weight) in neighbors {
                    let tentative_g_score = g_score[&current] + weight;

                    if tentative_g_score < g_score[&next] {
                        came_from.insert(next, current);
                        g_score.insert(next, tentative_g_score);
                        let f = tentative_g_score + self.heuristic(next, goal);
                        f_score.insert(next, f);
                        open_set.push((-OrderedFloat(f), next));
                    }
                }
            }
        }

        Vec::new()  // No path found
    }

    fn heuristic(&self, from: usize, to: usize) -> f64 {
        // Simple heuristic - can be improved based on your graph's properties
        1.0
    }

    fn reconstruct_path(&self, previous: &HashMap<usize, Option<usize>>, start: usize) -> Vec<usize> {
        let mut path = Vec::new();
        let mut current = start;
        path.push(current);

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

    pub fn bellman_ford(&self, start: usize) -> (Vec<f64>, bool) {
        if !self.nodes.contains_key(&start) {
            return (Vec::new(), false);
        }

        let n = self.nodes.len();
        let mut distances = vec![f64::INFINITY; n];
        distances[start] = 0.0;

        // Relax edges |V| - 1 times
        for _ in 0..n-1 {
            for (&u, edges) in &self.adj_list {
                for &(v, weight) in edges {
                    if distances[u] != f64::INFINITY && distances[u] + weight < distances[v] {
                        distances[v] = distances[u] + weight;
                    }
                }
            }
        }

        // Check for negative weight cycles
        let mut has_negative_cycle = false;
        for (&u, edges) in &self.adj_list {
            for &(v, weight) in edges {
                if distances[u] != f64::INFINITY && distances[u] + weight < distances[v] {
                    has_negative_cycle = true;
                    break;
                }
            }
            if has_negative_cycle {
                break;
            }
        }

        (distances, has_negative_cycle)
    }

    pub fn kruskal(&self) -> Vec<usize> {
        let mut edges = Vec::new();
        for (&u, neighbors) in &self.adj_list {
            for &(v, weight) in neighbors {
                edges.push((u, v, weight));
            }
        }
        edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let mut union_find = UnionFind::new(self.nodes.len());
        let mut mst = Vec::new();

        for (u, v, _) in edges {
            if union_find.find(u) != union_find.find(v) {
                union_find.union(u, v);
                mst.push(u);
                mst.push(v);
            }
        }

        mst
    }
}

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
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x != root_y {
            match self.rank[root_x].cmp(&self.rank[root_y]) {
                std::cmp::Ordering::Less => self.parent[root_x] = root_y,
                std::cmp::Ordering::Greater => self.parent[root_y] = root_x,
                std::cmp::Ordering::Equal => {
                    self.parent[root_y] = root_x;
                    self.rank[root_x] += 1;
                }
            }
        }
    }
}