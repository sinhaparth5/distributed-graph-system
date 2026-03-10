use wasm_bindgen::prelude::*;
use serde::Serialize;
use std::collections::HashMap;

const MAX_NODES: usize = 500;

#[derive(Serialize)]
struct NodeOut {
    id: String,
    label: String,
    degree: u32,
}

#[derive(Serialize)]
struct EdgeOut {
    id: String,
    source: String,
    target: String,
    weight: f64,
}

#[derive(Serialize)]
struct GraphOut {
    nodes: Vec<NodeOut>,
    edges: Vec<EdgeOut>,
    total_nodes: usize,
    total_edges: usize,
    max_degree: u32,
    avg_degree: f64,
    truncated: bool,
}

fn build(
    order: Vec<u64>,
    degrees: HashMap<u64, u32>,
    raw_edges: Vec<(u64, u64, f64)>,
) -> String {
    let total_nodes = order.len();
    let total_edges = raw_edges.len();
    let truncated   = total_nodes > MAX_NODES;

    let display: std::collections::HashSet<u64> =
        order.iter().take(MAX_NODES).cloned().collect();

    let max_degree = degrees.values().copied().max().unwrap_or(1);
    let avg_degree = if total_nodes > 0 {
        degrees.values().map(|&d| d as f64).sum::<f64>() / total_nodes as f64
    } else {
        0.0
    };

    let nodes = order.iter()
        .take(MAX_NODES)
        .map(|&id| NodeOut {
            id:     id.to_string(),
            label:  id.to_string(),
            degree: *degrees.get(&id).unwrap_or(&0),
        })
        .collect();

    let edges = raw_edges.iter()
        .filter(|(u, v, _)| display.contains(u) && display.contains(v))
        .enumerate()
        .map(|(i, &(u, v, w))| EdgeOut {
            id:     format!("e_{}", i),
            source: u.to_string(),
            target: v.to_string(),
            weight: w,
        })
        .collect();

    serde_json::to_string(&GraphOut {
        nodes, edges, total_nodes, total_edges,
        max_degree, avg_degree, truncated,
    })
    .unwrap_or_else(|_| "{}".to_string())
}

/// Parse a 2- or 3-column edge list.
/// Returns a JSON string: { nodes, edges, total_nodes, total_edges, max_degree, avg_degree, truncated }
#[wasm_bindgen]
pub fn parse_edge_list(content: &str) -> String {
    let mut degrees: HashMap<u64, u32> = HashMap::new();
    let mut order: Vec<u64>            = Vec::new();
    let mut raw_edges: Vec<(u64, u64, f64)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
            continue;
        }
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
    let mut degrees: HashMap<u64, u32> = HashMap::new();
    let mut order: Vec<u64>            = Vec::new();
    let mut raw_edges: Vec<(u64, u64, f64)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
            continue;
        }
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
