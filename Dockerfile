# OxideMUD Container Runtime Dockerfile
# Builds a lightweight image using the precompiled binary from the distribution package.

FROM debian:bookworm-slim AS runtime

# Install basic runtime certificates/libs if needed
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the precompiled binary from the package bin directory
COPY bin/oxide-server /app/bin/

# Copy example game content templates and configs
COPY content/ /app/content/

# Create directories for SQLite persistence and rotating logs
RUN mkdir -p /app/data /app/logs

# Expose the default MUD telnet port
EXPOSE 4000

# Set environment variable defaults
ENV OXIDE_CONTENT=/app/content
ENV TMPDIR=/app/logs

# Start the game server binding to 0.0.0.0 so Docker port forwarding works
CMD ["/app/bin/oxide-server", "--host", "0.0.0.0", "--port", "4000", "--config-path", "/app/content/server.toml", "--db-path", "/app/data/oxide.db"]
