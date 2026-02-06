# Security

## Overview

AgentFS provides multiple layers of security to protect your data and ensure safe operation in multi-agent environments.

## Security Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Architecture                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Layer 1: Access Control                                    │
│  ├─ Workspace permissions                                   │
│  ├─ User authentication                                     │
│  └─ API token management                                    │
│                                                             │
│  Layer 2: Data Protection                                   │
│  ├─ Encryption at rest                                      │
│  ├─ Encryption in transit                                   │
│  └─ Secure key management                                   │
│                                                             │
│  Layer 3: Execution Security                                │
│  ├─ Sandboxing                                              │
│  ├─ Resource limits                                         │
│  └─ Audit logging                                           │
│                                                             │
│  Layer 4: Network Security                                  │
│  ├─ TLS/SSL                                                 │
│  ├─ IP whitelisting                                         │
│  └─ Firewall rules                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Access Control

### Workspace Permissions

```bash
# Create read-only workspace
agentfs workspace create readonly-ws --read-only

# Restrict to specific agent
agentfs workspace config my-workspace \
  --agent-id agent-123 \
  --expires "2024-02-01T00:00:00Z"

# Workspace ownership
agentfs workspace config my-workspace \
  --owner user@example.com
```

### API Token Security

```bash
# Create scoped token
agentfs auth token create \
  --name "CI/CD Token" \
  --permissions workspace:read,workspace:write \
  --expires 30d \
  --allow-ip 192.168.1.0/24

# Rotate tokens regularly
agentfs auth token rotate <token-id>

# Revoke compromised token
agentfs auth token revoke <token-id>
```

### Role-Based Access Control

```toml
# ~/.config/agentfs/rbac.toml
[roles.admin]
permissions = ["*"]

[roles.developer]
permissions = [
    "workspace:create",
    "workspace:read",
    "workspace:write",
    "snapshot:*"
]

[roles.viewer]
permissions = [
    "workspace:read",
    "audit:read"
]

[users.alice]
role = "admin"

[users.bob]
role = "developer"
workspaces = ["project-a", "project-b"]
```

## Data Protection

### Encryption at Rest

```bash
# Enable encryption for local database
agentfs config set --encrypt-local-db true

# Set encryption key (or use system keychain)
agentfs config set --encryption-key-file ~/.agentfs/key

# Generate secure key
openssl rand -base64 32 > ~/.agentfs/key
chmod 600 ~/.agentfs/key
```

### Encryption in Transit

```bash
# Configure TLS for cloud sync
agentfs sync config my-workspace \
  --tls-version 1.3 \
  --ca-cert /path/to/ca.pem

# Enable certificate pinning
agentfs sync config my-workspace \
  --pin-certificate true
```

### Key Management

```bash
# Use hardware security module (HSM)
agentfs config set --hsm-module /usr/lib/pkcs11/lib.so

# Or use cloud KMS
agentfs config set --kms-provider aws \
  --kms-key-id arn:aws:kms:region:account:key/id
```

## Execution Security

### Sandboxing

```bash
# Enable sandbox for workspace
agentfs workspace config my-workspace \
  --sandbox true \
  --sandbox-profile restricted

# Available profiles:
# - restricted: No network, read-only filesystem
# - standard: Network allowed, workspace isolation
# - relaxed: Minimal restrictions
```

### Resource Limits

```bash
# Set resource limits
agentfs workspace config my-workspace \
  --max-cpu 2 \
  --max-memory 4G \
  --max-disk 10G \
  --max-processes 100

# Time limits
agentfs workspace config my-workspace \
  --max-runtime 1h \
  --idle-timeout 30m
```

### Network Restrictions

```bash
# Block all network
agentfs workspace config my-workspace --network none

# Allow specific hosts
agentfs workspace config my-workspace \
  --network restricted \
  --allow-hosts "api.example.com,db.internal"

# Allow specific ports
agentfs workspace config my-workspace \
  --allow-ports "443,8080"
```

## Audit Logging

### Comprehensive Auditing

All operations are logged with:
- Timestamp
- User/agent identity
- Operation type
- File paths
- Before/after checksums
- Source IP
- Session ID

```bash
# Enable detailed auditing
agentfs config set --audit-level detailed

# Audit specific operations only
agentfs config set --audit-operations read,write,delete

# Retention policy
agentfs config set --audit-retention 90d
```

### Audit Log Format

```json
{
  "id": "audit-12345",
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "info",
  "workspace": "my-workspace",
  "session": "sess-abc123",
  "user": "agent-1",
  "ip": "192.168.1.100",
  "operation": "write",
  "path": "/src/main.py",
  "details": {
    "size_before": 1024,
    "size_after": 1152,
    "checksum_before": "sha256:abc...",
    "checksum_after": "sha256:def...",
    "lines_added": 5,
    "lines_removed": 2
  },
  "metadata": {
    "agent_version": "1.0.0",
    "task_id": "task-456"
  }
}
```

### Audit Export

```bash
# Export to SIEM
agentfs audit export \
  --format json \
  --filter "level=warning" \
  --since 24h \
  --output /var/log/agentfs/audit.json

# Real-time streaming
agentfs audit stream \
  --format json \
  --output tcp://siem.example.com:514
```

## Network Security

### TLS Configuration

