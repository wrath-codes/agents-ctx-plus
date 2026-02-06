# Daemon System

The Beads daemon provides **background synchronization** between SQLite and JSONL layers, ensuring the database stays current while preventing race conditions and conflicts.

## 🔄 Daemon Role in Architecture

The daemon operates as the background coordinator in Beads' three-layer system:

```
┌─────────────────┐
│   Git Repo     │ ← Historical Source of Truth
│ (issues.jsonl) │
└────────┬────────┘
         │▲
    Watch/Sync │
         ││
┌────────▼────────┐
│   JSONL Files  │ ← Operational Source of Truth  
│ (append-only)  │
└────────┬────────┘
         │▲
  Auto-rebuild │
         ││
┌────────▼────────┐
│   SQLite DB    │ ← Fast Queries / Derived State
│  (beads.db)   │
└─────────────────┘
          ▲
          │
     Daemon monitoring
```

**Key Functions**:
- **File Watching**: Monitors `.beads/` directory for changes
- **Auto-Sync**: Triggers sync operations (5-second debounce)
- **Lock Management**: Prevents concurrent database access
- **Performance**: Keeps SQLite optimized and current

## 🏗️ Daemon Architecture

### Core Components

```go
// Main daemon structure
type Daemon struct {
    config       *Config           // Daemon configuration
    db          *sql.DB          // Database connection
    fileWatcher  *fsnotify.Watcher // File system watcher
    debounceTimer *time.Timer      // Debounce timer
    lockFile     string           // Daemon lock file path
    running      bool             // Daemon running state
    mutex        sync.Mutex       // Thread safety
}

// Daemon state
type DaemonState struct {
    PID         int       `json:"pid"`         // Process ID
    StartTime    time.Time `json:"start_time"` // When daemon started
    LastSync    time.Time `json:"last_sync"`  // Last sync operation
    SyncCount   int       `json:"sync_count"` // Total syncs performed
    Status      string    `json:"status"`      // "running", "stopped", "error"
}
```

### File System Integration

```go
// File watching setup
func (d *Daemon) setupFileWatcher() error {
    watcher, err := fsnotify.NewWatcher()
    if err != nil {
        return err
    }
    
    // Watch critical files and directories
    watchPaths := []string{
        ".beads/issues.jsonl",      // Main issue data
        ".beads/config.yaml",        // Configuration changes
        ".beads/interactions.jsonl", // Agent interactions
        ".beads/routes.jsonl",      // Routing rules
    }
    
    for _, path := range watchPaths {
        err = watcher.Add(path)
        if err != nil {
            return fmt.Errorf("failed to watch %s: %w", path, err)
        }
    }
    
    d.fileWatcher = watcher
    return nil
}
```

### Debounce Mechanism

```go
// Prevent excessive sync operations
func (d *Daemon) scheduleSync() {
    d.mutex.Lock()
    defer d.mutex.Unlock()
    
    // Cancel existing timer
    if d.debounceTimer != nil {
        d.debounceTimer.Stop()
    }
    
    // Schedule new sync after 5 seconds
    d.debounceTimer = time.AfterFunc(5*time.Second, func() {
        d.performSync()
    })
}
```

## 🚀 Daemon Lifecycle

### Starting the Daemon

```bash
# Command line启动
bd daemon start

# Automatic start on init
bd init --start-daemon

# Start with custom configuration
bd daemon start --config .beads/daemon.yaml
```

**Startup Process**:
1. Check for existing daemon (lock file)
2. Initialize file system watcher
3. Connect to SQLite database
4. Perform initial sync if needed
5. Create lock file with PID
6. Enter event loop
7. Fork to background (unless --foreground)

### Daemon Configuration

```yaml
# .beads/daemon.yaml
daemon:
  # Basic settings
  enabled: true
  foreground: false          # Run in foreground (debugging)
  log_level: "info"          # debug, info, warn, error
  
  # Sync behavior
  debounce_interval: "5s"    # Wait time before triggering sync
  auto_sync: true             # Enable automatic sync
  sync_on_startup: true       # Sync immediately on start
  
  # File watching
  watch_patterns:             # Files to monitor
    - "issues.jsonl"
    - "config.yaml"
    - "routes.jsonl"
    - "interactions.jsonl"
  
  # Performance tuning
  max_sync_frequency: "30s" # Minimum time between syncs
  batch_size: 100            # Max operations per sync
  
  # Resource limits
  max_memory: "100MB"        # Maximum memory usage
  max_cpu_percent: 10         # Maximum CPU percentage
```

### Daemon Status Monitoring

