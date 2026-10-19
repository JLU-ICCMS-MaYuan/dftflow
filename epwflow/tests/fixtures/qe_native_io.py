#!/usr/bin/python3
"""实际 QE 文件层级的独立进程替身；不计算物理量或产生科学参考。"""
import math, re, struct, sys
from pathlib import Path
import xml.etree.ElementTree as ET
ROOT = Path(__file__).resolve().parent
kind, *args = sys.argv[1:]
text = Path(args[args.index('-inp')+1]).read_text()
def value(name, default=None):
    m = re.search(r'\b'+name+r"\s*=\s*('(?:[^']|'')*'|[^,\n]+)", text, re.I)
    return m.group(1).strip().strip("'") if m else default
prefix = value('prefix')
out = Path(value('outdir'))
save = out/(prefix+'.save')
mode = (ROOT/'fixture-failure').read_text().strip() if (ROOT/'fixture-failure').exists() else ''
with (ROOT/'scientific-calls').open('a') as f: f.write(kind+'\n')
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def cross(a,b): return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def s(v): return ' '.join(format(x,'.17e') for x in v)
BOHR=.529177210903
if kind=='pw':
    cell = re.search(r'CELL_PARAMETERS[^\n]*\n([^\n]+)\n([^\n]+)\n([^\n]+)', text, re.I)
    lattice = [[float(x)/BOHR for x in cell.group(i).split()] for i in (1,2,3)]
    alat = math.sqrt(dot(lattice[0],lattice[0]))
    vol=dot(lattice[0],cross(lattice[1],lattice[2]))
    bg=[[alat*x/vol for x in cross(lattice[(i+1)%3],lattice[(i+2)%3])] for i in range(3)]
    at_block=re.search(r'ATOMIC_POSITIONS[^\n]*\n((?:\s*[A-Z][a-z]?\s+[^\n]+\n)+)',text)
    atoms=[]
    for line in at_block.group(1).splitlines():
        name,*xyz=line.split();xyz=list(map(float,xyz[:3])); atoms.append((name,[sum(xyz[i]*lattice[i][j] for i in range(3)) for j in range(3)]))
    nscf=value('calculation')=='nscf'
    points=[[0.,0.,0.,1.]];bands=2
    if nscf:
        assert (save/'charge-density.dat').read_bytes()==b'fixture-density', 'NSCF 未收到 SCF 电荷'
        block=re.search(r'K_POINTS\s+crystal\s*\n\s*(\d+)\s*\n(.*)',text,re.S|re.I)
        points=[list(map(float,l.split())) for l in block.group(2).splitlines()[:int(block.group(1))]]
        bands=int(value('nbnd'))
        if mode=='duplicate_k':points[-1]=points[0]
    save.mkdir(parents=True,exist_ok=True)
    root=ET.Element('espresso'); output=ET.SubElement(root,'output')
    conv=ET.SubElement(output,'convergence_info');ET.SubElement(conv,'convergence_achieved').text='true'
    structure=ET.SubElement(output,'atomic_structure',nat=str(len(atoms)),alat=str(alat))
    pos=ET.SubElement(structure,'atomic_positions')
    for name,xyz in atoms:ET.SubElement(pos,'atom',name=name).text=s(xyz)
    lat=ET.SubElement(structure,'cell')
    for i,row in enumerate(lattice,1):ET.SubElement(lat,'a'+str(i)).text=s(row)
    basis=ET.SubElement(output,'basis_set');ET.SubElement(basis,'fft_grid',nr1='2',nr2='2',nr3='2');rec=ET.SubElement(basis,'reciprocal_lattice')
    for i,row in enumerate(bg,1):ET.SubElement(rec,'b'+str(i)).text=s(row)
    energy=ET.SubElement(output,'total_energy');ET.SubElement(energy,'etot').text='-4' if mode=='energy' else '-5'
    band=ET.SubElement(output,'band_structure')
    for flag in ['lsda','noncolin','spinorbit']:ET.SubElement(band,flag).text='false'
    for name,v in [('nbnd',bands),('nks',len(points)),('fermi_energy',.1)]:ET.SubElement(band,name).text=str(v)
    for i,(x,y,z,w) in enumerate(points,1):
        cart=[x*bg[0][j]+y*bg[1][j]+z*bg[2][j] for j in range(3)]
        ks=ET.SubElement(band,'ks_energies');ET.SubElement(ks,'k_point',weight=str(w)).text=s(cart)
        ET.SubElement(ks,'eigenvalues',size=str(bands)).text=s([.01*j for j in range(bands)])
        (save/('wfc'+str(i)+'.dat')).write_bytes(b'fixture-wfc')
    if mode=='missing_wfc' and nscf:(save/('wfc'+str(len(points))+'.dat')).unlink()
    ET.ElementTree(root).write(save/'data-file-schema.xml')
    (save/'charge-density.dat').write_bytes(b'fixture-density')
    if not nscf:(out/'outside-save.wfc').write_bytes(b'fixture-outside-save-wfc')
    print('convergence has been achieved\n! total energy = -10.0 Ry\nJOB DONE.')
