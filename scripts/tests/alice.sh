#!/bin/bash
set -e

# --- 0. 环境与工具准备 ---
GOSH_CMD="gosh" # 确保已编译或 alias 到 cargo run
BASE_DIR="/tmp/alice_demo_signed"

# 隔离 Gosh 配置
export GOSH_CONFIG_PATH="$BASE_DIR/config"
# 隔离 SSH 密钥目录
SSH_DIR="$BASE_DIR/ssh_keys"
# 模拟仓库目录
REPO_DIR="$BASE_DIR/repos"

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${BLUE}[STEP]${NC} $1"; }
info() { echo -e "${GREEN}  ->${NC} $1"; }
err()  { echo -e "${RED}  -> ERROR:${NC} $1"; exit 1; }

# 检查 Git 版本 (SSH 签名需要 Git 2.34+)
GIT_VERSION=$(git --version | awk '{print $3}')
info "Git Version: $GIT_VERSION (SSH Signing requires 2.34+)"

# --- 1. 清理与沙盒初始化 ---
log "Initializing Sandbox at $BASE_DIR..."
rm -rf "$BASE_DIR"
mkdir -p "$GOSH_CONFIG_PATH"
mkdir -p "$SSH_DIR"
mkdir -p "$REPO_DIR"

# --- 2. 生成隔离的 SSH 密钥对 (模拟 Work 和 Personal) ---
log "Generating isolated SSH keys..."

# 生成 Work Key (无密码)
ssh-keygen -t ed25519 -C "alice@contoso.com" -f "$SSH_DIR/id_work" -N "" -q
info "Generated Work Key: $SSH_DIR/id_work"

# 生成 Personal Key (无密码)
ssh-keygen -t ed25519 -C "alice@alice.com" -f "$SSH_DIR/id_personal" -N "" -q
info "Generated Personal Key: $SSH_DIR/id_personal"

# --- 3. 使用 Gosh 创建 Profile 并注入签名配置 ---
log "Creating Gosh Profiles..."

# 3.1 创建基础 Work Profile
$GOSH_CMD -c "Alice Work" "alice@contoso.com" "work"

# 3.2 手动追加 SSH 签名配置到 Work Profile
# 这里演示了 Gosh 的灵活性：你可以手动编辑生成的 .gitconfig
WORK_PROFILE="$GOSH_CONFIG_PATH/profiles/work.gitconfig"
cat >> "$WORK_PROFILE" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_work.pub
[commit]
    gpgsign = true
[core]
    # 强制 SSH 使用指定的私钥，且忽略用户本机的 ~/.ssh/config
    sshCommand = ssh -i $SSH_DIR/id_work -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
info "Configured Work Profile with SSH Signing"

# 3.3 创建并配置 Personal Profile
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
info "Configured Personal Profile with SSH Signing"

# --- 4. 场景测试 ---

# === 场景 A: 克隆并验证签名 (Setup Mode) ===
log "Scenario A: Setup Mode (Work Repo)"
cd "$REPO_DIR"

# 模拟远程仓库
git init --bare --quiet "backend.git"

# 使用 Gosh 克隆 (Clone -> Infer -> Switch)
# 注意：这里我们 Clone 本地路径，但 core.sshCommand 依然会被配置进去，这是符合预期的
$GOSH_CMD work clone "$REPO_DIR/backend.git" work-backend
cd work-backend

# 提交代码
touch work.txt
git add work.txt
git commit -m "Work commit" > /dev/null

# 验证签名
# 检查 raw commit data 中是否包含 gpgsig 字段
if git cat-file commit HEAD | grep -q "gpgsig"; then
    info "✅ Commit is SIGNED."
else
    err "Commit is NOT signed."
fi

# 验证使用的是哪个 Key
SIGNER_KEY=$(git config user.signingkey)
if [[ "$SIGNER_KEY" == *"/id_work.pub" ]]; then
    info "✅ Signed with WORK Key."
else
    err "Wrong key used: $SIGNER_KEY"
fi

# === 场景 B: 切换身份并验证签名 (Switch Mode) ===
log "Scenario B: Switch Mode (Existing Repo)"
cd "$REPO_DIR"
git init --quiet "oss-project"
cd oss-project

# 切换到 Personal
$GOSH_CMD personal

# 提交
touch fun.txt
git add fun.txt
git commit -m "Personal commit" > /dev/null

# 验证签名
if git cat-file commit HEAD | grep -q "gpgsig"; then
    info "✅ Commit is SIGNED."
else
    err "Commit is NOT signed."
fi

SIGNER_KEY=$(git config user.signingkey)
if [[ "$SIGNER_KEY" == *"/id_personal.pub" ]]; then
    info "✅ Signed with PERSONAL Key."
else
    err "Wrong key used: $SIGNER_KEY"
fi

# === 场景 C: 临时执行与密钥隔离 (Exec Mode) ===
log "Scenario C: Ephemeral Execution (Security Check)"
# 当前在 oss-project (Personal)，我们想用 Work 身份签个名

# 执行 gosh work commit
$GOSH_CMD work commit --allow-empty -m "Hotfix via Exec" > /dev/null

# 验证最后一次提交的签名
# 注意：Exec 模式下，Gosh 会通过 -c user.signingkey="" 先清空，再注入 work profile
# 如果这一步成功且签名了，说明 Gosh 正确注入了 id_work.pub

LATEST_COMMIT_MSG=$(git log -1 --pretty=%B)
info "Latest commit: $LATEST_COMMIT_MSG"

# 这里的验证比较 tricky，因为 git log 不会直接显示是哪个 key 文件签的
# 但如果签名成功，且 Email 是 Work，基本证明逻辑通了
AUTHOR=$(git log -1 --pretty=format:'%ae')
if [ "$AUTHOR" == "alice@contoso.com" ]; then
    info "✅ Ephemeral commit author is Correct (Work)."
else
    err "Ephemeral commit author mismatch: $AUTHOR"
fi

if git cat-file commit HEAD | grep -q "gpgsig"; then
    info "✅ Ephemeral commit is SIGNED (Injection worked)."
else
    err "Ephemeral commit failed to sign (Injection failed)."
fi

# --- 5. 清理 ---
log "Done. To inspect, check $BASE_DIR before exiting."
# rm -rf "$BASE_DIR" # 注释掉此行以便你检查文件
echo -e "${GREEN}🎉 Demo completed without touching ~/.ssh or ~/.gnupg!${NC}"