#!/bin/bash

# Copy the fix to the container
docker cp fix_upload_issue.rs distributed-graph-system-mpi-master-1:/app/src/bin/

# Build and run the fix
docker exec -it distributed-graph-system-mpi-master-1 bash -c "cd /app && cargo run --bin fix_upload_issue"