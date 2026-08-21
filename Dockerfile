# OxideMUD Container Runtime Dockerfile
# Builds a lightweight image using precompiled binaries from the distribution package.
#
# Build context should be the install directory (default: ~/.oxidemud) containing
# bin/ and content/ subdirectories.

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy precompiled game server binary
COPY bin/oxide-server /app/bin/

# Copy game content templates and configs
COPY content/ /app/content/

# Copy the default server config to the base dir root
COPY server.toml /app/server.toml

# Create directories for SQLite persistence and rotating logs
RUN mkdir -p /app/data /app/logs

# Expose the default MUD telnet port (4000) and REST API / WebSocket port (8080)
EXPOSE 4000 8080

# Start the game server binding to 0.0.0.0 so Docker port forwarding works.
# All paths resolve under the base dir (/app).
CMD ["/app/bin/oxide-server", "--host", "0.0.0.0", "--port", "4000", "--base-dir", "/app"]
