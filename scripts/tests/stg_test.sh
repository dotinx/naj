#!/bin/bash
set -e

# ==========================================
# 0. 全局配置与环境准备
# ==========================================

# 编译 (确保用的是最新代码)
echo "Compiling naj..."
cargo build --quiet

# --- 关键修复：获取绝对路径 ---
PROJECT_ROOT=$(pwd)
NAJ_BIN="$PROJECT_ROOT/target/debug/naj"

if [ ! -f "$NAJ_BIN" ]; then
    echo -e "\033[0;31mError: Binary not found at $NAJ_BIN\033[0m"
    exit 1
fi

# 基础目录
BASE_DIR=$(mktemp -d)
# 关键修复 1: NAJ_CONFIG_PATH 指向目录
export NAJ_CONFIG_PATH="$BASE_DIR/naj_config"
export NAJ_DEBUG=1 # 启用 Debug 输出
# 配置文件具体路径
NAJ_TOML="$NAJ_CONFIG_PATH/config.toml"
# Profile 存放路径
PROFILE_DIR="$NAJ_CONFIG_PATH/profiles"

# SSH 和 仓库
SSH_DIR="$BASE_DIR/ssh_keys"
REPO_DIR="$BASE_DIR/repo"
ALLOWED_SIGNERS="$BASE_DIR/allowed_signers"

# 颜色
PASS='\033[0;32m'
FAIL='\033[0;31m'
INFO='\033[0;34m'
WARN='\033[1;33m'
NC='\033[0m'

log() { echo -e "\n${INFO}[STEP]${NC} $1"; }
ok()  { echo -e "${PASS}  ✓${NC} $1"; }
err() { echo -e "${FAIL}  ✗ ERROR:${NC} $1"; exit 1; }

# 初始化目录结构
rm -rf "$BASE_DIR"
mkdir -p "$NAJ_CONFIG_PATH" # 创建配置根目录
mkdir -p "$PROFILE_DIR"     # 创建 Profile 目录
mkdir -p "$SSH_DIR"
mkdir -p "$REPO_DIR"

# 预先生成 config.toml (指定 profile_dir)
# 默认策略设为 include
cat > "$NAJ_TOML" <<EOF
profile_dir = "$PROFILE_DIR"

[strategies]
switch = "include"
EOF

log "Sandbox initialized at: $BASE_DIR"
log "Naj Config: $NAJ_TOML"

# ==========================================
# 1. 准备 SSH 密钥与信任链
# ==========================================
log "Generating Keys & Establishing Trust..."

# Alice
ssh-keygen -t ed25519 -C "alice@corp.com" -f "$SSH_DIR/id_alice" -N "" -q
echo "alice@corp.com $(cat $SSH_DIR/id_alice.pub)" >> "$ALLOWED_SIGNERS"

# Bob
ssh-keygen -t ed25519 -C "bob@home.org" -f "$SSH_DIR/id_bob" -N "" -q
echo "bob@home.org $(cat $SSH_DIR/id_bob.pub)" >> "$ALLOWED_SIGNERS"

ok "Keys generated and added to allowed_signers"

# ==========================================
# 2. 创建 Profiles (带 SSH 签名配置)
# ==========================================
log "Creating Naj Profiles..."

# --> Alice Profile
$NAJ_BIN -c "Alice Corp" "alice@corp.com" alice
# 追加详细配置
cat >> "$PROFILE_DIR/alice.gitconfig" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_alice.pub
[commit]
    gpgsign = true
[core]
    sshCommand = ssh -i $SSH_DIR/id_alice -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
ok "Profile 'alice' created"

# --> Bob Profile
$NAJ_BIN -c "Bob Home" "bob@home.org" bob
cat >> "$PROFILE_DIR/bob.gitconfig" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_bob.pub
[commit]
    gpgsign = true
[core]
    sshCommand = ssh -i $SSH_DIR/id_bob -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
ok "Profile 'bob' created"

# ==========================================
# 3. 初始化 Git 仓库
# ==========================================
cd "$REPO_DIR"
git init --quiet
# 关键修复 2: 告诉 Git 信任这些公钥，否则签名状态会是 'U'
git config gpg.ssh.allowedSignersFile "$ALLOWED_SIGNERS"

# ==========================================
# 4. 辅助函数
# ==========================================

# 动态修改策略
set_strategy() {
    local strat=$1
    # 使用临时文件修改 TOML
    grep -v "switch =" "$NAJ_TOML" > "$NAJ_TOML.tmp"
    echo "switch = \"$strat\"" >> "$NAJ_TOML.tmp"
    mv "$NAJ_TOML.tmp" "$NAJ_TOML"
    echo -e "${WARN}Strategy set to: $strat${NC}"
}

