"""Command-line entry point for QE superconductivity post-processing."""

import argparse
import os

import numpy as np
from copy import deepcopy
from typing import Optional, Sequence

from .config import ScflowConfig, load_scflow_config
from .plotters import plot_article_view, plot_summary, plot_system
from .qe_outputs import Dyn, DynElph, Folder, PhOuts
from .sc_e import Superconducting
from .system import System
from .utils import (
    parse_formula,
    print_a2f,
    print_direct,
    print_tc,
    save_dict,
    save_result,
    save_structure,
    update_summary,
)



def eliashberg_solver_metadata(superconducting):
    """Distinguish the inherited search floor from a resolved transition."""
    return {
        "status": "resolution_limit" if superconducting.tc_e <= 0.001 else "resolved",
        "resolution_K": 0.001,
        "minimum_reported_Tc_K": 0.001,
        "matrix_size": superconducting.dim + 1,
    }


def run(config: ScflowConfig) -> None:
    os.makedirs(config.results_dir, exist_ok=True)

    folder = Folder(str(config.work_dir))
    dyn_elphs = [DynElph(path) for path in folder.dyn_elph_paths]
    dyns = [Dyn(path) for path in folder.dyn_paths] if folder.dyn_paths else []

    if folder.ph_outs_paths:
        ph_outs = PhOuts(folder.ph_outs_paths)
    else:
        print("WARNING: Found no ph.out file(s). The structure will not be read and written.")
        ph_outs = []

    if ph_outs:
        structure = ph_outs.struc()
        save_structure(structure, config.structure_tolerance, str(config.results_dir))
    else:
        structure = "Unknown"

    formula = "Unknown" if isinstance(structure, str) else parse_formula(structure)

    if dyns and len(folder.dyn_paths) == len(folder.dyn_elph_paths):
        print("Everything's ok with the weights. They will be used from *.dyn* files.")
        q_points = [dyn.q_point for dyn in dyns]
        weights = [dyn.weight for dyn in dyns]
        weights_sum = sum(weights)
        weights = [(weight / weights_sum) for weight in weights]
        weights = list(zip(q_points, weights))
    else:
        if dyns:
            print(
                "WARNING: The number of *.dyn* files is not equal to the number of "
                "*.dyn*.elph* files. Weights from *.dyn* files won't be used."
            )
        weights = []

    if not weights and ph_outs:
        print("Trying to get weights out of ph.out ...")
        weights = ph_outs.weights()
        if len(weights) != len(folder.dyn_elph_paths):
            print(
                "WARNING: Number of weights found in ph.out is not equal to the number "
                "of *.dyn*.elph* files. Weights from ph.out won't be used."
            )
            weights = []
        else:
            print("Everything's ok with the weights. They will be used from ph.out files.")

    if not weights:
        print("WARNING: Weights are not found. They will be set to equal values for all q-points")
        q_points = [dyn_elph.q_point for dyn_elph in dyn_elphs]
        uniform_weight = 1 / len(q_points)
        weights = [(q_point, uniform_weight) for q_point in q_points]

    smoothing = int(config.smoothing_thz) if config.smoothing_thz.is_integer() else config.smoothing_thz
    sigma = int(config.gaussian_sigma) if config.gaussian_sigma.is_integer() else config.gaussian_sigma

    lengths = [dyn_elph.length for dyn_elph in dyn_elphs]
    if not lengths or not all(length == lengths[0] for length in lengths):
        raise ValueError("所有 *.dyn*.elph* 文件必须包含相同数量的展宽区段")
    length = lengths[0]
    if length == 1:
        print("Assuming a Tetrahedron method")
    else:
        print("Assuming a Gaussian Broadening method with broadenings:")
        headers = dyn_elphs[0].headers
        print(headers)
        summary = {}

    for index in range(length):
        if length > 1:
            print(f"\nBroadening: {headers[index]}")
            broadening = float(headers[index])
        else:
            broadening = 0

        selected_dyn_elphs = deepcopy(dyn_elphs)
        for dyn_elph in selected_dyn_elphs:
            dyn_elph.lambdas = dyn_elph.lambdas[index]
            dyn_elph.gammas = dyn_elph.gammas[index]
            dyn_elph.DOS = dyn_elph.DOS[index]
            dyn_elph.E_F = dyn_elph.E_F[index]

        zero_modes = []
        for path, dyn_elph in zip(folder.dyn_elph_paths, selected_dyn_elphs):
            for mode in np.flatnonzero(dyn_elph.sqr_freqs == 0):
                zero_modes.append({
                    "file": path, "q_point": list(dyn_elph.q_point),
                    "mode_1based": int(mode) + 1,
                    "lambda": float(dyn_elph.lambdas[mode]),
                    "gamma_GHz": float(dyn_elph.gammas[mode]),
                })
        if zero_modes:
            print(f"WARNING: {len(zero_modes)} exact-zero modes: excluding only if lambda=0; "
                  "original gamma is recorded in result.json")
        system = System(selected_dyn_elphs, weights)
        system.get_direct(smoothing)
        print_direct(system)
        system.get_a2f(config.resolution, sigma, config.integration)
        print_a2f(system)
        system.get_tc(config.mu)
        print_tc(system)

        result_dir = config.results_dir / str(headers[index]) if length > 1 else config.results_dir
        os.makedirs(result_dir, exist_ok=True)
        result_dir_text = str(result_dir)
        save_dict(system, result_dir_text, b=broadening)
        plot_system(system, formula, result_dir_text, b=broadening)
        plot_article_view(system, formula, result_dir_text)

        superconducting = Superconducting(system.a2f)
        superconducting.get_tc_e(config.mu)
        nef = selected_dyn_elphs[0].DOS
        print("nef: ", nef)
        result = superconducting.get_all(system, nef, structure)
        result["eliashberg_solver"] = eliashberg_solver_metadata(superconducting)
        result["zero_frequency_handling"] = {
            "policy": "exclude_exact_zero_modes_only_if_lambda_is_zero",
            "modes": zero_modes,
        }
        if broadening > 0:
            result.update({"Broadening, Ry": broadening})
        save_result(result, result_dir_text)

        if length > 1:
            summary = update_summary(
                summary,
                result,
                headers[index],
                formula,
                smoothing,
                config.mu,
                str(config.results_dir),
            )
            plot_summary(
                summary,
                formula,
                smoothing,
                sigma,
                config.resolution,
                config.mu,
                str(config.results_dir),
            )


def main(argv: Optional[Sequence[str]] = None) -> None:
    parser = argparse.ArgumentParser(description="读取 QE EPC 输出并计算超导转变温度")
    parser.add_argument("-c", "--config", default="input.toml", help="scflow 配置文件路径")
    args = parser.parse_args(argv)
    run(load_scflow_config(args.config))


if __name__ == "__main__":
    main()
