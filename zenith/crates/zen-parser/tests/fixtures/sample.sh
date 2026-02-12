#!/bin/bash
# ═══════════════════════════════════════════════════════════════════
# sample.sh — Comprehensive Bash fixture for zen-parser extraction
# ═══════════════════════════════════════════════════════════════════

# ── Functions ───────────────────────────────────────────────────────

# Greet a user by name.
# Accepts one positional argument.
greet() {
    local name="${1:-World}"
    echo "Hello, ${name}!"
}

# Clean up temporary files.
# Removes all files matching the app prefix.
function cleanup {
    rm -rf /tmp/myapp_*
    echo "Cleanup complete"
}

# Deploy the application to a target environment.
# Usage: deploy <env> <version>
function deploy() {
    local env="$1"
    local version="$2"
    echo "Deploying v${version} to ${env}"
}

# A simple one-liner function
say_hi() { echo "Hi!"; }

# ── Variable assignments ───────────────────────────────────────────

FOO="bar"
BAZ=42
EMPTY=

# ── Export statements ──────────────────────────────────────────────

# Database connection URL
export DATABASE_URL="postgres://localhost:5432/mydb"

export API_KEY="sk-secret-key-123"

# Readonly constants
readonly MAX_RETRIES=3
readonly APP_NAME="myapp"

# Local variable (inside function context, but valid syntax)
local COUNTER=0

# Declare variants
declare -x EXPORTED_VAR="exported_value"
declare -i INTEGER_VAR=42
declare -r DECLARED_READONLY="immutable"

# ── Aliases ────────────────────────────────────────────────────────

# List files in long format
alias ll='ls -la'
alias gs='git status'
alias gp='git push origin main'

# ── Array declarations ─────────────────────────────────────────────

# Indexed array of fruits
declare -a FRUITS=(apple banana cherry)
# Associative array of config values
declare -A CONFIG=([host]=localhost [port]=8080 [debug]=true)

# ── Export of PATH and functions ───────────────────────────────────

export PATH="/usr/local/bin:$PATH"

# ── Conditional constructs ─────────────────────────────────────────

# Check if a file or directory exists
if [ -f "$1" ]; then
    echo "File exists"
elif [ -d "$1" ]; then
    echo "Directory exists"
else
    echo "Not found"
fi

# Case statement for command routing
case "$1" in
    start)
        echo "Starting service..."
        ;;
    stop)
        echo "Stopping service..."
        ;;
    restart)
        echo "Restarting..."
        ;;
    *)
        echo "Usage: $0 {start|stop|restart}"
        ;;
esac

# ── Loops ──────────────────────────────────────────────────────────

# Iterate over numbers
for i in 1 2 3 4 5; do
    echo "Number: $i"
done

# C-style for loop
for ((j=0; j<10; j++)); do
    echo "Index: $j"
done

# While loop reading lines
while read -r line; do
    echo "Line: $line"
done < input.txt

# Until loop with counter
until [ "$COUNTER" -ge 5 ]; do
    ((COUNTER++))
done

# ── Select statement ──────────────────────────────────────────────

select option in "Build" "Test" "Deploy" "Quit"; do
    case $option in
        "Quit") break ;;
        *) echo "Running: $option" ;;
    esac
done

# ── Heredocs ───────────────────────────────────────────────────────

cat <<EOF
This is a heredoc.
It spans multiple lines.
Variable expansion: $FOO
EOF

cat <<-INDENTED
	This is an indented heredoc.
	Tabs are stripped.
INDENTED

# Here string
grep "pattern" <<< "$FOO"

# ── Subshell and command groups ────────────────────────────────────

# Subshell runs in a child process
(cd /tmp && ls -la)

# Command group runs in current shell
{ echo "grouped"; echo "commands"; }

# ── Pipelines ──────────────────────────────────────────────────────

# Multi-stage pipeline
ls -la | grep ".rs" | sort | wc -l

ps aux | grep "node" | awk '{print $2}'

# ── Command substitution ──────────────────────────────────────────

CURRENT_DATE=$(date +%Y-%m-%d)
HOSTNAME_VAL=$(hostname)

# ── Traps ──────────────────────────────────────────────────────────

# Clean up on exit signals
trap 'echo "Caught signal"; cleanup' EXIT INT TERM

trap 'echo "Debugging line $LINENO"' DEBUG

# ── Source / dot commands ──────────────────────────────────────────

# Load utility functions
source ./lib/utils.sh
. ./lib/helpers.sh

# ── C-style for loop ──────────────────────────────────────────────

# Iterate with C-style index
for ((i=0; i<10; i++)); do
    echo "Index: $i"
done

# ── Negated commands ──────────────────────────────────────────────

# Negate grep exit status
! grep -q "error" /var/log/syslog

# ── Standalone test commands ─────────────────────────────────────

# Standalone test expression
[[ -f "/etc/hosts" && -r "/etc/hosts" ]]

# ── Unset commands ────────────────────────────────────────────────

# Remove a variable
unset TEMP_VAR

# Remove a function
unset -f old_func

# ── List (logical chains) ────────────────────────────────────────

# Conditional chain with && and ||
(( count > 5 )) && echo "big" || echo "small"

# ── Nested function with export ────────────────────────────────────

# Process data from stdin
function process_data() {
    local input
    input=$(cat -)
    echo "$input" | tr '[:lower:]' '[:upper:]'
}
