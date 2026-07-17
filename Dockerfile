# OxideMUD Container Runtime Dockerfile
# Builds a lightweight image using precompiled binaries from the distribution package.
#
# Build context should be the install directory (default: ~/.oxide) containing
# bin/ and content/ subdirectories.

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies and socat for TCP-to-stdio forwarding (MCP port)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    socat \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy precompiled binaries
COPY bin/oxide-server /app/bin/
COPY bin/oxide-mcp /app/bin/

# Copy game content templates and configs
COPY content/ /app/content/

# Create directories for SQLite persistence and rotating logs
RUN mkdir -p /app/data /app/logs

# Expose the default MUD telnet port and REST API port
EXPOSE 4000 8080

# Set environment variable defaults
ENV OXIDE_CONTENT=/app/content
ENV TMPDIR=/app/logs

# Start the game server binding to 0.0.0.0 so Docker port forwarding works
CMD ["/app/bin/oxide-server", "--host", "0.0.0.0", "--port", "4000", "--config-path", "/app/content/server.toml", "--db-path", "/app/data/oxide.db"]
