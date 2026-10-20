#!/usr/bin/env bash
# 机器配置只保存在 docs/local；本脚本仅准备输入，不提交任务。
base=$(cd "$(dirname "$0")" && pwd) || exit $?
repo=$(cd "$base/../.." && pwd) || exit $?
machine=${1:?用法：bash examples/MgB2/prepare.sh sugon-amd|inspur|chem}
case "$machine" in sugon-amd|inspur|chem) ;; *) echo "未知机器：$machine" >&2; exit 1;; esac
mode=${2:-index}
case "$mode" in nosplit|index|coords) ;; *) echo "未知模式：$mode" >&2; exit 1;; esac
settings="$repo/docs/local/MgB2-$machine.env"
[[ -f "$settings" ]] || { echo "请先配置 $settings，参照 $base/$machine/README.md" >&2; exit 1; }
source "$settings" || exit $?
: "${QE_BIN:?}" "${PP_DIR:?}" "${ENV_SCRIPT:?}" "${PARTITION:?}" "${NTASKS:?}" "${DFTFLOW_BIN:?}"
[[ "$NTASKS" =~ ^[1-9][0-9]*$ ]] || { echo 'NTASKS 必须为正整数' >&2; exit 1; }
for bin in pw ph q2r matdyn; do [[ -x "$QE_BIN/$bin.x" ]] || exit $?; done
[[ -r "$ENV_SCRIPT" && -x "$DFTFLOW_BIN/qe_phonon_nosplit" ]] || exit $?
work="$base/$machine/run/$mode"
[[ ! -e "$work" ]] || { echo "已有 $work；请先另行归档，避免覆盖计算。" >&2; exit 1; }
mkdir -p "$work/pp" || exit $?
cp "$repo/test_auto/MgB2/MgB2.vasp" "$work/POSCAR" || exit $?
mkdir -p "$work/qe_phonon" || exit $?
cp "$base/KPATH.in" "$work/qe_phonon/KPATH.in" || exit $?
cp "$PP_DIR/b_pbe_v1.4.uspp.F.UPF" "$PP_DIR/mg_pbe_v1.4.uspp.F.UPF" "$work/pp/" || exit $?
cat > "$work/input.toml" <<EOF || exit $?
pp_dir = "$work/pp"
[qe]
executable_path = "mpirun -np $NTASKS $QE_BIN"
lattice_mode = "ibrav_angstrom"
[qe_phonon]
prefix = "MgB2"
work_dir = "$work/qe_phonon"
[qe_phonon.scf]
prefix = "MgB2"
outdir = "./tmp"
kpoints = [4, 4, 2]
ecutwfc = 40.0
ecutrho = 320.0
occupations = "smearing"
smearing = "gauss"
degauss = 0.02
conv_thr = 1.0e-6
mixing_beta = 0.7
electron_maxstep = 200
[qe_phonon.ph]
ldisp = true
qpoints = [2, 2, 1]
tr2_ph = 1.0e-12
fildvscf = "dvscf"
recover = false
epsil = false
trans = true
search_sym = true
lqdir = false
[qe_phonon.q2r]
zasr = "simple"
[qe_phonon.phband]
kpath_file = "KPATH.in"
kpath_points = 20
q_in_band_form = true
[qe_phonon.phdos]
qpoints = [2, 2, 2]
ndos = 100
dos = true
EOF
# Slurm 环境随输入保存，子作业无需继承登录 shell 的临时设置。
cat >> "$work/input.toml" <<EOF || exit $?
[slurm]
header = """#!/bin/bash
#SBATCH --partition=$PARTITION
#SBATCH --nodes=1
#SBATCH --ntasks-per-node=$NTASKS
#SBATCH --cpus-per-task=1
#SBATCH --output=slurm-%j.out
#SBATCH --error=slurm-%j.err
${EXCLUDE:+#SBATCH --exclude=$EXCLUDE}
source $ENV_SCRIPT --force || exit \$?
export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
export PATH=$DFTFLOW_BIN:\$PATH
ulimit -s unlimited
"""
EOF
if [[ "$mode" != nosplit ]]; then
  (cd "$work" && PATH="$DFTFLOW_BIN:$PATH" "qe_phonon_split_$mode" -c "$work/input.toml" -i "$work/POSCAR") || exit $?
  {
    printf '#!/bin/bash\ncd %q || exit $?\n' "$work" || exit $?
    printf '%q -c %q -i %q -s\n' "$DFTFLOW_BIN/qe_phonon_split_$mode" "$work/input.toml" "$work/POSCAR" || exit $?
  } > "$work/submit_phonon.sh" || exit $?
  echo "准备完成；一次启动完整依赖链：cd '$work' && bash submit_phonon.sh"
  exit 0
fi
(cd "$work" && PATH="$DFTFLOW_BIN:$PATH" qe_phonon_nosplit -c "$work/input.toml") || exit $?
{
  printf '#!/bin/bash\n#SBATCH --job-name=MgB2_ph\n#SBATCH --partition=%s\n#SBATCH --nodes=1\n#SBATCH --ntasks-per-node=%s\n#SBATCH --cpus-per-task=1\n#SBATCH --output=MgB2_ph-%%j.out\n#SBATCH --error=MgB2_ph-%%j.err\n' "$PARTITION" "$NTASKS"
  [[ -z "${EXCLUDE:-}" ]] || printf '#SBATCH --exclude=%s\n' "$EXCLUDE"
  printf 'source %q --force || exit $?\nexport OMP_NUM_THREADS=1 MKL_NUM_THREADS=1\nulimit -s unlimited\n' "$ENV_SCRIPT" || exit $?
  printf 'export PATH=%q:"$PATH"\ncd %q || exit $?\nqe_bin=%q\nconfig=%q\n' "$DFTFLOW_BIN" "$work/qe_phonon" "$QE_BIN" "$work/input.toml" || exit $?
  printf 'mpirun -np %s "$qe_bin/pw.x" < scf.in > scf.out || exit $?\nmpirun -np %s "$qe_bin/ph.x" < ph.in > ph.out || exit $?\n' "$NTASKS" "$NTASKS" || exit $?
  cat <<'EOF' || exit $?
"$qe_bin/q2r.x" < q2r.in > q2r.out || exit $?
"$qe_bin/matdyn.x" < matdyn_phband.in > matdyn_phband.out || exit $?
"$qe_bin/matdyn.x" < matdyn_phdos.in > matdyn_phdos.out || exit $?
qe_process_phonon -c "$config" --collect || exit $?
qe_process_phonon -c "$config" --plot || exit $?
EOF
} > "$work/nosplit_job.sh" || exit $?
printf '#!/bin/bash\ncd %q || exit $?\nsbatch --parsable nosplit_job.sh\n' "$work" > "$work/submit_phonon.sh" || exit $?
echo "准备完成。检查输入后执行：cd '$work' && bash submit_phonon.sh"
