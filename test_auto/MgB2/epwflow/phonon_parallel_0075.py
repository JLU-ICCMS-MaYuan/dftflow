#!/usr/bin/env python3
"""#75 原生 MgB₂／真实 Slurm 验收，只通过公开入口提交与续算。"""
import argparse
import copy
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
import tomllib

import toml


REPO = Path(__file__).resolve().parents[3]
TERMINAL = {"COMPLETED", "FAILED", "CANCELLED", "TIMEOUT", "OUT_OF_MEMORY", "NODE_FAIL"}


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(case, argv, label, check=True):
    result = subprocess.run(list(map(str, argv)), cwd=case, capture_output=True, text=True,
                            env={**os.environ, "TMPDIR": str(case / "tmp")}, timeout=120)
    save(case / "evidence" / f"{time.time_ns()}-{label}.json", {
        "argv": list(map(str, argv)), "exit": result.returncode,
        "stdout": result.stdout, "stderr": result.stderr,
    })
    if check and result.returncode:
        raise RuntimeError(f"{label}: {result.stdout}\n{result.stderr}")
    return result


def controller(case, job):
    result = subprocess.run(["scontrol", "show", "job", "-o", str(job)],
                            capture_output=True, text=True, timeout=20)
    if result.returncode:
        return {}
    # 所需字段不含空格；完整原文另随关键检查点保存。
    fields = dict(item.split("=", 1) for item in result.stdout.split() if "=" in item)
    fields["raw"] = result.stdout
    assert fields["JobId"] == str(job)
    return fields


def completions(case):
    # filetxt 的 sacct 需要可写副本，绝不改原日志权限。
    config = subprocess.run(["scontrol", "show", "config"], check=True,
                            capture_output=True, text=True, timeout=20).stdout
    fields = dict((k.strip(), v.strip()) for line in config.splitlines()
                  if "=" in line for k, v in [line.split("=", 1)])
    assert fields["JobCompType"] == "jobcomp/filetxt"
    source = Path(fields["JobCompLoc"])
    target = case / "evidence" / "completion.log"
    target.write_bytes(source.read_bytes())
    result = subprocess.run(["sacct", "--completion", "--file", str(target),
                             "--noheader", "--parsable2",
                             "--format=JobIDRaw,State,ExitCode,Start,End,NCPUS,NNodes"],
                            check=True, capture_output=True, text=True, timeout=20)
    return {row[0]: row for line in result.stdout.splitlines()
            if len(row := line.split("|")) >= 7}


def wait_for(label, predicate, timeout=1200):
    print(f"等待：{label}", flush=True)
    deadline, progress = time.monotonic() + timeout, 0
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        if time.monotonic() >= progress:
            print(f"仍在等待：{label}", flush=True)
            progress = time.monotonic() + 30
        time.sleep(0.2)
    raise TimeoutError(label)


def state(case, mode):
    path = case / "run" / f"4.qe-phonon-{mode}" / "phonon-qpoints.json"
    return json.loads(path.read_text()) if path.exists() else None


def public(case, config, stage, label, *options, check=True):
    return run(case, [case.parent / "runtime" / "epw_phonon", "-c", config,
                      "--stage", stage, *options], label, check)


def protected_q_outputs(case, mode, points):
    result = {}
    for point in points:
        root = case / "run" / f"4.qe-phonon-{mode}" / "q" / point["directory"]
        # 明确选取日志、输入和动力学矩阵；不访问电荷密度或原始波函数。
        files = [root / "ph.in", root / "ph.out", *root.glob("*.dyn*.xml")]
        for path in files:
            result[str(path.relative_to(case))] = {
                "sha256": digest(path), "mtime_ns": path.stat().st_mtime_ns,
            }
    return result


