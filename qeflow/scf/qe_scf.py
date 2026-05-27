#!/usr/bin/env python3
from public.job import handle_job_action
from qeflow.qe_input.cli_adapter import run_qe_calculation_cli


def main() -> None:
    run_qe_calculation_cli("scf", "Quantum ESPRESSO SCF Setup Script", action_handler=handle_job_action)


if __name__ == "__main__":
    main()
