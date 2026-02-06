# Locations and Regions

## Overview

Turso Cloud operates a global network of edge locations, allowing you to deploy databases close to your users for minimal latency. Understanding the location architecture helps optimize performance and compliance.

## Global Network

### Available Locations

Turso operates in 30+ locations across 6 continents:

#### North America
| Code | Location | Latency (ms) |
|------|----------|--------------|
| iad | Ashburn, Virginia, USA | 1-20 |
| ord | Chicago, Illinois, USA | 10-30 |
| dfw | Dallas, Texas, USA | 15-40 |
| lax | Los Angeles, California, USA | 20-50 |
| sea | Seattle, Washington, USA | 15-45 |
| mia | Miami, Florida, USA | 20-45 |
| yyz | Toronto, Canada | 15-35 |
| mex | Mexico City, Mexico | 30-60 |

#### Europe
| Code | Location | Latency (ms) |
|------|----------|--------------|
| lhr | London, UK | 1-25 |
| cdg | Paris, France | 5-30 |
| fra | Frankfurt, Germany | 5-35 |
| ams | Amsterdam, Netherlands | 5-30 |
| mad | Madrid, Spain | 10-40 |
| mil | Milan, Italy | 10-40 |
| arn | Stockholm, Sweden | 15-45 |
| waw | Warsaw, Poland | 10-40 |

#### Asia Pacific
| Code | Location | Latency (ms) |
|------|----------|--------------|
| nrt | Tokyo, Japan | 1-30 |
| sin | Singapore | 5-40 |
| syd | Sydney, Australia | 15-50 |
| bom | Mumbai, India | 20-60 |
| hkg | Hong Kong | 5-35 |
| scl | Seoul, South Korea | 10-35 |
| tpe | Taipei, Taiwan | 10-40 |

#### South America
| Code | Location | Latency (ms) |
|------|----------|--------------|
| gru | São Paulo, Brazil | 20-60 |
| bog | Bogotá, Colombia | 30-70 |
| scl | Santiago, Chile | 35-80 |

#### Africa
| Code | Location | Latency (ms) |
|------|----------|--------------|
| jnb | Johannesburg, South Africa | 20-70 |
| los | Lagos, Nigeria | 40-100 |

#### Middle East
| Code | Location | Latency (ms) |
|------|----------|--------------|
| dxb | Dubai, UAE | 15-50 |
| tlv | Tel Aviv, Israel | 20-60 |

## Deploying to Locations

### Primary Location
```bash
# Create database in specific location
turso db create mydb --location lhr

# Primary location handles writes
# Determines data residency
```

### Adding Replicas
```bash
# Add read replica in another location
turso db replicate mydb cdg
turso db replicate mydb fra

# Add multiple replicas at once
turso db replicate mydb nrt sin syd

# List replicas
turso db show mydb --replicas
```

### Removing Replicas
```bash
# Remove replica
turso db unreplicate mydb cdg

# Confirm removal
turso db unreplicate mydb cdg --yes
```

## Location Strategy

### Geographic Distribution
```
Global App Architecture:
                    ┌──────────────┐
                    │   Global LB  │
                    └──────┬───────┘
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
  ┌────▼────┐         ┌───▼────┐         ┌────▼────┐
  │  AMS    │         │  IAD   │         │  NRT    │
  │ Europe  │         │ US East│         │  APAC   │
  └────┬────┘         └───┬────┘         └────┬────┘
       │                  │                   │
  ┌────▼────┐         ┌───▼────┐         ┌────▼────┐
  │  LHR    │         │  DFW   │         │  SIN    │
  │London   │         │US Cent │         │Singapore│
  └─────────┘         └────────┘         └─────────┘
```

### Latency Optimization
```bash
# Deploy closest to users
# Example: E-commerce with global customers

# Primary in US (largest market)
turso db create shop --location iad

# Replicas for other markets
turso db replicate shop lhr  # Europe
turso db replicate shop nrt  # Japan
turso db replicate shop sin  # SE Asia
turso db replicate shop gru  # Brazil
```

### Data Residency
```bash
# EU-only deployment for GDPR
turso db create eu-data --location fra
turso db replicate eu-data lhr
turso db replicate eu-data cdg
# All data stays in EU

# US-only deployment
turso db create us-data --location iad
turso db replicate us-data lax
turso db replicate us-data ord
```

## Performance Characteristics

### Read Latency
```
┌────────────────────────────────────────────┐
│         Typical Read Latencies             │
├────────────────────────────────────────────┤
│ Same region        │ 1-5 ms                │
│ Same continent     │ 10-50 ms              │
│ Cross-continent    │ 100-300 ms            │
│ Edge to origin     │ 50-150 ms             │
└────────────────────────────────────────────┘
```

### Write Latency
```
┌────────────────────────────────────────────┐
│         Typical Write Latencies            │
├────────────────────────────────────────────┤
│ Primary location   │ 1-10 ms               │
│ Remote region      │ 100-500 ms            │
│ Global replicas    │ Async (<1s delay)     │
└────────────────────────────────────────────┘
```

