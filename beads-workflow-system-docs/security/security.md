# Security Documentation

## Authentication

### JWT Authentication

The system uses JWT tokens for authentication.

**Token Format:**
```json
{
  "sub": "user-123",
  "user_id": "user-123",
  "roles": ["admin"],
  "permissions": ["workflow:*"],
  "iat": 1644230400,
  "exp": 1644316800,
  "jti": "token-id-123"
}
```

**Generate Token:**
```bash
workflow-server generate-token --user-id admin --role admin
```

**Use Token:**
```bash
curl -H "Authorization: Bearer <token>" \
     http://localhost:8080/api/v1/workflows
```

### API Key Authentication

For service-to-service authentication:

```bash
# Generate API key
workflow-server generate-api-key --name "ci-system" --permissions "workflow:read"

# Use API key
curl -H "X-API-Key: <api-key>" \
     http://localhost:8080/api/v1/workflows
```

## Authorization

### Role-Based Access Control (RBAC)

**Predefined Roles:**

| Role | Permissions |
|------|-------------|
| admin | All permissions |
| operator | workflow:read, workflow:write, agent:read |
| viewer | workflow:read |
| agent | workflow:read (own only), agent:read |

**Custom Roles:**
```yaml
# config/roles.yaml
roles:
  custom-operator:
    permissions:
      - workflow:read
      - workflow:write
      - analytics:read
    allowed_workflow_types:
      - research
      - poc
```

### Permission System

**Permission Format:** `<resource>:<action>`

Examples:
- `workflow:read` - Read workflows
- `workflow:write` - Create and update workflows
- `workflow:delete` - Cancel workflows
- `agent:read` - Read agent information
- `analytics:read` - Read analytics

## Data Security

### Encryption at Rest

**SQLite Encryption:**
```sql
-- Enable encryption for SQLite
PRAGMA key = 'your-encryption-key';
```

**Configuration Encryption:**
```yaml
# Encrypt sensitive config values
auth:
  jwt_secret: "${JWT_SECRET}"  # Loaded from environment or vault
```

### Encryption in Transit

**TLS Configuration:**
```yaml
server:
  enable_tls: true
  cert_file: "/etc/ssl/certs/workflow.crt"
  key_file: "/etc/ssl/private/workflow.key"
  
  # Optional: Client certificate authentication
  client_ca_file: "/etc/ssl/certs/ca.crt"
  require_client_cert: false
```

## Network Security

### Firewall Rules

**Recommended Ports:**
- 8080 - HTTP API (internal)
- 8443 - HTTPS API (external)
- 9090 - Metrics (internal only)

**IP Whitelisting:**
```yaml
server:
  trusted_proxies:
    - "10.0.0.0/8"
    - "172.16.0.0/12"
```

### Rate Limiting

```yaml
server:
  rate_limit:
    enabled: true
    requests_per_minute: 100
    burst: 20
    
    # Per-endpoint limits
    endpoints:
      workflow_start:
        requests_per_minute: 10
      workflow_status:
        requests_per_minute: 1000
```

## Secrets Management

### Environment Variables

**Sensitive Configuration:**
```bash
# Never commit these to version control
export WORKFLOW_AUTH_JWT_SECRET="your-secret-key"
export WORKFLOW_DATABASE_PASSWORD="db-password"
export WORKFLOW_REDIS_PASSWORD="redis-password"
```

### Vault Integration

**HashiCorp Vault:**
```go
import (
    "github.com/hashicorp/vault/api"
)

func loadSecretsFromVault() (*Config, error) {
    client, err := api.NewClient(&api.Config{
        Address: "https://vault.example.com:8200",
    })
    if err != nil {
        return nil, err
    }
    
    client.SetToken(os.Getenv("VAULT_TOKEN"))
    
    secret, err := client.KVv2("secret").Get(context.Background(), "workflow-system")
    if err != nil {
        return nil, err
    }
    
    config := &Config{}
    config.Auth.JWTSecret = secret.Data["jwt_secret"].(string)
    
    return config, nil
}
```

## Security Best Practices

### 1. Secure Defaults

```yaml
# Production secure defaults
server:
  enable_tls: true
  cors:
    allowed_origins: []  # Empty by default

auth:
  enabled: true
  token_expiry: "1h"  # Short-lived tokens
  require_mfa: false  # Enable for sensitive operations

logging:
  level: "info"
  mask_sensitive: true  # Mask tokens and passwords
```

### 2. Input Validation

```go
func validateStartWorkflowRequest(req *StartWorkflowRequest) error {
    if len(req.IssueTitle) > 200 {
        return errors.New("title too long")
    }
    
    if !isValidWorkflowType(req.WorkflowType) {
        return errors.New("invalid workflow type")
    }
    
    // Sanitize variables
    for key, value := range req.Variables {
        if strings.Contains(key, "..") {
            return errors.New("invalid variable name")
        }
        
        // Prevent code injection
        if str, ok := value.(string); ok {
            req.Variables[key] = sanitizeInput(str)
        }
    }
    
    return nil
}
```

### 3. Audit Logging

```yaml
logging:
  audit:
    enabled: true
    events:
      - workflow_start
      - workflow_cancel
      - agent_register
      - config_change
```

**Audit Log Entry:**
```json
{
  "timestamp": "2026-02-07T10:30:00Z",
  "event": "workflow_start",
  "user_id": "user-123",
  "ip_address": "10.0.0.1",
  "user_agent": "workflow-cli/1.0.0",
  "resource": {
    "type": "workflow",
    "id": "wf-research-001"
  },
  "action": "create",
  "result": "success"
}
```

## Security Checklist

### Development

- [ ] No hardcoded secrets in code
- [ ] Input validation on all endpoints
- [ ] SQL injection prevention (prepared statements)
- [ ] XSS prevention (output encoding)
- [ ] CSRF protection for web UI
- [ ] Security headers configured

### Deployment

- [ ] TLS enabled with valid certificates
- [ ] Strong authentication configured
- [ ] Rate limiting enabled
- [ ] Network segmentation
- [ ] Secrets management (Vault/KMS)
- [ ] Regular security scans
- [ ] Penetration testing completed

### Operations

- [ ] Regular security updates
- [ ] Log monitoring and alerting
- [ ] Incident response plan
- [ ] Backup and recovery tested
- [ ] Access reviews conducted

## Reporting Security Issues

**Email:** security@yourdomain.com
**PGP Key:** [security@yourdomain.com.asc](link-to-key)

**Please include:**
- Description of the issue
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

**Response timeline:**
- Acknowledgment: 24 hours
- Initial assessment: 72 hours
- Fix released: 30 days (critical: 7 days)