```bash
# Check daemon status
bd daemon status

# Output example:
Daemon Status: Running
┌─────────────┬──────────────────────┐
│ PID        │ 12345              │
│ Started    │ 2026-02-06 10:00:00 │
│ Last Sync  │ 2026-02-06 10:32:15 │
│ Sync Count │ 24                  │
│ Memory     │ 45MB                │
│ CPU        │ 2%                  │
└─────────────┴──────────────────────┘

# JSON output for scripting
bd daemon status --json
{
  "pid": 12345,
  "status": "running",
  "start_time": "2026-02-06T10:00:00Z",
  "last_sync": "2026-02-06T10:32:15Z",
  "sync_count": 24,
  "memory_usage": 47185920,
  "cpu_percent": 2.1
}
```

## 🔄 Sync Operations

### Automatic Sync Trigger

```go
// File change triggers sync
func (d *Daemon) handleFileEvent(event fsnotify.Event) {
    switch {
    case event.Op&fsnotify.Write == fsnotify.Write:
        log.Infof("File modified: %s", event.Name)
        d.scheduleSync()
        
    case event.Op&fsnotify.Create == fsnotify.Create:
        log.Infof("File created: %s", event.Name)
        d.scheduleSync()
        
    case event.Op&fsnotify.Remove == fsnotify.Remove:
        log.Infof("File removed: %s", event.Name)
        d.scheduleSync()
        
    case event.Op&fsnotify.Rename == fsnotify.Rename:
        log.Infof("File renamed: %s", event.Name)
        d.scheduleSync()
    }
}
```

### Sync Operation Flow

```go
// Complete sync operation
func (d *Daemon) performSync() error {
    log.Info("Starting sync operation")
    
    // 1. Check if sync is needed
    needsSync, err := d.needsSync()
    if err != nil {
        return err
    }
    if !needsSync {
        log.Debug("No sync needed")
        return nil
    }
    
    // 2. Acquire database lock
    lock, err := d.acquireLock()
    if err != nil {
        return fmt.Errorf("failed to acquire lock: %w", err)
    }
    defer lock.Release()
    
    // 3. Perform the actual sync
    err = d.executeSync()
    if err != nil {
        log.Errorf("Sync failed: %v", err)
        return err
    }
    
    // 4. Update daemon state
    d.mutex.Lock()
    d.lastSync = time.Now()
    d.syncCount++
    d.mutex.Unlock()
    
    log.Info("Sync completed successfully")
    return nil
}
```

### Sync Types

#### JSONL → SQLite Sync
```go
// Rebuild SQLite from JSONL changes
func (d *Daemon) syncJSONLToSQLite() error {
    // Get last sync position
    lastPosition, err := d.getLastSyncPosition()
    if err != nil {
        return err
    }
    
    // Read new JSONL operations
    operations, err := d.readNewJSONLOperations(lastPosition)
    if err != nil {
        return err
    }
    
    // Apply operations to SQLite
    for _, op := range operations {
        err = d.applyOperation(op)
        if err != nil {
            log.Errorf("Failed to apply operation %s: %v", op.ID, err)
            continue
        }
    }
    
    // Update last sync position
    return d.updateLastSyncPosition(operations)
}
```

#### Git Sync Integration
```go
// Integrate with Git operations
func (d *Daemon) performGitSync() error {
    // 1. Check for remote changes
    hasRemote, err := d.hasRemoteChanges()
    if err != nil {
        return err
    }
    if hasRemote {
        err = d.pullRemoteChanges()
        if err != nil {
            return err
        }
    }
    
    // 2. Commit local changes
    hasLocal, err := d.hasLocalChanges()
    if err != nil {
        return err
    }
    if hasLocal {
        err = d.commitLocalChanges()
        if err != nil {
            return err
        }
        
        err = d.pushChanges()
        if err != nil {
            return err
        }
    }
    
    return nil
}
```

## 🔒 Lock Management

### Daemon Lock File

```go
// Lock file structure
type LockFile struct {
    PID         int       `json:"pid"`         // Daemon process ID
    StartTime   time.Time `json:"start_time"` // When lock was created
    Hostname    string    `json:"hostname"`   // Machine hostname
    Version     string    `json:"version"`    // Beads version
    LastUpdate  time.Time `json:"last_update"` // Last lock update
}

// Lock file operations
func (d *Daemon) acquireLock() (*LockFile, error) {
    lockPath := filepath.Join(d.beadsDir, ".lock")
    
    // Check if lock already exists
    if _, err := os.Stat(lockPath); err == nil {
        // Lock exists - check if process is still running
        existingLock, err := d.readLockFile()
        if err != nil {
            return nil, err
        }
        
        if d.isProcessRunning(existingLock.PID) {
            return nil, fmt.Errorf("daemon already running (PID %d)", existingLock.PID)
        }
        
        // Stale lock - remove it
        os.Remove(lockPath)
    }
    
    // Create new lock
    lock := &LockFile{
        PID:        os.Getpid(),
        StartTime:  time.Now(),
        Hostname:   d.getHostname(),
        Version:    d.version,
        LastUpdate: time.Now(),
    }
    
    err = d.writeLockFile(lock)
    if err != nil {
        return nil, err
    }
    
    // Start lock update goroutine
    go d.maintainLock(lock)
    
    return lock, nil
}
```