### Replication Lag
```bash
# Check replication lag
turso db show mydb --replicas

# Output shows:
# Replica    Location    Lag (ms)    Status
# rep-xxx    cdg         23          synced
# rep-yyy    nrt         45          synced
```

## Routing and Load Balancing

### Automatic Routing
Turso automatically routes requests to the closest replica:
```
User in London → Request → Nearest replica (LHR)
                              ↓
                        If not found locally
                              ↓
                        Query primary (IAD)
                              ↓
                        Cache result locally
```

### Connection URL Patterns
```bash
# Primary only (writes, strong consistency)
libsql://mydb-org.turso.io

# Read replicas (reads, eventual consistency)
libsql://mydb-org.turso.io?read_replica=1

# Specific location
libsql://iad.mydb-org.turso.io
```

### Client-Side Routing
```rust
// Use embedded replica for local reads
let db = Builder::new_sync(
    "local-cache.db",
    "libsql://mydb-org.turso.io",
    token
)
.read_your_writes(true)  // Ensure read-after-write consistency
.build()
.await?;
```

## Location Management

### Listing Available Locations
```bash
# All available locations
turso locations list

# Filter by continent
turso locations list --region europe
turso locations list --region north-america

# Closest locations
turso locations list --closest
```

### Location Information
```bash
# Get location details
turso location show lhr

# Output:
# Location: lhr
# City: London
# Country: United Kingdom
# Region: Europe
# Status: Available
# Latency from you: 23ms
```

## Compliance and Regulations

### Data Residency Requirements
```bash
# GDPR-compliant deployment
turso db create eu-customers --location fra
turso db replicate eu-customers cdg
# Data never leaves EU

# HIPAA-compliant (Enterprise)
turso db create healthcare --location iad --encryption
# With BAA and encryption
```

### Cross-Border Considerations
```
┌─────────────────────────────────────────────────────┐
│          Data Residency Strategies                  │
├─────────────────────────────────────────────────────┤
│ 1. Geo-partitioning: Separate DB per region        │
│ 2. Replication: Primary in region, replicas global │
│ 3. Edge caching: Local replicas, central primary   │
│ 4. Compliance mode: Data never leaves region       │
└─────────────────────────────────────────────────────┘
```

## Cost Optimization

### Location-Based Pricing
```
┌────────────────────────────────────────────┐
│         Data Transfer Costs                │
├────────────────────────────────────────────┤
│ Inbound (to Turso)    │ Free              │
│ Outbound (reads)      │ Free              │
│ Cross-region writes   │ Included          │
│ Replicant storage     │ Same as primary   │
└────────────────────────────────────────────┘
```

### Minimizing Costs
```bash
# Use embedded replicas for high-read scenarios
# Reduces cross-region queries

# Strategic replica placement
# Only replicate to regions with users

# Monitor usage by location
turso db usage mydb --by-location
```

## Best Practices

### Multi-Region Architecture
```
┌─────────────────────────────────────────────────────────┐
│              Recommended Architecture                   │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐      ┌──────────────┐                │
│  │   Primary    │◄────►│   Replica    │  Europe       │
│  │     IAD      │      │     LHR      │                │
│  │  (Writes)    │      │  (Reads)     │                │
│  └──────┬───────┘      └──────────────┘                │
│         │                                               │
│         │      ┌──────────────┐                        │
│         └─────►│   Replica    │  Asia-Pacific          │
│                │     NRT      │                        │
│                │  (Reads)     │                        │
│                └──────────────┘                        │
│                                                         │
│  Strategy: Primary near main user base, replicas       │
│            in other high-traffic regions               │
└─────────────────────────────────────────────────────────┘
```

### Failover Planning
```bash
# Primary failure scenario
# 1. Turso automatically promotes replica
# 2. Update connection strings (if needed)
# 3. Add new replica in original location

# Manual failover (if needed)
turso db failover mydb --to-replica rep-xxx
```

## CLI Reference

```bash
# Location management
turso locations list
turso locations list --region <region>
turso locations list --closest
turso location show <code>

# Database replication
turso db replicate <db> <location>
turso db unreplicate <db> <location>
turso db show <db> --replicas

# Regional deployment
turso db create <name> --location <code>
turso group create <name> --location <code>
```

## Troubleshooting

### High Latency Issues
```bash
# Check replica lag
turso db show mydb --replicas

# If lag > 1000ms:
# - Check network connectivity
# - Consider closer primary location
# - Add intermediate replica
```

### Replication Problems
```bash
# Check replication status
turso db show mydb --replicas --verbose

# Restart replication if needed
turso db unreplicate mydb <location>
turso db replicate mydb <location>
```

## Next Steps

- **Authentication**: [05-authentication.md](./05-authentication.md)
- **Embedded Replicas**: [06-embedded-replicas.md](./06-embedded-replicas.md)
- **Branching**: [07-branching.md](./07-branching.md)