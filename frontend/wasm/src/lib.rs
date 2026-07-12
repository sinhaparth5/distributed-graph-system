use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const MAX_NODES: usize = 3000;

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeOut {
    id:     String,
    label:  String,
    degree: u32,
}

#[derive(Serialize)]
struct EdgeOut {
    id:     String,
    source: String,
    target: String,
    weight: f64,
}

#[derive(Serialize)]
struct GraphOut {
    nodes:        Vec<NodeOut>,
    edges:        Vec<EdgeOut>,
    total_nodes:  usize,
    total_edges:  usize,
    max_degree:   u32,
    avg_degree:   f64,
    truncated:    bool,
    // Pre-computed adjacency: node_id → [neighbor_ids]
    neighbor_map: HashMap<String, Vec<String>>,
}

// ── Core build helper ─────────────────────────────────────────────────────────

fn build(
    order:     Vec<u64>,
    degrees:   HashMap<u64, u32>,
    raw_edges: Vec<(u64, u64, f64)>,
) -> String {
    let total_nodes = order.len();
    let total_edges = raw_edges.len();
    let truncated   = total_nodes > MAX_NODES;

    let display: HashSet<u64> = order.iter().take(MAX_NODES).cloned().collect();

    let max_degree = degrees.values().copied().max().unwrap_or(1);
    let avg_degree = if total_nodes > 0 {
        degrees.values().map(|&d| d as f64).sum::<f64>() / total_nodes as f64
    } else {
        0.0
    };

    let nodes: Vec<NodeOut> = order.iter()
        .take(MAX_NODES)
        .map(|&id| NodeOut {
            id:     id.to_string(),
            label:  id.to_string(),
            degree: *degrees.get(&id).unwrap_or(&0),
        })
        .collect();

    // Build adjacency map while filtering displayed edges
    let mut neighbor_map: HashMap<String, Vec<String>> = HashMap::new();
    for id in display.iter() {
        neighbor_map.entry(id.to_string()).or_default();
    }

    let edges: Vec<EdgeOut> = raw_edges.iter()
        .filter(|(u, v, _)| display.contains(u) && display.contains(v))
        .enumerate()
        .map(|(i, &(u, v, w))| {
            let us = u.to_string();
            let vs = v.to_string();
            neighbor_map.entry(us.clone()).or_default().push(vs.clone());
            neighbor_map.entry(vs.clone()).or_default().push(us.clone());
            EdgeOut { id: format!("e_{}", i), source: us, target: vs, weight: w }
        })
        .collect();

    serde_json::to_string(&GraphOut {
        nodes, edges, total_nodes, total_edges,
        max_degree, avg_degree, truncated, neighbor_map,
    })
    .unwrap_or_else(|_| "{}".to_string())
}

// ── Graph parsers ─────────────────────────────────────────────────────────────

/// Parse a 2- or 3-column edge list.
/// Returns JSON: { nodes, edges, total_nodes, total_edges, max_degree, avg_degree, truncated, neighbor_map }
#[wasm_bindgen]
pub fn parse_edge_list(content: &str) -> String {
    let mut degrees:   HashMap<u64, u32>       = HashMap::new();
    let mut order:     Vec<u64>                = Vec::new();
    let mut raw_edges: Vec<(u64, u64, f64)>    = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let Ok(u) = parts[0].parse::<u64>() else { continue };
        let Ok(v) = parts[1].parse::<u64>() else { continue };
        let w = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0f64);
        if !degrees.contains_key(&u) { order.push(u); }
        if !degrees.contains_key(&v) { order.push(v); }
        *degrees.entry(u).or_insert(0) += 1;
        *degrees.entry(v).or_insert(0) += 1;
        raw_edges.push((u, v, w));
    }
    build(order, degrees, raw_edges)
}

/// Parse an adjacency list: `node: neighbor1,weight1 neighbor2,weight2 …`
#[wasm_bindgen]
pub fn parse_adjacency_list(content: &str) -> String {
    let mut degrees:   HashMap<u64, u32>    = HashMap::new();
    let mut order:     Vec<u64>             = Vec::new();
    let mut raw_edges: Vec<(u64, u64, f64)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') { continue; }
        let Some(colon) = line.find(':') else { continue };
        let Ok(u) = line[..colon].trim().parse::<u64>() else { continue };
        if !degrees.contains_key(&u) { order.push(u); }
        for part in line[colon + 1..].split_whitespace() {
            let mut it = part.split(',');
            let Ok(v) = it.next().unwrap_or("").parse::<u64>() else { continue };
            let w = it.next().and_then(|s| s.parse().ok()).unwrap_or(1.0f64);
            if !degrees.contains_key(&v) { order.push(v); }
            *degrees.entry(u).or_insert(0) += 1;
            *degrees.entry(v).or_insert(0) += 1;
            raw_edges.push((u, v, w));
        }
    }
    build(order, degrees, raw_edges)
}

