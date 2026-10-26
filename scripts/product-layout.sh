# DFTFlow 产品与兼容入口的唯一清单，供构建、发布和验证复用。
dftflow_main_products=(qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity epw_clear)
dftflow_all_products=("${dftflow_main_products[@]}" qf-gen-dyn0)

dftflow_module() {
    case "$1" in
        epw_phonon|epw_wannier|epw_superconductivity|epw_clear) printf '%s\n' epwflow ;;
        wannier90) printf '%s\n' wannierflow ;;
        substitution) printf '%s\n' substitutionflow ;;
        qf-gen-dyn0) printf '%s\n' tools ;;
        qeflow|vaspflow|scflow) printf '%s\n' "$1" ;;
        *) return 1 ;;
    esac
}

dftflow_entries() {
    printf '%s\n' "$1"
    case "$1" in
        qeflow) printf '%s\n' qe_scf qe_nscf qe_eband qe_eledos qe_process_eband qe_process_eledos \
            qe_ph_nosplit qe_merge qe_q2r qe_matdyn_phband qe_matdyn_phdos qe_process_phonon \
            qe_split_submit qe_check qe_phonon_nosplit qe_phonon_split_index qe_phonon_split_coords ;;
        vaspflow) printf '%s\n' vasp_scf vasp_scf_converge vasp_eband vasp_process_eband \
            vasp_eledos vasp_process_eledos vasp_idos_computed vasp_fermi vasp_cohp_uniform \
            vasp_cohp_fatband vasp_lobsterin vasp_plot_cohp vasp_plot_kcohp vasp_locpot \
            vasp_process_locpot vasp_charge vasp_charge_diff vasp_opt vasp_elf ;;
        wannier90) printf '%s\n' wannier_init wannier_post wannier_window wannier_plot ;;
        substitution) printf '%s\n' substitution_structure substitution_continue substitution_prepare_potcar \
            substitution_prepare_qe_pseudo substitution_plot_eledos substitution_check_phonon_stability ;;
    esac
}

dftflow_select_products() {
    local item seen=','
    IFS=',' read -r -a dftflow_products <<< "$1"
    [[ ${#dftflow_products[@]} -gt 0 && "$1" != *, ]] || return 1
    for item in "${dftflow_products[@]}"; do
        dftflow_module "$item" >/dev/null || { echo "未知产品: $item" >&2; return 1; }
        [[ "$seen" != *",$item,"* ]] || { echo "重复产品: $item" >&2; return 1; }
        seen="$seen$item,"
    done
}