```toml
# ~/.config/agentfs/tls.toml
[tls]
enabled = true
min_version = "1.3"
cipher_suites = [
    "TLS_AES_256_GCM_SHA384",
    "TLS_CHACHA20_POLY1305_SHA256"
]

[tls.certificates]
cert_file = "/etc/agentfs/server.crt"
key_file = "/etc/agentfs/server.key"
ca_file = "/etc/agentfs/ca.crt"
```

### IP Whitelisting

```bash
# Restrict access by IP
agentfs server config \
  --allow-ip 192.168.1.0/24 \
  --allow-ip 10.0.0.50

# Block specific IPs
agentfs server config \
  --deny-ip 192.168.1.200
```

### Firewall Integration

```bash
# UFW rules
sudo ufw allow from 192.168.1.0/24 to any port 8080
sudo ufw allow from 10.0.0.0/8 to any port 2049

# iptables rules
sudo iptables -A INPUT -p tcp -s 192.168.1.0/24 --dport 8080 -j ACCEPT
sudo iptables -A INPUT -p tcp -s 10.0.0.0/8 --dport 2049 -j ACCEPT
```

## Compliance

### GDPR Compliance

```bash
# Data retention policies
agentfs config set --data-retention 30d
agentfs config set --audit-retention 90d

# Right to deletion
agentfs workspace delete my-workspace --gdpr-delete

# Data export
agentfs workspace export my-workspace --format gdpr-bundle
```

### HIPAA Compliance

```bash
# Enable HIPAA mode
agentfs config set --hipaa-mode true

# Encryption requirements
agentfs config set --encryption-required true

# Access logging
agentfs config set --access-log-level detailed

# Audit requirements
agentfs audit enable-hipaa
```

### SOC 2 Compliance

```bash
# Enable SOC 2 logging
agentfs config set --soc2-logging true

# Change management
agentfs config set --require-approval-for-production true

# Access reviews
agentfs audit access-review --since 90d
```

## Threat Model

### Identified Threats

1. **Unauthorized Access**
   - Mitigation: Strong authentication, RBAC, API tokens

2. **Data Exfiltration**
   - Mitigation: Encryption, network restrictions, audit logging

3. **Malicious Code Execution**
   - Mitigation: Sandboxing, resource limits, read-only workspaces

4. **Privilege Escalation**
   - Mitigation: Principle of least privilege, workspace isolation

5. **Data Corruption**
   - Mitigation: CoW semantics, snapshots, audit trails

## Incident Response

### Security Incident Handling

```bash
# 1. Isolate affected workspace
agentfs workspace freeze compromised-ws

# 2. Export audit logs
agentfs audit export --workspace compromised-ws --since incident-time

# 3. Create forensic snapshot
agentfs snapshot create compromised-ws --name "forensic-$(date +%s)"

# 4. Quarantine workspace
agentfs workspace quarantine compromised-ws

# 5. Revoke compromised tokens
agentfs auth token revoke <token-id>
```

### Security Scanning

```bash
# Scan workspace for sensitive data
agentfs security scan my-workspace \
  --check-secrets \
  --check-pii \
  --check-malware

# Compliance check
agentfs security compliance-check my-workspace \
  --standard gdpr
```

## Best Practices

### 1. Authentication
- Use strong, unique API tokens
- Rotate tokens regularly (every 90 days)
- Enable multi-factor authentication where possible

### 2. Authorization
- Follow principle of least privilege
- Regular access reviews
- Remove unused permissions

### 3. Encryption
- Enable encryption at rest for all workspaces
- Use TLS 1.3 for all communications
- Secure key management with HSM or KMS

### 4. Auditing
- Enable comprehensive audit logging
- Export logs to SIEM
- Regular audit reviews

### 5. Sandboxing
- Use sandboxed workspaces for untrusted agents
- Set appropriate resource limits
- Monitor for anomalous behavior

### 6. Network
- Restrict access by IP where possible
- Use VPN for remote access
- Regular firewall rule reviews

## CLI Reference

```bash
# Authentication
agentfs auth token create [options]
agentfs auth token list
agentfs auth token revoke <id>
agentfs auth token rotate <id>

# Security configuration
agentfs security scan <workspace>
agentfs security compliance-check <workspace>
agentfs security incident create [options]

# Workspace security
agentfs workspace config <name> --sandbox true
agentfs workspace config <name> --read-only true
agentfs workspace freeze <name>
agentfs workspace quarantine <name>

# Audit
agentfs audit export [options]
agentfs audit stream [options]
agentfs audit access-review [options]

# Server security
agentfs server config --tls-cert <path> --tls-key <path>
agentfs server config --allow-ip <cidr>
```

## Security Checklist

### Pre-Deployment
- [ ] Enable encryption at rest
- [ ] Configure TLS for all communications
- [ ] Set up authentication and authorization
- [ ] Enable audit logging
- [ ] Configure resource limits
- [ ] Set up firewall rules
- [ ] Review and document threat model

### Ongoing
- [ ] Regular access reviews (quarterly)
- [ ] Token rotation (every 90 days)
- [ ] Security scanning (weekly)
- [ ] Audit log reviews (monthly)
- [ ] Penetration testing (annually)
- [ ] Incident response drills (annually)

## Next Steps

- [MCP Integration](./07-mcp-integration.md)
- [Cloud Sync](./08-cloud-sync.md)
- [NFS Export](./09-nfs-export.md)