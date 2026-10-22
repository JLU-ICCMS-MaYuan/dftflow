#!/usr/bin/env python3
"""真实 Slurm 验收辅助程序；只调用公开入口，不替代生产状态机。"""
import argparse
import csv
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
try:
    import tomllib
except ModuleNotFoundError:
    sys.exit("该测试需要 Python 3.11+（标准库 tomllib）；请用 PYTHON 指定已有新版解释器，不需要安装依赖。")

REPO = Path(__file__).resolve().parents[3]
BASE = REPO / "test_auto/MgB2/local_run"
STAGES = {
    "scf": ("epw_phonon", "1.qe-scf"),
    "nscf": ("epw_phonon", "2.qe-nscf"),
    "electronic": ("epw_phonon", "3.qe-electronic"),
    "wannier": ("epw_wannier", "5.epw-wannier"),
    "phonon": ("epw_phonon", "4.qe-phonon-nosplit"),
    "coarse": ("epw_wannier", "6.epw-coarse"),
    "fine": ("epw_superconductivity", "7.epw-fine"),
    "superconductivity": ("epw_superconductivity", "8.epw-superconductivity"),
}


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def command(case, argv, label, check=True, env=None):
    print("执行：", " ".join(map(str, argv)), flush=True)
    result = subprocess.run(list(map(str, argv)), text=True, capture_output=True,
                            cwd=case, env=env)
    log = case / "evidence" / f"{time.time_ns()}-{label}.json"
    write(log, {"argv": list(map(str, argv)), "exit": result.returncode,
                "stdout": result.stdout, "stderr": result.stderr})
    if result.returncode and check:
        raise RuntimeError(f"{label} 失败，见 {log}\n{result.stdout}\n{result.stderr}")
    return result


def prepare(case, source, binaries):
    if case.exists():
        raise RuntimeError(f"拒绝覆盖已有测试现场：{case}")
    config = tomllib.loads(source.read_text())
    if config["phonon"]["mode"] != "nosplit" or config["superconductivity"]["task"] != "gap":
        raise RuntimeError("本验收固定当前 nosplit／gap 算例，不自动改变科学模式")
    case.mkdir(parents=True)
    structure = (source.parent / config["structure"]).resolve()
    shutil.copy2(structure, case / "POSCAR")
    shutil.copy2(source.parent / config["kpath"], case / "KPATH.in")
    text = source.read_text()
    text = re.sub(r'^structure\s*=.*$', 'structure = "POSCAR"', text, flags=re.M)
    text = re.sub(r'^work_dir\s*=.*$', 'work_dir = "run"', text, flags=re.M)
    # 仅隔离路径；网格、窗口、温度、资源与共享预算全部沿用原配置。
    (case / "epwinput.toml").write_text(text)
    (case / "bin").mkdir()
    identities = {}
    for name in {x[0] for x in STAGES.values()}:
        shutil.copy2(binaries / name, case / "bin" / name)
        identities[name] = digest(case / "bin" / name)
    scientific = {name: {"path": item["path"], "sha256": digest(Path(item["path"]))}
                  for name, item in config["programs"].items() if isinstance(item, dict)}
    write(case / "baseline.json", {
        "source_config": str(source), "source_config_sha256": digest(source),
        "config_sha256": digest(case / "epwinput.toml"), "programs": identities,
        "scientific_programs": scientific,
        "structure_sha256": digest(structure),
        "ledger": config["budget"]["ledger"],
        "order": list(STAGES), "scope": "当前 PBE 小网格功能验收，不评判粗网格收敛或搜索 Tc",
    })
    write(case / "ledger-before.json", json.loads(Path(config["budget"]["ledger"]).read_text()))


def identities(case):
    baseline = json.loads((case / "baseline.json").read_text())
    assert digest(case / "epwinput.toml") == baseline["config_sha256"], "配置变化，请保留现场重新建案"
    for name, expected in baseline["programs"].items():
        assert digest(case / "bin" / name) == expected, f"受测程序被替换：{name}"
    active = case / "active-programs.json"
    if active.exists():
        revision = json.loads(active.read_text())
        for name, expected in revision["programs"].items():
            assert digest(case / revision["directory"] / name) == expected, f"修订程序被替换：{name}"
    return baseline


def binary(case, name):
    active = case / "active-programs.json"
    directory = json.loads(active.read_text())["directory"] if active.exists() else "bin"
    return case / directory / name


