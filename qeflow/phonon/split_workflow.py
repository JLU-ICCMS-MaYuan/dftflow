#!/usr/bin/env python3
import os
import shlex
from dataclasses import dataclass
from typing import Callable, Dict, Optional

from qeflow.phonon import console
from qeflow.phonon.context import PhononStepContext, create_context
from qeflow.phonon.root_steps import prepare_root_scf_step
from qeflow.phonon.runner import print_stage, run_root_scripts, run_shell
from qeflow.phonon.script_plan import write_labeled_script
from qeflow.phonon.ph_step import prepare_ph_nosplit_step, run_ph_to_dyn0
from qeflow.phonon.postprocess_workflow import run_process_phonon_steps
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_q2r import prepare_q2r_step
from qeflow.phonon.qe_merge import run_merge_step
from qeflow.phonon.qe_split_submit import run_split_submit
from qeflow.phonon.split_prepare import prepare_split_assign_coords_step, prepare_split_assign_index_step
from qeflow.phonon.status_machine import (
    all_split_tasks_finished,
    dyn0_state,
    merge_state,
    ph_tasks,
    print_resume_point,
    refresh_workflow_status,
    root_step_scripts,
    split_task_needs_run,
)


ROOT_SCRIPT_NAMES = [
    "run_scf.sh",
    "run_ph.sh",
    "run_q2r.sh",
    "run_matdyn_phband.sh",
    "run_matdyn_phdos.sh",
]

SplitPrepare = Callable[[PhononStepContext], int]
SubmitScript = Callable[[str, str], object]
RunShell = Callable[[str, str], None]
RunDyn0 = Callable[[str, str, str, str], None]


@dataclass(frozen=True)
class SplitPhononWorkflowSpec:
    mode: str
    workflow_name: str
    description: str
    prepare_split_step: SplitPrepare

    @property
    def resume_command(self) -> str:
        return f"{self.workflow_name} -s"


