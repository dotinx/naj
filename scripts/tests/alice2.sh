#!/bin/bash
set -e

# --- 0. 环境与工具准备 ---
GOSH_CMD="gosh" # 确保已编译或 alias 到 cargo run
BASE_DIR="/tmp/alice_demo_debug"

# 隔离 Gosh 配置
export GOSH_CONFIG_PATH="$BASE_DIR/config"
# 隔离 SSH 密钥目录
SSH_DIR="$BASE_DIR/ssh_keys"
# 模拟仓库目录
REPO_DIR="$BASE_DIR/repos"

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
info() { echo -e "${GREEN}  ->${NC} $1"; }
err()  { echo -e "${RED}  -> ERROR:${NC} $1"; exit 1; }

# === 调试核心函数 ===
debug_inspect() {
    echo -e "${YELLOW}--- 🔍 DEBUG INSPECTION ---${NC}"
    echo -e "${YELLOW}[1. Local Config (.git/config)]${NC}"
    # 只显示相关的配置
    git config --local --list | grep -E "user|include|core.sshCommand|gpg" || echo "  (Clean/No local config overrides)"
    
    echo -e "${YELLOW}[2. Latest Commit Details]${NC}"
    # 显示签名、作者、提交者
    git log -1 --show-signature --pretty=fuller
    echo -e "${YELLOW}---------------------------${NC}"
}

# 检查 Git 版本
GIT_VERSION=$(git --version | awk '{print $3}')
info "Git Version: $GIT_VERSION (SSH Signing requires 2.34+)"

# --- 1. 清理与沙盒初始化 ---
log "Initializing Sandbox at $BASE_DIR..."
rm -rf "$BASE_DIR"
mkdir -p "$GOSH_CONFIG_PATH"
mkdir -p "$SSH_DIR"
mkdir -p "$REPO_DIR"

# --- 2. 生成隔离的 SSH 密钥对 ---
log "Generating isolated SSH keys..."
ssh-keygen -t ed25519 -C "alice@contoso.com" -f "$SSH_DIR/id_work" -N "" -q
info "Generated Work Key: .../id_work"
ssh-keygen -t ed25519 -C "alice@alice.com" -f "$SSH_DIR/id_personal" -N "" -q
info "Generated Personal Key: .../id_personal"

# --- 3. 使用 Gosh 创建 Profile ---
log "Creating Gosh Profiles..."

# 3.1 Work Profile
$GOSH_CMD -c "Alice Work" "alice@contoso.com" "work"
WORK_PROFILE="$GOSH_CONFIG_PATH/profiles/work.gitconfig"
cat >> "$WORK_PROFILE" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_work.pub
[commit]
    gpgsign = true
[core]
    sshCommand = ssh -i $SSH_DIR/id_work -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
info "Configured Work Profile (SSH Signing Enabled)"

# 3.2 Personal Profile
$GOSH_CMD -c "Alice Personal" "alice@alice.com" "personal"
PERSONAL_PROFILE="$GOSH_CONFIG_PATH/profiles/personal.gitconfig"
cat >> "$PERSONAL_PROFILE" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_personal.pub
[commit]
    gpgsign = true
[core]
    sshCommand = ssh -i $SSH_DIR/id_personal -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
info "Configured Personal Profile (SSH Signing Enabled)"

# --- 4. 场景测试 ---

# === 场景 A: 克隆并验证签名 (Setup Mode) ===
log "Scenario A: Setup Mode (Work Repo)"
cd "$REPO_DIR"
git init --bare --quiet "backend.git"

# 使用 Gosh 克隆
info "Running: gosh work clone ..."
$GOSH_CMD work clone "$REPO_DIR/backend.git" work-backend
cd work-backend

# 提交代码
touch work.txt
git add work.txt
git commit -m "Work commit (Scenario A)"

# 🔍 查看日志
debug_inspect

# === 场景 B: 切换身份并验证签名 (Switch Mode) ===
log "Scenario B: Switch Mode (Existing Repo)"
cd "$REPO_DIR"
git init --quiet "oss-project"
cd oss-project

# 切换到 Personal
info "Running: gosh personal (Switching...)"
$GOSH_CMD personal

# 提交
touch fun.txt
git add fun.txt
git commit -m "Personal commit (Scenario B)"

# 🔍 查看日志
debug_inspect

# === 场景 C: 临时执行与密钥隔离 (Exec Mode) ===
log "Scenario C: Ephemeral Execution (Security Check)"
info "Current Profile is: Personal (oss-project)"
info "Executing 'gosh work commit' (Should use Work Identity temporarily)..."

# 执行 gosh work commit
# 注意：这里我们不再重定向到 /dev/null，我们要看 git 的原生输出
$GOSH_CMD work commit --allow-empty -m "Hotfix via Exec (Scenario C)"

# 🔍 查看日志
# 这里的重点是：
# 1. Author 必须是 Work
# 2. 签名必须有效 (Good signature)
# 3. 但 Local Config (上面显示的 [1]) 必须依然显示 Personal 的 include
debug_inspect

# 验证持久配置未变
if grep -q "personal.gitconfig" .git/config; then
    info "✅ Persistent config verification: Still using 'personal' profile."
else
    err "Persistent config was altered!"
fi

# --- 5. 清理 ---
log "Done. Check the debug logs above."
echo -e "${GREEN}🎉 Debug run completed.${NC}"