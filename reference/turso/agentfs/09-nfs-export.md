# NFS Export

## Overview

AgentFS can export workspaces as NFS (Network File System) shares, allowing traditional applications and systems to access workspace contents without knowing about AgentFS.

## NFS Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   NFS Export Architecture                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Client Systems                 AgentFS Server              │
│  ┌──────────────┐              ┌──────────────────┐        │
│  │  Application │              │  NFS Server      │        │
│  │  (standard)  │◄────────────►│  ┌────────────┐  │        │
│  └──────────────┘   NFS v4     │  │ Workspace  │  │        │
│                                 │  │  Files     │  │        │
│  ┌──────────────┐              │  └────────────┘  │        │
│  │  Editor      │              │       │          │        │
│  │  (VS Code)   │◄────────────►│  ┌────▼────┐     │        │
│  └──────────────┘              │  │ AgentFS │     │        │
│                                 │  │ Engine  │     │        │
│  ┌──────────────┐              │  └─────────┘     │        │
│  │  CI/CD       │              └──────────────────┘        │
│  │  (Jenkins)   │                                          │
│  └──────────────┘                                          │
│                                                             │
│  Benefits:                                                  │
│  • No SDK required on clients                               │
│  • Works with any NFS-capable application                   │
│  • Transparent CoW semantics                                │
│  • Audit logging still active                               │
└─────────────────────────────────────────────────────────────┘
```

## Enabling NFS Export

### CLI Setup

```bash
# Export workspace via NFS
agentfs nfs export my-workspace \
  --address 0.0.0.0 \
  --port 2049 \
  --path /exports/my-workspace

# Export with authentication
agentfs nfs export my-workspace \
  --auth kerberos \
  --allow-hosts 192.168.1.0/24

# List NFS exports
agentfs nfs list

# Stop NFS export
agentfs nfs unexport my-workspace
```

### Configuration File

```toml
# ~/.config/agentfs/nfs.toml
[nfs.server]
enabled = true
address = "0.0.0.0"
port = 2049

[nfs.exports.my-workspace]
workspace = "my-workspace"
path = "/exports/my-workspace"
read_only = false
allow_hosts = ["192.168.1.0/24", "10.0.0.0/8"]
squash = "root_squash"
```

## Mounting NFS Shares

### Linux

```bash
# Mount AgentFS workspace
sudo mount -t nfs4 \
  agentfs-server:/exports/my-workspace \
  /mnt/my-workspace

# With options
sudo mount -t nfs4 \
  -o rw,hard,intr,rsize=8192,wsize=8192 \
  agentfs-server:/exports/my-workspace \
  /mnt/my-workspace

# Add to /etc/fstab for persistence
agentfs-server:/exports/my-workspace /mnt/my-workspace nfs4 defaults 0 0
```

### macOS

```bash
# Mount via NFS
sudo mount -t nfs \
  -o resvport \
  agentfs-server:/exports/my-workspace \
  /Volumes/my-workspace

# Or use Finder
# Go → Connect to Server → nfs://agentfs-server/exports/my-workspace
```

### Docker

```bash
# Mount in container
docker run -v agentfs-server:/exports/my-workspace:/workspace \
  my-image

# Or in docker-compose
volumes:
  - type: nfs
    source: agentfs-server:/exports/my-workspace
    target: /workspace
```

## Use Cases

### IDE Integration

```bash
# Export workspace for IDE
agentfs nfs export my-project \
  --port 2049 \
  --allow-hosts 127.0.0.1

# Mount in VS Code
# Use Remote-SSH or direct NFS mount
# Edit files normally - changes go through AgentFS
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Mount AgentFS workspace
        run: |
          sudo apt-get install nfs-common
          sudo mount -t nfs4 \
            agentfs-server:/exports/test-workspace \
            /workspace
      
      - name: Run tests
        run: |
          cd /workspace
          npm test
```

### Legacy Application Integration

```bash
# Export for legacy app that doesn't support AgentFS
agentfs nfs export legacy-workspace \
  --read-only \
  --allow-hosts 10.0.0.10

# Legacy app reads from NFS mount
# Changes tracked by AgentFS
```

## Security

### Authentication Methods

**Anonymous (development only):**
```bash
agentfs nfs export my-workspace --auth none
```

**Kerberos (production):**
```bash
agentfs nfs export my-workspace \
  --auth kerberos \
  --kerberos-realm EXAMPLE.COM
