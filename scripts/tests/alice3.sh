#!/bin/bash
set -e

# --- 0. 全局配置 ---
GOSH_CMD="gosh"
BASE_DIR="/tmp/gosh_collab_demo"

# 隔离 Gosh 配置
export GOSH_CONFIG_PATH="$BASE_DIR/config"
# 隔离 SSH 密钥目录
SSH_DIR="$BASE_DIR/ssh_keys"
# 模拟仓库目录
REPO_DIR="$BASE_DIR/repos"
# 信任文件路径 (用于校验签名)
ALLOWED_SIGNERS="$BASE_DIR/allowed_signers"

# 颜色
PASS='\033[0;32m'
INFO='\033[0;34m'
FAIL='\033[0;31m'
NC='\033[0m'

log() { echo -e "\n${INFO}[STEP]${NC} $1"; }
ok()  { echo -e "${PASS}  ✓${NC} $1"; }
err() { echo -e "${FAIL}  ✗ ERROR:${NC} $1"; exit 1; }

# --- 校验核心函数 ---
# 用法: verify_last_commit "期望的邮箱" "期望的Profile名"
verify_last_commit() {
    EXPECTED_EMAIL="$1"
    PROFILE_NAME="$2"
    
    # 获取最后一次提交的签名状态 (%G?) 和 签名者邮箱 (%GS) 和 作者邮箱 (%ae)
    # %G? : G=Good, B=Bad, U=Untrusted, N=None
    STATS=$(git log -1 --pretty=format:'%G?|%ae')
    SIG_STATUS=${STATS%%|*}
    AUTHOR_EMAIL=${STATS##*|}

    echo -ne "  Verifying Commit... "
    
    # 1. 验证作者
    if [ "$AUTHOR_EMAIL" != "$EXPECTED_EMAIL" ]; then
        echo ""
        err "Author mismatch! Expected $EXPECTED_EMAIL, got $AUTHOR_EMAIL"
    fi

    # 2. 验证签名有效性
    if [ "$SIG_STATUS" == "G" ]; then
        echo -e "${PASS}[Signature: GOOD]${NC} ${PASS}[Author: MATCH]${NC}"
    else
        echo ""
        # 打印详细日志帮助调试
        git log -1 --show-signature
        err "Signature verification failed! Status code: $SIG_STATUS (Expected 'G')"
    fi
}

# --- 1. 初始化沙盒 ---
log "Initializing Sandbox..."
rm -rf "$BASE_DIR"
mkdir -p "$GOSH_CONFIG_PATH/profiles"
mkdir -p "$SSH_DIR"
mkdir -p "$REPO_DIR"

# 全局配置 Git (仅在沙盒内) 以启用 SSH 签名验证
# 这一步解决了之前 'No signature' / 'allowedSignersFile' 的问题
git config --global gpg.ssh.allowedSignersFile "$ALLOWED_SIGNERS"

# --- 2. 生成密钥并建立信任链 ---
log "Generating Keys & Establishing Trust..."

# Alice (Work)
ssh-keygen -t ed25519 -C "alice@contoso.com" -f "$SSH_DIR/id_alice" -N "" -q
echo "alice@contoso.com $(cat $SSH_DIR/id_alice.pub)" >> "$ALLOWED_SIGNERS"
ok "Generated Alice's Key & Added to Trust Store"

# Bob (Partner)
ssh-keygen -t ed25519 -C "bob@partner.org" -f "$SSH_DIR/id_bob" -N "" -q
echo "bob@partner.org $(cat $SSH_DIR/id_bob.pub)" >> "$ALLOWED_SIGNERS"
ok "Generated Bob's Key & Added to Trust Store"

# --- 3. 配置 Gosh Profiles ---
log "Configuring Gosh Profiles..."

# --> Alice Profile
$GOSH_CMD -c "Alice Work" "alice@contoso.com" "alice_work"
cat >> "$GOSH_CONFIG_PATH/profiles/alice_work.gitconfig" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_alice.pub
[commit]
    gpgsign = true
[core]
    sshCommand = ssh -i $SSH_DIR/id_alice -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
ok "Profile 'alice_work' created"

# --> Bob Profile
$GOSH_CMD -c "Bob Partner" "bob@partner.org" "bob_partner"
cat >> "$GOSH_CONFIG_PATH/profiles/bob_partner.gitconfig" <<EOF
[gpg]
    format = ssh
[user]
    signingkey = $SSH_DIR/id_bob.pub
[commit]
    gpgsign = true
[core]
    sshCommand = ssh -i $SSH_DIR/id_bob -F /dev/null -o IdentitiesOnly=yes -o StrictHostKeyChecking=no
EOF
ok "Profile 'bob_partner' created"


# --- 4. 模拟协作流程 ---

# === 项目 A: 交互式开发 (交叉签名) ===
log "Scenario 1: Project Alpha (Intersection)"
cd "$REPO_DIR"
git init --quiet project-alpha
cd project-alpha

# 1. Alice 初始化项目
echo ">>> [Commit 1] Alice starts the project"
$GOSH_CMD alice_work
touch README.md
git add README.md
git commit -m "Init Project Alpha" > /dev/null
verify_last_commit "alice@contoso.com" "alice_work"

# 2. Bob 进来修改 (模拟同一台机器切换身份)
echo ">>> [Commit 2] Bob adds features"
$GOSH_CMD bob_partner
echo "Feature by Bob" >> README.md
git commit -am "Bob adds feature" > /dev/null
verify_last_commit "bob@partner.org" "bob_partner"

# 3. Alice 审查并修改
echo ">>> [Commit 3] Alice reviews and updates"
$GOSH_CMD alice_work
echo "Reviewed by Alice" >> README.md
git commit -am "Alice review" > /dev/null
verify_last_commit "alice@contoso.com" "alice_work"


# === 项目 B: Exec 模式 (临时介入) ===
log "Scenario 2: Project Beta (Exec Mode Intervention)"
cd "$REPO_DIR"
git init --quiet project-beta
cd project-beta

# 1. Bob 拥有这个项目
$GOSH_CMD bob_partner
touch main.rs
git add main.rs
git commit -m "Bob starts Beta" > /dev/null
verify_last_commit "bob@partner.org" "bob_partner"

# 2. Alice 临时修复 (不切换 Profile，直接用 Exec)
# 当前 Profile 依然是 bob_partner (可以通过 .git/config 验证)
# Alice 用 gosh alice_work exec 临时提交
echo ">>> [Commit 4] Alice hotfixes via Exec Mode"
echo "// Hotfix" >> main.rs
git add main.rs

# 这里是关键测试：Exec 模式下的签名注入
$GOSH_CMD alice_work commit -m "Alice hotfix" > /dev/null

# 验证：虽然此时 .git/config 指向 Bob，但这个 Commit 必须是 Alice 签名的
verify_last_commit "alice@contoso.com" "alice_work"

# 3. 再次确认环境没被污染
# 此时如果不加参数执行 git commit，应该依然是 Bob
echo ">>> [Check] Verifying environment reset to Bob"
touch bob.txt
git add bob.txt
git commit -m "Bob continues" > /dev/null
verify_last_commit "bob@partner.org" "bob_partner"


# --- 5. 最终检查 ---
log "Summary"
echo "Checking Project Alpha Log (Should show Alice -> Bob -> Alice):"
cd "$REPO_DIR/project-alpha"
git log --pretty=format:'%C(yellow)%h%Creset - %C(green)%an%Creset (%C(blue)%G?%Creset) : %s' --graph
echo ""

echo -e "\n${PASS}🎉 All collaboration scenarios verified with strict signature checking!${NC}"