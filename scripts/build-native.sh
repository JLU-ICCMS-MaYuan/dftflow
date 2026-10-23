#!/usr/bin/env bash
# 按平台构建；成功的 build 自动发布本次产品，test/check 不更新公开程序。
repo_dir="$(cd "$(dirname "$0")/.." && pwd)" || exit $?
cd "$repo_dir" || exit $?
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=linux-x86_64 ;;
    Darwin-arm64) platform=macos-arm64 ;;
    *) echo "不支持的原生平台: $(uname -sm)" >&2; exit 1 ;;
esac

# 机器特有的编译器、依赖路径只存本地，不需要修改共享配置。
local_env="$repo_dir/docs/local/build-$platform.env"
if [[ -f "$local_env" ]]; then source "$local_env" || exit $?; fi
export PATH="$HOME/.cargo/bin:$PATH"
if [[ -z "${DFTFLOW_NATIVE_PREFIX:-}" && -d "$repo_dir/bin/.native/$platform" ]]; then
    export DFTFLOW_NATIVE_PREFIX="$repo_dir/bin/.native/$platform"
fi
case "${DFTFLOW_NATIVE_PREFIX:-}/" in
    "$HOME/.local/share/dftflow/"*) echo '旧用户依赖目录已退役，请将 DFTFLOW_NATIVE_PREFIX 改为仓库 bin/.native/<平台>。' >&2; exit 2 ;;
esac
if [[ -n "${DFTFLOW_NATIVE_PREFIX:-}" ]]; then
    export PATH="$DFTFLOW_NATIVE_PREFIX/bin:$PATH"
    export PKG_CONFIG_PATH="$DFTFLOW_NATIVE_PREFIX/lib/pkgconfig:$DFTFLOW_NATIVE_PREFIX/lib64/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
    export LIBRARY_PATH="$DFTFLOW_NATIVE_PREFIX/lib:$DFTFLOW_NATIVE_PREFIX/lib64${LIBRARY_PATH:+:$LIBRARY_PATH}"
    export BINDGEN_EXTRA_CLANG_ARGS="-I$DFTFLOW_NATIVE_PREFIX/include ${BINDGEN_EXTRA_CLANG_ARGS:-}"
fi
if [[ $# -eq 0 ]]; then set -- build --release --workspace --locked; fi
export CARGO_TARGET_DIR="$repo_dir/bin/.build/$platform"
for arg in "$@"; do
    case "$arg" in
        --target-dir|--target-dir=*) echo '构建缓存统一位于 bin/.build；请勿覆盖 --target-dir。' >&2; exit 2 ;;
        --help|-h|--version|-V) exec cargo --config ".cargo/$platform.toml" "$@" ;;
    esac
done
if [[ "$1" != build ]]; then
    exec cargo --config ".cargo/$platform.toml" "$@"
fi
command -v jq >/dev/null || { echo '自动发布需要 jq。' >&2; exit 1; }
mkdir -p "$CARGO_TARGET_DIR" || exit $?
report=$(mktemp "$CARGO_TARGET_DIR/artifacts.XXXXXX") || exit $?
trap 'rm -f -- "$report"' EXIT
for arg in "$@"; do
    case "$arg" in
        --message-format|--message-format=*) echo '自动发布需要 Cargo JSON 报告，请勿覆盖 --message-format。' >&2; exit 2 ;;
    esac
done
cargo --config ".cargo/$platform.toml" "$@" --message-format=json-render-diagnostics > "$report"
cargo_status=$?
jq -r 'select(.reason == "compiler-message") | .message.rendered // empty' "$report" >&2 || exit $?
[[ "$cargo_status" == 0 ]] || exit "$cargo_status"
# 显式交叉目标只构建，不用异平台制品替换宿主程序。
expected_target=x86_64-unknown-linux-gnu
[[ "$platform" != macos-arm64 ]] || expected_target=aarch64-apple-darwin
previous=""
for arg in "$@"; do
    if [[ "$previous" == --target && "$arg" != "$expected_target" ]] || \
        [[ "$arg" == --target=* && "${arg#--target=}" != "$expected_target" ]]; then
        echo '交叉目标构建完成；宿主模块程序保持。'
        exit 0
    fi
    previous="$arg"
done
bash "$repo_dir/scripts/publish-artifacts.sh" "$report" "$platform"
