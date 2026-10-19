#!/usr/bin/python3
"""独立进程协议替身；仅检验原生布局和执行顺序，不作为科学参考。"""
# 能力门的测试标记，仅用于 IO 契约：dis_proj_min dis_proj_max dis_froz_proj
import json, math, re, struct, sys
from functools import reduce
from operator import mul
def product(v):return reduce(mul,v,1)
from pathlib import Path
import xml.etree.ElementTree as ET
ROOT=Path(__file__).resolve().parent
assert sys.argv[1]=='--io-contract', 'Program.args 未传递'
text=Path(sys.argv[sys.argv.index('-inp')+1]).read_text()
def value(name):
    return re.search(r'(?m)^\s*'+re.escape(name)+r"\s*=\s*('(?:[^']|'')*'|[^\n]+)",text).group(1).strip().strip("'")
prefix=value('prefix');out=Path(value('outdir'));n=int(value('nbndsub'))
mode=(ROOT/'epw-failure').read_text().strip() if (ROOT/'epw-failure').exists() else ''
HA=27.211386245988;RY=13.605693122994
coarse=value('epwread')=='.false.'
reread=not coarse and 'band_plot = .true.' in text
with (ROOT/'epw-calls').open('a') as f:f.write(('coarse' if coarse else 'reread' if reread else 'fine')+' '+str(Path.cwd())+'\n')
def integer(v):return struct.pack('<'+'i'*len(v),*v)
def real(v):return struct.pack('<'+'d'*len(v),*v)
def bands(name, points, vals):
    Path(name).write_text(f'&plot nbnd= {len(vals)}, nks= {len(points)} /\n'+''.join(' '.join(map(str,q))+'\n'+' '.join(map(str,vals))+'\n' for q in points))
if coarse or reread:
    if coarse:
        assert (out/(prefix+'.save')/'charge-density.dat').read_bytes()==b'fixture-density'
        assert (Path(value('dvscf_dir'))/(prefix+'.dvscf_q2')).is_file(), 'PH 未交付'
    else:
        assert not (out/(prefix+'.save')).exists(), '重读不需要 QE 保存树'
        assert value('wannierize')=='.false.' and 'dis_froz_max' not in text
    xml=ET.parse((out/(prefix+'.save') if coarse else ROOT/'run/nscf/out'/ (prefix+'.save'))/'data-file-schema.xml').getroot()
    kpoints=[[float(x) for x in node.text.split()] for node in xml.findall('output/band_structure/ks_energies/k_point')]
    coarse_nk=len(kpoints)
    bg=[[float(x) for x in xml.find('output/basis_set/reciprocal_lattice/b'+str(i)).text.split()] for i in (1,2,3)]
    def cart(c):return [sum(c[i]*bg[i][j] for i in range(3)) for j in range(3)]
    if reread:
        kpoints=[cart(list(map(float,l.split()[:3]))) for l in Path('kpoints.dat').read_text().splitlines()[1:]]
    qcr=[list(map(float,l.split()[:3])) for l in Path('qpoints.dat').read_text().splitlines()[1:]]
    qpoints=list(map(cart,qcr))
    modes=6
    Path('epwdata.fmt').write_text('0.2\n'+f'{n} 1 {modes} 1 1\n'+'0\n'*27+'(0,0)\n'*(n*n+modes*modes))
    Path('crystal.fmt').write_text('2\n6\nfixture-IO\n')
    Path('wigner.fmt').write_text('1 1 1 1 1\n'+('0 0 0 0\n1\n')*3)
    Path('vmedata.fmt').write_text('(0,0)\n'*(6*n*n));Path('dmedata.fmt').write_text('(0,0)\n'*(3*n*n))
    (out/(prefix+'.epmatwp')).write_bytes(real([0.]*(2*n*n*modes)))
    if mode=='coarse-truncated':(out/(prefix+'.epmatwp')).write_bytes(b'broken')
    wout='Using band disentanglement : T\n<<< Disentanglement convergence criteria satisfied >>>\nCycle: 14\n<<< Wannierisation convergence criteria satisfied >>>\nFinal State\n'
    wout+=''.join(f'WF centre and spread {i} ( 0 0 0 ) 1.25\n' for i in range(1,n+1))
    if coarse:
        wout+=f'| Outer: {value("dis_win_min")} to {value("dis_win_max")} (eV)\n'
        wout+=f'| Inner: {value("dis_froz_min")} to {value("dis_froz_max")} (eV)\n'
    Path(prefix+'.wout').write_text(wout)
    kept=list(range(8,17));nk=coarse_nk
    Path(prefix+'.ukk').write_text('9 7\n'+'\n'.join(map(str,kept))+'\n'+'(0,0)\n'*(9*n*nk)+'T\n'*(9*nk)+'T\n'*7+'F\n'*9+'0 0 0\n'*n)
    if mode=='coarse-ukk':Path(prefix+'.ukk').write_text('9 7\n8\n')
    bands('band.eig',kpoints,[HA*.01*i for i in range(8,13)])
    bands('phband.freq',qpoints,[100/8.065543937+(20 if mode=='quality' else 0)]*6)
