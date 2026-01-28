#!/bin/bash

# --- 准备 ---
NAJ_CMD="naj" # 确保已编译或 alias
BASE_DIR="/tmp/naj_security_test"
UNSAFE_REPO="$BASE_DIR/root_owned_repo"

# 1. 初始化一个归属于 root 的仓库 (对当前用户来说是不安全的)
rm -rf "$BASE_DIR"
mkdir -p "$UNSAFE_REPO"

echo "[SETUP] Creating a repo owned by ROOT..."
# 使用 sudo 创建 .git，这样它就属于 root 了
sudo git init --quiet "$UNSAFE_REPO"
sudo touch "$UNSAFE_REPO/testfile"

# 确保当前用户对目录有读写权限(以便能进入)，但 .git 依然属于 root
sudo chmod -R 777 "$UNSAFE_REPO"

echo "[TEST] Running 'naj' in a dubious ownership repo..."
cd "$UNSAFE_REPO"

# 2. 尝试运行 naj (期望失败)
if $NAJ_CMD -l > /dev/null 2>&1; then
    # 注意：naj -l 不需要 git 仓库，所以应该成功。
    # 我们需要测 switch 或 exec，这需要 git 上下文
    echo "  (naj list works, which is fine)"
fi

echo "Attempting to switch profile..."
# 捕获输出
OUTPUT=$($NAJ_CMD testprofile 2>&1 || true)

# 3. 验证结果
if echo "$OUTPUT" | grep -q "fatal: detected dubious ownership"; then
    echo "✅ PASS: Naj propagated Git's security error."
    echo "   Git said: 'detected dubious ownership'"
    echo "   Naj refused to act."
elif echo "$OUTPUT" | grep -q "Not a git repository"; then
    echo "✅ PASS: Naj treated it as invalid (Git rev-parse failed)."
else
    echo "❌ FAIL: Naj tried to execute! This is dangerous."
    echo "Output was: $OUTPUT"
    exit 1
fi

# 清理 (需要 sudo 因为文件夹是 root 的)
cd /tmp
sudo rm -rf "$BASE_DIR"
echo "🎉 Security verification complete."