def upgrade(case, binaries, reason):
    identities(case)
    if not reason:
        raise RuntimeError("升级须通过 --reason 记录已定位缺陷；保留全部旧制品和科学检查点")
    # 不修改已提交脚本指向的程序；只在本现场全部作业终态后切换后续公开入口。
    jobs = {task["job"] for task in calls(case) if isinstance(task.get("job"), int)}
    for path in (case / "evidence").glob("*.json"):
        record = json.loads(path.read_text())
        jobs.update(map(int, re.findall(r"(?:job_id|job):\s*(\d+)", record.get("stdout", ""))))
    for job in sorted(jobs):
        result = command(case, ["sacct", "-j", job, "-n", "-P", "--format=JobIDRaw,State"], "upgrade-job-state")
        row = next((x.split("|") for x in result.stdout.splitlines() if x.startswith(f"{job}|")), None)
        if not row or row[1].split()[0] not in {"COMPLETED", "FAILED", "CANCELLED", "TIMEOUT", "OUT_OF_MEMORY", "NODE_FAIL"}:
            raise RuntimeError(f"作业 {job} 尚未确认终态，拒绝切换程序")
    directory = f"bin-revision-{time.time_ns()}"
    target = case / directory
    target.mkdir()
    programs = {}
    for name in {x[0] for x in STAGES.values()}:
        shutil.copy2(binaries / name, target / name)
        programs[name] = digest(target / name)
    revision = {"directory": directory, "programs": programs, "reason": reason,
                "previous_directory": str(binary(case, "epw_phonon").parent)}
    write(case / "evidence" / f"{directory}.json", revision)
    write(case / "active-programs.json", revision)


def calls(case):
    result = []
    for path in sorted((case / "run/epw-state/native-jobs").glob("*.json")):
        task = json.loads(path.read_text())
        result.append({k: task.get(k) for k in ("id", "key", "directory", "stem", "command", "state", "job")})
    return result


def wait_jobs(case, jobs):
    deadline = time.monotonic() + 3 * 3600
    while jobs:
        for job in list(jobs):
            result = subprocess.run(["sacct", "-j", str(job), "-n", "-P",
                                     "--format=JobIDRaw,State,ExitCode,ElapsedRaw,AllocNodes"],
                                    capture_output=True, text=True, check=True)
            rows = [line.split("|") for line in result.stdout.splitlines()]
            row = next((r for r in rows if r[0] == str(job)), None)
            if row and row[1].split()[0] not in {"PENDING", "RUNNING", "CONFIGURING", "COMPLETING", "REQUEUED"}:
                write(case / "evidence" / f"slurm-{job}.json", {"job": job, "row": row})
                print(f"作业 {job}：{row[1]}，退出 {row[2]}，{row[3]} 节点秒", flush=True)
                jobs.remove(job)
                if row[1] != "COMPLETED" or row[2] != "0:0":
                    raise RuntimeError(f"作业 {job} 失败；保留现场，修复后显式 --resume")
        if time.monotonic() > deadline:
            raise RuntimeError("等待超时；不取消或重复提交原作业，请查 Slurm")
        if jobs:
            time.sleep(10)


