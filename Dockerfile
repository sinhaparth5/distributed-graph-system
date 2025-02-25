FROM rust:1.85-bullseye as builder

# Install OpenMPI
RUN apt-get update && apt-get install -y \
    libopenmpi-dev \
    openmpi-bin \
    openmpi-common \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the cargo.toml file
COPY Cargo.toml .

# Create empty source files to trick cargo into building dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub mod graph { pub struct Graph; }" > src/lib.rs && \
    echo "pub mod file_processor;" >> src/lib.rs && \
    echo "pub mod mpi_processor;" >> src/lib.rs

# Build dependencies
RUN cargo build --release

# Remove the souce file created for the dummy build
RUN rm -rf src

# Copy the actual source code
COPY src src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install OpenMPI runtime
RUN apt-get update && apt-get install -y \
  libopenmpi-dev \
  openmpi-bin \
  openmpi-common \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/distributed-graph-system /app/distributed-graph-system

# Create temp directory
RUN mkdir -p /app/temp

# Set environment variable
ENV RUST_LOG=info

# Expose the port
EXPOSE 8000

ENTRYPOINT ["mpirun", "-n", "1", "/app/distributed-graph-system"]
