#!/bin/bash
set -e

# Use absolute path to the compiled binary
# DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." >/dev/null 2>&1 && pwd)"
NAJ_CMD="naj"
BASE_DIR="/tmp/init_test_env"

export NAJ_CONFIG_PATH="$BASE_DIR/config"
REPO_DIR="$BASE_DIR/repos"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${BLUE}[STEP]${NC} $1"; }
info() { echo -e "${GREEN}  ->${NC} $1"; }
err()  { echo -e "${RED}  -> ERROR:${NC} $1"; exit 1; }

log "Initializing Sandbox at $BASE_DIR..."
rm -rf "$BASE_DIR"
mkdir -p "$NAJ_CONFIG_PATH"
mkdir -p "$REPO_DIR"

log "Creating profile 'work'"
$NAJ_CMD -c "Work User" "work@example.com" "work"

log "Testing 'naj work init repo'"
cd "$REPO_DIR"

$NAJ_CMD work init my_repo

if [ ! -d "my_repo/.git" ]; then
    err "Repository was not created at my_repo/.git"
fi

cd my_repo
if ! git config --local --get include.path | grep -q "work.gitconfig"; then
    err "Profile was not applied correctly in the initialized repo."
fi

info "✅ Successfully applied profile to newly initialized repository."

log "Testing 'naj work init -b main another_repo'"
cd "$REPO_DIR"

$NAJ_CMD work init -b main another_repo

if [ ! -d "another_repo/.git" ]; then
    err "Repository was not created at another_repo/.git"
fi

cd another_repo
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
    err "Branch is not main, it is: $BRANCH"
fi

if ! git config --local --get include.path | grep -q "work.gitconfig"; then
    err "Profile was not applied correctly in the initialized repo with options."
fi

info "✅ Successfully applied profile with options."

log "Cleaning up"
rm -rf "$BASE_DIR"
echo -e "${GREEN}🎉 All init tests passed!${NC}"
