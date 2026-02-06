# Organizations

## Overview

Turso Cloud uses organizations to group databases, manage team access, and handle billing. Organizations provide isolation and collaboration features for teams.

## Organization Structure

```
Organization: "acme-corp"
├── Databases
│   ├── prod-api
│   ├── prod-analytics
│   └── staging-app
├── Groups
│   ├── production
│   └── staging
├── Members
│   ├── owner@acme.com (Owner)
│   ├── admin@acme.com (Admin)
│   └── dev@acme.com (Member)
└── Billing
    ├── Plan: Scaler
    └── Usage: 2.3GB / 10GB
```

## Creating Organizations

### Via CLI
```bash
# Create new organization
turso org create acme-corp

# Create with description
turso org create acme-corp --description "Acme Corporation Production"

# Switch to organization
turso org switch acme-corp
```

### Via Web UI
1. Go to https://app.turso.tech
2. Click "New Organization"
3. Enter organization name
4. Select plan
5. Configure billing

## Managing Organizations

### List Organizations
```bash
# List all organizations you belong to
turso org list

# Show current organization
turso org show
```

### Organization Settings
```bash
# View organization settings
turso org settings

# Update organization name
turso org update acme-corp --name "Acme Corporation"

# Update description
turso org update acme-corp --description "Updated description"
```

### Delete Organization
```bash
# Delete organization (destructive)
turso org destroy acme-corp --yes

# Must destroy all databases first
```

## Member Management

### Inviting Members
```bash
# Invite by email
turso org members invite acme-corp developer@example.com

# Invite with role
turso org members invite acme-corp admin@example.com --role admin

# Invite multiple members
turso org members invite acme-corp dev1@example.com dev2@example.com --role member
```

### Roles and Permissions

| Role | Permissions |
|------|-------------|
| **Owner** | Full access, billing, delete org |
| **Admin** | Manage databases, members (except owner) |
| **Member** | Create/use databases, no member management |

### Managing Members
```bash
# List members
turso org members list acme-corp

# Remove member
turso org members remove acme-corp developer@example.com

# Change role
turso org members update acme-corp developer@example.com --role admin
```

## Billing Plans

### Plan Tiers

#### Starter (Free)
```
- 1 organization
- 1 database
- 500MB storage
- 1B rows read/month
- 1M rows written/month
- Community support
```

#### Scaler ($29/month)
```
- Unlimited organizations
- 10 databases per org
- 10GB per database
- 100B rows read/month
- 100M rows written/month
- Email support
- Custom domains
```

#### Enterprise (Custom)
```
- Unlimited everything
- Custom limits
- Dedicated support
- SLA guarantees
- SSO/SAML
- Audit logs
```

### Managing Billing
```bash
# View current plan
turso org billing show acme-corp

# Upgrade plan
turso org billing upgrade acme-corp --plan scaler

# View usage
turso org billing usage acme-corp

# View usage for period
turso org billing usage acme-corp --from 2024-01-01 --to 2024-01-31
```

### Payment Methods
```bash
# Add payment method
turso org billing payment-method add acme-corp --card

# List payment methods
turso org billing payment-method list acme-corp

# Set default
turso org billing payment-method default acme-corp pm_xxx

# Remove payment method
turso org billing payment-method remove acme-corp pm_xxx
```

### Usage Limits
```bash
# Set spending limit
turso org billing limit acme-corp --monthly 500

# Set storage limit
turso org billing limit acme-corp --storage 100GB

# View limits
turso org billing limits acme-corp
```

## Usage Tracking

### Monitoring Usage
```bash
# Current month usage
turso org usage acme-corp

# Detailed breakdown
turso org usage acme-corp --detailed

# Specific database
turso org usage acme-corp --db mydb
```

### Usage Alerts
```bash
# Set usage alert at 80%
turso org billing alert acme-corp --threshold 80 --email admin@acme.com

# List alerts
turso org billing alerts acme-corp

# Remove alert
turso org billing alert remove acme-corp alert-id
```

## Multi-Organization Setup

### Development Workflow
```bash
# Create separate orgs for environments
turso org create acme-dev
turso org create acme-staging  
turso org create acme-prod

# Switch between orgs
turso org switch acme-dev
turso db create myapp

turso org switch acme-prod
turso db create myapp
```

### Team Isolation
```
acme-corp (Production)
├── Databases: prod-*
├── Members: Senior staff only
└── Plan: Enterprise

acme-labs (Experiments)
├── Databases: experiment-*
├── Members: All engineers
└── Plan: Scaler
```

## API Access

### Organization API Tokens
```bash
# Create API token for organization
turso org api-tokens create acme-corp --name "CI/CD" --role admin

# List tokens
turso org api-tokens list acme-corp

# Revoke token
turso org api-tokens revoke acme-corp token-id
```

### Using API Tokens
```bash
# Set token for CLI
turso auth login --token $TURSO_API_TOKEN

# Or use environment variable
export TURSO_API_TOKEN=your-token
```

## Security Best Practices

### Access Control
```bash
# Regular audit of members
turso org members list acme-corp

# Remove inactive members
turso org members remove acme-corp former-employee@acme.com

# Use least privilege principle
# Only give Admin to senior staff
```

### API Token Management
```bash
# Rotate tokens regularly
turso org api-tokens create acme-corp --name "New Token"
turso org api-tokens revoke acme-corp old-token-id

# Use descriptive names
turso org api-tokens create acme-corp --name "GitHub Actions - Production"
```

### Billing Security
```bash
# Require approval for high spending
turso org billing approval acme-corp --threshold 1000

# Multiple payment methods for redundancy
turso org billing payment-method add acme-corp --backup
```

## Organization Migration

### Transferring Databases
```bash
# Export from old org
turso org switch old-org
turso db dump mydb --output ./mydb.sql

# Import to new org
turso org switch new-org
turso db create mydb --from-dump ./mydb.sql
```

### Member Migration
```bash
# Invite members to new org
turso org members invite new-org member@example.com

# Remove from old org after migration
turso org members remove old-org member@example.com
```

## CLI Reference

```bash
# Organization management
turso org create <name> [options]
turso org list
turso org show
turso org switch <name>
turso org update <name> [options]
turso org destroy <name> [options]

# Member management
turso org members invite <org> <email> [options]
turso org members list <org>
turso org members remove <org> <email>
turso org members update <org> <email> [options]

# Billing
turso org billing show <org>
turso org billing upgrade <org> [options]
turso org billing usage <org> [options]
turso org billing limit <org> [options]
turso org billing alert <org> [options]

# API tokens
turso org api-tokens create <org> [options]
turso org api-tokens list <org>
turso org api-tokens revoke <org> <token-id>
```

## Next Steps

- **Locations**: [04-locations-regions.md](./04-locations-regions.md)
- **Authentication**: [05-authentication.md](./05-authentication.md)
- **Embedded Replicas**: [06-embedded-replicas.md](./06-embedded-replicas.md)