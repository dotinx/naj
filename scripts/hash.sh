#!/bin/bash

# 定义目标目录
TARGET_DIR="$PWD/dist"

# 检查目录是否存在
if [ ! -d "$TARGET_DIR" ]; then
  echo "Error: Directory $TARGET_DIR does not exist."
  exit 1
fi

# 输出表格表头
echo "| Filename | SHA256 | Status |"
echo "| :--- | :--- | :--- |"

# 遍历所有的 .tar.gz 文件
for file in "$TARGET_DIR"/*.tar.gz; do
    # 检查是否有匹配的文件，防止 glob 失败
    [ -e "$file" ] || continue

    filename=$(basename "$file")
    sha_file="${file}.sha256"

    # 计算当前的 SHA256
    # 使用 awk 只取第一列（哈希值）
    actual_sha=$(sha256sum "$file" | awk '{print $1}')

    # 校验逻辑
    if [ -f "$sha_file" ]; then
        # 从文件中读取预期的 SHA256
        expected_sha=$(cat "$sha_file" | awk '{print $1}')
        
        if [ "$actual_sha" == "$expected_sha" ]; then
            status="✅ OK"
        else
            status="❌ Mismatch"
        fi
    else
        status="⚠️ No .sha256 file"
    fi

    # 输出表格行
    echo "| $filename | \`$actual_sha\` | $status |"
done