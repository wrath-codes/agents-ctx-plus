# Deployment Guide

## Docker Deployment

### Build Image

```bash
# Build production image
docker build -t beads-workflow-system:latest .

# Build with specific tag
docker build -t beads-workflow-system:v1.0.0 .
```

### Run Container

```bash
# Run with default config
docker run -d \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/configs:/app/configs \
  beads-workflow-system:latest

# Run with environment variables
docker run -d \
  -p 8080:8080 \
  -e WORKFLOW_LOG_LEVEL=debug \
  -e WORKFLOW_CONFIG_PATH=/app/configs/production.yaml \
  beads-workflow-system:latest
```

### Docker Compose

```yaml
version: '3.8'

services:
  workflow-api:
    image: beads-workflow-system:latest
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./data:/app/data
      - ./configs:/app/configs
    environment:
      - WORKFLOW_CONFIG_PATH=/app/configs/production.yaml
      - WORKFLOW_LOG_LEVEL=info
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped

  # Optional: Redis for caching
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

volumes:
  redis_data:
```

## Kubernetes Deployment

### Namespace

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: workflow-system
```

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: workflow-config
  namespace: workflow-system
data:
  production.yaml: |
    server:
      host: "0.0.0.0"
      port: 8080
    
    database:
      coordination_db:
        path: "/data/coordination.db"
        max_open_conns: 1
        busy_timeout: "30s"
    
    logging:
      level: "info"
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: workflow-system
  namespace: workflow-system
spec:
  replicas: 3
  selector:
    matchLabels:
      app: workflow-system
  template:
    metadata:
      labels:
        app: workflow-system
    spec:
      containers:
      - name: workflow-api
        image: beads-workflow-system:v1.0.0
        ports:
        - containerPort: 8080
        - containerPort: 9090
        env:
        - name: CONFIG_PATH
          value: "/app/configs/production.yaml"
        volumeMounts:
        - name: config
          mountPath: /app/configs
        - name: data
          mountPath: /data
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: workflow-config
      - name: data
        persistentVolumeClaim:
          claimName: workflow-data
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: workflow-service
  namespace: workflow-system
spec:
  selector:
    app: workflow-system
  ports:
  - name: http
    port: 80
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
  type: LoadBalancer
```

### Persistent Volume

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: workflow-data
  namespace: workflow-system
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

## Production Checklist

### Pre-deployment

- [ ] Database migrations applied
- [ ] Configuration validated
- [ ] Secrets configured
- [ ] Health checks enabled
- [ ] Monitoring setup
- [ ] Backup strategy defined

### Security

- [ ] TLS certificates configured
- [ ] Authentication enabled
- [ ] Rate limiting configured
- [ ] Network policies defined
- [ ] Resource limits set

### Monitoring

- [ ] Prometheus metrics exposed
- [ ] Grafana dashboards imported
- [ ] Alerting rules configured
- [ ] Log aggregation setup
- [ ] Distributed tracing enabled

## Rollback Strategy

```bash
# Rollback deployment
kubectl rollout undo deployment/workflow-system

# Check rollout status
kubectl rollout status deployment/workflow-system

# View rollout history
kubectl rollout history deployment/workflow-system
```