#!/bin/bash
# Test the API with curl

# Create a test_graph.txt file if it doesn't exist
if [ ! -f "test_graph.txt" ]; then
  echo "Creating test graph file..."
  cat > test_graph.txt << EOF
0 1 2.0
0 2 4.0
1 2 1.0
1 3 5.0
2 3 3.0
3 4 2.0
4 0 7.0
EOF
fi

echo "Testing health endpoint..."
curl -v http://localhost:8000/health

echo -e "\nTesting BFS algorithm..."
curl -v -X POST http://localhost:8000/process_file \
  -F "file=@test_graph.txt" \
  -F 'request={"algorithm":"bfs","file_format":"edgeList","start_node":0}'

echo -e "\nTesting DFS algorithm..."
curl -v -X POST http://localhost:8000/process_file \
  -F "file=@test_graph.txt" \
  -F 'request={"algorithm":"dfs","file_format":"edgeList","start_node":0}'

echo -e "\nTest complete"