#!/usr/bin/env bash
# 按本机平台选择配置；传入参数可执行 test/check 等 Cargo 命令。
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_dir"
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=linux-x86_64 ;;
    Darwin-arm64) platform=macos-arm64 ;;
    *) echo "不支持的原生平台: $(uname -sm)" >&2; exit 1 ;;
esac

# 机器特有的编译器、依赖路径只存本地，不需要修改共享配置。
local_env="$repo_dir/docs/local/build-$platform.env"
if [[ -f "$local_env" ]]; then source "$local_env"; fi
export PATH="$HOME/.cargo/bin:$PATH"
if [[ -n "${DFTFLOW_NATIVE_PREFIX:-}" ]]; then
    export PATH="$DFTFLOW_NATIVE_PREFIX/bin:$PATH"
    export PKG_CONFIG_PATH="$DFTFLOW_NATIVE_PREFIX/lib/pkgconfig:$DFTFLOW_NATIVE_PREFIX/lib64/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
    export LIBRARY_PATH="$DFTFLOW_NATIVE_PREFIX/lib:$DFTFLOW_NATIVE_PREFIX/lib64${LIBRARY_PATH:+:$LIBRARY_PATH}"
    export BINDGEN_EXTRA_CLANG_ARGS="-I$DFTFLOW_NATIVE_PREFIX/include ${BINDGEN_EXTRA_CLANG_ARGS:-}"
fi
if [[ $# -eq 0 ]]; then set -- build --release --workspace --locked; fi
exec cargo --config ".cargo/$platform.toml" "$@"
