#!/usr/bin/env bash
# 只发布本次 Cargo 报告的自有产品，预检完成前不替换公开程序。
script_dir=$(cd "$(dirname "$0")" && pwd) || exit $?
repo_dir=$(cd "$script_dir/.." && pwd) || exit $?
source "$script_dir/product-layout.sh" || exit $?
report="${1:?缺少 Cargo JSON 报告}"
platform="${2:?缺少平台}"
command -v jq >/dev/null || { echo '自动发布需要 jq 解析 Cargo 制品报告。' >&2; exit 1; }
jq empty "$report" || exit $?
mkdir -p "$repo_dir/bin/.build" || exit $?
stage=$(mktemp -d "$repo_dir/bin/.build/publish.XXXXXX") || exit $?
trap 'rm -rf -- "$stage"' EXIT
mkdir "$stage/source" || exit $?
products=()
artifacts=()
while IFS= read -r -d '' product && IFS= read -r -d '' artifact; do
    dftflow_module "$product" >/dev/null || continue
    [[ " ${products[*]} " != *" $product "* ]] || { echo "本次报告包含重复产品: $product" >&2; exit 1; }
    [[ -f "$artifact" && -x "$artifact" ]] || { echo "Cargo 制品缺失: $artifact" >&2; exit 1; }
    cp -p "$artifact" "$stage/source/$product" || exit $?
    products+=("$product")
    artifacts+=("$artifact")
done < <(jq -j 'select(.reason == "compiler-artifact" and .executable != null and (.profile.test != true) and (.target.kind | index("bin"))) | .target.name, "\u0000", .executable, "\u0000"' "$report")
[[ ${#products[@]} -gt 0 ]] || { echo '本次没有可发布的 DFTFlow 产品。'; exit 0; }
csv=$(IFS=,; echo "${products[*]}")
prepared="$stage/source"
if [[ "$platform" == linux-x86_64 && -n "${DFTFLOW_NATIVE_PREFIX:-}" ]]; then
    bash "$script_dir/linux/package-native-runtime.sh" --products "$csv" \
        "$stage/source" "$stage/prepared" "$DFTFLOW_NATIVE_PREFIX" || exit $?
    prepared="$stage/prepared"
fi
# 安装器会启动全部选中兼容入口，检查目标冲突与活动引用，再逐文件替换。
bash "$script_dir/install-bin.sh" --products "$csv" "$prepared" "$repo_dir/bin" || exit $?
mkdir -p "$repo_dir/bin/.build/published" || exit $?
for ((index=0; index<${#products[@]}; index++)); do
    product="${products[index]}"
    module=$(dftflow_module "$product") || exit 1
    source_sha=$(shasum -a 256 "$stage/source/$product") || exit $?
    source_sha="${source_sha%% *}"
    published_sha=$(shasum -a 256 "$repo_dir/bin/$module/$product") || exit $?
    published_sha="${published_sha%% *}"
    metadata="$repo_dir/bin/.build/published/$product.json"
    jq -n --arg product "$product" --arg source "${artifacts[index]}" \
        --arg source_sha256 "$source_sha" --arg published_sha256 "$published_sha" \
        --arg platform "$platform" '{product:$product, source:$source, source_sha256:$source_sha256, published_sha256:$published_sha256, platform:$platform}' \
        > "$metadata.tmp" || exit $?
    mv -f "$metadata.tmp" "$metadata" || exit $?
done
