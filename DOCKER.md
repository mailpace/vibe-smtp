# Docker Multi-Port Setup Guide

This guide explains how to use Vibe Gateway with Docker in multi-port mode, supporting all standard SMTP ports with appropriate TLS configurations.

## Overview

The Docker multi-port setup provides industry-standard SMTP port configurations:

| Port | Protocol | Description | TLS Mode | Use Case |
|------|----------|-------------|----------|----------|
| **25** | SMTP | Standard mail transfer | STARTTLS optional | Mail servers, relay agents |
| **587** | Submission | Message submission | STARTTLS optional | Email clients (recommended) |
| **2525** | Alternative | Development/testing | STARTTLS optional | Development, non-standard setups |
| **465** | SMTPS | SMTP over SSL | Implicit TLS | Legacy email clients |

## Quick Start

### Option 1: Helper Script (Recommended)

```bash
# Start multi-port mode
./docker-run.sh multi-port --token your_mailpace_api_token

# Start with debug logging
./docker-run.sh multi-port --dev --token your_mailpace_api_token

# Start single-port mode (port 2525 only)
./docker-run.sh single-port --token your_mailpace_api_token
```

### Option 2: Docker Compose

```bash
# Copy and configure environment
cp .env.example .env
# Edit .env file with your API token

# Start services
docker-compose up -d

# View logs
docker-compose logs -f
```

### Option 3: Direct Docker Commands

```bash
# Build image
docker build -t vibe-gateway .

# Run multi-port mode
docker run -d \
  -p 25:25 -p 587:587 -p 2525:2525 -p 465:465 \
  -e MAILPACE_API_TOKEN=your_token \
  vibe-gateway --docker-multi-port

# Run single-port mode
docker run -d \
  -p 2525:2525 \
  -e MAILPACE_API_TOKEN=your_token \
  vibe-gateway --listen 0.0.0.0:2525 --enable-tls
```

## Port-Specific Configuration

### Port 25 - Standard SMTP
- **Purpose**: Traditional SMTP mail transfer
- **TLS**: STARTTLS optional
- **Use**: Mail servers, postfix relay
- **Client Config**: 
  ```
  Host: your_server
  Port: 25
  Encryption: None/STARTTLS
  ```

### Port 587 - Message Submission (Recommended)
- **Purpose**: Email client message submission
- **TLS**: STARTTLS optional (recommended)
- **Use**: Email clients, applications
- **Client Config**:
  ```
  Host: your_server
  Port: 587
  Encryption: STARTTLS
  Auth: Required
  ```

### Port 2525 - Alternative SMTP
- **Purpose**: Development and non-standard setups
- **TLS**: STARTTLS optional
- **Use**: Development, firewalled environments
- **Client Config**:
  ```
  Host: your_server
  Port: 2525
  Encryption: None/STARTTLS
  ```

### Port 465 - SMTP over SSL
- **Purpose**: Legacy SMTP over SSL
- **TLS**: Implicit TLS (no STARTTLS command)
- **Use**: Legacy email clients
- **Client Config**:
  ```
  Host: your_server
  Port: 465
  Encryption: SSL/TLS
  Auth: Required
  ```

## TLS Certificate Management

### Development (Default)
The Docker image includes self-signed test certificates for development:
- `test_cert.pem` - Certificate file
- `test_key.pem` - Private key file

### Production
For production deployments, mount your own certificates:

```bash
# Using Docker run
docker run -d \
  -p 25:25 -p 587:587 -p 2525:2525 -p 465:465 \
  -v /path/to/your/cert.pem:/app/test_cert.pem:ro \
  -v /path/to/your/key.pem:/app/test_key.pem:ro \
  -e MAILPACE_API_TOKEN=your_token \
  vibe-gateway --docker-multi-port
```

```yaml
# Using Docker Compose override
# docker-compose.override.yml
services:
  vibe-gateway:
    volumes:
      - ./production_cert.pem:/app/test_cert.pem:ro
      - ./production_key.pem:/app/test_key.pem:ro
```

## Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `MAILPACE_API_TOKEN` | MailPace API token | None | Optional* |
| `MAILPACE_ENDPOINT` | MailPace API endpoint | `https://app.mailpace.com/api/v1/send` | No |

*Required if users don't authenticate with their own tokens via SMTP AUTH

## Health Checks

The Docker container includes built-in health checks:

