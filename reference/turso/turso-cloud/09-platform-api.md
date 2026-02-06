# Platform API

## Overview

The Turso Platform API provides programmatic access to Turso Cloud management features, allowing you to automate database operations, integrate with CI/CD pipelines, and build custom tooling.

## API Basics

### Base URL
```
https://api.turso.tech
```

### Authentication
```bash
# Use API token in header
curl -H "Authorization: Bearer $TURSO_API_TOKEN" \
  https://api.turso.tech/v1/organizations
```

### Response Format
All responses are JSON:
```json
{
  "data": { ... },
  "meta": {
    "page": 1,
    "limit": 20,
    "total": 100
  }
}
```

## Organizations API

### List Organizations
```bash
GET /v1/organizations
```

**Response:**
```json
{
  "organizations": [
    {
      "name": "acme-corp",
      "slug": "acme-corp",
      "plan": "scaler",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

### Get Organization
```bash
GET /v1/organizations/{organization}
```

**Response:**
```json
{
  "organization": {
    "name": "acme-corp",
    "slug": "acme-corp",
    "plan": "scaler",
    "billing_email": "billing@acme.com",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

### Update Organization
```bash
PATCH /v1/organizations/{organization}
```

**Request:**
```json
{
  "name": "Acme Corporation",
  "billing_email": "new-billing@acme.com"
}
```

## Databases API

### List Databases
```bash
GET /v1/organizations/{organization}/databases
```

**Query Parameters:**
- `group` - Filter by group
- `limit` - Results per page (default: 20)
- `page` - Page number

**Response:**
```json
{
  "databases": [
    {
      "name": "production-api",
      "hostname": "production-api-acme.turso.io",
      "created_at": "2024-01-15T10:30:00Z",
      "group": "production"
    }
  ],
  "meta": {
    "page": 1,
    "limit": 20,
    "total": 5
  }
}
```

### Create Database
```bash
POST /v1/organizations/{organization}/databases
```

**Request:**
```json
{
  "name": "new-database",
  "group": "production",
  "location": "iad",
  "schema": "template-db"
}
```

**Response:**
```json
{
  "database": {
    "name": "new-database",
    "hostname": "new-database-acme.turso.io",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

### Get Database
```bash
GET /v1/organizations/{organization}/databases/{database}
```

**Response:**
```json
{
  "database": {
    "name": "production-api",
    "hostname": "production-api-acme.turso.io",
    "created_at": "2024-01-15T10:30:00Z",
    "group": "production",
    "locations": ["iad", "lhr"],
    "size": 104857600,
    "usage": {
      "rows_read": 15000000,
      "rows_written": 500000
    }
  }
}
```

### Update Database
```bash
PATCH /v1/organizations/{organization}/databases/{database}
```

**Request:**
```json
{
  "group": "staging",
  "settings": {
    "size_limit": "10GB",
    "allow_attach": false
  }
}
```

### Delete Database
```bash
DELETE /v1/organizations/{organization}/databases/{database}
```

## Locations API

### List Locations
```bash
GET /v1/locations
```

**Response:**
```json
{
  "locations": [
    {
      "code": "iad",
      "city": "Ashburn",
      "country": "United States",
      "region": "North America",
      "latitude": 39.0438,
      "longitude": -77.4874
    }
  ]
}
```

### Get Location
```bash
GET /v1/locations/{location}
```

## Replicas API

### List Replicas
```bash
GET /v1/organizations/{organization}/databases/{database}/replicas
```

**Response:**
```json
{
  "replicas": [
    {
      "uuid": "rep-abc123",
      "location": "lhr",
      "created_at": "2024-01-15T10:30:00Z",
      "status": "active"
    }
  ]
}
```

### Create Replica
```bash
POST /v1/organizations/{organization}/databases/{database}/replicas
```

**Request:**
```json
{
  "location": "nrt"
}
```

### Delete Replica
```bash
DELETE /v1/organizations/{organization}/databases/{database}/replicas/{replica}
```

## API Tokens API

### List API Tokens
```bash
GET /v1/organizations/{organization}/api-tokens
```

### Create API Token
```bash
POST /v1/organizations/{organization}/api-tokens
```

**Request:**
```json
{
  "name": "CI/CD Pipeline",
  "permissions": ["db:read", "db:write"],
  "expires_at": "2024-12-31T23:59:59Z"
}
```

**Response:**
```json
{
  "token": {
    "id": "token-abc123",
    "name": "CI/CD Pipeline",
    "token": "actual-token-value-only-shown-once",
    "created_at": "2024-01-15T10:30:00Z",
    "expires_at": "2024-12-31T23:59:59Z"
  }
}
```

### Revoke API Token
```bash
DELETE /v1/organizations/{organization}/api-tokens/{token}
```

## Usage API

### Get Organization Usage
```bash
GET /v1/organizations/{organization}/usage
```

**Query Parameters:**
- `from` - Start date (ISO 8601)
- `to` - End date (ISO 8601)

**Response:**
```json
{
  "usage": {
    "rows_read": 150000000,
    "rows_written": 5000000,
    "storage_bytes": 1073741824,
    "period": {
      "from": "2024-01-01T00:00:00Z",
      "to": "2024-01-31T23:59:59Z"
    }
  }
}
```

### Get Database Usage
```bash
GET /v1/organizations/{organization}/databases/{database}/usage
```

## Members API

### List Members
```bash
GET /v1/organizations/{organization}/members
```

### Invite Member
```bash
POST /v1/organizations/{organization}/members
```

**Request:**
```json
{
  "email": "new-member@example.com",
  "role": "member"
}
```

### Remove Member
```bash
DELETE /v1/organizations/{organization}/members/{member}
```

## Groups API

### List Groups
```bash
GET /v1/organizations/{organization}/groups
```

### Create Group
```bash
POST /v1/organizations/{organization}/groups
```

**Request:**
```json
{
  "name": "production",
  "location": "iad"
}
```

### Delete Group
```bash
DELETE /v1/organizations/{organization}/groups/{group}
```

## Error Handling

### Error Format
```json
{
  "error": {
    "code": "database_not_found",
    "message": "Database 'mydb' not found",
    "details": {
      "database": "mydb"
    }
  }
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `unauthorized` | 401 | Invalid or missing token |
| `forbidden` | 403 | Insufficient permissions |
| `not_found` | 404 | Resource not found |
| `conflict` | 409 | Resource already exists |
| `rate_limited` | 429 | Too many requests |
| `internal_error` | 500 | Server error |

## Rate Limiting

### Limits
- 100 requests per minute for most endpoints
- 10 requests per minute for database creation
- Burst allowance: 10 requests

### Headers
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640995200
```

## SDK Examples

### Python
```python
import requests

class TursoAPI:
    def __init__(self, token):
        self.token = token
        self.base_url = "https://api.turso.tech"
    
    def _headers(self):
        return {
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json"
        }
    
    def list_databases(self, org):
        response = requests.get(
            f"{self.base_url}/v1/organizations/{org}/databases",
            headers=self._headers()
        )
        response.raise_for_status()
        return response.json()
    
    def create_database(self, org, name, **kwargs):
        response = requests.post(
            f"{self.base_url}/v1/organizations/{org}/databases",
            headers=self._headers(),
            json={"name": name, **kwargs}
        )
        response.raise_for_status()
        return response.json()

# Usage
api = TursoAPI(os.environ["TURSO_API_TOKEN"])
databases = api.list_databases("acme-corp")
```

### JavaScript
```javascript
class TursoAPI {
  constructor(token) {
    this.token = token;
    this.baseUrl = 'https://api.turso.tech';
  }
  
  async request(path, options = {}) {
    const response = await fetch(`${this.baseUrl}${path}`, {
      ...options,
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json',
        ...options.headers
      }
    });
    
    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }
    
    return response.json();
  }
  
  async listDatabases(org) {
    return this.request(`/v1/organizations/${org}/databases`);
  }
  
  async createDatabase(org, name, options = {}) {
    return this.request(`/v1/organizations/${org}/databases`, {
      method: 'POST',
      body: JSON.stringify({ name, ...options })
    });
  }
}

// Usage
const api = new TursoAPI(process.env.TURSO_API_TOKEN);
const databases = await api.listDatabases('acme-corp');
```

### Go
```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
    "os"
)

