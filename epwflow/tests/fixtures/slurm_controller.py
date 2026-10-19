#!/usr/bin/python3
"""控制器协议替身；仅调用同一测试目录的账表替身，不接触真实 Slurm。"""
import pathlib, subprocess, sys
assert sys.argv[1:4] == ['show', 'job', '-o']
job = sys.argv[4]
rows = subprocess.check_output([str(pathlib.Path(__file__).with_name('sacct'))], universal_newlines=True).splitlines()
rows = [r.split('|') for r in rows if r.split('|')[0] == job]
assert len(rows) == 1 and len(rows[0]) >= 4
r = rows[0]
print(f'JobId={job} JobState={r[1]} NumNodes={r[3]}')
