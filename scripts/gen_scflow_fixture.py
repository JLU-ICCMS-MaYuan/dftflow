#!/usr/bin/env python3
"""生成合成 QE EPC（dyn*.elph + ph.out）样例，供 Python 参考 scflow 与 Rust scflow 对照。"""
import os
import sys

out_dir = sys.argv[1] if len(sys.argv) > 1 else "/tmp/scfix/work"
os.makedirs(out_dir, exist_ok=True)

# 六方 MgB2 型结构（P6_3/mmc）
alat = 5.8316  # bohr
a = 3.086      # Å
c = 3.524
c_alat = c / a * (a / (alat * 0.529177249))  # c in alat units = c/(alat*bohr->A)
c_alat = c / (alat * 0.529177249)

ph_out = f"""     Program PHONON v.7.2 starts
     lattice parameter (alat)  =  {alat:.4f}  (a.u.)
     a(1) = (   {alat:.4f}   0.0000   0.0000 )
     a(2) = (  {-alat/2:.4f}   {alat*0.8660254:.4f}   0.0000 )
     a(3) = (   0.0000   0.0000   {alat*c_alat:.4f} )
     site n.     atom          positions (alat units)
          1           Mg  tau(   1) = (   0.0000   0.0000   0.0000 )
          2           B   tau(   2) = (   0.0000   0.5774   {c_alat/2:.4f} )
          3           B   tau(   3) = (   0.0000  -0.5774  {c_alat/2:.4f} )
     number of atoms/cell      =           3
     number of k points=     4                       method=MP
     Number of q in the star:  1
     (star of q points)
                            q = ( 0.0000000 0.0000000 0.0000000
     Number of q in the star:  2
     (star of q points)
                            q = ( 0.0179539 0.0000000 0.0000000
     END OF PHONON
"""
with open(os.path.join(out_dir, "MgB2.ph.out"), "w") as f:
    f.write(ph_out)

# 两个 q 点，每点 6 个模式；3 个展宽区段
q_points = [
    (0.0000000, 0.0000000, 0.0000000),
    (0.0179539, 0.0000000, 0.0000000),
]
weights = [1, 2]
broadenings = [0.01, 0.02, 0.04]
n_modes = 6

# 合成平方频率（Ry²）与 λ、γ（GHz）：确定性公式，量级接近真实 MgB2
base_freqs_thz = [0.0, 3.0, 4.2, 5.5, 7.0, 8.8]  # 第一个为（近）零频声学支
for iq, (q, w) in enumerate(zip(q_points, weights), start=1):
    lines = [f"  {q[0]:.7f}  {q[1]:.7f}  {q[2]:.7f}"]
    sqr_values = []
    for m in range(n_modes):
        f_thz = base_freqs_thz[m] * (1.0 + 0.05 * iq) if base_freqs_thz[m] > 0 else 0.0
        sqr = (f_thz / 3289.84367720502) ** 2
        sqr_values.append(sqr)
    # QE 原生格式：平方频率为纯数字行（每行一个值）
    for sqr in sqr_values:
        lines.append(f"  {sqr:.12f}")
    for b in broadenings:
        lines.append(f"  Gaussian Broadening {b:.3f} Ry")
        lines.append("   DOS =     3.14159265 states/spin/Ry Ef=  6.500000eV")
        for m in range(n_modes):
            lam = 0.9 * (1.0 - 0.12 * m) * (1.0 - 0.1 * iq) / (m + 1)
            if m == 0:
                lam = 0.0  # 零频模式 λ=0（exact-zero 规则）
            gamma = 120.0 * (m + 1) * (1.0 + 0.2 * b) * (1.0 - 0.1 * iq)
            lines.append(f"   mode #{m+1:4d}: lambda= {lam:.6f}  gamma= {gamma:.3f} (GHz)")
    with open(os.path.join(out_dir, f"MgB2.dyn{iq}.elph"), "w") as f:
        f.write("\n".join(lines) + "\n")

print(f"synthetic EPC fixtures written to {out_dir}")