### Database Access Lock

```go
// Prevent concurrent database access
type DatabaseLock struct {
    file   *os.File
    path   string
    mutex  sync.Mutex
}

func (d *Daemon) acquireDatabaseLock() (*DatabaseLock, error) {
    lockPath := filepath.Join(d.beadsDir, "db.lock")
    
    file, err := os.OpenFile(lockPath, os.O_CREATE|os.O_EXCL|os.O_RDWR, 0644)
    if err != nil {
        if os.IsExist(err) {
            return nil, fmt.Errorf("database locked by another process")
        }
        return nil, err
    }
    
    return &DatabaseLock{
        file: file,
        path: lockPath,
    }, nil
}

func (l *DatabaseLock) Release() error {
    l.mutex.Lock()
    defer l.mutex.Unlock()
    
    if l.file != nil {
        l.file.Close()
        return os.Remove(l.path)
    }
    return nil
}
```

## 🛠️ Daemon Management

### Daemon Commands

```bash
# Start daemon
bd daemon start [--foreground] [--config path]

# Stop daemon
bd daemon stop [--force]

# Restart daemon
bd daemon restart [--config path]

# Check status
bd daemon status [--json]

# Kill all daemons (useful for recovery)
bd daemons killall

# Show daemon logs
bd daemon logs [--tail] [--since]
```

### Process Management

```go
// Daemon process control
func startDaemon(config *Config) error {
    // Check if already running
    if isDaemonRunning() {
        return fmt.Errorf("daemon already running")
    }
    
    // Fork process for background mode
    if !config.Foreground {
        pid, err := os.Fork()
        if err != nil {
            return err
        }
        if pid > 0 {
            // Parent process exits
            fmt.Printf("Started daemon with PID %d\n", pid)
            os.Exit(0)
        }
    }
    
    // Child process continues
    return runDaemon(config)
}

func stopDaemon() error {
    lock, err := readLockFile()
    if err != nil {
        if os.IsNotExist(err) {
            return fmt.Errorf("daemon not running")
        }
        return err
    }
    
    // Send SIGTERM to daemon
    process, err := os.FindProcess(lock.PID)
    if err != nil {
        return err
    }
    
    err = process.Signal(syscall.SIGTERM)
    if err != nil {
        return fmt.Errorf("failed to stop daemon: %w", err)
    }
    
    // Wait for graceful shutdown
    return waitForDaemonShutdown(lock.PID)
}
```

## 🔍 Monitoring and Logging

### Daemon Logging

```go
// Structured logging configuration
type Logger struct {
    level    LogLevel
    output   io.Writer
    format   string  // "json" or "text"
    fields   map[string]interface{}
}

func (l *Logger) Info(msg string, fields ...interface{}) {
    if l.level <= LogLevelInfo {
        l.log(LogLevelInfo, msg, fields...)
    }
}

func (l *Logger) Error(msg string, fields ...interface{}) {
    if l.level <= LogLevelError {
        l.log(LogLevelError, msg, fields...)
    }
}

// Log entry structure
type LogEntry struct {
    Timestamp time.Time              `json:"timestamp"`
    Level     string                 `json:"level"`
    Message   string                 `json:"message"`
    PID       int                    `json:"pid"`
    Operation string                 `json:"operation,omitempty"`
    Duration  string                 `json:"duration,omitempty"`
    Error     string                 `json:"error,omitempty"`
    Metadata  map[string]interface{}  `json:"metadata,omitempty"`
}
```

### Performance Metrics

```go
// Daemon performance tracking
type Metrics struct {
    SyncCount        int64         `json:"sync_count"`
    LastSyncTime     time.Duration  `json:"last_sync_time"`
    TotalSyncTime    time.Duration  `json:"total_sync_time"`
    AverageSyncTime  time.Duration  `json:"average_sync_time"`
    ErrorCount       int64         `json:"error_count"`
    MemoryUsage      int64         `json:"memory_usage"`
    CPUUsage         float64       `json:"cpu_usage"`
    lastUpdate       time.Time
    mutex           sync.RWMutex
}

func (m *Metrics) RecordSync(duration time.Duration, err error) {
    m.mutex.Lock()
    defer m.mutex.Unlock()
    
    m.SyncCount++
    m.TotalSyncTime += duration
    m.AverageSyncTime = m.TotalSyncTime / time.Duration(m.SyncCount)
    m.LastSyncTime = duration
    m.lastUpdate = time.Now()
    
    if err != nil {
        m.ErrorCount++
    }
}
```

