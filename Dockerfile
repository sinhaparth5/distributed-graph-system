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

# Set up supervisor to manage services
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Run supervisor as the entrypoint
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/supervisord.conf"]