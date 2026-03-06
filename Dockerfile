FROM ubuntu:22.04

# Install required packages
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    libopenmpi-dev \
    openmpi-bin \
    openssh-server \
    git \
    pkg-config \
    python3 \
    supervisor \
    libclang-dev \
    clang \
    libssl-dev \
    uuid-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Configure SSH for MPI
RUN mkdir /var/run/sshd
RUN echo 'root:password' | chpasswd
RUN sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config
RUN sed -i 's/#StrictHostKeyChecking ask/StrictHostKeyChecking no/' /etc/ssh/ssh_config

# SSH login fix
RUN sed 's@session\s*required\s*pam_loginuid.so@session optional pam_loginuid.so@g' -i /etc/pam.d/sshd

# Create SSH key for passwordless access
RUN mkdir -p /root/.ssh
RUN ssh-keygen -t rsa -N "" -f /root/.ssh/id_rsa
RUN cat /root/.ssh/id_rsa.pub >> /root/.ssh/authorized_keys
RUN chmod 600 /root/.ssh/authorized_keys

# Environment variables for MPI
ENV OMPI_ALLOW_RUN_AS_ROOT=1
ENV OMPI_ALLOW_RUN_AS_ROOT_CONFIRM=1

# Set up working directory
WORKDIR /app

# Expose SSH and Rocket web server ports
EXPOSE 22 8000

# MPI hostfile — used by mpirun on the master to locate worker processes
COPY hostfile /etc/mpi-hostfile

# Copy both supervisor configurations
COPY supervisor-master.conf /etc/supervisor/conf.d/supervisor-master.conf
COPY supervisor-worker.conf /etc/supervisor/conf.d/supervisor-worker.conf

# Create startup script to select appropriate config based on NODE_ROLE
RUN echo '#!/bin/bash\n\
if [ "$NODE_ROLE" = "master" ]; then\n\
    echo "Starting as master node with web server"\n\
    cp /etc/supervisor/conf.d/supervisor-master.conf /etc/supervisor/conf.d/supervisor.conf\n\
else\n\
    echo "Starting as worker node without web server"\n\
    cp /etc/supervisor/conf.d/supervisor-worker.conf /etc/supervisor/conf.d/supervisor.conf\n\
fi\n\
\n\
/usr/bin/supervisord -c /etc/supervisor/supervisord.conf\n'\
> /start.sh

# Make the startup script executable
RUN chmod +x /start.sh

# Run the startup script as the entrypoint
CMD ["/start.sh"]