class SplitPhononWorkflow:
    def __init__(
        self,
        spec: SplitPhononWorkflowSpec,
        *,
        submit_script: SubmitScript = console.submit_script,
        split_run_shell: RunShell = run_shell,
        run_dyn0: RunDyn0 = run_ph_to_dyn0,
    ) -> None:
        self.spec = spec
        self.submit_script = submit_script
        self.split_run_shell = split_run_shell
        self.run_dyn0 = run_dyn0

    def run(
        self,
        config_file: str,
        struct_file: Optional[str],
        run_calc: bool,
        submit_job: bool,
    ) -> int:
        context = create_context(config_file, struct_file, workflow_name=self.spec.workflow_name)
        prepare_root_scf_step(context)
        prepare_ph_nosplit_step(context, dyn0_only=True)
        prepare_q2r_step(context)
        prepare_matdyn_phband_step(context)
        prepare_matdyn_phdos_step(context)
        console.generated_scripts(ROOT_SCRIPT_NAMES)

        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if workflow["scf"] != "finished" or dyn0_state(workflow) != "finished":
            result = self._advance_pre_stage(context, workflow, run_calc, submit_job)
            if result is not None:
                return result
            workflow = refresh_workflow_status(context.workspace, context.prefix)

        workflow = self._ensure_split_materialized(config_file, struct_file)

        if split_task_needs_run(workflow):
            result = self._advance_split_tasks(context, workflow, config_file, struct_file, run_calc, submit_job)
            if result is not None:
                return result

        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if all_split_tasks_finished(workflow):
            result = self._advance_post_stage(context, workflow, config_file, struct_file, run_calc, submit_job)
            if result is not None:
                return result

        print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
        return 0

    def _ensure_split_materialized(self, config_file: str, struct_file: Optional[str]) -> Dict[str, object]:
        context = create_context(config_file, struct_file, workflow_name=self.spec.workflow_name)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        q_index_map = os.path.join(context.workspace, "q_index_map.json")
        if dyn0_state(workflow) == "finished" and not os.path.exists(q_index_map):
            print_stage(3, "生成 split q 点输入")
            self.spec.prepare_split_step(context)
            workflow = refresh_workflow_status(context.workspace, context.prefix)
        return workflow

    def _write_pre_and_split_stage_script(self, context: PhononStepContext, workflow: Dict[str, object]) -> str:
        steps = []
        if workflow["scf"] != "finished":
            steps.append(("运行 SCF", "bash run_scf.sh"))
        if dyn0_state(workflow) != "finished":
            steps.append(("运行 dyn0", "bash run_ph.sh"))
        steps.append(
            (
                "准备并提交 split q 点任务",
                f"qe_split_submit -c {shlex.quote(context.config_file)} --mode {self.spec.mode}",
            )
        )
        return write_labeled_script(
            context,
            "submit_pre_and_split_stage.sh",
            steps,
            start_index=1,
            total_steps=11,
            stage="pre_and_split",
            completion_message="1/11-3/11 pre + split 提交阶段",
        )

    def _write_merge_and_process_stage_script(self, context: PhononStepContext) -> str:
        steps = [
            ("merge", f"qe_merge -c {shlex.quote(context.config_file)}"),
            ("q2r", "bash run_q2r.sh"),
            ("matdyn_phband", "bash run_matdyn_phband.sh"),
            ("collect phband", f"qe_process_phonon -c {shlex.quote(context.config_file)} --collect phband"),
            ("matdyn_phdos", "bash run_matdyn_phdos.sh"),
            ("collect phdos", f"qe_process_phonon -c {shlex.quote(context.config_file)} --collect phdos"),
            ("plot", f"qe_process_phonon -c {shlex.quote(context.config_file)} --plot"),
            ("done", "true"),
        ]
        return write_labeled_script(
            context,
            "submit_merge_and_process_phonon.sh",
            steps,
            start_index=4,
            total_steps=11,
            stage="merge_and_process",
            completion_message="4/11-11/11 merge + phonon 后处理阶段",
        )

    def _advance_pre_stage(
        self,
        context: PhononStepContext,
        workflow: Dict[str, object],
        run_calc: bool,
        submit_job: bool,
    ) -> Optional[int]:
        if run_calc:
            if workflow["scf"] != "finished":
                run_root_scripts(
                    context,
                    ["run_scf.sh"],
                    labels={"run_scf.sh": "运行根目录 SCF"},
                    stage_no=1,
                )
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            if dyn0_state(workflow) != "finished":
                print_stage(2, "运行根目录 dyn0 预处理")
                self.run_dyn0(
                    context.workspace,
                    context.commands["ph"],
                    os.path.join(context.workspace, f"{context.prefix}.dyn0"),
                    os.path.join(context.workspace, "ph.out"),
                )
            return None

        if submit_job:
            if workflow["scf"] != "finished" or dyn0_state(workflow) != "finished":
                aggregate = self._write_pre_and_split_stage_script(context, workflow)
                console.generated(os.path.basename(aggregate))
                self.submit_script(context.workspace, os.path.basename(aggregate))
            console.next_step(f"等待 split q 点任务完成后，再执行 {self.spec.resume_command}")
            return 0
        return None

    def _advance_split_tasks(
        self,
        context: PhononStepContext,
        workflow: Dict[str, object],
        config_file: str,
        struct_file: Optional[str],
        run_calc: bool,
        submit_job: bool,
    ) -> Optional[int]:
        if run_calc:
            tasks = ph_tasks(workflow)
            split_task_no = 1
            for q_name in sorted(tasks):
                if tasks.get(q_name) != "finished":
                    print_stage(f"3_{split_task_no}", f"运行 split 任务 {q_name}")
                    self.split_run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
                    refresh_workflow_status(context.workspace, context.prefix)
                    split_task_no += 1
            return None

        if submit_job:
            run_split_submit(config_file, struct_file, self.spec.mode)
            print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
            return 0
        return None

    def _advance_post_stage(
        self,
        context: PhononStepContext,
        workflow: Dict[str, object],
        config_file: str,
        struct_file: Optional[str],
        run_calc: bool,
        submit_job: bool,
    ) -> Optional[int]:
        if run_calc:
            if merge_state(workflow) != "finished":
                print_stage(4, "执行 merge")
                if run_merge_step(config_file, struct_file, workflow_name=self.spec.workflow_name) != 0:
                    print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
                    return 1
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            run_process_phonon_steps(context, include_plot=True)
            return None

        if submit_job:
            remaining_scripts = root_step_scripts(workflow, include_ph=False)
            process_needed = bool(remaining_scripts) or workflow.get("phonon_plot") != "finished"
            if merge_state(workflow) != "finished" or process_needed:
                aggregate = self._write_merge_and_process_stage_script(context)
                console.generated(os.path.basename(aggregate))
                self.submit_script(context.workspace, os.path.basename(aggregate))
                console.next_step("查看对应 slurm 输出，等待 4/11-11/11 完成")
                return 0
        return None


SPLIT_WORKFLOW_SPECS = {
    "index": SplitPhononWorkflowSpec(
        mode="index",
        workflow_name="qe_phonon_split_index",
        description="Quantum ESPRESSO 声子总工作流，按不可约 q 点编号拆分",
        prepare_split_step=prepare_split_assign_index_step,
    ),
    "coords": SplitPhononWorkflowSpec(
        mode="coords",
        workflow_name="qe_phonon_split_coords",
        description="Quantum ESPRESSO 声子总工作流，按不可约 q 点坐标拆分",
        prepare_split_step=prepare_split_assign_coords_step,
    ),
}


def create_split_phonon_workflow(
    mode: str,
    *,
    submit_script: SubmitScript = console.submit_script,
    split_run_shell: RunShell = run_shell,
    run_dyn0: RunDyn0 = run_ph_to_dyn0,
) -> SplitPhononWorkflow:
    return SplitPhononWorkflow(
        SPLIT_WORKFLOW_SPECS[mode],
        submit_script=submit_script,
        split_run_shell=split_run_shell,
        run_dyn0=run_dyn0,
    )
