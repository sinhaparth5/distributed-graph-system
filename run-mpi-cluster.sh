#!/bin/bash

# Build all Docker images
echo "Building Docker images..."
docker-compose build

# Create shared temp directory if it doesn't exist
mkdir -p temp

# Start the MPI cluster
echo "Starting MPI Cluster..."
docker-compose up -d

echo "MPI cluster is now running"
echo "Master node is accessible at http://localhost:8000"
echo ""
echo "To view logs, run:"
echo "  docker-compose logs -f"
echo ""
echo "To stop the cluster, run:"
echo "  docker-compose down"
