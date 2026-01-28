#!/bin/bash
set -e  # 遇到错误立即停止

# --- 配置部分 ---
APP_NAME="naj"
OUTPUT_DIR="dist"
# 新增：获取 dist 目录的绝对物理路径
mkdir -p "$OUTPUT_DIR"
ABS_OUTPUT_DIR="$(realpath "$OUTPUT_DIR")"

# 你想要支持的 Target 列表
# cross 支持的列表可见: https://github.com/cross-rs/cross#supported-targets
TARGETS=(
    "x86_64-unknown-linux-gnu"      # 标准 Linux x64
    "x86_64-unknown-linux-musl"     # 静态链接 Linux x64 (推荐: 无依赖，兼容性最好)
    # "i686-unknown-linux-gnu" # Linux x86
    # "i586-unknown-linux-musl" # Linux x86 (静态链接)
    "aarch64-unknown-linux-gnu"     # Linux ARM64 (如树莓派 4, docker ARM 容器)
    # "x86_64-unknown-freebsd"  # FreeBSD x64
    # "x86_64-pc-windows-gnu"         # Windows x64 (使用 MinGW)
    # "aarch64-apple-darwin"        # macOS ARM64 (注意: cross 对 macOS 支持有限，通常建议在 Mac 上原生编译)
)

# --- 检查依赖 ---
if ! command -v cross &> /dev/null; then
    echo "❌ Error: 'cross' is not installed."
    echo "👉 Please install it: cargo install cross"
    exit 1
fi

if ! command -v podman &> /dev/null; then
    echo "❌ Error: 'podman' is not installed or not in PATH."
    exit 1
fi

# --- 获取版本号 ---
# 简单地从 Cargo.toml 提取版本号
VERSION=$(grep "^version" Cargo.toml | head -n 1 | cut -d '"' -f 2)
echo "🚀 Preparing to build $APP_NAME v$VERSION..."

# 清理并创建输出目录
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# --- 循环编译 ---
for target in "${TARGETS[@]}"; do
    echo "------------------------------------------------"
    echo "🔨 Building target: $target"
    echo "------------------------------------------------"

    # 1. 使用 cross 编译 release 版本
    cross build --target "$target" --release

    # 2. 准备打包
    BINARY_NAME="$APP_NAME"
    if [[ $target == *"windows"* ]]; then
        BINARY_NAME="${APP_NAME}.exe"
    fi

    # 查找编译生成的二进制文件位置
    BUILD_BIN_PATH="target/$target/release/$BINARY_NAME"

    if [ ! -f "$BUILD_BIN_PATH" ]; then
        echo "❌ Error: Binary not found at $BUILD_BIN_PATH"
        exit 1
    fi

    # 3. 打包文件名格式: naj-v0.1.0-x86_64-unknown-linux-musl.tar.gz
    ARCHIVE_NAME="${APP_NAME}-v${VERSION}-${target}"

    # 进入输出目录进行打包操作
    # 创建一个临时目录来存放二进制文件和文档（如果有 README/LICENSE）
    TMP_DIR=$(mktemp -d)
    cp "$BUILD_BIN_PATH" "$TMP_DIR/"
    # 如果有 README 或 LICENSE，也可以 cp 到 TMP_DIR
    # cp README.md LICENSE "$TMP_DIR/"

    echo "📦 Packaging $ARCHIVE_NAME..."

    # if [[ $target == *"windows"* ]]; then
    #     # Windows 使用 zip
    #     ARCHIVE_FILE="${ARCHIVE_NAME}.zip"
    #     (cd "$TMP_DIR" && zip -r "../../$OUTPUT_DIR/$ARCHIVE_FILE" .)
    # else
        # Linux/Unix 使用 tar.gz
        ARCHIVE_FILE="${ARCHIVE_NAME}.tar.gz"
        (cd "$TMP_DIR" && tar -czf "$ABS_OUTPUT_DIR/$ARCHIVE_FILE" .)
    # fi

    # 清理临时目录
    rm -rf "$TMP_DIR"

    # 4. 生成校验和 (SHA256)
    (cd "$ABS_OUTPUT_DIR" && shasum -a 256 "$ARCHIVE_FILE" > "${ARCHIVE_FILE}.sha256")
    
    echo "✅ Success: $OUTPUT_DIR/$ARCHIVE_FILE"
done

echo "------------------------------------------------"
echo "🎉 Build finished! All artifacts are in '$OUTPUT_DIR/'"
ls -lh "$OUTPUT_DIR"