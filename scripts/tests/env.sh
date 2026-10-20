#!/usr/bin/env bash
# 使用真实家族入口验证 Bash/Zsh 环境加载及仓库移动、失败保护。
repo_dir=$(cd "$(dirname "$0")/../.." && pwd) || exit $?
test_dir=$(mktemp -d) || exit $?
trap 'rm -rf -- "$test_dir"' EXIT
mkdir -p "$test_dir/home" "$test_dir/other" "$test_dir/repo with spaces/scripts" "$test_dir/repo with spaces/bin" || exit $?
cp "$repo_dir/scripts/env.sh" "$test_dir/repo with spaces/scripts/env.sh" || exit $?
for family in epwflow qeflow substitutionflow vaspflow wannierflow scflow tools; do
    ln -s "$repo_dir/bin/$family" "$test_dir/repo with spaces/bin/$family" || exit $?
done
cat > "$test_dir/verify.sh" <<'SH' || exit $?
env_file="$1"
expected_bin="$2"
cd "$3" || exit $?
before="$PATH"
. "$env_file" || exit $?
after="$PATH"
. "$env_file" || exit $?
[ "$PATH" = "$after" ] || exit $?
[ "$PATH" = "$expected_bin/epwflow:$expected_bin/qeflow:$expected_bin/substitutionflow:$expected_bin/vaspflow:$expected_bin/wannierflow:$expected_bin/scflow:$expected_bin/tools:$before" ] || exit $?
count=0
for entry in "$expected_bin/"*/*; do
    [ -f "$entry" ] && [ -x "$entry" ] || continue
    name="${entry##*/}"
    [ "$(command -v "$name")" = "$entry" ] || exit $?
    "$name" --help >/dev/null || exit $?
    count=$((count + 1))
done
[ "$count" -eq 55 ] || exit $?
printf '%s\n' '55 个真实入口、仓库外调用、重复加载及原路径保留通过'
SH
cat > "$test_dir/missing.sh" <<'SH' || exit $?
before="$PATH"
if . "$1" 2>/dev/null; then
    echo '缺少 bin 时应加载失败' >&2
    exit 1
fi
[ "$PATH" = "$before" ] || exit $?
SH
mkdir -p "$test_dir/missing/scripts" || exit $?
cp "$repo_dir/scripts/env.sh" "$test_dir/missing/scripts/env.sh" || exit $?
for shell in bash zsh; do
    command -v "$shell" >/dev/null || { echo "缺少验收 shell: $shell" >&2; exit 1; }
    shell_bin=$(command -v "$shell") || exit $?
    if [[ "$shell" == bash ]]; then flags=(--noprofile --norc); else flags=(-f); fi
    for env_file in "$repo_dir/scripts/env.sh" "$test_dir/repo with spaces/scripts/env.sh"; do
        expected_bin=$(cd "$(dirname "$env_file")/../bin" && pwd -P) || exit $?
        env -i HOME="$test_dir/home" PATH="/usr/bin:/bin" \
            "$shell_bin" "${flags[@]}" "$test_dir/verify.sh" "$env_file" "$expected_bin" "$test_dir/other" || exit $?
    done
    env -i HOME="$test_dir/home" PATH="/usr/bin:/bin" \
        "$shell_bin" "${flags[@]}" "$test_dir/missing.sh" "$test_dir/missing/scripts/env.sh" || exit $?
    if "$shell_bin" "${flags[@]}" "$repo_dir/scripts/env.sh" >"$test_dir/direct.log" 2>&1; then
        echo "直接执行应给出 source 提示: $shell" >&2
        exit 1
    fi
    echo "$shell：缺失 bin 不修改 PATH、拒绝直接执行通过"

    # 只在临时 HOME 写启动文件，验证新交互终端会自动加载。
    if [[ "$shell" == bash ]]; then startup=.bashrc; else startup=.zshrc; fi
    printf 'source %q\n' "$repo_dir/scripts/env.sh" > "$test_dir/home/$startup" || exit $?
    env -i HOME="$test_dir/home" PATH="/usr/bin:/bin" \
        "$shell_bin" -ic '[ "$(command -v epw_phonon)" = "$1/bin/epwflow/epw_phonon" ] && epw_phonon --help >/dev/null' \
        dftflow-env-test "$repo_dir" >"$test_dir/startup.log" 2>&1 || exit $?
    echo "$shell：临时 HOME 新交互终端自动加载通过"
done
