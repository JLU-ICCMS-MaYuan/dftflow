#!/usr/bin/env python3
import argparse
import os

from substitutionflow.selection import render_selection_preview
from substitutionflow.status import create_initial_status
from substitutionflow.structure_substitution import (
    generate_cases,
    load_substitutionflow_config,
    resolve_input_structure_path,
)


def main():
    parser = argparse.ArgumentParser(description="准备 XA/XB 占位结构的高通量工作流输入目录。")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径。")
    parser.add_argument("-o", "--output-dir", required=True, help="输出根目录。")
    args = parser.parse_args()

    config = load_substitutionflow_config(args.config)
    ht = config["substitutionflow"]
    if ht.get("preview", True):
        print(render_selection_preview("XA", ht["XA"]))
        print()
        print(render_selection_preview("XB", ht["XB"]))
        print()

    input_path = resolve_input_structure_path(args.config)
    result = generate_cases(input_path, args.config, args.output_dir)
    create_initial_status(args.output_dir)
    print(f"已生成 {len(result['cases'])} 个替换结构目录于 {os.path.abspath(args.output_dir)}")


if __name__ == "__main__":
    main()
