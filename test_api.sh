#!/bin/bash
# Distributed Graph System - API Test Script
# Graph: 8 nodes (0-7), 14 directed edges
# Topology: 0->1->3->5->7, shortcuts via 0->2->4->6->7, with cross edges

BASE_URL="http://localhost:8000"
GRAPH_FILE="test_graph.txt"
PASS=0
FAIL=0

# Pretty-print JSON if python3 is available, otherwise raw output
pretty() {
    if command -v python3 &>/dev/null; then
        python3 -m json.tool 2>/dev/null || cat
    else
        cat
    fi
}

run_test() {
    local label="$1"
    local data="$2"
    echo "────────────────────────────────────────"
    echo "TEST: $label"
    local response
    response=$(curl -s -X POST "$BASE_URL/process_file" \
        -F "file=@$GRAPH_FILE" \
        -F "request=$data")
    local exit_code=$?

    if [ $exit_code -ne 0 ]; then
        echo "FAIL: curl error (is the server running?)"
        FAIL=$((FAIL + 1))
        return
    fi

    echo "$response" | pretty

    local verdict
    verdict=$(echo "$response" | python3 -c "
import json, sys
d = json.load(sys.stdin)
if d.get('error'):
    print('FAIL: ' + d['error'])
else:
    print('PASS  [MPI: {} process(es), {} mode]'.format(d.get('mpi_processes','?'), d.get('mpi_mode','?')))
" 2>/dev/null || echo "FAIL: could not parse response")
    echo "$verdict"
    if [[ "$verdict" == PASS* ]]; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
}

# ── Health check ──────────────────────────────
echo "════════════════════════════════════════"
echo "TEST: Health check"
response=$(curl -s "$BASE_URL/health")
if [ "$response" = "OK" ]; then
    echo "Response: $response"
    echo "PASS"
    PASS=$((PASS + 1))
else
    echo "FAIL: expected 'OK', got '$response'"
    FAIL=$((FAIL + 1))
fi

# ── MPI status (check BEFORE process_file calls so MPI is still fresh) ───────
echo "────────────────────────────────────────"
echo "TEST: MPI status"
response=$(curl -s "$BASE_URL/mpi_status")
echo "$response" | pretty
if echo "$response" | python3 -c "import json,sys; d=json.load(sys.stdin); exit(0 if 'mpi_processes' in d else 1)" 2>/dev/null; then
    echo "PASS"
    PASS=$((PASS + 1))
else
    echo "FAIL"
    FAIL=$((FAIL + 1))
fi

# ── Graph traversals ──────────────────────────
run_test "BFS from node 0" \
    '{"algorithm":"bfs","file_format":"edgeList","start_node":0}'

run_test "DFS from node 0" \
    '{"algorithm":"dfs","file_format":"edgeList","start_node":0}'

# ── Shortest paths ────────────────────────────
run_test "Dijkstra from node 0" \
    '{"algorithm":"dijkstra","file_format":"edgeList","start_node":0}'

run_test "Bellman-Ford from node 0" \
    '{"algorithm":"bellman-ford","file_format":"edgeList","start_node":0}'

run_test "A* from node 0 to node 7" \
    '{"algorithm":"astar","file_format":"edgeList","start_node":0,"end_node":7}'

# ── Minimum Spanning Tree ─────────────────────
run_test "Kruskal MST" \
    '{"algorithm":"kruskal","file_format":"edgeList"}'

# ── Edge cases ────────────────────────────────
run_test "BFS from node 5 (mid-graph start)" \
    '{"algorithm":"bfs","file_format":"edgeList","start_node":5}'

run_test "Dijkstra from node 3" \
    '{"algorithm":"dijkstra","file_format":"edgeList","start_node":3}'

run_test "A* from node 2 to node 6" \
    '{"algorithm":"astar","file_format":"edgeList","start_node":2,"end_node":6}'

# ── Summary ───────────────────────────────────
echo "════════════════════════════════════════"
echo "RESULTS: $PASS passed, $FAIL failed"
echo "════════════════════════════════════════"