### Health Checks

```go
// Daemon health monitoring
func (d *Daemon) performHealthCheck() error {
    // 1. Check database connectivity
    err := d.db.Ping()
    if err != nil {
        return fmt.Errorf("database connection failed: %w", err)
    }
    
    // 2. Check file system access
    _, err = os.Stat(d.beadsDir)
    if err != nil {
        return fmt.Errorf("beads directory inaccessible: %w", err)
    }
    
    // 3. Check lock file validity
    lock, err := d.readLockFile()
    if err != nil {
        return fmt.Errorf("lock file corrupted: %w", err)
    }
    
    // 4. Check process health
    if !d.isProcessRunning(lock.PID) {
        return fmt.Errorf("daemon process not running")
    }
    
    // 5. Check resource usage
    if d.getMemoryUsage() > d.config.MaxMemory {
        log.Warnf("Memory usage exceeds limit")
    }
    
    if d.getCPUUsage() > d.config.MaxCPUPercent {
        log.Warnf("CPU usage exceeds limit")
    }
    
    return nil
}
```

## 🔧 Configuration and Tuning

### Performance Tuning

```yaml
# High-performance configuration
daemon:
  # Reduce sync frequency for better performance
  debounce_interval: "10s"
  max_sync_frequency: "60s"
  
  # Increase batch size for large repositories
  batch_size: 500
  
  # Optimize memory usage
  max_memory: "200MB"
  
  # Reduce CPU impact
  max_cpu_percent: 5
  
  # Enable performance monitoring
  enable_metrics: true
  metrics_interval: "30s"
```

### Resource Limits

```yaml
# Resource-constrained environment
daemon:
  # Minimal resource usage
  debounce_interval: "30s"        # Less frequent syncs
  max_sync_frequency: "5m"        # Maximum sync interval
  batch_size: 50                   # Smaller batches
  
  # Strict resource limits
  max_memory: "50MB"
  max_cpu_percent: 2
  max_file_descriptors: 100
  
  # Disable resource-intensive features
  enable_metrics: false
  enable_health_checks: false
```

## 🚫 Common Daemon Issues

### Race Conditions in Multi-Clone Scenarios

**Problem**: Multiple git clones of same repository running daemons simultaneously.

**Symptoms**:
- Intermittent sync failures
- Database corruption
- Lost operations
- Git merge conflicts

**Prevention**:
```bash
# Before switching clones
bd daemons killall           # Stop all daemons
git worktree prune           # Clean orphaned worktrees

# After switching
git checkout other-branch
bd daemon start              # Start daemon in new location
```

**Detection**:
```go
// Detect potential race conditions
func (d *Daemon) detectRaceCondition() bool {
    // Check for multiple lock files in worktrees
    worktrees, err := d.getGitWorktrees()
    if err != nil {
        return false
    }
    
    runningDaemons := 0
    for _, wt := range worktrees {
        lockPath := filepath.Join(wt.Path, ".beads", ".lock")
        if _, err := os.Stat(lockPath); err == nil {
            runningDaemons++
        }
    }
    
    return runningDaemons > 1
}
```

### Daemon Recovery

**Stale Lock Files**:
```bash
# Remove stale daemon locks
rm .beads/.lock
rm .beads/db.lock

# Restart daemon
bd daemon start
```

**Corrupted State**:
```bash
# Complete daemon reset
bd daemons killall           # Stop all daemons
rm .beads/beads.db*         # Remove corrupted database
bd daemon start              # Restart with fresh state
```

## 🔗 Related Documentation

- [Architecture Overview](overview.md) - Three-layer system context
- [JSONL Layer](jsonl-layer.md) - Operational layer details
- [SQLite Layer](sqlite-layer.md) - Database layer information
- [Data Flow](data-flow.md) - Complete system flow
- [Recovery](../recovery/) - Daemon failure recovery
- [Multi-Machine Considerations](overview.md#multi-machine-sync-considerations)

## 📚 See Also

- [CLI Reference](../cli-reference/daemon-commands.md) - Daemon command reference
- [Recovery Sync Failures](../recovery/sync-failures.md) - Sync troubleshooting
- [Multi-Agent Coordination](../multi-agent/) - Multi-daemon scenarios
- [Performance](../best-practices/performance.md) - Performance optimization