// ── Neighbor map (standalone) ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct EdgeIn { source: String, target: String }

/// Build an undirected adjacency map from a JSON edge array.
/// Input:  [{source, target}, …]
/// Output: {nodeId: [neighborId, …], …}
#[wasm_bindgen]
pub fn build_neighbor_map(edges_json: &str) -> String {
    let edges: Vec<EdgeIn> = serde_json::from_str(edges_json).unwrap_or_default();
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for e in &edges {
        map.entry(e.source.clone()).or_default().push(e.target.clone());
        map.entry(e.target.clone()).or_default().push(e.source.clone());
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

// ── Path sets ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PathSets {
    path_node_set: Vec<String>,
    path_edge_set: Vec<String>,
    mst_edge_set:  Vec<String>,
    path_array:    Vec<String>,
}

/// Compute node/edge highlight sets from a result path.
/// Input:  path_json = JSON number array, algorithm = string
/// Output: {path_node_set, path_edge_set, mst_edge_set, path_array}
#[wasm_bindgen]
pub fn build_path_sets(path_json: &str, algorithm: &str) -> String {
    let path: Vec<i64> = serde_json::from_str(path_json).unwrap_or_default();
    let mut path_node_set: HashSet<String> = HashSet::new();
    let mut path_edge_set: HashSet<String> = HashSet::new();
    let mut mst_edge_set:  HashSet<String> = HashSet::new();

    if algorithm == "kruskal" {
        let mut i = 0;
        while i + 1 < path.len() {
            let u = path[i].to_string();
            let v = path[i + 1].to_string();
            path_node_set.insert(u.clone());
            path_node_set.insert(v.clone());
            mst_edge_set.insert(format!("{}-{}", u, v));
            mst_edge_set.insert(format!("{}-{}", v, u));
            i += 2;
        }
    } else {
        for &id in &path { path_node_set.insert(id.to_string()); }
        for i in 0..path.len().saturating_sub(1) {
            let u = path[i].to_string();
            let v = path[i + 1].to_string();
            path_edge_set.insert(format!("{}-{}", u, v));
            path_edge_set.insert(format!("{}-{}", v, u));
        }
    }

    serde_json::to_string(&PathSets {
        path_node_set: path_node_set.into_iter().collect(),
        path_edge_set: path_edge_set.into_iter().collect(),
        mst_edge_set:  mst_edge_set.into_iter().collect(),
        path_array:    path.iter().map(|id| id.to_string()).collect(),
    })
    .unwrap_or_else(|_| "{}".to_string())
}

// ── Node style computation ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct NodeIn { id: String, degree: u32 }

#[derive(Serialize)]
struct NodeStyle { id: String, fill: String, size: f64 }

#[inline]
fn degree_color(degree: u32, max_deg: u32) -> &'static str {
    let r = if max_deg > 0 { degree as f64 / max_deg as f64 } else { 0.0 };
    if r > 0.95 { "#f97316" }
    else if r > 0.80 { "#f59e0b" }
    else if r > 0.40 { "#22d3ee" }
    else { "#0891b2" }
}

#[inline]
fn degree_size(degree: u32, max_deg: u32) -> f64 {
    let r = if max_deg > 0 { degree as f64 / max_deg as f64 } else { 0.0 };
    2.0 + r * 8.0
}

const SCC_FILLS: &[&str] = &[
    "#22d3ee", "#a855f7", "#f59e0b", "#4ade80",
    "#f87171", "#38bdf8", "#fb923c",
];

