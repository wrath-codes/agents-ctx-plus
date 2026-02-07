package config

import (
	"time"

	"github.com/spf13/viper"
)

// Config holds all application configuration
type Config struct {
	Server    ServerConfig    `mapstructure:"server"`
	Database  DatabaseConfig  `mapstructure:"database"`
	Logging   LoggingConfig   `mapstructure:"logging"`
	Agents    AgentsConfig    `mapstructure:"agents"`
	Workflows WorkflowsConfig `mapstructure:"workflows"`
	API       APIConfig       `mapstructure:"api"`
	Migrations MigrationsConfig `mapstructure:"migrations"`
}

// ServerConfig holds server configuration
type ServerConfig struct {
	Host           string        `mapstructure:"host"`
	Port           int           `mapstructure:"port"`
	Timeout        time.Duration `mapstructure:"timeout"`
	ReadTimeout    time.Duration `mapstructure:"read_timeout"`
	WriteTimeout   time.Duration `mapstructure:"write_timeout"`
	MaxHeaderBytes int           `mapstructure:"max_header_bytes"`
}

// DatabaseConfig holds database configuration
type DatabaseConfig struct {
	CoordinationDB DBConfig `mapstructure:"coordination_db"`
	BeadsDB        DBConfig `mapstructure:"beads_db"`
	TempoliteDB    DBConfig `mapstructure:"tempolite_db"`
}

// DBConfig holds individual database configuration
type DBConfig struct {
	Path            string        `mapstructure:"path"`
	MaxOpenConns    int           `mapstructure:"max_open_conns"`
	MaxIdleConns    int           `mapstructure:"max_idle_conns"`
	ConnMaxLifetime time.Duration `mapstructure:"conn_max_lifetime"`
	BusyTimeout     time.Duration `mapstructure:"busy_timeout"`
}

// LoggingConfig holds logging configuration
type LoggingConfig struct {
	Level  string `mapstructure:"level"`
	Format string `mapstructure:"format"`
	Output string `mapstructure:"output"`
}

// AgentsConfig holds agent configuration
type AgentsConfig struct {
	DefaultMaxWorkload int           `mapstructure:"default_max_workload"`
	HeartbeatInterval  time.Duration `mapstructure:"heartbeat_interval"`
	Timeout            time.Duration `mapstructure:"timeout"`
}

// WorkflowsConfig holds workflow configuration
type WorkflowsConfig struct {
	DefaultPriority int           `mapstructure:"default_priority"`
	MaxRetries      int           `mapstructure:"max_retries"`
	RetryDelay      time.Duration `mapstructure:"retry_delay"`
}

// APIConfig holds API configuration
type APIConfig struct {
	CORS      CORSConfig      `mapstructure:"cors"`
	RateLimit RateLimitConfig `mapstructure:"rate_limit"`
}

// CORSConfig holds CORS configuration
type CORSConfig struct {
	AllowedOrigins []string `mapstructure:"allowed_origins"`
	AllowedMethods []string `mapstructure:"allowed_methods"`
	AllowedHeaders []string `mapstructure:"allowed_headers"`
}

// RateLimitConfig holds rate limiting configuration
type RateLimitConfig struct {
	Enabled           bool `mapstructure:"enabled"`
	RequestsPerMinute int  `mapstructure:"requests_per_minute"`
	Burst             int  `mapstructure:"burst"`
}

// MigrationsConfig holds migration configuration
type MigrationsConfig struct {
	Directory   string `mapstructure:"directory"`
	AutoMigrate bool   `mapstructure:"auto_migrate"`
}

// Load loads configuration from file
func Load(path string) (*Config, error) {
	viper.SetConfigFile(path)
	
	// Set defaults
	setDefaults()
	
	if err := viper.ReadInConfig(); err != nil {
		return nil, err
	}
	
	var config Config
	if err := viper.Unmarshal(&config); err != nil {
		return nil, err
	}
	
	return &config, nil
}

// setDefaults sets default configuration values
func setDefaults() {
	viper.SetDefault("server.host", "0.0.0.0")
	viper.SetDefault("server.port", 8080)
	viper.SetDefault("server.timeout", "30s")
	viper.SetDefault("server.read_timeout", "60s")
	viper.SetDefault("server.write_timeout", "60s")
	
	viper.SetDefault("database.coordination_db.path", "./data/coordination.db")
	viper.SetDefault("database.coordination_db.max_open_conns", 1)
	viper.SetDefault("database.coordination_db.max_idle_conns", 1)
	viper.SetDefault("database.coordination_db.conn_max_lifetime", "1h")
	
	viper.SetDefault("logging.level", "info")
	viper.SetDefault("logging.format", "json")
	viper.SetDefault("logging.output", "stdout")
	
	viper.SetDefault("agents.default_max_workload", 5)
	viper.SetDefault("agents.heartbeat_interval", "30s")
	viper.SetDefault("agents.timeout", "30m")
	
	viper.SetDefault("workflows.default_priority", 2)
	viper.SetDefault("workflows.max_retries", 3)
	viper.SetDefault("workflows.retry_delay", "5s")
	
	viper.SetDefault("api.rate_limit.enabled", true)
	viper.SetDefault("api.rate_limit.requests_per_minute", 100)
	viper.SetDefault("api.rate_limit.burst", 20)
	
	viper.SetDefault("migrations.directory", "./migrations")
	viper.SetDefault("migrations.auto_migrate", true)
}