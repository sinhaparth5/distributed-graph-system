# Rust + MPI base image
FROM rust:latest

# Install OpenMPI
RUN apt-get update && \
    apt-get install -y openmpi-bin libopenmpi-dev && \
    rm -rf /var/lib/apt/lists/*

# Allow MPI to run as root in containers
ENV OMPI_ALLOW_RUN_AS_ROOT=1
ENV OMPI_ALLOW_RUN_AS_ROOT_CONFRIM=1

WORKDIR /app
COPY . .

# Build with rsmpi 
RUN cargo build --release

# Expose Rocket and MPI ports
EXPOSE 8000
# MPI port range
EXPOSE 10000-10100 