```bash
# Check container health
docker inspect --format='{{.State.Health.Status}}' container_name

# Manual health check
docker exec container_name timeout 5 bash -c '</dev/tcp/localhost/2525'
```

Health check tests SMTP connectivity on port 2525 every 30 seconds.

## Testing

### Automated Testing
```bash
# Run comprehensive multi-port test
./test-docker-multiport.sh
```

### Manual Testing
```bash
# Test SMTP connection on each port
telnet localhost 25
telnet localhost 587
telnet localhost 2525
telnet localhost 465  # Will show TLS handshake
```

### Python Test Script
```python
# test_multiport.py
import smtplib
import ssl

def test_port(port, use_tls=False):
    try:
        if port == 465:  # Implicit TLS
            context = ssl.create_default_context()
            server = smtplib.SMTP_SSL('localhost', port, context=context)
        else:
            server = smtplib.SMTP('localhost', port)
            if use_tls:
                server.starttls()
        
        server.login('your_api_token', 'your_api_token')
        print(f"✓ Port {port} connection successful")
        server.quit()
    except Exception as e:
        print(f"✗ Port {port} connection failed: {e}")

# Test all ports
test_port(25)
test_port(587, use_tls=True)
test_port(2525)
test_port(465)  # Implicit TLS
```

## Monitoring and Logs

### View Logs
```bash
# Docker run logs
docker logs container_name

# Docker Compose logs
docker-compose logs -f vibe-gateway

# Follow specific service logs
docker-compose logs -f --tail=100 vibe-gateway
```

### Log Levels
- Production: `INFO` level (default)
- Development: `DEBUG` level (use `--dev` flag)

### Key Log Messages
- `SMTP server listening on X.X.X.X:PORT (MODE)` - Server started
- `New connection from X.X.X.X on X.X.X.X:PORT` - Client connected
- `TLS connection established` - STARTTLS successful
- `Failed to establish implicit TLS connection` - TLS handshake failed

## Production Deployment

### Security Considerations
1. **Firewall**: Only expose necessary ports (typically 587 and 465)
2. **TLS Certificates**: Use valid certificates from a trusted CA
3. **API Token**: Store securely, use environment variables or secrets
4. **Network**: Consider running behind a reverse proxy
5. **Monitoring**: Set up log aggregation and alerting

### Scaling
```yaml
# docker-compose.yml for load balancing
services:
  vibe-gateway-1:
    build: .
    ports:
      - "25:25"
      - "587:587"
    # ... config
  
  vibe-gateway-2:
    build: .
    ports:
      - "1025:25"    # Alternative port mapping
      - "1587:587"
    # ... config
  
  nginx:
    image: nginx
    # Configure load balancing
```

### Docker Swarm / Kubernetes
For container orchestration, see the `k8s/` directory for Kubernetes manifests and Helm charts.

## Troubleshooting

### Common Issues

**Port already in use**
```bash
# Find what's using the port
lsof -i :25
netstat -tulpn | grep :25

# Kill the process or change port mapping
docker run -p 1025:25 ...  # Map to different host port
```

**TLS certificate errors**
```bash
# Verify certificate files
openssl x509 -in test_cert.pem -text -noout
openssl rsa -in test_key.pem -check

# Generate new test certificates
openssl req -x509 -newkey rsa:4096 -keyout test_key.pem -out test_cert.pem -days 365 -nodes
```

**Connection refused**
```bash
# Check if container is running
docker ps

# Check container logs
docker logs container_name

# Test network connectivity
docker exec container_name netstat -tlpn
```

**Health check failing**
```bash
# Check health status
docker inspect --format='{{.State.Health}}' container_name

# Run health check manually
docker exec container_name timeout 5 bash -c '</dev/tcp/localhost/2525'
```

### Debug Mode

Enable debug logging for detailed troubleshooting:

```bash
# Using helper script
./docker-run.sh multi-port --dev --token your_token

# Using Docker directly
docker run -e MAILPACE_API_TOKEN=your_token vibe-gateway --docker-multi-port --debug
```

Debug mode provides:
- Detailed SMTP command logging
- TLS handshake information
- Email parsing details
- API request/response logging

## Support

For issues and questions:
- GitHub Issues: [vibe-smtp repository](https://github.com/mailpace/vibe-smtp)
- Documentation: [MailPace Docs](https://docs.mailpace.com)
- Email: support@mailpace.com