def prepare(root, source, modes):
    root.mkdir(parents=True, exist_ok=False)
    (root / "runtime").mkdir()
    binary = REPO / "bin/epwflow/epw_phonon"
    shutil.copy2(binary, root / "runtime/epw_phonon")
    original = tomllib.loads(source.read_text())
    config = copy.deepcopy(original)
    for key in ["structure", "kpath"]:
        config[key] = str((source.parent / config[key]).resolve())
    config["pseudos"] = {k: str((source.parent / v).resolve())
                          for k, v in config["pseudos"].items()}
    config["work_dir"] = "run"
    config["stages"]["1.qe-scf"]["K_POINTS"] = [4, 4, 2, 0, 0, 0]
    config["stages"]["2.qe-nscf"]["K_POINTS"] = [4, 4, 2]
    for stage in ["5.epw-wannier", "6.epw-coarse", "7.epw-fine"]:
        config["stages"][stage]["nk"] = [4, 4, 2]
        config["stages"][stage]["nq"] = [2, 2, 1]
    for mode in modes:
        case = root / mode
        (case / "tmp").mkdir(parents=True)
        local = copy.deepcopy(config)
        for section in ["stages", "workflow"]:
            candidates = [k for k in local[section] if k.startswith("4.qe-phonon-")]
            assert len(candidates) == 1
            local[section][f"4.qe-phonon-{mode}"] = local[section].pop(candidates[0])
        phonon = local["stages"][f"4.qe-phonon-{mode}"]
        phonon["nq"] = [2, 2, 1]
        phonon["ldisp"], phonon["only_init"], phonon["lqdir"] = True, True, True
        if "q" not in phonon:
            phonon["q"] = {k: v for k, v in phonon.items() if not isinstance(v, dict)}
        phonon["q"]["nq"] = [2, 2, 1]
        phonon["q"]["only_init"] = False
        phonon["q"]["ldisp"] = mode == "index"
        local["workflow"][f"4.qe-phonon-{mode}"]["points_per_edge"] = 20
        local["programs"]["ph_args"] = ["-nk", "1"]
        local["slurm"]["header"] = (
            "#SBATCH --partition=local\n#SBATCH --nodes=1\n#SBATCH --ntasks=12\n"
            "#SBATCH --cpus-per-task=1\n#SBATCH --mem=6G\n#SBATCH --time=00:15:00\n"
            f"export TMPDIR={case / 'tmp'}\nexport OMP_NUM_THREADS=1\nexport MKL_NUM_THREADS=1\n"
        )
        local["programs"]["launcher"] = ["mpirun", "-np", "12"]
        (case / "scf.toml").write_text(toml.dumps(local))
        local["programs"]["launcher"] = ["mpirun", "-np", "6"]
        local["programs"]["pw_args"] = ["-nk", "3"]
        local["programs"]["epw_args"] = ["-nk", "6"]
        local["slurm"]["header"] = local["slurm"]["header"].replace("--ntasks=12", "--ntasks=6")
        (case / "input.toml").write_text(toml.dumps(local))
        assert tomllib.loads((case / "input.toml").read_text()) == local
    save(root / "baseline.json", {
        "source": str(source), "source_sha256": digest(source),
        "cli_sha256": digest(root / "runtime/epw_phonon"),
        "scientific_programs": {name: {"path": config["programs"][name],
                                      "sha256": digest(Path(config["programs"][name]))}
                                for name in ["pw_x", "ph_x", "q2r_x", "matdyn_x"]},
        "scf_mpi": 12, "q_mpi": 6, "q_grid": [2, 2, 1], "k_grid": [4, 4, 2],
        "scope": "真实原生小网格调度、取消续算和后处理验收，不作为网格收敛证据",
    })


