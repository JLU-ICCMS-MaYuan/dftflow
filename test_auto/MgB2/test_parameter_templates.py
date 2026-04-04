from pathlib import Path


def test_vasp_template_override_priority(chdir_work):
    from public.config_templates import load_config_file
    from vaspflow.templates import apply_vasp_config_overrides, get_vasp_scf_parameters_template

    config_path = Path("tmp_vasp_template.toml")
    config_path.write_text(
        """
[vasp]
executable_path = "mpirun -np 8 /path/to/vasp_std"

[vasp_scf]
ENCUT = 520
""".strip()
    )

    merged = apply_vasp_config_overrides(
        get_vasp_scf_parameters_template(),
        load_config_file(str(config_path)),
        ("vasp", "slurm", "potcar_dir", "chgcar_path", "wavecar_path", "vasp_scf"),
    )

    assert merged["vasp_scf"]["PREC"] == "Accurate"
    assert merged["vasp_scf"]["ENCUT"] == 520
    assert merged["vasp"]["executable_path"] == "mpirun -np 8 /path/to/vasp_std"


def test_qe_template_override_priority(chdir_work):
    from public.config_templates import load_config_file
    from qeflow.templates import apply_qe_config_overrides, get_qe_scf_parameters_template

    config_path = Path("tmp_qe_template.toml")
    config_path.write_text(
        """
[qe]
executable_path = "mpirun -np 8 /path/to/qe/bin"

[qe_scf.SYSTEM]
ecutwfc = 60.0
""".strip()
    )

    merged = apply_qe_config_overrides(
        get_qe_scf_parameters_template(),
        load_config_file(str(config_path)),
        ("qe", "slurm", "pseudo_map", "qe_scf"),
    )

    assert merged["qe_scf"]["k_points"]["kmesh"] == 0.04
    assert merged["qe_scf"]["SYSTEM"]["ecutwfc"] == 60.0
    assert merged["qe_scf"]["SYSTEM"]["ecutrho"] == 400.0


def test_wannier_template_override_priority(chdir_work):
    from public.config_templates import load_config_file
    from wannier90flow.templates import (
        apply_wannier_config_overrides,
        get_wannier_init_parameters_template,
    )

    config_path = Path("tmp_w90_template.toml")
    config_path.write_text(
        """
[wannier_init]
num_wann = 8

[wannier_init.projections]
list = ["Mg:s", "B:p"]
""".strip()
    )

    merged = apply_wannier_config_overrides(
        get_wannier_init_parameters_template(),
        load_config_file(str(config_path)),
        ("slurm", "wannier90", "pw2wannier90", "postw90", "wannier_init"),
    )

    assert merged["wannier_init"]["bands_plot"] is False
    assert merged["wannier_init"]["num_wann"] == 8
    assert merged["wannier_init"]["projections"]["list"] == ["Mg:s", "B:p"]
