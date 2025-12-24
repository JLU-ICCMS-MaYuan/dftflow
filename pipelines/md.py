import logging
import shutil
from pathlib import Path
from typing import Optional, List

from vasp.pipelines.base import BasePipeline
from vasp.pipelines.utils import prepare_potcar, ensure_poscar
from vasp.pipelines import defaults as dft
from vasp.utils.job import load_job_config, write_job_script, submit_job, JobConfig

logger = logging.getLogger(__name__)


class MdPipeline(BasePipeline):
    """分子动力学（NVT）Pipeline。"""

    def __init__(
        self,
        structure_file: Path,
        work_dir: Path,
        potim: float = dft.DEFAULT_MD["potim"],
        tebeg: float = dft.DEFAULT_MD["tebeg"],
        teend: float = dft.DEFAULT_MD["teend"],
        nsw: int = dft.DEFAULT_MD["nsw"],
        kspacing: float = dft.DEFAULT_KSPACING,
        encut: Optional[float] = None,
        queue_system: Optional[str] = None,
        mpi_procs: Optional[str] = None,
        custom_steps: Optional[List[str]] = None,
        pressure: float = 0.0,
        potcar_map: Optional[dict[str, str]] = None,
        job_cfg: Optional[JobConfig] = None,
        config_path: Optional[Path] = None,
        **kwargs,
    ):
        super().__init__(structure_file, work_dir, pressure=pressure, **kwargs)

        self.job_cfg = job_cfg or load_job_config(config_path)
        self.potim = potim
        self.tebeg = tebeg
        self.teend = teend
        self.nsw = nsw
        self.kspacing = kspacing
        self.encut = encut
        self.queue_system = queue_system or (self.job_cfg.default_queue if self.job_cfg else None)
        if not self.queue_system:
            raise ValueError("未找到队列配置，请在 job_templates.local.toml 的 [templates] 中至少提供一个队列头")
        self.mpi_procs = mpi_procs
        self.potcar_map = potcar_map or {}
        self.custom_steps = self._normalize_steps(custom_steps)
        self.pressure = pressure

        self.md_dir = self.work_dir / "01_md"

    def get_steps(self):
        if self.custom_steps:
            return self.custom_steps
        return ["md"]

    def execute_step(self, step_name: str) -> bool:
        if step_name == "md":
            return self._run_md()
        logger.error(f"未知步骤: {step_name}")
        return False

    def _normalize_steps(self, custom_steps: Optional[List[str]]) -> Optional[List[str]]:
        if not custom_steps:
            return None
        allowed = ["md"]
        normalized: List[str] = []
        for step in custom_steps:
            name = step.strip().lower()
            if name in allowed:
                if name not in normalized:
                    normalized.append(name)
            else:
                logger.warning(f"忽略未支持的步骤: {name}")
        return normalized or None

    def _run_md(self) -> bool:
        logger.info("执行分子动力学计算...")

        self.md_dir.mkdir(parents=True, exist_ok=True)
        ensure_poscar(Path(self.structure_file), self.md_dir / "POSCAR")

        self._write_md_incar(self.md_dir / "INCAR")
        self._write_kpoints(self.md_dir / "KPOINTS", self.md_dir / "POSCAR", self.kspacing)

        if not self.potcar_map:
            logger.error("缺少 [potcar] 配置，无法生成 POTCAR")
            return False
        if not prepare_potcar(self.md_dir / "POSCAR", self.potcar_map, self.md_dir / "POTCAR"):
            logger.error("POTCAR准备失败")
            return False

        job_script = self._write_job_script(self.md_dir, "md")
        job_id = self._submit_job(self.md_dir, job_script)

        if self.prepare_only:
            logger.info("submit=false，仅准备 MD 输入，不提交任务")
            return True

        if not self._wait_for_job(job_id, self.md_dir, self.queue_system):
            return False

        if not self._check_job_completed(self.md_dir):
            logger.error("MD 计算未完成")
            return False

        logger.info("分子动力学计算完成")
        return True

    def _write_md_incar(self, incar_file: Path):
        """写入 MD INCAR。"""
        with open(incar_file, "w") as f:
            f.write("# Molecular Dynamics INCAR\n")
            f.write("SYSTEM = MD Simulation\n\n")
            f.write("PREC = Accurate\n")
            f.write(f"ENCUT = {self.encut if self.encut else dft.DEFAULT_MD['encut']}\n")
            f.write(f"EDIFF = {dft.DEFAULT_MD['ediff']}\n")
            f.write(f"ISMEAR = {dft.DEFAULT_MD['ismear']}\n")
            f.write(f"SIGMA = {dft.DEFAULT_MD['sigma']}\n\n")
            f.write("IBRION = 0\n")
            f.write("MDALGO = 2\n")
            f.write(f"NSW = {self.nsw}\n")
            f.write(f"POTIM = {self.potim}\n")
            f.write(f"TEBEG = {self.tebeg}\n")
            f.write(f"TEEND = {self.teend}\n")
            f.write("SMASS = 0\n")
            f.write(f"PSTRESS = {self.pressure_kbar}\n")
            f.write("LWAVE = .FALSE.\n")
            f.write("LCHARG = .FALSE.\n")

            f.write("LWAVE = .FALSE.\n")
            f.write("LCHARG = .FALSE.\n")

    def _write_kpoints(self, kpoints_file: Path, poscar_file: Path, kspacing: float):
        from vasp.pipelines.utils import kspacing_to_mesh
        n1, n2, n3 = kspacing_to_mesh(poscar_file, kspacing)
        with open(kpoints_file, "w") as f:
            f.write("Automatic mesh\n")
            f.write("0\n")
            f.write("Gamma\n")
            f.write(f"{n1} {n2} {n3}\n")
            f.write("0 0 0\n")

    def _write_job_script(self, work_dir: Path, job_name: str) -> str:
        script_path = write_job_script(
            work_dir=work_dir,
            job_name=job_name,
            queue_system=self.queue_system,
            cfg=self.job_cfg,
            mpi_procs=self.mpi_procs,
        )
        return str(script_path)

    def _submit_job(self, work_dir: Path, job_script: str) -> str:
        if self.prepare_only:
            logger.info("submit=false，未提交任务")
            return "prepare_only"
        return submit_job(Path(job_script), self.queue_system)