else:
    assert (save/'charge-density.dat').read_bytes()==b'fixture-density', 'PH 未收到 SCF 电荷'
    xml=ET.parse(save/'data-file-schema.xml').getroot().find('output')
    structure=xml.find('atomic_structure');alat=float(structure.attrib['alat']);lattice=[[float(x) for x in structure.find('cell').find('a'+str(i)).text.split()] for i in (1,2,3)]
    bg=[[float(x) for x in xml.find('basis_set/reciprocal_lattice').find('b'+str(i)).text.split()] for i in (1,2,3)]
    atoms=[(a.attrib['name'],[float(x) for x in a.text.split()]) for a in structure.find('atomic_positions')]
    modes=3*len(atoms)
    is_coords=value('ldisp')=='.false.'
    grid=[int(value('nq'+str(i),'1')) for i in (1,2,3)]
    points=[[0.,0.,0.],[.5*x for x in bg[0]]]
    full_points=points
    if is_coords:points=[[float(x) for x in text.strip().splitlines()[-1].split()]]
    phsave=out/('_ph0/'+prefix+'.phsave');phsave.mkdir(parents=True,exist_ok=True)
    control=ET.Element('Root');qp=ET.SubElement(control,'Q_POINTS');ET.SubElement(qp,'NUMBER_OF_Q_POINTS').text=str(len(points));ET.SubElement(qp,'MESH_DIMENSIONS').text=' '.join(map(str,grid));ET.SubElement(qp,'Q-POINT_COORDINATES').text='\n'.join(s(q) for q in points)
    ET.ElementTree(control).write(phsave/'control_ph.xml')
    if not is_coords:
        Path(prefix+'.dyn0').write_text(' '.join(map(str,grid))+'\n2\n'+'\n'.join(s(q) for q in points)+'\n')
    if value('only_init')!='.true.':
        indices=[int(value('start_q'))] if value('start_q') else list(range(1,len(points)+1))
        for index in indices:
            q=points[index-1]
            root=ET.Element('Root');geo=ET.SubElement(root,'GEOMETRY_INFO');ET.SubElement(geo,'NUMBER_OF_ATOMS').text=str(len(atoms));ET.SubElement(geo,'AT').text=s([v/alat for row in lattice for v in row])
            for i,(name,xyz) in enumerate(atoms,1):ET.SubElement(geo,'ATOM.'+str(i),SPECIES=name,TAU=s([v/alat for v in xyz]))
            dyn=ET.SubElement(root,'DYNAMICAL_MAT_.1');ET.SubElement(dyn,'Q_POINT').text=s([.1,0,0] if mode=='wrong_q' else q)
            for i in range(1,len(atoms)+1):
                for j in range(1,len(atoms)+1):ET.SubElement(dyn,'PHI.'+str(i)+'.'+str(j)).text=s([0.]*18)
            freq=ET.SubElement(root,'FREQUENCIES_THZ_CMM1')
            for i in range(1,modes+1):ET.SubElement(freq,'OMEGA.'+str(i)).text='1 100'
            ET.ElementTree(root).write(prefix+'.dyn'+('' if is_coords else str(index))+'.xml')
            pat=ET.Element('Root');info=ET.SubElement(pat,'IRREPS_INFO');ET.SubElement(info,'QPOINT_NUMBER').text=str(index);ET.SubElement(info,'NUMBER_IRR_REP').text=str(modes)
            for m in range(modes):
                rep=ET.SubElement(info,'REPRESENTION.'+str(m+1));ET.SubElement(rep,'NUMBER_OF_PERTURBATIONS').text='1';pert=ET.SubElement(rep,'PERTURBATION.1');vec=[0.]*(2*modes);vec[2*m]=1.
                ET.SubElement(pert,'DISPLACEMENT_PATTERN').text=s(vec)
            ET.ElementTree(pat).write(phsave/('patterns.'+str(index)+'.xml'))
            dv_dir=out/'_ph0'
            if any(abs(x)>1e-9 for x in q):dv_dir=dv_dir/(prefix+'.q_'+str(index))
            dv_dir.mkdir(parents=True,exist_ok=True)
            data=struct.pack('<d',.001)*(8*modes*2)
            if mode=='truncated_dvscf':data=data[:-8]
            (dv_dir/(prefix+'.dvscf1')).write_bytes(data)
    # 修改独立副本，测试必须证明快照与其他任务保存树没有被更改。
    (save/'stage-local-marker').write_text(str(Path.cwd()))
    print('JOB DONE.')
