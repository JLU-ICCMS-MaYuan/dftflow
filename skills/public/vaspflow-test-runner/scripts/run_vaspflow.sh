#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"

INPUT=""
CONFIG="/home/mayuan/code/vaspflow/test/job_templates.toml"
PYTHON_BIN="python"
PRESSURES=()

usage() {
  cat <<'EOF'
用法: run_vaspflow.sh -i <输入文件或目录> [选项]
选项:
  -i, --input PATH       必填，结构文件或目录
  -c, --config PATH      TOML 配置路径，默认 /home/mayuan/code/vaspflow/test/job_templates.toml
      --pressures "0 5"  覆盖压强列表，空则使用配置中的值
      --python PYTHON    指定 Python 解释器，默认 python
  -h, --help             显示本帮助
说明:
  - prepare/submit 由配置 [settings].submit 决定；
  - 并行度由配置 [tasks].max_workers 或 [settings].max_workers 控制。
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -i|--input)
      INPUT="$2"
      shift 2
      ;;
    -c|--config)
      CONFIG="$2"
      shift 2
      ;;
    --pressures)
      IFS=' ' read -r -a PRESSURES <<<"$2"
      shift 2
      ;;
    --python)
      PYTHON_BIN="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "未知参数: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$INPUT" ]]; then
  echo "错误: 必须指定 --input" >&2
  usage
  exit 1
fi

if [[ ! -e "$INPUT" ]]; then
  echo "错误: 输入路径不存在: $INPUT" >&2
  exit 1
fi

if [[ ! -f "$CONFIG" ]]; then
  echo "错误: 配置文件不存在: $CONFIG" >&2
  exit 1
fi

cd "$REPO_ROOT"

CLI_ARGS=("-i" "$INPUT" "--config" "$CONFIG")
if [[ ${#PRESSURES[@]} -gt 0 ]]; then
  CLI_ARGS+=("-p" "${PRESSURES[@]}")
fi

set -x
"$PYTHON_BIN" cli.py "${CLI_ARGS[@]}"
set +x