def run_stage(case, stage, resume):
    identities(case)
    program, directory = STAGES[stage]
    was_completed = (case / "run" / directory / "completion.json").is_file()
    argv = [binary(case, program), "-c", case / "epwinput.toml", "--stage", stage, "-s"]
    before = calls(case)
    if resume:
        argv.append("--resume")
    result = command(case, argv, stage)
    print(result.stdout, flush=True)
    # 以公开输出的作业号为准，包括双方法批次的两个独立作业。
    jobs = set(map(int, re.findall(r"(?:job_id|job):\s*(\d+)", result.stdout)))
    if not jobs:
        for path in (case / "run/epw-state").glob("native-active*.json"):
            record = json.loads(path.read_text())
            if record.get("state") not in {"validated", "complete", "succeeded"} and record.get("job"):
                jobs.add(record["job"])
    wait_jobs(case, jobs)
    # 1／2 是单原生任务，结束后公开命令核验；3～8 必须先检查作业内发布。
    if stage in {"scf", "nscf"}:
        again = command(case, argv, stage + "-validate")
        assert not re.search(r"job_id:\s*\d+", again.stdout), "单任务核验意外重提"
    stage_dir = case / "run" / directory
    assert (stage_dir / "completion.json").is_file(), f"缺少 {stage} 科学完成记录"
    if stage not in {"scf", "nscf", "fine"}:
        manifests = list((case / "run/results" / directory).rglob("stage-manifest.json"))
        assert manifests, f"{stage} 作业结束后没有即时发布结果"
        command(case, [binary(case, "epw_superconductivity"), "--results-check", "--results",
                       case / "run/results", "--json"], stage + "-results")
    if stage == "wannier":
        if not was_completed:
            assert not (case / "run/4.qe-phonon-nosplit/completion.json").exists(), "电子验收之前已运行 PH"
            assert not (case / "run/6.epw-coarse/completion.json").exists(), "电子验收之前已运行 coarse"
        assert (stage_dir / "comparison-native/electron/band.eig").is_file()
        for name in ("electronic-bands.png", "electronic-dos.png", "metrics.json", "sampling.json"):
            assert (case / "run/results" / directory / name).is_file(), f"缺少电子交付 {name}"
    if stage == "coarse":
        assert not (stage_dir / "comparison-native/electron").exists(), "第 6 步仍拥有电子比较目录"
        sources = json.loads((stage_dir / "comparison-native/sources.json").read_text())
        assert "5.epw-wannier/comparison-native/electron" in sources["epw_bands"]["path"]
        assert "3.qe-electronic/qe_eband" in sources["qe_bands_xml"]["path"]
    if stage == "superconductivity":
        run = json.loads((stage_dir / "manifest.json").read_text())
        assert run["execution_complete"] and run["data_complete"], "六点原生执行或能隙数据不完整"
        assert len(run["batches"]) == 2 and len(run["attempts"]) == 6, "预期两个方法批次、六个有效点"
        for batch in run["batches"]:
            assert len(batch["requests"]) == 3 and batch["collected"]
        assert all(point["status"] == "valid" for point in run["attempts"])
    after = calls(case)
    assert not any("wannier90.x" == Path(arg).name for t in after for arg in t.get("command", [])), "出现独立 Wannier90 调用"
    write(case / "evidence" / f"{stage}-verified.json", {"calls_before": before, "calls_after": after,
                                                       "immediate_publication": stage not in {"scf", "nscf", "fine"}})
    # 核验已交付阶段幂等：不会产生新科学任务或 Slurm 提交。
    repeated = command(case, argv, stage + "-reuse")
    assert "job_id:" not in repeated.stdout, "成功阶段重复提交了作业"
    assert calls(case) == after, "成功阶段重复执行改变了科学任务记录"
    print(f"{stage}：真实作业、交付和复用验证通过", flush=True)


def portable(case, name):
    identities(case)
    if Path(name).name != name or name in {".", ".."}:
        raise RuntimeError("重绘副本须使用单层目录名")
    copied = case / name
    if copied.exists():
        raise RuntimeError("移动重绘副本已存在，拒绝覆盖既有证据")
    shutil.copytree(case / "run/results", copied)
    tables = {str(p.relative_to(copied)): digest(p) for p in copied.rglob("*")
              if p.is_file() and (p.suffix in {".csv", ".dat", ".eig"}
                                  or p.name.startswith("decay.") or p.name == "decay-data.json")}
    environment = dict(os.environ, PATH="/nonexistent")
    command(case, [binary(case, "epw_superconductivity"), "--plot", "--results", copied],
            "portable-redraw", env=environment)
    command(case, [binary(case, "epw_superconductivity"), "--results-check", "--results", copied, "--json"],
            "portable-check", env=environment)
    assert all(digest(copied / name) == value for name, value in tables.items()), "重绘改变数值表"
    write(case / f"{name}-verification.json", {"path_without_scientific_programs": True, "tables": tables})


