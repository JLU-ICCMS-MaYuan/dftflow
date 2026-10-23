#!/usr/bin/env bash
# 用实际 Cargo 小工程验证构建、制品准备和模块发布边界；不运行科学程序。
repo_dir=$(cd "$(dirname "$0")/../.." && pwd) || exit $?
test_dir=$(mktemp -d) || exit $?
running_pid=""
cleanup() {
    [[ -z "$running_pid" ]] || kill "$running_pid" 2>/dev/null
    rm -rf -- "$test_dir"
}
trap cleanup EXIT
fixture="$test_dir/repo with spaces"
mkdir -p "$fixture/scripts/linux" "$fixture/.cargo" "$fixture/qeflow/src" "$fixture/epwflow/src" || exit $?
for name in build-native.sh product-layout.sh publish-artifacts.sh install-bin.sh install-compat.sh check-bin-in-use.sh; do
    cp "$repo_dir/scripts/$name" "$fixture/scripts/" || exit $?
done
cp "$repo_dir/scripts/linux/package-native-runtime.sh" "$fixture/scripts/linux/" || exit $?
cp "$repo_dir/.cargo/"*.toml "$fixture/.cargo/" || exit $?
printf '[workspace]\nmembers=["qeflow","epwflow"]\nresolver="2"\n' > "$fixture/Cargo.toml" || exit $?
printf '[package]\nname="qeflow"\nversion="0.1.0"\nedition="2021"\n' > "$fixture/qeflow/Cargo.toml" || exit $?
printf 'pub fn value() -> u32 { 1 }\n' > "$fixture/qeflow/src/lib.rs" || exit $?
printf 'fn main() { if std::env::var_os("FAIL_PRODUCT").is_some() { std::process::exit(17); } if std::env::args().any(|v| v == "--hold") { std::thread::sleep(std::time::Duration::from_secs(60)); } println!("first"); }\n' > "$fixture/qeflow/src/main.rs" || exit $?
printf '[package]\nname="epwflow"\nversion="0.1.0"\nedition="2021"\n' > "$fixture/epwflow/Cargo.toml" || exit $?
mkdir "$fixture/epwflow/src/bin" || exit $?
for name in epw_phonon epw_wannier epw_superconductivity; do
    printf 'fn main() { println!("%s"); }\n' "$name" > "$fixture/epwflow/src/bin/$name.rs" || exit $?
done
run_build() { bash "$fixture/scripts/build-native.sh" "$@" > "$test_dir/build.log" 2>&1; }
run_build build --release --workspace || { cat "$test_dir/build.log"; exit 1; }
original=$(shasum -a 256 "$fixture/bin/qeflow/qeflow") || exit $?
[[ $("$fixture/bin/qeflow/qe_scf" --help) == first ]] || exit 1
[[ -d "$fixture/bin/.build" && ! -e "$fixture/target" ]] || exit 1
# Fresh 增量构建仍报告并发布同一产品。
run_build build --release --workspace || { cat "$test_dir/build.log"; exit 1; }
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
run_build check --workspace || exit $?
run_build test --workspace || exit $?
run_build build -p qeflow --lib || exit $?
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
if FAIL_PRODUCT=1 run_build build --release -p qeflow; then echo '启动失败应拒绝发布' >&2; exit 1; fi
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
cp "$fixture/qeflow/src/main.rs" "$test_dir/main.rs" || exit $?
printf 'invalid rust source\n' > "$fixture/qeflow/src/main.rs" || exit $?
if run_build build --release -p qeflow; then echo '编译失败不应成功' >&2; exit 1; fi
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
cp "$test_dir/main.rs" "$fixture/qeflow/src/main.rs" || exit $?
# 从选中模块重新生成缺失入口，其他模块不可触碰。
rm "$fixture/bin/epwflow/epw_wannier" || exit $?
run_build build --release -p epwflow || { cat "$test_dir/build.log"; exit 1; }
[[ -x "$fixture/bin/epwflow/epw_wannier" ]] || exit 1
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
# 目标冲突必须在任何产品替换前拒绝。
rm "$fixture/bin/qeflow/qe_scf" || exit $?
mkdir "$fixture/bin/qeflow/qe_scf" || exit $?
if run_build build --release -p qeflow; then echo '目标目录冲突应拒绝发布' >&2; exit 1; fi
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
rmdir "$fixture/bin/qeflow/qe_scf" || exit $?
# 独立工具复用同一个 wrapper。
mkdir -p "$fixture/tools/qf-gen-dyn0/src" || exit $?
printf '[package]\nname="qf-gen-dyn0"\nversion="0.1.0"\nedition="2021"\n[workspace]\n' > "$fixture/tools/qf-gen-dyn0/Cargo.toml" || exit $?
printf 'fn main() { println!("dyn0"); }\n' > "$fixture/tools/qf-gen-dyn0/src/main.rs" || exit $?
run_build build --release --manifest-path "$fixture/tools/qf-gen-dyn0/Cargo.toml" || { cat "$test_dir/build.log"; exit 1; }
[[ $("$fixture/bin/tools/qf-gen-dyn0" --help) == dyn0 ]] || exit 1
# 活动作业引用由可控调度入口复现，不提交真实作业。
mkdir "$test_dir/commands" || exit $?
printf '#!/bin/sh\nprintf "123|%%s\\n" "$CHECK_JOB_SCRIPT"\n' > "$test_dir/commands/squeue" || exit $?
chmod +x "$test_dir/commands/squeue" || exit $?
printf '"%s" --help\n' "$fixture/bin/qeflow/qeflow" > "$test_dir/job.sh" || exit $?
if PATH="$test_dir/commands:$PATH" CHECK_JOB_SCRIPT="$test_dir/job.sh" run_build build --release -p qeflow; then
    echo '活动作业应阻止覆盖绑定产品' >&2; exit 1
fi
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
run_build build --release -p qeflow || exit $?
"$fixture/bin/qeflow/qeflow" --hold >/dev/null &
running_pid=$!
if run_build build --release -p qeflow; then
    echo '本机运行进程应阻止覆盖产品' >&2; exit 1
fi
[[ $(shasum -a 256 "$fixture/bin/qeflow/qeflow") == "$original" ]] || exit 1
kill "$running_pid" || exit $?
wait "$running_pid" 2>/dev/null
running_pid=""
echo '自动发布通过：实际 Cargo、fresh、模块隔离、非交付、编译／启动失败、目标冲突、本机进程、活动作业、独立工具和带空格路径。'