else:
    assert value('nstemp')=='1' and float(value('temps'))==5., '未显式采用固定温度，EPW 会默认 300 K'
    assert Path(prefix+'.ukk').is_file(), 'epwread 仍须 loadbm 读取原 ukk'
    assert Path('epwdata.fmt').exists() and (out/(prefix+'.epmatwp')).stat().st_size==16*n*n*6
    assert not Path('save').exists(), '细网格应重读粗网格，无需复制响应势'
    assert not Path('selecq.fmt').exists(), '不能复用粗网格验证点选择'
    if mode=='fine-exit' or (mode=='fine-second-exit' and Path.cwd().name=='smearing-0002'):print('fine failed evidence');sys.exit(17)
    matrix=out/(prefix+'.ephmat');matrix.mkdir()
    grid=[int(value('nkf'+str(i))) for i in (1,2,3)];qgrid=[int(value('nqf'+str(i))) for i in (1,2,3)]
    e=integer([2,*grid,2,n])+real([0.2,0.2,2.,float(value('degaussw'))/RY,float(value('fsthick'))/RY])
    for k in (1,2):e+=real([.5,0,0,0])+integer([k,k])+real([.2]*n)
    (matrix/'egnv').write_bytes(e)
    nq=product(qgrid);freq=integer([nq,*qgrid,6])+real([0.,0.,0.,.001,.002,.003,.004,.005,.006])*nq
    (matrix/'freq').write_bytes(freq);(matrix/'npool').write_bytes(integer([1]));(matrix/'ikmap').write_bytes(integer([1]*(2*product(grid))))
    (matrix/'ephmat1').write_bytes(integer([1,2])+real([.001]))
    if mode=='fine-truncated':(matrix/'ephmat1').write_bytes(integer([1,2]))
    selected=nq-1
    Path('restart.fmt').write_text(str(selected)+'\n0\n0\n1\n1\n0\n')
    Path('selecq.fmt').write_text(f'{selected}\n{nq}\n'+ ' '.join(map(str,range(1,selected+1)))+'\n')
    if mode=='fine-selection':Path('selecq.fmt').write_text(f'{selected}\n{nq+1}\n'+ ' '.join(map(str,range(1,selected+1)))+'\n')
    scale=float(value('degaussq'))
    samples=int(value('nqstep'))
    a2f='w[meV] a2f and integrated 2*a2f/w for 1 smearing values\n'
    for i in range(1,samples+1):a2f+=f'{i} {i*scale} {2*i*scale}\n'
    a2f+=f'Integrated el-ph coupling\n # {2*samples*scale}\nSummed el-ph coupling {2*samples*scale}\n'
    Path(prefix+'.a2f').write_text(a2f)
print(f'Program EPW v.5.9 starts\nibndmin = 1 ebndmin = -1.0 eV\nibndmax = {n} ebndmax = 1.0 eV\nTotal program execution\nEPW : 1s CPU 2s WALL')