type TursoClient struct {
    token   string
    baseURL string
    client  *http.Client
}

func NewTursoClient(token string) *TursoClient {
    return &TursoClient{
        token:   token,
        baseURL: "https://api.turso.tech",
        client:  &http.Client{},
    }
}

func (c *TursoClient) request(method, path string, body interface{}) (*http.Response, error) {
    var bodyReader *bytes.Reader
    if body != nil {
        jsonBody, _ := json.Marshal(body)
        bodyReader = bytes.NewReader(jsonBody)
    } else {
        bodyReader = bytes.NewReader([]byte{})
    }
    
    req, err := http.NewRequest(method, c.baseURL+path, bodyReader)
    if err != nil {
        return nil, err
    }
    
    req.Header.Set("Authorization", "Bearer "+c.token)
    req.Header.Set("Content-Type", "application/json")
    
    return c.client.Do(req)
}

func (c *TursoClient) ListDatabases(org string) ([]map[string]interface{}, error) {
    resp, err := c.request("GET", fmt.Sprintf("/v1/organizations/%s/databases", org), nil)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    var result struct {
        Databases []map[string]interface{} `json:"databases"`
    }
    err = json.NewDecoder(resp.Body).Decode(&result)
    return result.Databases, err
}
```

## Next Steps

- **SDKs**: [10-sdks/](./10-sdks/)
- **AgentFS**: [../agentfs/01-overview.md](../agentfs/01-overview.md)