/// Compute fill color + size for every visible node in one WASM call.
///
/// Inputs (all strings):
///   nodes_json       [{id, degree}]
///   max_degree       u32
///   algorithm        e.g. "bfs", "pagerank", "scc", ""
///   pagerank_json    [[nodeId_number, score_f64], …]  (sorted desc by score)
///   scc_json         [[nodeId_number, …], …]          (component arrays)
///   path_nodes_json  [nodeId_string, …]               (currently visible path)
///   start_node       string
///   end_node         string
///
/// Output: [{id, fill, size}]
#[wasm_bindgen]
pub fn compute_node_styles(
    nodes_json:       &str,
    max_degree:       u32,
    algorithm:        &str,
    pagerank_json:    &str,
    scc_json:         &str,
    path_nodes_json:  &str,
    start_node:       &str,
    end_node:         &str,
) -> String {
    let nodes: Vec<NodeIn> = serde_json::from_str(nodes_json).unwrap_or_default();

    // PageRank map: id → normalised 0..1
    let mut pr_map: HashMap<String, f64> = HashMap::new();
    if algorithm == "pagerank" {
        if let Ok(scores) = serde_json::from_str::<Vec<(i64, f64)>>(pagerank_json) {
            let max_score = scores.first().map(|s| s.1).unwrap_or(1.0).max(1e-10);
            for (id, score) in scores {
                pr_map.insert(id.to_string(), score / max_score);
            }
        }
    }

    // SCC map: id → component index
    let mut scc_map: HashMap<String, usize> = HashMap::new();
    if algorithm == "scc" {
        if let Ok(comps) = serde_json::from_str::<Vec<Vec<i64>>>(scc_json) {
            for (idx, comp) in comps.iter().enumerate() {
                for id in comp { scc_map.insert(id.to_string(), idx); }
            }
        }
    }

    // Visible path nodes set
    let path_nodes: Vec<String> = serde_json::from_str(path_nodes_json).unwrap_or_default();
    let path_set: HashSet<&str> = path_nodes.iter().map(|s| s.as_str()).collect();

    let styles: Vec<NodeStyle> = nodes.iter().map(|n| {
        let fill = if algorithm == "pagerank" {
            let rank = pr_map.get(&n.id).copied().unwrap_or(0.0);
            if rank > 0.8      { "#a855f7".to_string() }
            else if rank > 0.5 { "#7c3aed".to_string() }
            else if rank > 0.2 { "#22d3ee".to_string() }
            else               { "#0891b2".to_string() }
        } else if algorithm == "scc" && !scc_map.is_empty() {
            match scc_map.get(&n.id) {
                Some(&idx) => SCC_FILLS[idx % SCC_FILLS.len()].to_string(),
                None       => "#334155".to_string(),
            }
        } else {
            // Degree-based baseline + path overlay
            let mut color = degree_color(n.degree, max_degree).to_string();
            if path_set.contains(n.id.as_str()) {
                if n.id == start_node {
                    color = "#4ade80".to_string();
                } else if n.id == end_node && algorithm == "astar" {
                    color = "#f87171".to_string();
                } else {
                    color = "#22d3ee".to_string();
                }
            }
            color
        };

        NodeStyle {
            id:   n.id.clone(),
            fill,
            size: degree_size(n.degree, max_degree),
        }
    }).collect();

    serde_json::to_string(&styles).unwrap_or_else(|_| "[]".to_string())
}

// ── Edge style computation ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct EdgeStyleIn { id: String, source: String, target: String }

#[derive(Serialize)]
struct EdgeStyle { id: String, fill: String, size: f64 }

/// Compute fill + size for every edge based on path/MST membership.
/// Input:  edges_json  [{id, source, target}]
///         path_edge_set_json  ["u-v", …]   (bidirectional keys)
///         mst_edge_set_json   ["u-v", …]
/// Output: [{id, fill, size}]
#[wasm_bindgen]
pub fn compute_edge_styles(
    edges_json:          &str,
    path_edge_set_json:  &str,
    mst_edge_set_json:   &str,
) -> String {
    let edges: Vec<EdgeStyleIn> = serde_json::from_str(edges_json).unwrap_or_default();
    let path_set: HashSet<String> = serde_json::from_str(path_edge_set_json).unwrap_or_default();
    let mst_set:  HashSet<String> = serde_json::from_str(mst_edge_set_json).unwrap_or_default();

    let styles: Vec<EdgeStyle> = edges.iter().map(|e| {
        let fwd  = format!("{}-{}", e.source, e.target);
        let rev  = format!("{}-{}", e.target, e.source);
        let is_path = path_set.contains(&fwd) || path_set.contains(&rev);
        let is_mst  = mst_set.contains(&fwd)  || mst_set.contains(&rev);

        EdgeStyle {
            id:   e.id.clone(),
            fill: if is_mst        { "#4ade80".to_string() }
                  else if is_path  { "#22d3ee".to_string() }
                  else             { "#1e3a5f".to_string() },
            size: if is_mst || is_path { 2.0 } else { 0.7 },
        }
    }).collect();

    serde_json::to_string(&styles).unwrap_or_else(|_| "[]".to_string())
}
