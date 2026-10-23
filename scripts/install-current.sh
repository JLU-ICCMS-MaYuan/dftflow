#!/usr/bin/env bash
# 0017 已统一为仓库 bin；明确退役旧入口，避免重新创建用户目录。
printf '%s\n' 'install-current.sh 已退役。请使用 scripts/build-native.sh，成功构建会自动更新仓库 bin/<模块>/。' >&2
exit 2
