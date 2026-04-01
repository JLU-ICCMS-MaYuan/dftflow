#!/usr/bin/env python3
import os
import sys
import argparse

try:
    import tomllib  # Python 3.11+
except ImportError:
    try:
        import toml
    except ImportError:
        toml = None

def parse_args():
    parser = argparse.ArgumentParser(description="生成 LOBSTER 输入文件并准备后处理目录")
    parser.add_argument('-e', '--COHPEnergy', nargs=2, type=float, default=[-10.0, 5.0], help="COHP 起始终止能量, 例如 -e -10 5")
    parser.add_argument('-s', '--species_custom', nargs=2, type=str, required=True, help="元素对, 例如 -s Si O")
    parser.add_argument('-z', '--zvalances', nargs="+", type=str, required=True, help="元素及其价电子轨道, 例如 -z Si:3s,3p O:2s,2p")
    parser.add_argument('-d', '--d_limit', nargs=2, type=float, required=True, help="原子间最小最大距离, 例如 -d 1.0 2.5")
    parser.add_argument('-m', '--mode', type=int, default=5, help="选择生成的输入文件版本: 5 或 0 (默认 5)")
    parser.add_argument('--custom_basis', nargs="+", type=str, help="使用自定义基组, 例如 --custom_basis H:h.sto La:la.sto")
    parser.add_argument('-c', '--config', default="input.toml", help="配置文件路径 (默认为 input.toml)")
    return parser.parse_args()

def parse_zvalances(zvalances):
    zval_dict = {}
    for entry in zvalances:
        if ":" not in entry:
            print(f"错误: 价电子参数格式不对 '{entry}'。应为 Element:orb1,orb2")
            sys.exit(1)
        element, orbitals = entry.split(":")
        zval_dict[element] = ' '.join(orbitals.split(','))
    return zval_dict

def parse_custom_basis(custom_basis_args):
    if not custom_basis_args:
        return None
    basis_dict = {}
    for entry in custom_basis_args:
        element, sto_file = entry.split(":")
        basis_dict[element] = sto_file
    return basis_dict

def write_lobsterin(dirname, mode=5, COHPstartEnergy=None, COHPendEnergy=None, species_custom1=None, species_custom2=None, lower_d=None, upper_d=None, zval_dict=None, custom_basis_dict=None):
    lobsterin_path = os.path.join(dirname, "lobsterin")
    with open(lobsterin_path, "w") as f:
        f.write('COHPstartEnergy  {}\n'.format(COHPstartEnergy))
        f.write('COHPendEnergy    {}\n'.format(COHPendEnergy))

        if custom_basis_dict:
            f.write('useBasisSet custom\n')
            for element, sto_file in custom_basis_dict.items():
                f.write('customSTOforAtom {}  {}\n'.format(element, sto_file))
        else:
            f.write('usebasisset pbeVaspFit2015\n')

        if mode == 0:
            f.write('gaussianSmearingWidth 0.05\n')

        for element, orbitals in zval_dict.items():
            f.write('basisfunctions {} {}\n'.format(element, orbitals))
        
        f.write("cohpGenerator from {} to {} type {} type {} orbitalWise\n".format(lower_d, upper_d, species_custom1, species_custom2))

def main():
    args = parse_args()

    COHPstartEnergy = args.COHPEnergy[0]
    COHPendEnergy = args.COHPEnergy[1]
    species_custom1 = args.species_custom[0]
    species_custom2 = args.species_custom[1]
    lower_d = args.d_limit[0]
    upper_d = args.d_limit[1]
    mode = args.mode

    zval_custom = parse_zvalances(args.zvalances)
    custom_basis = parse_custom_basis(args.custom_basis)

    print("Parsed Z-Valences:")
    for element, orbitals in zval_custom.items():
        print(f"{element}: {orbitals}")

    dirs = "{}_{}_{}_{}".format(species_custom1, species_custom2, lower_d, upper_d)
    if not os.path.exists(dirs):
        os.makedirs(dirs)
        print(f"创建目录: {dirs}")

    # LOBSTER 需要的文件列表
    # 注意：CONTCAR 在非自洽后通常与 POSCAR 相同，但 LOBSTER 有时需要 CONTCAR
    files = ['WAVECAR', 'CONTCAR', 'KPOINTS', 'OUTCAR', 'POTCAR', 'vasprun.xml', 'POSCAR']
    for file in files:
        if os.path.exists(file):
            target = os.path.join(dirs, file)
            if os.path.exists(target):
                os.remove(target)
            os.symlink(os.path.abspath(file), target)
        else:
            # 如果没有 CONTCAR，尝试用 POSCAR 代替
            if file == 'CONTCAR' and os.path.exists('POSCAR'):
                 os.symlink(os.path.abspath('POSCAR'), os.path.join(dirs, 'CONTCAR'))
            else:
                print(f"警告: {file} 不存在，LOBSTER 可能运行失败。 সন")

    write_lobsterin(dirs, mode=mode, COHPstartEnergy=COHPstartEnergy, COHPendEnergy=COHPendEnergy, 
                   species_custom1=species_custom1, species_custom2=species_custom2, 
                   lower_d=lower_d, upper_d=upper_d, zval_dict=zval_custom, custom_basis_dict=custom_basis)

    # 读取配置文件生成 slurm 脚本
    config = {}
    config_path = args.config
    if os.path.exists(config_path):
        if 'tomllib' in globals() and tomllib:
            with open(config_path, "rb") as f:
                config = tomllib.load(f)
        elif 'toml' in globals() and toml:
            config = toml.load(config_path)
    else:
        print(f"警告: 找不到配置文件 {config_path}，将使用默认配置。")
    
    slurm_config = config.get("slurm", {})
    slurm_header = slurm_config.get("header", "#!/bin/bash")
    
    slurm_script_path = os.path.join(dirs, "slurm.sh")
    with open(slurm_script_path, "w") as f:
        f.write(slurm_header.strip() + "\n\n")
        f.write("lobster-5.1.0\n")
    os.chmod(slurm_script_path, 0o755)

    print(f"\nLOBSTER 输入文件和 slurm 脚本已在 {dirs} 准备就绪。 সন")
    print("\n提示: -------------------------------")
    print("1. 如果运行 LOBSTER 后得不到 COHP，请检查 NBANDS 是否足够大。 সন")
    print("2. 确保在 VASP 计算中设置了 LWAVE = .TRUE. 以生成 WAVECAR。 সন")
    print("3. 运行 LOBSTER 命令: cd {} && lobster-5.1.0".format(dirs))

if __name__ == "__main__":
    main()
