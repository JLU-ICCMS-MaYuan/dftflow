#!/usr/bin/python3
"""只模拟 sbatch 的指令、参数与日志定位；不运行脚本或科学程序。"""
import json
import pathlib
import shlex
import sys

root = pathlib.Path(__file__).resolve().parent
script = pathlib.Path(sys.argv[-1]).resolve()
options = {}


def parse(tokens):
    values = iter(tokens)
    for token in values:
        if not token.startswith('--') or token == '--parsable':
            continue
        key, separator, value = token[2:].partition('=')
        options[key] = value if separator else next(values)


for line in script.read_text().splitlines():
    if line.startswith('#SBATCH '):
        parse(shlex.split(line[len('#SBATCH '):]))
    elif line.strip() and not line.startswith('#'):
        break
directives = options.copy()
parse(sys.argv[1:-1])
record_path = root / 'scheduler.jsonl'
previous = record_path.read_text().splitlines() if record_path.exists() else []
job = 201 + len(previous)
directory = pathlib.Path(options.get('chdir', str(pathlib.Path.cwd())))
if not directory.is_absolute():
    directory = pathlib.Path.cwd() / directory
paths = {}
for key, default in [('output', 'slurm-%j.out'), ('error', 'slurm-%j.out')]:
    path = directory / options.get(key, default).replace('%j', str(job))
    path.write_text('scheduler fixture ' + key + '\n')
    paths[key] = str(path)
record = dict(script=str(script), cwd=str(pathlib.Path.cwd()), argv=sys.argv[1:],
              directives=directives, options=options, job=job, **paths)
with record_path.open('a') as stream:
    stream.write(json.dumps(record) + '\n')
print(job)
