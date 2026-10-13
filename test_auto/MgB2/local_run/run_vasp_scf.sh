#!/bin/bash
set -e
echo '[qeflow] ------------------------------------------------------------'
echo '[qeflow] workflow: '
echo '[qeflow] prefix: vasp_scf'
echo '[qeflow] step: 1/1 运行 vasp_scf'
echo '[qeflow] work_dir: .'
echo '[qeflow] command: cd vasp_scf && mpirun -np 4 /Users/mayuan/Codes/dftflow/external/vasp.6.4.3/bin/vasp_std > vasp.log && cd ..'
if ! cd "."; then
echo '[qeflow] failed: 1/1 运行 vasp_scf'
  exit 1
fi
if ! ( cd vasp_scf && mpirun -np 4 /Users/mayuan/Codes/dftflow/external/vasp.6.4.3/bin/vasp_std > vasp.log && cd .. ); then
echo '[qeflow] failed: 1/1 运行 vasp_scf'
  exit 1
fi
echo '[qeflow] done: 1/1 运行 vasp_scf'
echo '[qeflow] 完成: vasp_scf 完成'
