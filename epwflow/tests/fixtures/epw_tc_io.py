#!/usr/bin/python3
"""两模式原生 IO 替身；边界数值只验证搜索与交付，不能作为科学 Tc。"""
import sys,re,math,pathlib,struct,json
text=pathlib.Path(sys.argv[-1]).read_text()
def value(name):
    return re.search(r'(?m)^\s*'+re.escape(name)+r'\s*=\s*([^\n]+)',text).group(1).strip()
prefix=value('prefix').strip("'")
t=float(value('temps(1)'))
assert value('nstemp')=='1'
assert value('imag_read')=='.false.'
assert value('epwread')=='.true.' and value('epbread')=='.false.'
assert value('epwwrite')=='.false.' and value('wannierize')=='.false.'
assert value('fila2f')=="'source.a2f'"
assert pathlib.Path('source.a2f').is_file(), '必须重读已有谱，不能重算谱'
iso=value('liso')=='.true.'
linear=value('tc_linear')=='.true.'
kb=1.380649e-23/1.602176634e-19
nf=int(.5*(float(value('wscut'))/(math.pi*kb*t)-1))+1
print('Program EPW v.5.9 starts')
boundary=32 if iso else 36
with (pathlib.Path(__file__).resolve().parent/'tc-calls.jsonl').open('a') as f:
    f.write(json.dumps({'mode':'iso' if iso else 'aniso','temperature_k':t,'linear':linear,'work':str(pathlib.Path.cwd())})+'\n')
# 测试自有标志模拟暂态进程失败，不改动冻结的程序与输入身份。
if iso and ((pathlib.Path(__file__).resolve().parent/'tc-failure').exists() or (t==30 and (pathlib.Path(__file__).resolve().parent/'gap-fail30').exists())):
    print('fixture transient failure', file=sys.stderr)
    sys.exit(17)
energies=[]
if not iso:
    if '--engine-fixture-states' in sys.argv:
        # 旧 Engine 单测显式使用两态映射；完整入口必须读取实际 binary egnv。
        energies=[-.01,.02]
    else:
        data=(pathlib.Path(value('outdir').strip("'"))/(prefix+'.ephmat')/'egnv').read_bytes()
        nks,nx,ny,nz,nk,bands=struct.unpack_from('<6i',data,0)
        ef,ef0,dos,smear,thick=struct.unpack_from('<5d',data,24)
        offset=64
        for _ in range(nk):
            weight,x,y,z=struct.unpack_from('<4d',data,offset);offset+=32
            ik,ix=struct.unpack_from('<2i',data,offset);offset+=8
            for energy in struct.unpack_from('<'+'d'*bands,data,offset):
                if abs(energy-ef0)<thick:energies.append((energy-ef0)*13.605693122994)
            offset+=8*bands
        assert offset==len(data) and energies, '原生 egnv 记录不完整'

if linear:
    print('Start: Solving (isotropic) linearized Eliashberg equation with solver = lapack')
    print(f'For the first Temp. {t:.2f} K')
    print(f'Total number of frequency points nsiw(1) = {nf}')
    print('Temp. Max. nsiw wscut Nr. of iters')
    rho=1.2 if t<boundary else .8
    cut=(2*nf+1)*math.pi*kb*t
    print(f'{t:.2f} {rho:.7f} {nf} {cut:.4f} 1')
    print('Finish: Solving (isotropic) linearized Eliashberg equation')
else:
    print(f'temp(1) = {t:.5f} K')
    print('Solve '+('isotropic' if iso else 'anisotropic')+' Eliashberg equations on imaginary-axis')
    print(f'Total number of frequency points nsiw(1) = {nf}')
    d=2.0 if t<boundary else 1e-6
    print('iter ethr znormi deltai [meV]')
    print(f'1 1e-2 1.5 {d}')
    print(f'2 1e-10 1.5 {d}')
    fault=(pathlib.Path(__file__).resolve().parent/'gap-fault')
    fault=fault.read_text().strip() if fault.exists() else ''
    print('Convergence was not reached' if fault=='unconverged' else 'Convergence was reached in nsiter = 2')
    name=prefix+'.imag_'+('iso' if iso else 'aniso')+f'_{t:06.2f}'
    with open(name,'w') as f:
        f.write('w [eV] znorm(w) delta(w) [eV]\n' if iso else '# w [eV] Enk-Ef [eV] znorm(w) delta(w) [eV]\n')
        for n in range(nf):
            w=(2*n+1)*math.pi*kb*t
            for e in ([None] if iso else energies):
                f.write(f'{w:.10e} '+('' if e is None else f'{e:.10e} ')+f'1.5 {d/1000:.10e}\n')
if not linear:
    if fault=='missing':pathlib.Path(name).unlink()
    if fault=='nan':pathlib.Path(name).write_text(pathlib.Path(name).read_text().replace('1.5 ', 'NaN '))
    if fault=='no-ending':sys.exit(0)
print('Total program execution')
print('EPW : 0.01s CPU 0.02s WALL')