```

**TLS (encrypted transport):**
```bash
agentfs nfs export my-workspace \
  --tls-cert /path/to/cert.pem \
  --tls-key /path/to/key.pem
```

### Access Control

```bash
# Restrict by IP
agentfs nfs export my-workspace \
  --allow-hosts 192.168.1.0/24,10.0.0.50

# Read-only access
agentfs nfs export my-workspace --read-only

# Root squashing (recommended)
agentfs nfs export my-workspace --squash root

# All squashing
agentfs nfs export my-workspace --squash all
```

### Firewall Configuration

```bash
# Allow NFS traffic
sudo ufw allow from 192.168.1.0/24 to any port 2049

# Or iptables
sudo iptables -A INPUT -p tcp -s 192.168.1.0/24 --dport 2049 -j ACCEPT
```

## Performance Tuning

### Mount Options

```bash
# Optimized for read-heavy workloads
sudo mount -t nfs4 \
  -o rsize=1048576,wsize=1048576,hard,timeo=600,retrans=2 \
  agentfs-server:/exports/my-workspace \
  /mnt/my-workspace

# Optimized for write-heavy workloads
sudo mount -t nfs4 \
  -o rsize=32768,wsize=32768,async,noatime \
  agentfs-server:/exports/my-workspace \
  /mnt/my-workspace
```

### Server Options

```toml
[nfs.server]
threads = 16
read_buffer_size = 1048576
write_buffer_size = 1048576
max_read_size = 1048576
max_write_size = 1048576
```

### Caching

```bash
# Enable attribute caching
sudo mount -t nfs4 \
  -o actimeo=60 \
  agentfs-server:/exports/my-workspace \
  /mnt/my-workspace

# Or in /etc/fstab
agentfs-server:/exports/my-workspace /mnt/my-workspace nfs4 actimeo=60 0 0
```

## Monitoring

### Server Status

```bash
# Check NFS server status
agentfs nfs status

# Output:
# NFS Server: running
# Address: 0.0.0.0:2049
# Exports: 3
# Connections: 12
```

### Client Connections

```bash
# List connected clients
agentfs nfs clients

# Output:
# Client: 192.168.1.100
#   Workspace: my-workspace
#   Mount: /mnt/my-workspace
#   Connected: 2h 30m
```

### Performance Metrics

```bash
# Get NFS metrics
agentfs nfs metrics

# Output:
# Read throughput: 45.2 MB/s
# Write throughput: 12.8 MB/s
# Latency (avg): 2.3ms
# Connections: 12
```

## Troubleshooting

### Connection Issues

```bash
# Test connectivity
showmount -e agentfs-server

# Check if port is open
nc -zv agentfs-server 2049

# Verify export exists
agentfs nfs list
```

### Mount Failures

```bash
# Check mount errors
sudo mount -v -t nfs4 agentfs-server:/exports/my-workspace /mnt/test

# Common fixes:
# 1. Install nfs-common
sudo apt-get install nfs-common

# 2. Check firewall
sudo ufw status

# 3. Verify permissions
ls -la /exports/my-workspace
```

### Performance Issues

```bash
# Check NFS stats
nfsstat -s

# Monitor in real-time
nfsiostat 1

# Check network latency
ping agentfs-server
```

## CLI Reference

```bash
# Export management
agentfs nfs export <workspace> [options]
agentfs nfs unexport <workspace>
agentfs nfs list

# Server management
agentfs nfs start [options]
agentfs nfs stop
agentfs nfs restart
agentfs nfs status

# Client management
agentfs nfs clients
agentfs nfs disconnect <client-id>

# Monitoring
agentfs nfs metrics
agentfs nfs logs
```

## Limitations

- **Symbolic links**: May not work as expected across workspaces
- **Hard links**: Not supported
- **File locking**: Advisory locking only
- **Permissions**: Unix permissions mapped to AgentFS ACLs
- **Special files**: Device files, pipes not supported

## Best Practices

1. **Use read-only exports** when possible
2. **Enable Kerberos** in production
3. **Restrict by IP** to trusted hosts
4. **Monitor connections** regularly
5. **Use TLS** for sensitive data
6. **Set appropriate mount options** for workload

## Next Steps

- [Security](./10-security.md)
- [Cloud Sync](./08-cloud-sync.md)
- [Turso Cloud](../../turso-cloud/01-overview.md)