# 验证身份和签名
verify_commit() {
    local expected_email=$1
    local mode=$2 # "include" or "override"

    # 1. 验证配置文件结构
    if [ "$mode" == "include" ]; then
        if ! grep -q "\[include\]" .git/config; then err "Expected [include] in .git/config"; fi
    else
        if grep -q "\[include\]" .git/config; then err "Expected NO [include] in .git/config"; fi
        if ! grep -q "\[user\]" .git/config; then err "Expected [user] in .git/config"; fi
    fi

    # 2. 验证最后一次提交的作者和签名
    local stats=$(git log -1 --pretty=format:'%G?|%ae')
    local sig_status=${stats%%|*}
    local author=${stats##*|}

    if [ "$author" != "$expected_email" ]; then
        err "Author mismatch! Got: $author, Expected: $expected_email"
    fi

    if [ "$sig_status" != "G" ]; then
        git log -1 --show-signature
        err "Signature failed! Status: $sig_status (Expected 'G')"
    fi
    
    ok "Verified: $expected_email ($mode mode) [Sig: $sig_status]"
}

inject_dirty() {
    git config user.name "Dirty Hacker"
    git config user.email "dirty@hack.com"
}

make_commit() {
    touch "file_$RANDOM"
    git add .
    git commit -m "$1" > /dev/null
}

dump_state() {
    echo -e "\n${WARN}[DEBUG STATE]${NC} --------------------------------"
    echo -e "${INFO}1. Current Strategy (in $NAJ_TOML):${NC}"
    grep "switch =" "$NAJ_TOML" || echo "ERROR: No switch strategy found!"
    
    echo -e "${INFO}2. Git Config Content (.git/config):${NC}"
    cat .git/config
    echo -e "${WARN}---------------------------------------------${NC}\n"
}

# ==========================================
# 5. 执行 8 轮矩阵测试
# ==========================================

log "🚀 Starting 8-Round Matrix Test"

# R1: Alice (Soft Include)
log "Round 1: Alice (include)"
set_strategy "include"
inject_dirty 
# 修复：移除 switch，直接跟 profile id
$NAJ_BIN alice
make_commit "R1"
verify_commit "alice@corp.com" "include"
# ======================================================
# 🔍 DEBUGGING ROUND 2
# ======================================================
log "Round 2: Bob (INCLUDE - Cleaning)"
set_strategy "INCLUDE"

# 1. 注入脏数据
inject_dirty 

echo -e "${YELLOW}>>> BEFORE execution:${NC}"
dump_state # 打印执行前的状态

# 2. 执行命令并捕获输出
echo -e "${YELLOW}>>> EXECUTING 'naj bob'...${NC}"
$NAJ_BIN bob
CMD_EXIT_CODE=$?

echo -e "${YELLOW}>>> AFTER execution:${NC}"
dump_state # 打印执行后的状态

# 3. 分析结果
if [ $CMD_EXIT_CODE -ne 0 ]; then
    err "Command failed with exit code $CMD_EXIT_CODE"
fi

# 检查脏数据是否被删除
# 重点调试：如果是 INCLUDE 模式，naj 应该物理删除了 [user] 块
if grep -q "Dirty Hacker" .git/config; then 
    echo -e "${FAIL}DEBUG INFO: Found 'Dirty Hacker' in config.${NC}"
    echo -e "${FAIL}Hypothesis: Naj logic treated 'INCLUDE' as soft 'include'.${NC}"
    err "INCLUDE strategy failed to clean dirty config"
fi

make_commit "R2"
verify_commit "bob@home.org" "include"

# ... (后面的 Round 3 - 8 保持不变) ...
# R3: Alice (Soft Override)
log "Round 3: Alice (override)"
set_strategy "override"
$NAJ_BIN alice
make_commit "R3"
verify_commit "alice@corp.com" "override"

# R4: Bob (Hard OVERRIDE)
log "Round 4: Bob (OVERRIDE - Cleaning)"
set_strategy "OVERRIDE"
# 注入一个脏的 include path
git config --local include.path "/tmp/fake"
$NAJ_BIN bob
if grep -q "include.path" .git/config; then err "OVERRIDE strategy failed to clean include"; fi
make_commit "R4"
verify_commit "bob@home.org" "override"

# R5: Alice (Include from Override)
log "Round 5: Alice (include)"
set_strategy "include"
$NAJ_BIN alice
make_commit "R5"
verify_commit "alice@corp.com" "include"

# R6: Bob (Hard INCLUDE)
log "Round 6: Bob (INCLUDE)"
set_strategy "INCLUDE"
$NAJ_BIN bob
make_commit "R6"
verify_commit "bob@home.org" "include"

# R7: Alice (Manual Mess + Override)
log "Round 7: Alice (override with mess)"
set_strategy "override"
git config core.sshCommand "echo malicious"
$NAJ_BIN alice
make_commit "R7"
verify_commit "alice@corp.com" "override"

# R8: Bob (Soft Strategy + Force Flag)
log "Round 8: Bob (switch -f)"
set_strategy "include" 
inject_dirty
# 修复：使用 -f bob，移除 switch，符合 Usage: naj [OPTIONS] [PROFILE_ID]
$NAJ_BIN -f bob 
if grep -q "Dirty Hacker" .git/config; then err "Switch -f failed to sanitize"; fi
make_commit "R8"
verify_commit "bob@home.org" "include"

echo ""
echo -e "${PASS}🎉🎉 ALL TESTS PASSED! Naj is solid. 🎉🎉${NC}"
rm -rf "$BASE_DIR"