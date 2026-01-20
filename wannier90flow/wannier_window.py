#!/usr/bin/env python3
import argparse
import xml.dom.minidom


def get_energies(xml_name):
    # VASP EIGENVAL format
    if xml_name == "EIGENVAL":
        with open("EIGENVAL", "r") as eigenval:
            raw_content = eigenval.readlines()
        content = raw_content[7:]
        entries = []
        for i, line in enumerate(content):
            if len(line.split()) == 4:
                entries.append(i)
        eng_full = []
        for i in entries:
            eng_ki = []
            for line in content[i+1:]:
                if len(line.split()) == 3:
                    eng_ki.append(float(line.split()[1]))
                else:
                    break
            eng_full.append(eng_ki)
    # Quantum ESPRESSO and FLEUR xml format
    else:
        Har2eV = 13.60569253 * 2
        dom = xml.dom.minidom.parse(xml_name)
        root = dom.documentElement
        eng_full = []
        if root.nodeName == "fleurOutput":
            eigenvalues = root.getElementsByTagName("eigenvalues")[-1]
            eks = eigenvalues.getElementsByTagName("eigenvaluesAt")
            eng_full = [[float(f) * Har2eV for f in ek.childNodes[0].data.split()] for ek in eks]
        elif root.nodeName == "qes:espresso":
            eigenvalues = root.getElementsByTagName("eigenvalues")
            eng_full = [[float(f) * Har2eV for f in ek.childNodes[0].data.split()] for ek in eigenvalues]
        else:
            raise RuntimeError("Unknown xml output")
    return eng_full


def build_parser():
    parser = argparse.ArgumentParser(
        description="解析能量窗口：模式 e 提取某条能带的能量范围；模式 n 统计能量区间内的能带数。"
    )
    parser.add_argument(
        "--xml",
        help="能量文件：EIGENVAL 或 QE/FLEUR XML（含能带信息）",
    )

    subparsers = parser.add_subparsers(dest="mode", required=True)

    parser_e = subparsers.add_parser(
        "-e",
        help="指定 band 序号输出该带的能量范围",
    )
    parser_e.add_argument(
        "band_index",
        type=int,
        help="能带序号（从 1 开始）",
    )

    parser_n = subparsers.add_parser(
        "-n",
        help="指定能量区间统计各 k 点包含的能带数",
    )
    parser_n.add_argument(
        "--emin",
        type=float,
        help="能量下限（eV）",
    )
    parser_n.add_argument(
        "--emax",
        type=float,
        help="能量上限（eV）",
    )
    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()

    eng_full = get_energies(args.xml)

    if args.mode == "e":
        # Band index is counted from 1, NOT 0.
        if args.band_index < 1:
            parser.error("band_index 必须从 1 开始")
        eng_selected = [eng[args.band_index - 1] for eng in eng_full]
        emin = min(eng_selected)
        emax = max(eng_selected)
        print(f"emin = {emin:.6f}")
        print(f"emax = {emax:.6f}")
    else:
        # Energies are in eV, not Hartree.
        emin = args.emin
        emax = args.emax
        nbnd = []
        for ik, ek in enumerate(eng_full):
            num_bands = 0
            for eng in ek:
                if emin <= eng <= emax:
                    num_bands += 1
            print(f"ik = {ik + 1}, nbnd = {num_bands}")
            nbnd.append(num_bands)
        print(min(nbnd), max(nbnd))


if __name__ == "__main__":
    main()
