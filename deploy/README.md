# Docker Deployment

This directory contains everything needed to deploy the MCP server using Docker Compose.

## Quick Start

1. Make sure you have a `config.toml` file in the project root (copy from one of the `*_config.toml.example` if needed)
2. Run the deployment:

```bash
cd deploy
docker-compose up -d
```

## Configuration

The docker-compose setup automatically mounts the `config.toml` file from the project root directory into the container. This keeps configuration simple and avoids complex environment variable handling.

### Files

- `docker-compose.yml` - Main Docker Compose configuration
- `Dockerfile` - Multi-stage build for the Rust application

### Port Configuration

The service runs on port 8080 by default. You can change this by:

1. Updating the port in your `config.toml` file
2. Updating the port mapping in `docker-compose.yml` if needed

### Template Usage

This setup is designed to be template-friendly:

- Configuration is file-based, not environment variable based
- Simple volume mount from project root
- No complex build arguments or environment setup
- Easy to customize for different deployments

## Commands

```bash
# Start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down

# Rebuild and restart
docker-compose up -d --build

# Clean up everything
docker-compose down --volumes --rmi local
```