def audit(case):
    baseline = identities(case)
    for item in baseline["scientific_programs"].values():
        assert digest(Path(item["path"])) == item["sha256"], "外部科学程序身份变化"
    for _, directory in STAGES.values():
        assert (case / "run" / directory / "completion.json").is_file(), f"缺少阶段科学完成记录：{directory}"
    tasks = calls(case)
    names = {"pw.x", "ph.x", "bands.x", "dos.x", "q2r.x", "matdyn.x", "epw.x", "postw90.x", "wannier90.x"}
    counts = Counter(Path(arg).name for task in tasks for arg in task["command"] if Path(arg).name in names)
    expected = {"pw.x": 3, "ph.x": 1, "bands.x": 1, "dos.x": 1,
                "q2r.x": 2, "matdyn.x": 3, "epw.x": 6, "postw90.x": 1}
    assert dict(counts) == expected, f"科学任务数量与本固定算例不符：{counts}"
    assert all(t["state"] == "validated" for t in tasks), "存在未核验原生任务"
    elements = case / "run/results/4.qe-phonon-nosplit/phdos/phonon_dos_elements.csv"
    rows = list(csv.DictReader(elements.open()))
    residual = max(abs(float(r["Mg"]) + float(r["B"]) - float(r["TDOS"])) for r in rows)
    assert residual <= 2e-6, f"Mg／B 元素贡献不能还原总 DOS：{residual}"
    decay = case / "run/results/6.epw-coarse/decay"
    panels = ["decay-H", "decay-dynmat", "decay-epmate", "decay-epmatp", "decay-overview"]
    for panel in panels:
        for extension in ("png", "svg", "pdf"):
            assert (decay / f"{panel}.{extension}").is_file(), f"缺少衰减图：{panel}.{extension}"
    decay_data = json.loads((decay / "decay-data.json").read_text())
    decay_counts = {}
    for i, name in enumerate(("decay.H", "decay.dynmat", "decay.epmate", "decay.epmatp")):
        source = case / "run/6.epw-coarse" / name
        assert digest(source) == digest(decay / name), f"衰减原始副本变化：{name}"
        raw = [[float(v.replace("D", "E").replace("d", "E")) for v in line.split()[:2]]
               for line in source.read_text().splitlines() if line.strip() and not line.lstrip().startswith("#")]
        assert raw == decay_data["panels"][i], f"衰减数据丢点或被合并：{name}"
        decay_counts[name] = len(raw)
    config = tomllib.loads((case / "epwinput.toml").read_text())
    solver = json.loads((case / "run/8.epw-superconductivity/manifest.json").read_text())
    assert solver["execution_complete"] and solver["data_complete"]
    assert len(solver["batches"]) == 2 and len(solver["attempts"]) == 6
    for batch in solver["batches"]:
        # 每个方法的原生输入必须显式包含完整温度列表，不接受逐点多次执行。
        path = Path(batch["task"]["directory"]) / "epw.in"
        text = path.read_text()
        temperatures = list(map(float, re.findall(r"(?m)^\s*temps\(\d+\)\s*=\s*([\d.]+)", text)))
        assert temperatures == config["superconductivity"]["gap_temperatures_k"], f"原生温度列表不同：{path}"
    assert all(p["status"] == "valid" for p in solver["attempts"])
    summary = json.loads((case / "run/results/8.epw-superconductivity/summary.json").read_text())
    points = summary["gap_run"]["points"]
    assert len(points) == 6 and all(p["status"]["execution_complete"] and p["status"]["converged"]
                                  and p["status"]["data_status"] == "valid" for p in points)
    command(case, [binary(case, "epw_phonon"), "-c", case / "epwinput.toml", "--status", "--json"], "audit-status")
    command(case, [binary(case, "epw_superconductivity"), "--results-check", "--results", case / "run/results", "--json"], "audit-results")
    write(case / "audit.json", {"native_task_counts": dict(counts), "element_dos_max_residual": residual,
                              "valid_gap_points": 6, "no_independent_wannier90": True,
                              "decay_rows": decay_counts, "decay_figure_count": 15})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["prepare", "run", "portable", "upgrade", "audit"])
    parser.add_argument("--case", type=Path, default=BASE / "012-stage-results")
    parser.add_argument("--source", type=Path, default=REPO / "examples/MgB2/sugon-amd/run/epwflow/epwinput.toml")
    parser.add_argument("--bin", type=Path, default=REPO / "bin/epwflow")
    parser.add_argument("--stage", choices=list(STAGES))
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--reason")
    parser.add_argument("--portable-name", default="portable-results")
    args = parser.parse_args()
    case = args.case.resolve()
    if not case.is_relative_to(BASE.resolve()):
        parser.error("真实运行现场必须在 test_auto/MgB2/local_run 内")
    if args.operation == "prepare":
        prepare(case, args.source.resolve(), args.bin.resolve())
    elif args.operation == "portable":
        portable(case, args.portable_name)
    elif args.operation == "upgrade":
        upgrade(case, args.bin.resolve(), args.reason)
    elif args.operation == "audit":
        audit(case)
    else:
        for stage in [args.stage] if args.stage else STAGES:
            run_stage(case, stage, args.resume)


if __name__ == "__main__":
    main()
