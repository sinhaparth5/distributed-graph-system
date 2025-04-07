#!/bin/bash

# Check if Docker Swarm is initialized
if ! docker info | grep -q "Swarm: active"; then
  echo "initializing Docker Swarm..."
  docker swarm init

  # Generate a worker join token and display it
  JOIN_TOKEN=$(docker swarm join-token worker -q)
  echo "To add worker nodes to this swarm, run the following command on each machine:"
  echo "docker swarm join --token $JOIN_TOKEN $(hostname -I | awk '{print $1}'):2377"
  echo ""
  echo "Make sure to add at least 2 worker nodes continuing."
  echo "Press Enter when you have added the worker nodes..."
  read
fi

# Build the docker images locally
docker build -t distributed-graph-system:latest .

#Create the MPI hostfile on the manager node
cat >mpi-hostfile <<EOF
mpi-master slots=1
mpi-worker.1 slots=1
mpi-worker.2 slots=1
EOF

# Deploy the stack
echo "Deploying to Docker Swarm..."
docker stack deploy -c compose.swarm.yml mpi-graph-system

echo "Deployment initiated. Stack name: mpi-graph-system"
echo ""
echo "To check the status of the deployment, run:"
echo "  docker service ls"
echo ""
echo "To view logs from the master node, run:"
echo "  docker services logs mpi-graph-system_mpi-master"
echo ""
echo "To remove the stack, run:"
echo "  docker stack rm mpi-graph-system"
