use serde::{ Deserialize, Serialize };
use std::collections::{HashSet, VecDeque, HashMap};

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

pub struct Graph {
    nodes: HashMap<usize, Node>,
    adj_list: HashMap<usize, Vec<(usize, f64)>>
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: HashMap::new(),
            adj_list: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id, node.clone());
        self.adj_list.entry(node.id).or_insert(Vec::new());
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.adj_list
            .entry(edge.from)
            .or_insert(Vec::new())
            .push((edge.to, edge.weight));
    }

    pub fn dfs(&self, start: usize) -> Vec<usize> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        self.dfs_util(start, &mut visited, &mut path);
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
        path // Return the path of visited nodes
    }

    pub fn dijkstra(&self, start: usize) -> (Vec<f64>, Vec<usize>) {
        let mut distances: HashMap<usize, f64> = HashMap::new();
        let mut prev: HashMap<usize, usize> = HashMap::new();
        let mut pq = priority_queue::PriorityQueue::new();
        let mut visited = HashSet::new();

        for &node_id in self.nodes.keys() {
            distances.insert(node_id, f64::INFINITY);
        }
        distances.insert(start, 0.0);
        pq.push(start, std::cmp::Reverse(0.0));

        while let Some((current, _)) = pq.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(neighbors) = self.adj_list.get(&current) {
                for &(next, weight) in neighbors {
                    let new_dist = distances[&current] + weight;
                    if new_dist < distances[&next] {
                        distances.insert(next, new_dist);
                        prev.insert(next, current);
                        pq.push(next, std::cmp::Reverse(new_dist));
                    }
                }
            }
        }

        let mut dist_vec = vec![f64::INFINITY; self.nodes.len()];
        for (k, v) in distances {
            dist_vec[k] = v;
        }
        (dist_vec, Vec::new())
    }

    pub fn astar(&self, start: usize, goal: usize) -> Vec<usize> {
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut g_score: HashMap<usize, f64> = HashMap::new();
        let mut f_score: HashMap<usize, f64> = HashMap::new();
        let mut open_set = priority_queue::PriorityQueue::new();

        g_score.insert(start, 0.0);
        f_score.insert(start, self.heuristic(start, goal));
        open_set.push(start, std::cmp::Reverse(f_score[&start]));

        while let Some((current, _)) = open_set.pop() {
            if current == goal {
                return self.reconstruct_path(&came_from, current);
            }

            if let Some(neighbours) = self.adj_list.get(&current) {
                for &(next, weight) in neighbours {
                    let tentative_g_score = g_score[&current] + weight;

                    if tentative_g_score < *g_score.get(&next).unwrap_or(&f64::INFINITY) {
                        came_from.insert(next, current);
                        g_score.insert(next, tentative_g_score);
                        let f = tentative_g_score + self.heuristic(next, goal);
                        f_score.insert(next, f);
                        open_set.push(next, std::cmp::Reverse(f));
                    }
                }
            }
        }
        Vec::new() // No path found
    }

    fn heuristic(&self, from: usize, to: usize) -> f64 {
        1.0
    }

    fn reconstruct_path(&self, came_from: &HashMap<usize, usize>, current: usize) -> Vec<usize> {
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
        let n = self.nodes.len();
        let mut distances = vec![f64::INFINITY; n];
        distances[start] = 0.0;

        for _ in 0..n-1 {
            for (u, edges) in &self.adj_list {
                for &(v, weight) in edges {
                    if distances[*u] != f64::INFINITY && distances[*u] + weight < distances[v] {
                        distances[v] = distances[*u] + weight;
                    }
                }
            }
        }

        let mut has_negative_cycle = false;
        for (u, edges) in &self.adj_list {
            for &(v, weight) in edges {
                if distances[*u] != f64::INFINITY && distances[*u] + weight < distances[v] {
                    has_negative_cycle = true;
                    break;
                }
            }
        }

        (distances, has_negative_cycle)
    }

    pub fn kruskal(&self) -> Vec<usize> {
        let mut edges: Vec<(usize, usize, f64)> = Vec::new();
        for (u, neighbours) in &self.adj_list {
            for &(v, weight) in neighbours {
                edges.push((*u, v, weight));
            }
        }
        edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

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

// Union-Find data structure for Kruskal's algorithm
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