def exercise(case, mode):
    print(f"开始原生 {mode} 模式：{case}", flush=True)
    public(case, "scf.toml", "scf", "submit-scf", "-s")
    jobs_file = case / "run/1.qe-scf/1.qe-scf.jobs"
    scf_jobs = [line.split("\t") for line in jobs_file.read_text().splitlines()[1:]]
    scf_job = next(int(job) for name, job in scf_jobs if name == "Stage allocation")
    scf = wait_for("SCF 分配结束", lambda: (
        row if (row := controller(case, scf_job)).get("JobState") in TERMINAL else None))
    save(case / "evidence/scf-terminal.json", scf)
    assert scf["JobState"] == "COMPLETED", scf
    public(case, "input.toml", "phonon", "submit-phonon", "-s")

    def all_submitted():
        s = state(case, mode)
        if s and s["init"]["status"] == "failed":
            raise RuntimeError(f"init 失败：{s}")
        return s if s and len(s["qpoints"]) >= 2 and all(
            q.get("job_id") for q in s["qpoints"]) and s["collect"].get("job_id") else None

    submitted = wait_for("所有 q 点和 collect 入队", all_submitted)
    save(case / "evidence/submitted.json", submitted)
    root = case / "run" / f"4.qe-phonon-{mode}"
    cancel = submitted["qpoints"][-1]
    cancel_job, collect_job = cancel["job_id"], submitted["collect"]["job_id"]
    initial_jobs = [submitted["init"]["job_id"], collect_job,
                    *(q["job_id"] for q in submitted["qpoints"])]
    save(case / "evidence/initial-controllers.json", {
        str(job): controller(case, job) for job in initial_jobs})
    initial = json.loads((case / "evidence/initial-controllers.json").read_text())
    for q in submitted["qpoints"]:
        assert f"afterok:{submitted['init']['job_id']}" in initial[str(q["job_id"])]["Dependency"]
        assert int(initial[str(q["job_id"])]["NumTasks"]) == 6
        assert f"afterok:{q['job_id']}" in initial[str(collect_job)]["Dependency"]

    def native_running():
        observed = controller(case, cancel_job)
        output = root / "q" / cancel["directory"] / "ph.out"
        if observed.get("JobState") in TERMINAL:
            raise RuntimeError("目标 q 在故障注入前已结束；保留现场，不伪造取消成功")
        return observed if observed.get("JobState") == "RUNNING" and output.exists() and (
            "Program PHONON" in output.read_text(errors="replace")) else None

    active = wait_for("目标 q 的原生 PH 已启动", native_running)
    assert int(active["NumCPUs"]) == 6 and int(active["NumTasks"]) == 6, active
    save(case / "evidence/cancel-before.json", active)
    shutil.copy2(root / "q" / cancel["directory"] / "ph.out", case / "evidence/cancelled-ph-started.out")
    run(case, ["scancel", str(cancel_job)], "cancel-one-native-q")

    def cancelled_and_others_complete():
        s = state(case, mode)
        observed = controller(case, cancel_job)
        others = [q for q in s["qpoints"] if q["job_id"] != cancel_job]
        if any(q["status"] == "failed" for q in others):
            raise RuntimeError(f"未注入故障的 q 点失败：{s}")
        return s if observed.get("JobState") == "CANCELLED" and all(
            q["status"] == "completed" for q in others) else None

    before = wait_for("取消终态及其余 q 点完成", cancelled_and_others_complete)
    blocked = controller(case, collect_job)
    save(case / "evidence/blocked-collect.json", blocked)
    assert blocked["JobState"] == "PENDING", blocked
    assert "Dependency" in blocked["Reason"], blocked
    assert not list(root.rglob("q2r.out")), "失败前驱不能启动后处理"
    successful = [q for q in before["qpoints"] if q["job_id"] != cancel_job]
    protected = protected_q_outputs(case, mode, successful)
    save(case / "evidence/successful-q-before.json", protected)
    public(case, "input.toml", "phonon", "resume-phonon", "--resume", "-s")
    resumed = state(case, mode)
    save(case / "evidence/resumed.json", resumed)
    assert resumed["init"]["job_id"] == submitted["init"]["job_id"]
    assert resumed["collect"]["job_id"] == collect_job
    for old, new in zip(submitted["qpoints"], resumed["qpoints"]):
        assert old["global_index"] == new["global_index"]
        assert (old["job_id"] != new["job_id"]) == (old["job_id"] == cancel_job)
    replacement = next(q["job_id"] for q in resumed["qpoints"]
                       if q["global_index"] == cancel["global_index"])
    save(case / "evidence/resumed-collect-controller.json", controller(case, collect_job))
    assert replacement in resumed["collect"]["dependencies"]

    def collected():
        s = state(case, mode)
        if s["collect"]["status"] == "failed" or any(
                q["status"] == "failed" for q in s["qpoints"]):
            raise RuntimeError(f"续算失败：{s}")
        return s if s["collect"]["status"] == "completed" else None

    done = wait_for("所有原生 q、q2r、matdyn 和绘图完成", collected)
    terminal = wait_for("collect Slurm 成功终态", lambda: (
        row if (row := controller(case, collect_job)).get("JobState") in TERMINAL else None))
    assert terminal["JobState"] == "COMPLETED", terminal
    assert protected_q_outputs(case, mode, successful) == protected
    save(case / "evidence/final-state.json", done)
    verify_finished(case, mode)


