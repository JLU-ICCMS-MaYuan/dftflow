#!/usr/bin/env python3
"""
参考预言机：用纯 Python 复刻 QE 的 kpoint_grid + q_points（dyn0 写出），
对真实的 MgB2 dyn0 做字节级比对，证明算法与 E24.15 格式可精确复现 ph.x 输出。
Rust 实现是此算法的忠实移植（IEEE 双精度、运算次序一致）。
"""
import math
import os
import xml.etree.ElementTree as ET

XML = "/Users/mayuan/Codes/dftflow/test_auto/MgB2/MgB2_calc/qe_scf/tmp/Mg1B2.save/data-file-schema.xml"
REAL_DYN0 = "/Users/mayuan/Codes/dftflow/test_auto/MgB2/MgB2_calc/qe_phonon_nosplit/Mg1B2.dyn0"
EPS = 1.0e-5


def nint(x: float) -> int:
    # GFortran NINT: 半数远离零 (与 Rust f64::round() 一致)
    f = math.floor(abs(x) + 0.5)
    return f if x >= 0 else -f


def parse_xml(path: str):
    root = ET.parse(path).getroot()
    def vec(tag):
        el = root.find(f".//{tag}")
        vals = [float(v) for v in el.text.split()]
        return vals
    b1 = vec("b1"); b2 = vec("b2"); b3 = vec("b3")
    bg = [b1, b2, b3]  # bg[vector][component]
    rotations = []
    for sym in root.findall(".//symmetry"):
        rot = sym.find("rotation")
        vals = [int(round(float(v))) for v in rot.text.split()]
        matrix = [vals[0:3], vals[3:6], vals[6:9]]
        rotations.append(matrix)
    return bg, rotations


def kpoint_grid(bg, rotations, nk1, nk2, nk3, k1=0, k2=0, k3=0, time_reversal=True):
    nrot = len(rotations)
    nkr = nk1 * nk2 * nk3
    xkg = [[0.0, 0.0, 0.0] for _ in range(nkr + 1)]
    for i in range(1, nk1 + 1):
        for j in range(1, nk2 + 1):
            for k in range(1, nk3 + 1):
                n = (k - 1) + (j - 1) * nk3 + (i - 1) * nk2 * nk3 + 1
                xkg[n][0] = (i - 1) / nk1 + k1 / 2 / nk1
                xkg[n][1] = (j - 1) / nk2 + k2 / 2 / nk2
                xkg[n][2] = (k - 1) / nk3 + k3 / 2 / nk3
    equiv = list(range(nkr + 1))
    wkk = [0.0] * (nkr + 1)
    for nk in range(1, nkr + 1):
        if equiv[nk] == nk:
            wkk[nk] = 1.0
            for ns in range(nrot):
                s = rotations[ns]
                xkr = [0.0, 0.0, 0.0]
                for ii in range(3):
                    xkr[ii] = s[ii][0] * xkg[nk][0] + s[ii][1] * xkg[nk][1] + s[ii][2] * xkg[nk][2]
                    xkr[ii] = xkr[ii] - nint(xkr[ii])
                # t_rev == 0 for non-magnetic -> no flip
                for sign in ([1.0] if not time_reversal else [1.0, -1.0]):
                    xx = sign * xkr[0] * nk1 - 0.5 * k1
                    yy = sign * xkr[1] * nk2 - 0.5 * k2
                    zz = sign * xkr[2] * nk3 - 0.5 * k3
                    in_list = abs(xx - nint(xx)) <= EPS and abs(yy - nint(yy)) <= EPS and abs(zz - nint(zz)) <= EPS
                    if in_list:
                        ii = (nint(sign * xkr[0] * nk1 - 0.5 * k1 + 2 * nk1) % nk1) + 1
                        jj = (nint(sign * xkr[1] * nk2 - 0.5 * k2 + 2 * nk2) % nk2) + 1
                        kk = (nint(sign * xkr[2] * nk3 - 0.5 * k3 + 2 * nk3) % nk3) + 1
                        n = (kk - 1) + (jj - 1) * nk3 + (ii - 1) * nk2 * nk3 + 1
                        if n > nk and equiv[n] == n:
                            equiv[n] = nk
                            wkk[nk] += 1.0
                        elif not (equiv[n] != n or n < nk):
                            pass
    # order irreducible
    qpoints = []
    fact = 0.0
    for nk in range(1, nkr + 1):
        if equiv[nk] == nk:
            crystal = [xkg[nk][c] - nint(xkg[nk][c]) for c in range(3)]
            # cryst_to_cart: result[k] = sum_v bg[v][k] * crystal[v]
            cart = [0.0, 0.0, 0.0]
            for c in range(3):
                cart[c] = bg[0][c] * crystal[0] + bg[1][c] * crystal[1] + bg[2][c] * crystal[2]
            w = wkk[nk]
            fact += w
            qpoints.append({"crystal": crystal, "cart": cart, "weight": w})
    for q in qpoints:
        q["weight"] /= fact
    return qpoints


def format_e24_15(v: float) -> str:
    if v == 0.0:
        return f"{'0.000000000000000E+00':>24}"
    neg = v < 0
    av = abs(v)
    e = math.ceil(math.log10(av))
    m = av * (10.0 ** (-e))
    if m >= 1.0:
        m /= 10.0
        e += 1
    scaled = m * 1e15
    digits = nint(scaled)
    if digits >= 10 ** 15:  # carry
        digits = 10 ** 15
        e += 1
    digits_str = f"{int(digits):015d}"
    sign = "-" if neg else ""
    exp_sign = "-" if e < 0 else "+"
    mantissa = f"{sign}0.{digits_str}E{exp_sign}{abs(e):02d}"
    return f"{mantissa:>24}"


def write_dyn0(qpoints, nk1, nk2, nk3, prefix, out_dir):
    lines = []
    lines.append(f"{nk1:>4d}{nk2:>4d}{nk3:>4d}")
    lines.append(f"{len(qpoints):>4d}")
    for q in qpoints:
        lines.append(format_e24_15(q["cart"][0]) + format_e24_15(q["cart"][1]) + format_e24_15(q["cart"][2]))
    text = "\n".join(lines) + "\n"
    path = os.path.join(out_dir, f"{prefix}.dyn0")
    with open(path, "w") as f:
        f.write(text)
    return path, text


def main():
    bg, rotations = parse_xml(XML)
    qpoints = kpoint_grid(bg, rotations, 2, 2, 1, 0, 0, 0, time_reversal=True)
    out_dir = "/tmp/validate_dyn0"
    os.makedirs(out_dir, exist_ok=True)
    path, text = write_dyn0(qpoints, 2, 2, 1, "Mg1B2", out_dir)

    with open(REAL_DYN0, "rb") as f:
        real = f.read()
    with open(path, "rb") as f:
        mine = f.read()

    print("generated:")
    print(text)
    print("real:")
    print(real.decode())
    if real == mine:
        print("BYTE-EXACT MATCH ✅")
    else:
        print("MISMATCH ❌")
        # show line diffs
        rl = real.decode().splitlines()
        ml = text.splitlines()
        for i in range(max(len(rl), len(ml))):
            a = rl[i] if i < len(rl) else "<none>"
            b = ml[i] if i < len(ml) else "<none>"
            if a != b:
                print(f"  line {i+1}: real={a!r} mine={b!r}")


if __name__ == "__main__":
    main()
