# 在 Bash/Zsh 中 source 本文件，将仓库产品家族加入 PATH。
# 用法：source "/path/to/dftflow/scripts/env.sh"
if [ -n "${BASH_VERSION-}" ]; then
    if [ "${BASH_SOURCE[0]}" = "$0" ]; then
        printf '%s\n' '请使用 source 加载 env.sh，直接执行不能修改父 shell 的 PATH。' >&2
        exit 2
    fi
    _dftflow_env_file="${BASH_SOURCE[0]}"
elif [ -n "${ZSH_VERSION-}" ]; then
    case "$ZSH_EVAL_CONTEXT" in
        *:file) ;;
        *) printf '%s\n' '请使用 source 加载 env.sh，直接执行不能修改父 shell 的 PATH。' >&2; exit 2 ;;
    esac
    _dftflow_env_file="${(%):-%x}"
else
    printf '%s\n' 'env.sh 支持在 Bash 或 Zsh 中通过 source 加载。' >&2
    return 2 2>/dev/null || exit 2
fi

_dftflow_bin=$(CDPATH= cd -- "$(dirname -- "$_dftflow_env_file")/../bin" && pwd -P) || {
    printf '%s\n' '找不到仓库 bin 目录；请先完成 DFTFlow 分组安装。' >&2
    unset _dftflow_env_file _dftflow_bin
    return 1
}

# 倒序逐项前插，避免重复加载不断增长；只暴露公开产品目录。
for _dftflow_family in tools scflow wannierflow vaspflow substitutionflow qeflow epwflow; do
    _dftflow_dir="$_dftflow_bin/$_dftflow_family"
    case ":${PATH-}:" in
        *":$_dftflow_dir:"*) ;;
        *) PATH="$_dftflow_dir${PATH:+:$PATH}" ;;
    esac
done
export PATH
unset _dftflow_env_file _dftflow_bin _dftflow_family _dftflow_dir
