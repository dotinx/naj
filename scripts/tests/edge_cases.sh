#!/bin/bash
set -e

# --- 配置 ---
NAJ_CMD="naj"
BASE_DIR="/tmp/naj_edge_test"
export NAJ_CONFIG_PATH="$BASE_DIR/config"
REPO_DIR="$BASE_DIR/repos"

# 颜色
PASS='\033[0;32m'
FAIL='\033[0;31m'
NC='\033[0m'
log() { echo -e "\n\033[0;34m[TEST] $1\033[0m"; }

# --- 初始化 ---
rm -rf "$BASE_DIR"
mkdir -p "$NAJ_CONFIG_PATH" "$REPO_DIR"

# 创建一个 Profile
log "Creating Profile..."
$NAJ_CMD -c "Edge User" "edge@test.com" "edge"

# --- 测试 1: 子目录执行 ---
log "Scenario 1: Running from a deep subdirectory"
cd "$REPO_DIR"
git init --quiet deep-repo
cd deep-repo
mkdir -p src/deep/level
cd src/deep/level

echo "Current dir: $(pwd)"
echo "Executing 'naj edge' from subdirectory..."

# 执行 switch
$NAJ_CMD edge

# 验证
# 我们需要回到根目录看 config，或者直接用 git config
CONFIG_EMAIL=$(git config user.email)
if [ "$CONFIG_EMAIL" == "edge@test.com" ]; then
    echo -e "${PASS}✓ Subdirectory switch worked!${NC}"
else
    echo -e "${FAIL}✗ Failed! Git config not updated correctly from subdir.${NC}"
    exit 1
fi

# --- 测试 2: 带空格的路径 ---
log "Scenario 2: Repository path with SPACES"
cd "$REPO_DIR"
# 创建带空格的目录
DIR_WITH_SPACE="My Cool Project"
mkdir "$DIR_WITH_SPACE"
cd "$DIR_WITH_SPACE"
git init --quiet

echo "Current dir: $(pwd)"
echo "Executing 'naj edge'..."

$NAJ_CMD edge

# 验证
CONFIG_EMAIL=$(git config user.email)
if [ "$CONFIG_EMAIL" == "edge@test.com" ]; then
    echo -e "${PASS}✓ Path with spaces worked!${NC}"
else
    echo -e "${FAIL}✗ Failed! Path with spaces broke the include.${NC}"
    # 调试信息：打印出 config 看看路径变成啥样了
    cat .git/config
    exit 1
fi

echo -e "\n${PASS}🎉 All Edge Cases Passed! v1.0 is ready to ship.${NC}"