def verify_finished(case, mode):
    """只核验已有现场；恢复验收器时不重复科学计算或故障注入。"""
    done = state(case, mode)
    submitted = json.loads((case / "evidence/submitted.json").read_text())
    cancelled = json.loads((case / "evidence/cancel-before.json").read_text())
    protected = json.loads((case / "evidence/successful-q-before.json").read_text())
    cancel_job = int(cancelled["JobId"])
    collect_job = submitted["collect"]["job_id"]
    replacement = next(q["job_id"] for q in done["qpoints"]
                       if q["job_id"] not in [p["job_id"] for p in submitted["qpoints"]])
    successful = [q for q in submitted["qpoints"] if q["job_id"] != cancel_job]
    assert done["collect"]["status"] == "completed"
    assert all(q["status"] == "completed" for q in done["qpoints"])
    assert protected_q_outputs(case, mode, successful) == protected
    root = case / "run" / f"4.qe-phonon-{mode}"
    frequencies = {}
    for q in done["qpoints"]:
        output = root / "q" / q["directory"] / "ph.out"
        text = output.read_text()
        assert "JOB DONE" in text and re.search(r"running on\s+6\s+processor", text)
        values = re.findall(r"freq\s+\(\s*\d+\)\s*=\s*\S+\s+\[THz\]\s*=\s*([-\d.]+)\s+\[cm-1\]", text)
        assert len(values) == 9, "MgB2 每个 q 点应有九个原生频率"
        frequencies[q["directory"]] = list(map(float, values))
    post = {str((root / name).relative_to(case)): digest(root / name)
            for name in ["q2r.out", "matdyn_phband.out", "matdyn_phdos.out"]}
    for name in post:
        assert "JOB DONE" in (case / name).read_text()
    plot = case / "run/results" / f"4.qe-phonon-{mode}" / "phonon.png"
    assert plot.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
    status = public(case, "input.toml", "phonon", "final-status", "--status", "--json")
    assert json.loads(status.stdout)["steps"]["phonon"]["reusable"]
    public(case, "input.toml", "phonon", "repeat-submit", "-s")
    assert state(case, mode) == done, "重复入口不能创建新 q 或 collect"
    scf_job = int(json.loads((case / "evidence/scf-terminal.json").read_text())["JobId"])
    jobs = [submitted["init"]["job_id"], collect_job,
            *(q["job_id"] for q in submitted["qpoints"]), replacement, scf_job]
    accounting = completions(case)
    records = {str(job): accounting[str(job)] for job in jobs}
    assert records[str(cancel_job)][1] == "CANCELLED"
    for job in jobs:
        if job != cancel_job:
            assert records[str(job)][1:3] == ["COMPLETED", "0:0"], records[str(job)]
    initial = json.loads((case / "evidence/initial-controllers.json").read_text())
    init_job = submitted["init"]["job_id"]
    init_end = records[str(init_job)][4]
    intervals = []
    for q in submitted["qpoints"]:
        job = str(q["job_id"])
        assert initial[job]["SubmitTime"] <= init_end, "不能等前一个 q 完成后才提交"
        assert f"afterok:{init_job}" in initial[job]["Dependency"]
        assert records[job][3] >= init_end, "q 不能早于 init 成功结束"
        assert records[job][5] == "6"
        intervals.append([datetime.fromisoformat(records[job][3]), datetime.fromisoformat(records[job][4])])
    overlap = (min(end for _, end in intervals) - max(start for start, _ in intervals)).total_seconds()
    assert overlap > 0, "两个真实六路 q 作业必须有重叠执行区间"
    assert records[str(collect_job)][3] >= max(records[str(q["job_id"])][4] for q in done["qpoints"])
    assert submitted["collect"]["job_id"] == done["collect"]["job_id"]
    baseline = json.loads((case.parent / "baseline.json").read_text())
    assert digest(case.parent / "runtime/epw_phonon") == baseline["cli_sha256"]
    for program in baseline["scientific_programs"].values():
        assert digest(Path(program["path"])) == program["sha256"]
    save(case / "result.json", {"mode": mode, "passed": True, "jobs": records,
                                "cancelled_q_job": cancel_job, "replacement_q_job": replacement,
                                "collect_job_preserved": collect_job,
                                "successful_q_unchanged": protected, "postprocessing": post,
                                "initial_q_overlap_seconds": overlap, "frequencies_cm1": frequencies,
                                "plot": {"path": str(plot.relative_to(case)), "sha256": digest(plot)}})
    print(f"通过：{mode}；取消 {cancel_job} → 续算 {replacement}；collect {collect_job}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=REPO / "examples/MgB2/ubuntu/input.toml")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--modes", nargs="+", choices=["index", "coords"], default=["index", "coords"])
    args = parser.parse_args()
    root = args.root.resolve()
    assert root.is_relative_to(REPO / "test_auto"), "真实验收必须在 test_auto 内"
    prepare(root, args.source.resolve(), args.modes)
    for mode in args.modes:
        exercise(root / mode, mode)
    save(root / "result.json", {"passed": True, "modes": args.modes})


if __name__ == "__main__":
    main()
