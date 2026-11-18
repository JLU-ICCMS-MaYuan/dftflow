//! 三个公开程序与实际脚本的连续进程契约；全部数值为隔离替身，非科学验收。
#![cfg(unix)]
#[path = "support/native_fixture.rs"]
mod native_fixture;
#[path = "support/nscf_grid_contract.rs"]
mod nscf_grid_contract;
#[path = "support/explicit_programs.rs"]
mod explicit_programs;
use epwflow::foundation::{config, slurm::shell_quote, store};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};

const BINS: [&str; 3] = [
    env!("CARGO_BIN_EXE_epw_phonon"),
    env!("CARGO_BIN_EXE_epw_wannier"),
    env!("CARGO_BIN_EXE_epw_superconductivity"),
];
struct Case {
    root: PathBuf,
    preserve: bool,
}
impl Drop for Case {
    fn drop(&mut self) {
        if !self.preserve && !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
impl Case {
    fn new() -> Self {
        let retained = std::env::var_os("DFTFLOW_THREE_STAGE_TEST_ROOT");
        let preserve = retained.is_some();
        let parent = retained
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!("public-chain-{}", store::token()));
        fs::create_dir(&root).unwrap();
        eprintln!("Full access process evidence:{}", root.display());
        let c = Self { root, preserve };
        c.put(
            "POSCAR",
            "Si\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n2\nDirect\n0 0 0\n0.25 0.25 0.25\n",
        );
        c.put("KPATH.in", "0 0 0 G\n0.5 0 0 X\n");
        c.put("qe_native_io.py", include_str!("fixtures/qe_native_io.py"));
        c.put(
            "epw_native_io.py",
            include_str!("fixtures/epw_native_io.py"),
        );
        c.put("epw_tc_io.py", include_str!("fixtures/epw_tc_io.py"));
        for kind in ["pw", "ph", "q2r", "matdyn"] {
            c.script(
                &format!("{kind}.x"),
                &format!(
                    "#!/bin/sh\nexec /usr/bin/python3 {} {kind} \"$@\"\n",
                    shell_quote(c.root.join("qe_native_io.py").to_str().unwrap())
                ),
            );
        }
        c.script("epw.x", &format!(r#"#!/usr/bin/python3
import io,pathlib,re,runpy,sys
root=pathlib.Path({root})
text=pathlib.Path(sys.argv[sys.argv.index('-inp')+1]).read_text() if '-inp' in sys.argv else sys.stdin.read()
sys.stdin=io.StringIO(text)
if (root/'fail-coarse').exists() and re.search(r'(?m)^\s*epwread\s*=\s*\.false\.',text) and not re.search(r'(?m)^\s*wannierize\s*=\s*\.true\.',text):
    sys.exit(17)
assert not any(re.search(r'\b'+name+r'\s*=',text) for name in ['dis_proj_min','dis_proj_max','dis_froz_proj'])
name='epw_tc_io.py' if re.search(r'(?m)^\s*(?:liso|laniso)\s*=\s*\.true\.',text) else 'epw_native_io.py'
runpy.run_path(str(root/name),run_name='__main__')
"#, root=serde_json::to_string(c.root.to_str().unwrap()).unwrap()));
        c.script(
            "sacct",
            &format!(
                "#!/bin/sh\ncat {}\n",
                shell_quote(c.root.join("accounting").to_str().unwrap())
            ),
        );
        c.script("scontrol", include_str!("fixtures/slurm_controller.py"));
        c.script("squeue", "#!/bin/sh\nexit 0\n");
        c.script("sbatch", &format!("#!/bin/bash\nprintf '%s\\n' \"$*\" >> {calls}\nn=$(wc -l < {calls})\nprintf '%s\\n' \"$((200+n))\"\n", calls=shell_quote(c.root.join("submissions").to_str().unwrap())));
        for name in ["bands", "dos", "projwfc", "postw90"] {
            c.script(&format!("{name}.x"), "#!/bin/sh\nexit 91\n");
        }
        let mut cfg: config::Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
        cfg.work_dir = c.root.join("run");
        cfg.outdir = c.root.join("scratch");
        cfg.structure = c.root.join("POSCAR");
        cfg.kpath = Some(c.root.join("KPATH.in"));
        cfg.pseudos = [("Si".into(), cfg.structure.clone())].into();
        cfg.atomic_masses.clear();
        cfg.programs.launcher.clear();
        cfg.programs.environment_script = None;
        let programs = &mut cfg.programs;
        for (program, name) in [
            (&mut programs.pw, "pw.x"),
            (&mut programs.ph, "ph.x"),
            (&mut programs.epw, "epw.x"),
        ] {
            program.path = c.root.join(name);
            program.args.clear();
        }
        cfg.programs.epw.args = vec!["--io-contract".into()];
        let programs = &mut cfg.programs;
        for (program, name) in [
            (&mut programs.bands, "bands"),
            (&mut programs.dos, "dos"),
            (&mut programs.projwfc, "projwfc"),
            (&mut programs.q2r, "q2r"),
            (&mut programs.matdyn, "matdyn"),
            (&mut programs.postw90, "postw90"),
        ] {
            *program = Some(config::Program { path: c.root.join(format!("{name}.x")), args: vec![] });
        }

        cfg.programs.wannier90 = None;
        cfg.programs.pw2wannier90 = None;
        cfg.qe.prefix = "scf".into();
        cfg.qe.scf.k_grid = [2; 3];
        cfg.qe.nscf.k_grid = [2; 3];
        cfg.qe.nscf.nbnd = 16;
        cfg.phonon.mode = config::PhMode::Nosplit;
        cfg.phonon.q_grid = [2; 3];
        cfg.wannier.num_wann = 5;
        cfg.wannier.selection_mode = qf_wannier::SelectionMode::EnergyOnly;
        cfg.wannier.projections = vec!["Si:sp3".into()];
        cfg.wannier.energy_reference = config::EnergyReference::QeZero;
        cfg.wannier.exclude_bands = (1..=7).collect();
        cfg.wannier.outer_window_ev = Some([1.5, 5.]);
        cfg.wannier.frozen_window_ev = Some([2.2, 3.3]);
        cfg.wannier.frozen_max_ev = None;
        cfg.interpolation.k_grid = [4; 3];
        cfg.interpolation.q_grid = [2; 3];
        cfg.interpolation.spectral_points = 4;
        cfg.interpolation.phonon_smearings_mev = vec![0.5];
        cfg.comparison = Some(config::Comparison {
            points_per_edge: 2,
            ..Default::default()
        });
        cfg.superconductivity.mode = config::Mode::Both;
        cfg.superconductivity.frequency_cutoff_ev = 0.03;
        cfg.superconductivity.gap_temperatures_k = vec![5., 20.];
        let tc = cfg.superconductivity.iso.get_or_insert(config::Tc{min_k:5.,max_k:50.,precision_k:0.5,max_solves:100,eigen_margin:1e-5,normal_gap_mev:1e-4,super_gap_mev:1e-3});
        tc.min_k = 5.;
        tc.max_k = 50.;
        cfg.superconductivity.aniso = cfg.superconductivity.iso.clone();
        native_fixture::matrix(&mut cfg);
        cfg.validate().unwrap();
        c.put("inputepw.toml", &toml::to_string_pretty(&cfg).unwrap());
        c.accounting(None);
        c
    }
    fn put(&self, name: &str, text: &str) {
        fs::write(self.root.join(name), text).unwrap();
    }
    fn script(&self, name: &str, text: &str) {
        self.put(name, text);
        fs::set_permissions(self.root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn assert_scripts_without_time_limits(&self) {
        let mut pending = vec![self.root.join("run")];
        let mut scripts = 0;
        while let Some(dir) = pending.pop() {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|e| e == "sh") {
                    scripts += 1;
                    let text = fs::read_to_string(&path).unwrap();
                    for line in text.lines() {
                        if let Some(options) = line.trim().strip_prefix("#SBATCH") {
                            assert!(!options.split_whitespace().any(|a| a.starts_with("--time") || a.starts_with("-t")), "{}: {line}", path.display());
                        }
                        assert!(!line.trim_start().starts_with("timeout ") && !line.contains("ulimit -t"), "{}: {line}", path.display());
                    }
                }
            }
        }
        assert!(scripts > 0);
        let submissions = fs::read_to_string(self.root.join("submissions")).unwrap_or_default();
        assert!(!submissions.split_whitespace().any(|a| a.starts_with("--time") || a == "-t"));
    }
    fn accounting(&self, active: Option<u64>) {
        self.put(
            "accounting",
            &(200..=220)
                .map(|id| {
                    format!(
                        "{id}|{}|1|1|\n",
                        if Some(id) == active {
                            "RUNNING"
                        } else {
                            "COMPLETED"
                        }
                    )
                })
                .collect::<String>(),
        );
    }
    fn environment(&self, cmd: &mut Command, job: Option<u64>) {
        cmd.current_dir(&self.root)
            .env("PATH", format!("{}:/usr/bin:/bin", self.root.display()))
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env("XDG_STATE_HOME", self.root.join("xdg"))
            .env_remove("SLURM_JOB_ID")
            .env_remove("SLURM_JOBID");
        if let Some(job) = job {
            cmd.env("SLURM_JOB_ID", job.to_string());
        }
    }
    fn output(&self, mut cmd: Command, job: Option<u64>, label: &str) -> Output {
        self.environment(&mut cmd, job);
        let o = cmd.output().unwrap();
        self.put(
            &format!("{label}.stdout"),
            &String::from_utf8_lossy(&o.stdout),
        );
        self.put(
            &format!("{label}.stderr"),
            &String::from_utf8_lossy(&o.stderr),
        );
        store::write(
            &self.root.join(format!("{label}.json")),
            &json!({"exit_code":o.status.code(),"job":job}),
        )
        .unwrap();
        eprintln!("{label}: {:?}", o.status.code());
        o
    }
    fn cli(&self, i: usize, args: &[&str], label: &str) -> Output {
        let mut cmd = Command::new(BINS[i]);
        cmd.args(args);
        self.output(cmd, None, label)
    }
    fn allocated_cli(&self, i: usize, args: &[&str], job: u64, label: &str) -> Output {
        let mut cmd = Command::new(BINS[i]);
        cmd.args(args);
        self.output(cmd, Some(job), label)
    }
    fn bash(&self, path: impl AsRef<std::path::Path>, job: Option<u64>, label: &str) -> Output {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(path.as_ref());
        self.output(cmd, job, label)
    }
    fn workflow(&self) -> Value {
        store::read(&self.root.join("run/epw-state/workflow.json")).unwrap()
    }
    fn calls(&self) -> (String, String, String) {
        let read = |name| fs::read_to_string(self.root.join(name)).unwrap_or_default();
        (
            read("scientific-calls"),
            read("epw-calls"),
            read("tc-calls.jsonl"),
        )
    }
}
fn success(o: Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
fn copy_results(source: &std::path::Path, target: &std::path::Path) {
    fs::create_dir(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let to = target.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            copy_results(&entry.path(), &to);
        } else {
            assert!(kind.is_file(), "The result package can only contain normal files and directories");
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

#[test]
fn legacy_submit_requires_explicit_migration_without_submitting_a_wrapper() {
    let c = Case::new();
    for bin in 0..3 {
        let out = c.cli(bin, &["-s"], "legacy-native-migration");
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("migration_required"));
    }
    assert!(!c.root.join("submissions").exists());
    assert_eq!(c.calls(), (String::new(), String::new(), String::new()));
}

// 0018：编号工作区协议；替身仍仅验证 IO、命令、依赖与复用，不作科学验收。
impl Case {
    fn numbered() -> Self {
        let c=Self::new();
        // 日志归档用例预建目录；新默认生成只准备当前就绪任务。
        fs::create_dir_all(c.root.join("run/4.qe-phonon-nosplit")).unwrap();
        let mut cfg=config::Config::load(&c.root.join("inputepw.toml")).unwrap();
        cfg.layout=Some("numbered".into());
        cfg.programs.epw.args.extend(["-nk".into(),"1".into()]);
        c.put("input.toml",&toml::to_string_pretty(&cfg).unwrap());
        let qe=include_str!("fixtures/qe_native_io.py").replace("nscf=value('calculation')=='nscf'","nscf=value('calculation') in ('nscf','bands')")
            .replace("nat = len(ET.parse(Path(value('outdir','./out'))/(prefix+'.save')/'data-file-schema.xml').getroot().findall('output/atomic_structure/atomic_positions/atom'))", "nat = int(ET.parse(next(iter(list(Path.cwd().glob('*.dyn*.xml'))+list(Path.cwd().glob('save/*.dyn_q*.xml'))))).getroot().find('GEOMETRY_INFO/NUMBER_OF_ATOMS').text)");
        c.put("qe_native_io.py",&qe);
        let epw=include_str!("fixtures/epw_native_io.py")
            .replace("coarse=value('epwread')=='.false.'","coarse=value('epwread')=='.false.'\nwannier_only='elph = .false.' in text\nreuse=coarse and 'wannierize = .false.' in text")
            .replace("('coarse' if coarse else", "('wannier' if wannier_only else 'coarse' if coarse else")
            .replace("assert (Path(value('dvscf_dir'))/(prefix+'.dvscf_q2')).is_file(), 'PH 未交付'", "assert wannier_only or (Path(value('dvscf_dir'))/(prefix+'.dvscf_q2')).is_file(), 'PH 未交付'")
            .replace("kpoints=[cart(list(map(float,l.split()[:3]))) for l in Path('kpoints.dat').read_text().splitlines()[1:]]", "kpoints=kpoints if wannier_only else [cart(list(map(float,l.split()[:3]))) for l in Path('kpoints.dat').read_text().splitlines()[1:]]")
            .replace("qcr=[list(map(float,l.split()[:3])) for l in Path('qpoints.dat').read_text().splitlines()[1:]]", "qcr=[[0,0,0]] if wannier_only else [list(map(float,l.split()[:3])) for l in Path('qpoints.dat').read_text().splitlines()[1:]]")
            .replace("if coarse:\n        lower=window_min()", "if coarse and not reuse:\n        lower=window_min()")
            .replace("if coarse and value('wannierize')=='.true.': Path(prefix+'.wout').write_text(wout)","if coarse and not reuse:Path(prefix+'.wout').write_text(wout)\n    if wannier_only:\n        Path(prefix+'.win').write_text('num_wann = '+str(n)+'\\n'+'\\n'.join(re.findall(r\"(?m)^\\s*wdata\\(\\d+\\)\\s*=\\s*'([^']*)'\",text))+'\\n')\n        Path(prefix+'.chk').write_text('fixture checkpoint')")
            .replace("kept=list(range(8,17));nk=coarse_nk", "kept=list(range(8,17));nk=coarse_nk\n    if wannier_only:\n        Path(prefix+'.bvec').write_text('native bvec\\n'+str(nk)+' 1\\n'+'1 0 0 1\\n'*nk)\n        Path(prefix+'.mmn').write_text('(0,0)\\n'*(9*9*nk))\n    if reuse:\n        assert Path(prefix+'.bvec').read_text().splitlines()[1].split()==[str(nk),'1'], '粗网格缺同一拟合 bvec'\n        assert len(Path(prefix+'.mmn').read_text().splitlines())==9*9*nk, '粗网格缺同一拟合 mmn'")
            .replace("parent/'nscf/out'", "parent/'2.qe-nscf/out'")
            .replace("ROOT/'run/nscf/out'", "ROOT/'run/2.qe-nscf/out'");
        c.put("epw_native_io.py",&epw);
        for kind in ["bands","dos","projwfc","wannier90","postw90"] {
            c.script(&format!("{kind}.x"),&format!(r#"#!/usr/bin/python3
from pathlib import Path
import re,sys,xml.etree.ElementTree as ET
kind={kind:?}
if kind in ['wannier90','postw90']:
    prefix=sys.argv[1]
    with Path(__file__).with_name('post-calls').open('a') as f:f.write(kind+'\n')
    win=Path(prefix+'.win').read_text()
    mesh=re.search(r'(?m)^mp_grid\s*=\s*(\d+)\s+(\d+)\s+(\d+)',win)
    assert mesh, '独立程序必须有 mp_grid'
    for block in ['unit_cell_cart','atoms_cart','kpoints']:
        assert 'begin '+block in win and 'end '+block in win, block+' 缺失'
    points=re.search(r'begin kpoints\s*\n(.*?)end kpoints',win,re.S).group(1).strip().splitlines()
    assert len(points)==int(mesh[1])*int(mesh[2])*int(mesh[3]), '独立 k 点数量不符'
    assert re.search(r'(?m)^num_bands\s*=\s*9\s*$',win), '应使用排除后的 9 带'
    assert 'dos_smr_index' not in win and 'dos_smr_type = gauss' in win, '应使用原生输入关键字，不能写内部变量'
    eig=[line.split() for line in Path(prefix+'.eig').read_text().splitlines()]
    assert len(eig)==len(points)*9 and all(int(row[0])==i%9+1 and int(row[1])==i//9+1 for i,row in enumerate(eig)), 'eig 必须绑定原 NSCF 点序和排除后带序'
    if kind=='postw90' and (Path(__file__).parent/'post-failure').exists():
        Path(prefix+'.wpout').write_text('failed postprocessing log')
        Path(prefix+'.werr').write_text('Error: standalone post failed despite STOP zero')
        print('Error: standalone post failed despite STOP zero')
        sys.exit(0)
if kind in ['bands','dos','projwfc']:
    text=Path(sys.argv[sys.argv.index('-inp')+1]).read_text() if '-inp' in sys.argv else sys.stdin.read()
    prefix=re.search(r"prefix\s*=\s*'([^']+)'",text).group(1)
    outdir=re.search(r"outdir\s*=\s*'([^']+)'",text).group(1)
    xml=ET.parse(Path(outdir)/(prefix+'.save')/'data-file-schema.xml').getroot()
    ks=xml.findall('output/band_structure/ks_energies')
    if kind=='bands':
        Path(prefix+'_band').write_text('fixture band file')
        Path(prefix+'_band.gnu').write_text(''.join('0 '+str(i)+'\n1 '+str(i)+'\n\n' for i in range(16)))
    elif kind=='projwfc':
        if (Path(__file__).parent/'fail-pdos').exists(): sys.exit(17)
        with Path(__file__).with_name('pdos-calls').open('a') as f:f.write(kind+'\n')
        assert re.search(r'ngauss\s*=\s*0',text) and float(re.search(r'degauss\s*=\s*([0-9.eE+-]+)',text).group(1))>0
        atoms=xml.findall('output/atomic_structure/atomic_positions/atom')
        filpdos=re.search(r"filpdos\s*=\s*'([^']+)'",text).group(1)
        for i,atom in enumerate(atoms,1):
            print('state #',i,': atom',i,'('+atom.attrib['name']+'), wfc 1 (l=0 m=1)')
            Path(filpdos+'.pdos_atm#'+str(i)+'('+atom.attrib['name']+')_wfc#1(s)').write_text('\n'.join(str(j/10)+' 0.2 0.2' for j in range(-100,100)))
        Path(filpdos+'.pdos_tot').write_text('\n'.join(str(j/10)+' 1 0.6' for j in range(-100,100)))
    else:
        Path(prefix+'.tdos').write_text('\n'.join(str(i/10)+' 1' for i in range(-100,100)))
    print('JOB DONE.')
elif kind=='wannier90':
    prefix=sys.argv[1]
    assert 'restart = plot' in Path(prefix+'.win').read_text()
    assert re.search(r'(?m)^write_bvec\s*=\s*\.false\.\s*$',win), '重绘不能重写已绑定的 bvec'
    Path(prefix+'_band.dat').write_text(''.join('0 '+str(i)+'\n1 '+str(i)+'\n\n' for i in range(5)))
    Path(prefix+'_band.labelinfo.dat').write_text('G 1 0\nX 2 1\n')
else:
    prefix=sys.argv[1]
    assert re.search(r'(?m)^num_elec_per_state\s*=\s*2\s*$',win), '非磁标量 DOS 应显式计入两重自旋'
    Path(prefix+'.wpout').write_text('Number of electrons per state : 2 |\n')
    Path(prefix+'_dos.dat').write_text('\n'.join(str(i/10)+' 1' for i in range(-100,100)))
"#));
        }
        c
    }
    fn numbered_cli(&self,i:usize,args:&[&str],job:Option<u64>,label:&str)->Output {
        if let Some(stage) = args.windows(2).find(|a| a[0] == "--stage").map(|a| a[1]) { self.put("last-stage-target", stage); }
        let mut cmd=Command::new(BINS[i]);cmd.args(["-c",self.root.join("input.toml").to_str().unwrap()]).args(args);
        self.environment(&mut cmd,job);if job.is_some(){let cfg=config::Config::load(&self.root.join("input.toml")).unwrap_or_else(|e|panic!("Test configuration loading failed:{e:#}"));cmd.env("SLURM_NTASKS", (epwflow::foundation::parallelism::mpi_count(&cfg).unwrap() + 4).to_string());}
        let o=cmd.output().unwrap();self.put(&format!("{label}.stderr"),&String::from_utf8_lossy(&o.stderr));self.put(&format!("{label}.stdout"),&String::from_utf8_lossy(&o.stdout));o
    }
    fn numbered_change(&self,change:impl FnOnce(&mut config::Config)) {
        let p=self.root.join("input.toml");let mut cfg=config::Config::load(&p).unwrap();
        let own_wannier=cfg.native_value::<Value>(&["5.epw-wannier","epw","INPUTEPW"]).unwrap();
        change(&mut cfg);native_fixture::matrix(&mut cfg);
        // PH 的 q 网格不属于独立的电子拟合；夹具不能把它继承给第 5 步。
        for key in ["nq1","nq2","nq3"] {cfg.set_native(&["5.epw-wannier","epw","INPUTEPW",key],own_wannier[key].clone()).unwrap();}
        fs::write(p,toml::to_string_pretty(&cfg).unwrap()).unwrap();
    }
}

#[test]
fn numbered_wannier_post_zero_exit_error_preserves_fit_and_resumes() {
    let c = Case::numbered();
    c.put("post-failure", "simulate serial Fortran STOP");
    c.accounting(Some(201));
    let failed = c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "post-zero-failed");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("standalone post failed"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    assert!(epwflow::foundation::stages::reusable(&cfg, epwflow::foundation::stages::Step::Wannier).unwrap());
    let receipt = fs::read(c.root.join("run/5.epw-wannier/scf.ukk")).unwrap();
    let wout = store::hash_file(&c.root.join("run/5.epw-wannier/scf.wout")).unwrap();
    let before = c.calls();
    fs::remove_file(c.root.join("post-failure")).unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "--resume", "-r"], Some(202), "post-zero-resume"));
    assert_eq!(before, c.calls(), "Reprocessing can't run again QE or EPW Proposed");
    assert_eq!(receipt, fs::read(c.root.join("run/5.epw-wannier/scf.ukk")).unwrap());
    assert_eq!(wout, store::hash_file(&c.root.join("run/5.epw-wannier/scf.wout")).unwrap());
    assert!(!c.root.join("run/5.epw-wannier/scf.werr").exists());
    assert!(c.root.join("run/results/5.epw-wannier/electronic.png").is_file());
    let norm: Value = store::read(&c.root.join("run/5.epw-wannier/electronic-dos-normalization.json")).unwrap();
    assert_eq!(norm["postw90_multiplier"], 1);
    assert_eq!(norm["postw90_num_elec_per_state"], 2);
    let csv = fs::read_to_string(c.root.join("run/5.epw-wannier/electronic-dos.csv")).unwrap();
    assert!(csv.lines().filter(|line| line.starts_with("EPW,")).all(|line| line.rsplit(',').next().unwrap().parse::<f64>().unwrap() == 1.0),
        "Native DOS Counted for rotation,CSV We can't zoom in again");
}

#[test]
fn numbered_scripts_leave_shell_options_unset_and_source_environment_directly() {
    let c = Case::numbered();
    let environment = c.root.join("environment with spaces.sh");
    c.numbered_change(|cfg| cfg.programs.environment_script = Some(environment.clone()));
    fs::write(&environment, "export EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n").unwrap();
    success(c.numbered_cli(0, &[], None, "generate-with-environment"));
    let work = c.root.join("run");
    let mut checked = 0;
    for entry in epwflow::qe_data::save::files(&work).unwrap() {
        if entry.relative_path.extension().is_none_or(|ext| ext != "sh") { continue; }
        let script = fs::read_to_string(work.join(&entry.relative_path)).unwrap();
        let (prefix, invocation) = script.trim_end().rsplit_once('\n').unwrap();
        assert!(!invocation.contains("set -"));
        let run = |suffix: &str| Command::new("/bin/bash")
            .args(["-c", &format!("{prefix}\n{suffix}")])
            .env_remove("EPW_TEST_OPTIONAL")
            .output().unwrap();
        let out = run("[[ $- != *u* && $- != *e* ]] || exit 1\nif shopt -qo pipefail; then exit 1; fi\nprintf '%s' \"$EPW_TEST_ENV\"");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.stdout, b"ready");
        fs::write(&environment, "return 17\n").unwrap();
        let out = run("printf source-returned");
        assert_eq!(out.status.code(), Some(17), "Environmental script failure must spread exit code");
        assert!(out.stdout.is_empty());
        fs::write(&environment, "export EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n").unwrap();
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn numbered_public_qe_wannier_coarse_and_fine_are_distinct_and_reusable() {
    use epwflow::foundation::{stages::{self,Step},layout};
    let c=Case::numbered();c.numbered_change(|cfg|{cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.gap_temperatures_k=vec![20.,30.];});c.accounting(Some(201));
    success(c.numbered_cli(0,&["-r"],Some(201),"numbered-qe"));
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    for step in [Step::Scf,Step::Nscf,Step::Electronic,Step::Phonon] {assert!(stages::reusable(&cfg,step).unwrap());}
    assert!(c.root.join("run/1.qe-scf/scf.in").is_file());
    assert!(fs::read_to_string(c.root.join("run/1.qe-scf/scf.in")).unwrap().starts_with("&CONTROL\n"));
    assert!(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2").is_file());
    let qe_calls=c.calls().0;
    c.accounting(Some(202));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(202),"numbered-wannier"));
    assert!(!c.root.join("run/6.epw-coarse/epw_coarse.out").exists());
    let wan_hash=store::hash_file(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap();
    let key=stages::scientific_key(&cfg,Step::Wannier).unwrap();
    c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","coarse","--resume","-r"],Some(203),"numbered-coarse"));
    assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),qe_calls.lines().filter(|s|*s=="pw").count()); // 常规路径直接复用第 3 步，不新增 PW。
    assert_eq!(wan_hash,store::hash_file(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap());
    for suffix in ["bvec", "mmn"] {
        let file = format!("scf.{suffix}");
        assert_eq!(store::hash_file(&c.root.join("run/5.epw-wannier").join(&file)).unwrap(),
            store::hash_file(&c.root.join("run/6.epw-coarse").join(&file)).unwrap());
    }
    assert!(fs::read_to_string(c.root.join("run/6.epw-coarse/epw_coarse.in")).unwrap().contains("wannierize = .false."));
    assert_eq!(stages::scientific_key(&cfg,Step::Wannier).unwrap(),key);
    assert!(epwflow::interpolation::manifest::InterpolationManifest::load(&layout::path(&cfg.work_dir,"coarse/manifest.json")).unwrap().coarse.is_some());
    c.accounting(Some(204));
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(204),"numbered-fine"));
    for (directory, stem) in [
        ("3.qe-electronic/qe_eband", "elebanddata"),
        ("3.qe-electronic/qe_eledos", "eletdos"),
        ("5.epw-wannier", "epw_wannier"),
        ("6.epw-coarse", "epw_coarse"),
        ("7.epw-fine", "epw_fine"),
    ] {
        let dir = cfg.work_dir.join(directory);
        assert!(dir.join(format!("{stem}.in")).is_file());
        assert!(dir.join(format!("{stem}.out")).is_file());
        assert!(!dir.join(format!("{stem}.exitcode")).exists());
        let script = fs::read_to_string(dir.join(format!("{stem}.sh"))).unwrap();
        assert!(script.contains(&format!("> {stem}.out 2>&1")), "{script}");
        for old in ["epw.out", "bands.out", "dos.out"] {
            assert!(!dir.join(old).exists(), "New tasks should not generate old logs {old}");
        }
    }
    let calls=c.calls();
    c.numbered_change(|cfg|cfg.interpolation.q_grid=[4;3]);
    c.accounting(Some(205));
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(205),"numbered-fine-new-grid"));
    assert_eq!(c.calls().0,calls.0,"The fine grid change cannot be executed QE");
    assert_eq!(c.calls().1.matches("wannier ").count(),1,"The fine grid changes cannot be repeated");
    assert!(c.root.join("run/7.epw-fine/epw_fine.sh").is_file());
    let before=c.calls();
    c.script("launcher","#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"2".into()];cfg.programs.epw.args[2]="2".into();});
    c.accounting(None);
    success(c.numbered_cli(1,&["--stage","coarse","-s"],None,"numbered-resource-reuse"));
    assert_eq!(before,c.calls(),"Change of resources cannot re-cost existing scientific results");
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().electronic_sigma_ev=0.15);
    c.accounting(Some(206));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(206),"numbered-post-only"));
    assert_eq!(before,c.calls(),"The extension can only run the native processing and redrawing");
    assert!(stages::reusable(&config::Config::load(&c.root.join("input.toml")).unwrap(),Step::Fine).unwrap());
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().electronic_sigma_ev=0.1);c.accounting(Some(211));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(211),"numbered-post-revert"));assert_eq!(before,c.calls());
    assert!(!c.root.join("run/epw-state/post").exists());
    c.accounting(Some(207));
    success(c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(207),"numbered-native-gap"));
    let completed_calls=c.calls();
    let completed:Value=store::read(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
    assert_eq!(completed["execution_complete"],true);
    c.numbered_change(|cfg|{cfg.programs.launcher[2]="3".into();cfg.programs.epw.args[2]="3".into();});
    c.accounting(Some(210));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(210),"numbered-superconductivity-reuse"));
    c.assert_scripts_without_time_limits();
    assert_eq!(completed_calls,c.calls());
    assert_eq!(before.0,c.calls().0);assert!(!c.calls().2.is_empty());
    let coarse_calls=c.calls().1.matches("coarse ").count();
    let config_path=c.root.join("input.toml");
    let original_config=fs::read(&config_path).unwrap();
    let original=config::Config::load(&config_path).unwrap();
    let mut changed=original.clone();
    changed.set_native(&["6.epw-coarse","workflow","points_per_edge"],json!(3)).unwrap();
    fs::write(&config_path,toml::to_string_pretty(&changed).unwrap()).unwrap();
    c.accounting(Some(212));
    let changed_path=c.numbered_cli(1,&["--stage","coarse","--check"],None,"numbered-path-density");
    success(changed_path);
    assert_eq!(coarse_calls,c.calls().1.matches("coarse ").count(),"Independent density checks cannot be recosted EPC");
    assert_eq!(c.calls().1.matches("wannier ").count(),1);
    assert!(stages::reusable(&original,Step::Fine).unwrap());
    // 恢复原始密度；只读检查不能改变已成功的科学数据。
    fs::write(&config_path,original_config).unwrap();c.accounting(Some(213));
    success(c.numbered_cli(1,&["--stage","coarse","--resume","-r"],Some(213),"numbered-path-restore"));
    let before=c.calls();
    c.numbered_change(|cfg|cfg.wannier.outer_window_ev.as_mut().unwrap()[0]=1.4);
    c.accounting(Some(208));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(208),"numbered-new-window"));
    assert_eq!(before.0,c.calls().0,"Can't recalculate a window change QE");
    assert_eq!(c.calls().1.matches("wannier ").count(),2);
    let wan=store::Identity::capture(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap();
    c.numbered_change(|cfg|cfg.phonon.q_grid=[2,1,2]);
    c.accounting(Some(209));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(209),"numbered-new-coarse-q"));
    wan.verify().unwrap();assert_eq!(c.calls().1.matches("wannier ").count(),2,"Crude q We can't repeat the match");
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    assert!(stages::reusable(&cfg,Step::Coarse).unwrap());assert!(!stages::reusable(&cfg,Step::Fine).unwrap());
    c.numbered_change(|cfg|{cfg.programs.launcher[2]="16".into();cfg.programs.epw.args[2]="16".into();});
    success(c.numbered_cli(2,&["--stage","fine","--check"],None,"fine-resources-exceed-cached-coarse-grid"));
    // 分配内执行不额外生成第 5～8 步包装脚本；原生提交脚本由 direct_submit 覆盖。
    for (stage, name) in [("5.epw-wannier", "wannier.sh"), ("6.epw-coarse", "coarse.sh"), ("7.epw-fine", "fine.sh")] {
        assert!(!cfg.work_dir.join(stage).join(name).exists());
    }
}

#[test]
fn numbered_large_differences_continue_to_native_fine_and_both_solvers_but_reject_damage() {
    use epwflow::foundation::{layout, stages::{self, Step}};
    let c = Case::numbered();
    c.numbered_change(|cfg| {cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.gap_temperatures_k=vec![20.,30.];});
    c.put("epw-failure", "large-difference");
    c.accounting(Some(201));
    let coarse = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(201), "warning-coarse");
    assert!(!String::from_utf8_lossy(&coarse.stderr).contains("quality_warning:"));
    success(coarse);
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let manifest = layout::path(&cfg.work_dir, "wannier/manifest.json");
    let value: Value = serde_json::to_value(epwflow::interpolation::manifest::InterpolationManifest::load(&manifest).unwrap()).unwrap();
    assert!(value.get("quality").is_none());
    assert!(stages::reusable(&cfg, Step::Coarse).unwrap());
    assert!(!Step::Coarse.directory(&cfg).join("quality-sources").exists());
    assert!(!cfg.work_dir.join("quality-report.json").exists());
    let comparison = epwflow::interpolation::comparison_data::load(
        &layout::path(&cfg.work_dir, "interpolation/comparison"),
    ).unwrap();
    assert!(comparison.differences.unwrap().phonon_path.maximum.unwrap().absolute_difference > 50.);
    let calls = c.calls();
    c.accounting(Some(202));
    success(c.numbered_cli(2, &["--stage", "fine", "-r"], Some(202), "warning-fine"));
    assert_eq!(calls.0, c.calls().0, "There's no recosting for renewal QE");
    let value: Value = serde_json::to_value(epwflow::interpolation::manifest::InterpolationManifest::load(&layout::path(&cfg.work_dir,"interpolation/manifest.json")).unwrap()).unwrap();
    assert!(value.get("quality").is_none());
    assert!(!value["spectra"].as_array().unwrap().is_empty());
    // 大差值可续算，篡改原生数据仍须拒绝。
    let band = Step::Coarse.directory(&cfg).join("epwdata.fmt");
    let saved = fs::read(&band).unwrap();
    fs::write(&band, "tampered").unwrap();
    let before_calls = c.calls();
    c.accounting(Some(203));
    assert!(!c.numbered_cli(2, &["--stage", "superconductivity", "-r"], Some(203), "warning-damaged").status.success());
    assert_eq!(before_calls, c.calls());
    fs::write(&band, saved).unwrap();
    c.accounting(Some(204));
    success(c.numbered_cli(2, &["--stage", "superconductivity", "--resume", "-r"], Some(204), "warning-both-solvers"));
    assert_eq!(calls.0, c.calls().0);
    let modes: Value = store::read(&Step::Superconductivity.directory(&cfg).join("manifest.json")).unwrap();
    assert_eq!(modes["plan"].as_array().unwrap().len(),4);
    assert!(!c.calls().2.is_empty());
    let results = epwflow::delivery::check_results(
        &cfg.work_dir.join("results/8.epw-superconductivity"),
    ).unwrap();
    assert!(results.get("quality_status").is_none());
    assert!(results.get("quality_warning").is_none());
    let comparison_results=epwflow::delivery::check_results(&cfg.work_dir.join("results/6.epw-coarse")).unwrap();
    assert_eq!(comparison_results["stage_results_verified"],true);
    assert_eq!(results["workflow_complete"], true,
        "The margin does not change the execution completion status");
}

#[test]
fn numbered_empty_pools_and_slurm_mismatch_stop_before_native_execution() {
    let c = Case::numbered();
    c.script("launcher", "#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg| {
        cfg.qe.scf.k_grid=[4,4,2]; cfg.qe.nscf.k_grid=[4,4,2];
        cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"64".into()];
        cfg.programs.epw.args=vec!["--io-contract".into(),"-nk".into(),"64".into()];
    });
    let out=c.numbered_cli(1,&[],None,"numbered-empty-pool");
    assert!(!out.status.success());
    let error=String::from_utf8_lossy(&out.stderr);
    assert!(error.contains("k points=32") && error.contains("pool=64") && error.contains("empty pools"),"{error}");
    assert_eq!(c.calls().0, "");
    c.numbered_change(|cfg| {  cfg.programs.launcher[2]="32".into(); cfg.programs.epw.args[2]="32".into(); });
    success(c.numbered_cli(1,&[],None,"numbered-legal-generation"));
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let root = c.root.join("run");
    let active = logs::read(&root, Step::Coarse).unwrap();
    let script = fs::read_to_string(logs::resolve(&root, active.get("Stage allocation", "Script").unwrap())).unwrap();
    assert!(script.contains("#SBATCH --ntasks=1"));
    assert!(!script.contains("#SBATCH --ntasks=32"));
    assert!(script.contains("epw_wannier") && script.contains("--allocated-stage coarse"));
    assert_eq!(c.calls().0, "");
}


#[test]
fn numbered_submission_freezes_config_and_blocks_duplicate_queued_job() {
    let c=Case::numbered(); c.stage_setup();
    success(c.numbered_cli(0,&["--stage","electronic","-s"],None,"numbered-submit"));
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let root=c.root.join("run");
    let active=logs::read(&root,Step::Electronic).unwrap();
    let script=logs::resolve(&root,active.get("Stage allocation","Script").unwrap());
    let before=fs::read(&script).unwrap();
    let control=store::Lock::acquire(&c.root.join("run/epw-state/control")).unwrap();
    let blocked=c.numbered_cli(0,&["--stage","electronic","-s"],None,"numbered-control-busy");
    assert!(!blocked.status.success());
    assert!(c.root.join("run/epw-state/control.lock/owner.json").is_file());
    drop(control);
    c.numbered_change(|cfg|cfg.slurm.header=Some("#SBATCH --partition=other".into()));
    assert!(!c.numbered_cli(0,&["--stage","electronic","-s","--resume"],None,"numbered-duplicate").status.success());
    assert_eq!(fs::read(&script).unwrap(),before);
    assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(),1);
    c.put("input.toml","invalid pending edits must not affect submitted native job");
    success(c.stage_execute());
    assert_eq!(logs::read(&root,Step::Electronic).unwrap().get("Stage allocation","Status"),Some("completed"));
    assert!(c.root.join("run/results/3.qe-electronic/electronic.png").is_file());
}

#[test]
fn numbered_guiding_centres_invalidates_only_the_wannier_branch() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    let before = config::Config::load(&c.root.join("input.toml")).unwrap();
    let mut after = before.clone();
    after.wannier.guiding_centres = Some(true);
    for step in [Step::Scf, Step::Nscf, Step::Electronic, Step::Phonon] {
        assert_eq!(stages::scientific_key(&before, step).unwrap(), stages::scientific_key(&after, step).unwrap());
    }
    for step in [Step::Wannier, Step::Coarse, Step::Fine, Step::Superconductivity] {
        assert_ne!(stages::scientific_key(&before, step).unwrap(), stages::scientific_key(&after, step).unwrap());
    }
}

#[test]
fn numbered_phonon_post_retry_reuses_verified_responses_and_restores_dyn0() {
    let c = Case::numbered();
    c.put("post-failure", "q2r fails after PH collection");
    c.accounting(Some(201));
    let failed = c.numbered_cli(0, &["-r"], Some(201), "ph-post-failed");
    assert!(!failed.status.success());
    let ph = c.root.join("run/4.qe-phonon-nosplit");
    let saved = fs::read(ph.join("save/scf.dyn0.xml")).unwrap();
    assert!(!saved.is_empty());
    let before = c.calls();
    let manifest = fs::read(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q1")).unwrap();
    // 后处理直接使用正式矩阵列表，不需要根目录 dyn0。
    assert!(!ph.join("scf.dyn0").exists());
    fs::remove_file(c.root.join("post-failure")).unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(0, &["--resume", "-r"], Some(202), "ph-post-resume"));
    assert_eq!(saved, fs::read(ph.join("save/scf.dyn0.xml")).unwrap());
    assert!(!ph.join("scf.dyn0").exists());
    assert_eq!(manifest, fs::read(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q1")).unwrap());
    assert_eq!(before.0.lines().filter(|s| *s == "ph" || *s == "pw").collect::<Vec<_>>(),
        c.calls().0.lines().filter(|s| *s == "ph" || *s == "pw").collect::<Vec<_>>(),
        "Reprocessing recovery cannot run again PW/PH");
    assert!(epwflow::foundation::stages::reusable(&config::Config::load(&c.root.join("input.toml")).unwrap(), epwflow::foundation::stages::Step::Phonon).unwrap());
}

#[test]
fn numbered_phonon_partial_rejects_damaged_save_without_native_execution() {
    let c = Case::numbered();
    c.put("post-failure", "q2r failure");
    c.accounting(Some(201));
    assert!(!c.numbered_cli(0, &["-r"], Some(201), "ph-damage-initial").status.success());
    fs::write(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2"), "damaged").unwrap();
    let before = c.calls();
    c.accounting(Some(202));
    let out = c.numbered_cli(0, &["--resume", "-r"], Some(202), "ph-damage-resume");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("data_invalid"));
    assert_eq!(before, c.calls());
}

#[test]
fn numbered_stage_archive_preserves_the_open_slurm_log() {
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;
    let c = Case::numbered();
    success(c.numbered_cli(0, &[], None, "log-prepare"));
    let stage = c.root.join("run/4.qe-phonon-nosplit");
    let stage_inode = fs::metadata(&stage).unwrap().ino();
    let path = c.root.join("run/4.qe-phonon-nosplit/slurm-201.out");
    let mut log = fs::File::create(&path).unwrap();
    log.write_all(b"before archive\n").unwrap();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "log-run"));
    assert_eq!(stage_inode, fs::metadata(&stage).unwrap().ino());
    log.write_all(b"after archive\n").unwrap();
    assert_eq!(log.metadata().unwrap().ino(), fs::metadata(&path).unwrap().ino());
    assert_eq!(fs::read_to_string(path).unwrap(), "before archive\nafter archive\n");
}

#[test]
fn numbered_obsolete_history_does_not_block_scientific_recovery() {
    let c = Case::numbered();
    success(c.numbered_cli(0, &[], None, "old-history-prepare"));
    c.put("run/epw-state/history", "obsolete state");
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "old-history-run"));
    assert!(!c.root.join("run/epw-state/history").exists());
    assert!(c.calls().0.contains("ph"));
}

#[test]
fn numbered_three_ph_modes_deliver_verified_save_and_local_scripts() {
    use epwflow::foundation::{stages::Step,config::PhMode};
    for mode in [PhMode::Nosplit,PhMode::Coords,PhMode::Index] {
        let c=Case::numbered();c.numbered_change(|cfg|cfg.phonon.mode=mode);c.accounting(Some(201));
        success(c.numbered_cli(0,&["-r"],Some(201),"three-ph-modes"));
        let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
        let ph=Step::Phonon.directory(&cfg);
        let m=epwflow::qe_data::manifest::QeManifest::load(&ph.join("save/manifest.json")).unwrap();m.verify().unwrap();
        assert_eq!(m.qpoints.len(),2);assert!(ph.join("save/scf.dvscf_q2").is_file());
        for q in m.qpoints {assert_eq!(q.local_index,q.global_index,"When it's back together save Use Unified Global Numbering");}
        if mode!=PhMode::Nosplit {assert!(ph.join("init/ph_init.sh").is_file());}
        assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),3,"SCF/NSCF/We can take each other once,PH No increase SCF");
    }
}

#[test]
fn numbered_import_verifies_legacy_source_and_materializes_without_qe() {
    use epwflow::foundation::stages::{self,Step};
    let c=Case::numbered();c.accounting(Some(201));
    // 清除夹具为日志测试预建的空目录，旧快照要求全新目标。
    fs::remove_dir_all(c.root.join("run")).unwrap();
    success(c.cli(0,&["-c","inputepw.toml"],"legacy-source-generate"));
    success(c.bash(c.root.join("run/epw-state/generated/qe-data.sh"),Some(201),"legacy-source-qe"));
    c.accounting(None);
    // 布局导入仍支持已验证的版本 2；旧科学配置明确拒绝，不做隐式转换。
    let historical = c.root.join("run/input-init.toml");
    let binding_path = c.root.join("run/run-binding.json");
    let mut binding: Value = store::read(&binding_path).unwrap();
    binding["config_sha256"] = store::hash_file(&historical).unwrap().into();
    let removed_program=c.root.join("historical-dynmat.x");
    fs::write(&removed_program,"historical program bytes").unwrap();
    binding["programs"]["dynmat"]=serde_json::to_value(store::Identity::capture(&removed_program).unwrap()).unwrap();
    fs::remove_file(&removed_program).unwrap();
    store::write(&binding_path, &binding).unwrap();
    fs::create_dir_all(c.root.join("run/quality-sources")).unwrap();
    c.put("run/quality-sources/unused-reference", "historical bytes");
    let source=c.root.join("run");let before=epwflow::qe_data::save::files(&source).unwrap();let calls=c.calls();
    c.numbered_change(|cfg|cfg.work_dir=c.root.join("numbered"));
    success(c.numbered_cli(0,&["--import-work",source.to_str().unwrap()],None,"numbered-import"));
    assert_eq!(before,epwflow::qe_data::save::files(&source).unwrap());assert_eq!(calls,c.calls());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    for step in [Step::Scf,Step::Nscf,Step::Phonon] {assert!(stages::reusable(&cfg,step).unwrap());}
    assert!(Step::Phonon.directory(&cfg).join("scf.phdos.dat").is_file());
    assert!(!cfg.work_dir.join("quality-sources").exists());
    assert!(!cfg.work_dir.join("6.epw-coarse/quality-sources").exists());
    success(c.numbered_cli(0,&["--import-work",source.to_str().unwrap()],None,"numbered-import-repeat"));
    assert_eq!(before,epwflow::qe_data::save::files(&source).unwrap());
}

#[test]
fn numbered_failed_coarse_resume_changes_resources_and_preserves_qe() {
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(201),"fit"));
    c.put("fail-coarse","test");c.accounting(Some(202));
    assert!(!c.numbered_cli(1,&["--stage","coarse","-r"],Some(202),"failed-coarse").status.success());
    let calls=c.calls().0;
    let reference=store::Identity::capture(&c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q1")).unwrap();
    c.script("launcher","#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"2".into()];cfg.programs.epw.args[2]="2".into();});
    fs::remove_file(c.root.join("fail-coarse")).unwrap();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","coarse","--resume","-r"],Some(203),"coarse-recovered"));
    reference.verify().unwrap();assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),calls.lines().filter(|s|*s=="pw").count(),"Electronic Reference Reuse No 3 Step, successful QE Reservations");
}


// 0020：原生 bands 后处理失败与 PW 保存树的粒度恢复。
impl Case {
    fn fail_bands(&self) {
        self.numbered_change(|cfg| {
            cfg.programs.bands.as_mut().unwrap().args = vec!["-pd".into(), ".true.".into()];
            cfg.programs.dos.as_mut().unwrap().args = vec!["-pd".into(), ".true.".into()];
        });
        let p = self.root.join("bands.x");
        let s = fs::read_to_string(&p).unwrap();
        let probe = format!(r#"
root=Path({root})
with (root/'electronic-calls').open('a') as f: f.write(kind+' '+str(sys.argv[1:])+'\n')
assert sys.argv[1:3]==['-pd','.true.'], 'bands 必须启用 pencil 分解'
assert not any(a in sys.argv for a in ['-nk','-npool']), '不得继承 PW/EPW pool'
if (root/'fail-bands').exists():
    print('Error in routine fft_type_set (6): there are processes with no planes',file=sys.stderr)
    sys.exit(6)
"#, root=serde_json::to_string(self.root.to_str().unwrap()).unwrap());
        self.script("bands.x", &s.replace("kind=\"bands\"", &format!("kind=\"bands\"\n{probe}")));
        let p=self.root.join("dos.x");let s=fs::read_to_string(p).unwrap();
        self.script("dos.x", &s.replace("kind=\"dos\"", "kind=\"dos\"\nassert sys.argv[1:3]==['-pd','.true.'] and '-nk' not in sys.argv"));
        self.put("fail-bands", "fail");
    }
    fn begin_failed_electronic(&self) {
        self.accounting(Some(201));
        let o=self.numbered_cli(1,&["--stage","wannier","-r"],Some(201),"failed-electronic");
        assert!(!o.status.success());
        assert!(String::from_utf8_lossy(&o.stderr).contains("exit"),"{}",String::from_utf8_lossy(&o.stderr));
        let cfg = config::Config::load(&self.root.join("input.toml")).unwrap();
        assert!(epwflow::foundation::readable_state::completed_files(&cfg, epwflow::foundation::stages::Step::Electronic, "Electronic substep · pw-bands").unwrap().is_some());
    }
}

#[test]
fn numbered_electronic_retry_preserves_pw_with_32_mpi_and_then_changed_resources() {
    let c=Case::numbered();c.fail_bands();
    c.script("launcher", "#!/bin/sh\n[ \"$1\" = -np ] || exit 11\n[ \"$2\" -le \"$SLURM_NTASKS\" ] || exit 12\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{
        cfg.qe.scf.k_grid=[4,4,2]; cfg.qe.nscf.k_grid=[4,4,2];
        cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"32".into()];
        cfg.programs.epw.args[2]="32".into();
    });
    c.begin_failed_electronic();
    let before=c.calls();assert_eq!(before.0.lines().filter(|l|*l=="pw").count(),3);
    let save=c.root.join("run/3.qe-electronic/qe_eband/out");
    let saved=epwflow::qe_data::save::files(&save).unwrap();
    c.numbered_change(|cfg|{cfg.programs.launcher[2]="16".into();cfg.programs.epw.args[2]="16".into();});
    fs::remove_file(c.root.join("fail-bands")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(202),"electronic-recovered"));
    assert_eq!(before.0,c.calls().0,"Don't run again SCF/NSCF/PW bands");
    assert_eq!(saved,epwflow::qe_data::save::files(&save).unwrap());
    assert_eq!(fs::read_to_string(c.root.join("electronic-calls")).unwrap().lines().count(),2);
    let calls=c.calls();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(203),"electronic-idempotent"));
    assert_eq!(calls,c.calls());
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().electronic_sigma_ev=0.12);c.accounting(Some(204));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(204),"electronic-dos-update"));
    assert_eq!(calls,c.calls(),"Only DOS Settings cannot be recalculated PW or Wannier");
    assert_eq!(fs::read_to_string(c.root.join("electronic-calls")).unwrap().lines().count(),2);
}

#[test]
fn numbered_electronic_receipt_rejects_corrupted_save_and_adopts_verified_old_pw() {
    let c=Case::numbered();c.fail_bands();c.begin_failed_electronic();
    let calls=c.calls();
    let prefix=config::Config::load(&c.root.join("input.toml")).unwrap().qe.prefix.clone();
    let wfc=c.root.join(format!("run/3.qe-electronic/qe_eband/out/{prefix}.save/wfc1.dat"));
    let xml=wfc.parent().unwrap().join("data-file-schema.xml");
    let original=fs::read(&xml).unwrap();fs::write(&xml,"").unwrap();
    c.accounting(Some(202));
    let o=c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(202),"corrupted-pw");
    assert!(!o.status.success(), "Damage XML It has to stop");
    assert_eq!(calls,c.calls());fs::write(&xml,original).unwrap();
    // 模拟 0018：有零退出码和冻结尝试，没有新原生任务收据。

    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).unwrap();
    let density=wfc.parent().unwrap().join("charge-density.dat");
    let original_density=fs::read(&density).unwrap();fs::write(&density,b"wrong density").unwrap();
    epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).unwrap();
    fs::write(density,original_density).unwrap();
    fs::remove_file(&wfc).unwrap();
    epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).unwrap();
    fs::write(&wfc,b"fixture-wfc").unwrap();

    let input=c.root.join("run/3.qe-electronic/qe_eband/eband.in");let original=fs::read(&input).unwrap();
    fs::write(&input,"&CONTROL calculation='bands' /\n").unwrap();assert!(epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).is_err());fs::write(input,original).unwrap();
    fs::remove_file(c.root.join("fail-bands")).unwrap();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(203),"legacy-pw-recovered"));
    assert_eq!(calls.0,c.calls().0);
}

#[test]
fn numbered_electronic_script_change_is_rejected_and_fresh_attempt_recovers() {
    let c=Case::numbered(); c.stage_setup();
    success(c.numbered_cli(1,&["--stage","wannier","-s"],None,"submit-before-edit"));
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let root = c.root.join("run");
    let doc = logs::read(&root, Step::Wannier).unwrap();
    let script = logs::resolve(&root, doc.get("Stage allocation", "Script").unwrap());
    let original = fs::read(&script).unwrap();
    fs::write(&script, "#!/bin/bash\n# manually edited\nexit 0\n").unwrap();
    let out = c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "edited-script");
    assert!(!out.status.success());
    assert_eq!(c.calls(), (String::new(), String::new(), String::new()));
    fs::write(&script, original).unwrap();
    success(c.stage_execute());
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "completed-stage"));
    assert_eq!(logs::read(&root, Step::Wannier).unwrap().get("Stage allocation", "Status"), Some("completed"));
}

#[test]
#[ignore = "Visible setting of the home MgB2 configuration;read-only validation, not submitting or executing scientific programs"]
fn numbered_real_legacy_pw_is_read_only() {
    let input=std::env::var_os("DFTFLOW_0020_REAL_CONFIG").expect("Visible Host Configuration");
    let cfg=config::Config::load(std::path::Path::new(&input)).unwrap();
    let before=epwflow::qe_data::save::files(&cfg.work_dir.join("3.qe-electronic/qe_eband")).unwrap();
    let record=epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).unwrap();
    assert_eq!(before,epwflow::qe_data::save::files(&cfg.work_dir.join("3.qe-electronic/qe_eband")).unwrap());
    println!("{}",serde_json::to_string_pretty(&record).unwrap());
}


// 0021：从公开提交及实际脚本覆盖 DOS 失败、旧作业拒绝和新尝试恢复。
#[test]
fn numbered_dos_failure_public_resubmit_preserves_pw_and_bands() {
    direct_submit::dos_failure_preserves_verified_pw_and_bands();
}

#[test]
fn numbered_deleted_wannier_directory_rebuilds_without_repeating_upstream() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "initial-phonon"));
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(202), "initial-fit"));
    let before = c.calls();
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for f in epwflow::qe_data::save::files(&cfg.work_dir).unwrap() {
        assert!(!f.relative_path.parent().unwrap().components().any(|part| part.as_os_str().to_string_lossy().starts_with(".")), "Unable to create hidden running directories: {:?}", f.relative_path);
    }
    let upstream = store::Identity::capture(&c.root.join("run/2.qe-nscf/nscf.out")).unwrap();
    fs::remove_dir_all(c.root.join("run/5.epw-wannier")).unwrap();
    c.accounting(None);
    let files = epwflow::qe_data::save::files(&cfg.work_dir).unwrap();
    success(c.numbered_cli(1, &["--status", "--json"], None, "deleted-status"));
    assert_eq!(files, epwflow::qe_data::save::files(&cfg.work_dir).unwrap(), "Status query must read only");
    for step in [Step::Wannier, Step::Coarse, Step::Fine, Step::Superconductivity] {
        assert!(!stages::reusable(&cfg, step).unwrap());
    }
    c.script("sbatch", "#!/bin/sh\nprintf '203\\n'\n");
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "deleted-submit"));
    assert!(c.root.join("run/5.epw-wannier/wannier.sh").is_file());
    assert_eq!(before, c.calls(), "Submission cannot carry out scientific procedures in advance");
    c.stage_setup();
    success(c.stage_execute());
    assert_eq!(before.0, c.calls().0, "Full upstream QE You can't run again");
    upstream.verify().unwrap();
    assert_eq!(c.calls().1.matches("wannier ").count(), before.1.matches("wannier ").count() + 1);
    assert!(stages::reusable(&cfg, Step::Wannier).unwrap());
    for step in [Step::Coarse, Step::Fine, Step::Superconductivity] {
        assert!(!stages::receipt(&cfg.work_dir, step).exists(), "The old downstream can't come back from recalculation");
    }
}

#[test]
fn numbered_missing_checkpoint_rebuilds_but_modified_or_corrupt_evidence_is_rejected() {
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "fit-for-missing-file"));
    c.accounting(None);
    let chk = c.root.join("run/5.epw-wannier/scf.ukk");
    let original = fs::read(&chk).unwrap();
    fs::write(&chk, "modified checkpoint").unwrap();
    let bad = c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "modified-checkpoint");
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("data_invalid"));
    fs::write(&chk, original).unwrap();
    let receipt = c.root.join("run/epw-state/5.epw-wannier.log");
    let raw = fs::read(&receipt).unwrap();
    fs::write(&receipt, "corrupt receipt").unwrap();
    assert!(!c.numbered_cli(1, &["--status"], None, "corrupt-receipt").status.success());
    fs::write(receipt, raw).unwrap();
    fs::remove_file(chk).unwrap();
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "missing-checkpoint-submit"));
}

#[test]
fn numbered_deleted_stage_never_bypasses_queued_or_unknown_submission() {
    let c=Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0,&["--stage","electronic","-r"],Some(201),"queued-upstream"));
    c.direct_setup();c.put("job-201.state","COMPLETED");
    success(c.numbered_cli(1,&["--stage","wannier","-s"],None,"queued-wannier"));
    let active=c.root.join("run/epw-state/5.epw-wannier.log");
    let before=fs::read(&active).unwrap();
    fs::remove_dir_all(c.root.join("run/5.epw-wannier")).unwrap();
    assert!(!c.numbered_cli(1,&["--stage","wannier","-s"],None,"queued-deleted-stage").status.success());
    assert_eq!(before,fs::read(active).unwrap());
    assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(),1);
}

impl Case {
    fn hide_numbered_state(&self) {
        use epwflow::foundation::stages::{self, Step};
        let cfg = config::Config::load(&self.root.join("input.toml")).unwrap();
        use epwflow::foundation::readable_state as logs;
        // 先收集完整依赖链；删除上游日志后再判断会把下游误判为未完成。
        let completed = Step::ALL.into_iter().filter(|&step| stages::reusable(&cfg,step).unwrap_or(false)).collect::<Vec<_>>();
        for step in Step::ALL {
            if completed.contains(&step) {
                let dest=cfg.work_dir.join("epw-state/steps").join(format!("{}.json",step.name()));
                let completion=stages::Completion {schema_version:1,step,scientific_key:stages::scientific_key(&cfg,step).unwrap(),upstream:stages::upstream(&cfg,step).unwrap(),outputs:vec![],provenance:None};
                store::write(&dest,&completion).unwrap();
            }
            let file=logs::path(&cfg.work_dir,step).unwrap();
            if file.exists(){fs::remove_file(file).unwrap();}
        }
        // 历史布局的编号原文位于输入旁；不能把新格式快照伪造成旧隐藏状态内容。
        for entry in fs::read_dir(cfg.work_dir.join("epw-state")).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name().to_string_lossy().ends_with("-config.toml") {
                fs::rename(entry.path(), self.root.join(entry.file_name())).unwrap();
            }
        }
        store::write(&cfg.work_dir.join("epw-state/layout.json"), &json!({"schema_version":3,"phonon_mode":"nosplit"})).unwrap();
        fs::write(cfg.work_dir.join("epw-state/current-config.toml"),fs::read(self.root.join("input.toml")).unwrap()).unwrap();
        store::write(&cfg.work_dir.join("epw-state/active.json"),&json!({"job":201,"state":"succeeded"})).unwrap();
        fs::rename(cfg.work_dir.join("epw-state"), cfg.work_dir.join(".epwflow")).unwrap();
    }
}

#[test]
fn numbered_legacy_migration_preserves_history_and_reuses_without_hidden_tree() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "migration-qe"));
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(202), "migration-fit"));
    c.accounting(None);
    c.hide_numbered_state();
    let old = c.root.join("run/.epwflow");

    let blocked = c.numbered_cli(1, &["--status"], None, "migration-required");
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("--migrate-state"));
    assert!(!c.root.join("run/epw-state").exists(), "Read-only queries do not migrate");
    success(c.numbered_cli(1, &["--migrate-state"], None, "migration"));
    assert!(!old.exists());
    assert!(!c.root.join("fixture-ledger.json").exists());
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for s in [Step::Scf, Step::Nscf, Step::Phonon, Step::Wannier] {
        assert!(stages::reusable(&cfg, s).unwrap(), "Migration should be reused {}", s.name());
    }
    assert!(!c.root.join("run/epw-state/migration.json").exists());
    let before = c.calls();
    success(c.numbered_cli(0, &["--status"], None, "migration-independent-status"));
    success(c.numbered_cli(1, &["--stage", "wannier"], None, "migration-independent-generate"));
    assert_eq!(before, c.calls());
    assert!(!c.root.join("run/.epwflow").exists());
    // 阶段脚本生成不归集旧电子记录；显式迁移复查仍应如实报告未核验结果。
    let out = c.numbered_cli(1, &["--migrate-state"], None, "migration-recheck");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("The electronic reference sub-task has not yet been fully checked"));
    assert_eq!(before, c.calls());
}

#[test]
fn numbered_migration_rejects_activity_and_retains_a_deleted_stage_as_pending() {
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "migration-before-delete"));
    c.hide_numbered_state();
    let active_path = c.root.join("run/.epwflow/active.json");
    let before = fs::read(&active_path).unwrap();
    assert!(!c.numbered_cli(1, &["--migrate-state"], None, "migration-active").status.success());
    assert_eq!(before, fs::read(&active_path).unwrap());
    assert!(!c.root.join("run/epw-state").exists());
    c.put("accounting", "");
    assert!(!c.numbered_cli(1, &["--migrate-state"], None, "migration-unknown").status.success());
    c.accounting(None);
    let lock = store::Lock::acquire(&c.root.join("run/.epwflow/control")).unwrap();
    assert!(!c.numbered_cli(1, &["--migrate-state"], None, "migration-locked").status.success());
    assert!(c.root.join("run/.epwflow/control.lock/owner.json").is_file());
    drop(lock);
    fs::remove_dir_all(c.root.join("run/5.epw-wannier")).unwrap();
    // 201 已被原分配登记；迁移后模拟调度器必须返回新的作业号。
    c.script("sbatch", "#!/bin/sh\nprintf '202\\n'\n");
    success(c.numbered_cli(1, &["--migrate-state"], None, "migration-after-delete"));
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "migration-deleted-submit"));
}

#[test]
fn numbered_old_wannier_receipt_rebuilds_unbound_velocity_files_and_rejects_tampering() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "velocity-initial"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    fs::remove_file(Step::Wannier.directory(&cfg).join("scf.mmn")).unwrap();
    assert!(!stages::reusable(&cfg, Step::Wannier).unwrap());
    assert!(stages::reusable(&cfg, Step::Scf).unwrap());
    assert!(stages::reusable(&cfg, Step::Nscf).unwrap());
    let qe = c.calls().0;
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(202), "velocity-rebuild"));
    assert_eq!(c.calls().0, qe, "The old bid receipt upgrade is not recosted QE");
    assert_eq!(c.calls().1.matches("wannier ").count(), 2);
    for suffix in ["bvec", "mmn"] {
        assert!(Step::Wannier.directory(&cfg).join(format!("scf.{suffix}")).is_file());
    }
    assert!(stages::reusable(&cfg, Step::Wannier).unwrap());
    fs::write(Step::Wannier.directory(&cfg).join("scf.mmn"), "(9,9)\n").unwrap();
    let calls = c.calls();
    c.accounting(Some(203));
    assert!(!c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(203), "velocity-tampered").status.success());
    assert_eq!(c.calls(), calls, "Obstructed document tampered with must be refused before native execution");
}

#[test]
fn numbered_coarse_retry_after_overwriting_the_driver_keeps_qe_results() {
    use std::io::Write;
    let c = Case::numbered();
    let driver = c.root.join("mutable-epw-wannier");
    fs::copy(BINS[1], &driver).unwrap();
    let run = |job, label| {
        let mut cmd = Command::new(&driver);
        cmd.args(["-c", c.root.join("input.toml").to_str().unwrap(), "--stage", "coarse", "-r"])
            .env("SLURM_NTASKS", "1");
        if job != 201 {cmd.arg("--resume");}
        c.output(cmd, Some(job), label)
    };
    c.put("fail-coarse", "intentional native failure");
    c.accounting(Some(201));
    let initial = run(201, "driver-before-upgrade");
    assert!(!initial.status.success());
    assert!(String::from_utf8_lossy(&initial.stderr).contains("EPW Quit Status Some(17)"));
    let before = c.calls().0;
    fs::OpenOptions::new().append(true).open(&driver).unwrap().write_all(b"upgraded-driver-marker").unwrap();
    c.accounting(Some(202));
    let retry = run(202, "driver-after-upgrade");
    assert!(!retry.status.success());
    assert!(String::from_utf8_lossy(&retry.stderr).contains("EPW Quit Status Some(17)"), "{}", String::from_utf8_lossy(&retry.stderr));
    assert_eq!(before, c.calls().0);
}

#[test]
fn numbered_comparison_upgrade_reuses_native_coarse_and_rejects_damaged_science() {
    use std::io::Write;
    let c = Case::numbered();
    let driver = c.root.join("mutable-comparison-driver");
    fs::copy(BINS[1], &driver).unwrap();
    let run = |job, label| {
        let mut cmd = Command::new(&driver);
        cmd.args(["-c", c.root.join("input.toml").to_str().unwrap(), "--stage", "coarse", "-r"])
            .env("SLURM_NTASKS", "1");
        if job == 203 {cmd.arg("--resume");}
        c.output(cmd, Some(job), label)
    };
    c.accounting(Some(200));
    success(c.numbered_cli(0,&["--stage","phonon","-r"],Some(200),"comparison-reference"));
    c.put("bad-path-frequencies", "intentional nonfinite frequency");
    c.accounting(Some(201));
    let first = run(201, "comparison-parse-failed");
    assert!(!first.status.success());
    assert!(String::from_utf8_lossy(&first.stderr).contains("not finite"), "{}", String::from_utf8_lossy(&first.stderr));
    let request = c.root.join("run/6.epw-coarse/epwdata.fmt");
    let coarse = c.root.join("run/6.epw-coarse/epw_coarse.in");
    let protected_calls = || c.calls().0.lines().filter(|l| !matches!(*l, "q2r" | "matdyn")).collect::<Vec<_>>().join("\n");
    let before = (store::hash_file(&request).unwrap(), store::hash_file(&coarse).unwrap(), protected_calls());
    let coarse_calls = c.calls().1.lines().filter(|l| l.starts_with("coarse ")).count();
    fs::OpenOptions::new().append(true).open(&driver).unwrap().write_all(b"comparison-driver-upgrade").unwrap();
    let matrix = c.root.join("run/6.epw-coarse/epwdata.fmt");
    let raw = fs::read(&matrix).unwrap(); fs::write(&matrix, "damaged matrix").unwrap();
    c.accounting(Some(202));
    assert!(!run(202, "comparison-damaged-coarse").status.success());
    assert_eq!(before.2, protected_calls());
    fs::write(matrix, raw).unwrap();
    fs::remove_file(c.root.join("bad-path-frequencies")).unwrap();
    c.accounting(Some(203));
    success(run(203, "comparison-upgraded-resume"));
    assert_eq!(before, (store::hash_file(&request).unwrap(), store::hash_file(&coarse).unwrap(), protected_calls()));
    assert_eq!(coarse_calls, c.calls().1.lines().filter(|l| l.starts_with("coarse ")).count(), "Crude EPC No recalculation");
}

#[test]
fn numbered_native_retry_preserves_open_logs_and_ignores_legacy_log_inputs_only() {
    use std::{io::Write, os::unix::fs::MetadataExt};
    let c = Case::numbered();
    c.put("fail-coarse", "intentional failure");
    c.accounting(Some(201));
    let initial = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(201), "log-input-initial");
    assert!(String::from_utf8_lossy(&initial.stderr).contains("EPW Quit Status Some(17)"));
    let dir = c.root.join("run/6.epw-coarse");
    let old_log = dir.join("slurm-201.err"); fs::write(&old_log, "before failure\n").unwrap();
    let scratch = dir.join("out/scf.wfc1");
    fs::write(&scratch, "native changed working wavefunction").unwrap();
    fs::OpenOptions::new().append(true).open(&old_log).unwrap().write_all(b"normal failure appended\n").unwrap();
    let log = dir.join("slurm-202.out");
    let mut handle = fs::OpenOptions::new().create(true).append(true).open(&log).unwrap();
    handle.write_all(b"before archive\n").unwrap();
    let before = c.calls().0;
    c.accounting(Some(202));
    let retry = c.numbered_cli(1, &["--stage", "coarse", "--resume", "-r"], Some(202), "log-input-retry");
    assert!(String::from_utf8_lossy(&retry.stderr).contains("EPW Quit Status Some(17)"), "{}", String::from_utf8_lossy(&retry.stderr));
    assert_eq!(before, c.calls().0, "Retry cannot repeat independent references or upstream QE");
    handle.write_all(b"after archive\n").unwrap();
    assert_eq!(handle.metadata().unwrap().ino(), fs::metadata(&log).unwrap().ino());
    assert_eq!(fs::read_to_string(&log).unwrap(), "before archive\nafter archive\n");
    assert!(!c.root.join("run/epw-state/native-attempts").exists());
    fs::write(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2"), "damaged scientific input").unwrap();
    c.accounting(Some(203));
    let corrupt = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(203), "log-input-corrupt-science");
    assert!(!corrupt.status.success());
    assert!(String::from_utf8_lossy(&corrupt.stderr).contains("scf.dvscf_q2"));
    assert_eq!(before, c.calls().0);
}

#[path = "support/direct_submit.rs"]
mod direct_submit;

#[path = "support/phonon_chain.rs"]
mod phonon_chain;

#[path = "support/phonon_parallel.rs"]
mod phonon_parallel;

#[path = "support/qe_stage_plots.rs"]
mod qe_stage_plots;

#[test]
fn gap_points_report_invalid_data_and_resume_failed_point_without_repeating_success() {
    for (mode,fault,status) in [(config::Mode::Aniso,"nan","invalid"),(config::Mode::Iso,"missing","missing")] {
        let c=Case::numbered();
        c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=mode;s.gap_temperatures_k=vec![20.,30.,40.];s.mixing_beta=-0.3;s.iso=None;s.aniso=None;});
        c.put("gap-fault",fault);c.accounting(Some(201));
        success(c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-invalid-data"));
        let root=c.root.join("run/8.epw-superconductivity");
        let r=epwflow::superconductivity::points::Run::load(&root.join("manifest.json")).unwrap();
        assert!(r.execution_complete);assert!(!r.data_complete);assert_eq!(r.attempts.len(),3);
        for a in r.attempts{let record=epwflow::superconductivity::verify::record(&a.record.unwrap().path).unwrap();assert!(record.gap.is_none());assert_eq!(record.gap_status.unwrap().data_status,status);}
        let (_,data)=epwflow::delivery::native::load(&c.root.join("run/results/8.epw-superconductivity")).unwrap();assert!(data.gaps.is_empty()&&data.states.is_empty());
        assert_eq!(c.calls().2.lines().count(),3);
    }
    let c=Case::numbered();c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Iso;s.gap_temperatures_k=vec![20.,30.,40.];s.iso=None;s.aniso=None;});
    c.accounting(Some(201));c.put("gap-fault","no-ending");
    assert!(!c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-no-ending").status.success());
    fs::remove_file(c.root.join("gap-fault")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(202),"gap-failed-resume"));
    let r=epwflow::superconductivity::points::Run::load(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
    assert!(r.execution_complete&&r.data_complete);assert_eq!(r.attempts.len(),3);assert!(r.attempts.iter().all(|a|a.status=="valid"));
    assert_eq!(c.calls().2.lines().count(),3);
    let c=Case::numbered();
    c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Iso;s.gap_temperatures_k=vec![20.,30.,40.];s.iso=None;s.aniso=None;});
    c.accounting(Some(201));c.put("gap-fault","unconverged");
    assert!(!c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-unconverged-prefix").status.success());
    let path=c.root.join("run/8.epw-superconductivity/manifest.json");
    let r=epwflow::superconductivity::points::Run::load(&path).unwrap();
    assert!(!r.execution_complete && !r.data_complete);
    let first=r.attempts[0].record.clone().unwrap();
    assert_eq!(epwflow::superconductivity::verify::record(&first.path).unwrap().gap_status.unwrap().data_status,"not_converged");
    fs::remove_file(c.root.join("gap-fault")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(202),"gap-unconverged-remaining"));
    first.verify().unwrap();
    let r=epwflow::superconductivity::points::Run::load(&path).unwrap();
    assert!(r.execution_complete && !r.data_complete);
    assert_eq!(c.calls().2.lines().count(),3,"Only two temperatures that have not yet been reached continue");
}

#[test]
fn gap_resume_keeps_successful_temperature_and_overwrites_failed_point() {
    let c=Case::numbered();c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Iso;s.gap_temperatures_k=vec![20.,30.,40.];s.iso=None;s.aniso=None;});
    c.put("gap-fail30","failure");c.accounting(Some(201));
    assert!(!c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-fail30").status.success());
    let path=c.root.join("run/8.epw-superconductivity/manifest.json");
    let first=epwflow::superconductivity::points::Run::load(&path).unwrap();assert_eq!(first.attempts.len(),3);assert_eq!(first.attempts[0].status,"valid");assert_eq!(first.attempts[1].status,"failed");
    let id=first.attempts[0].record.clone().unwrap();
    fs::remove_file(c.root.join("gap-fail30")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(202),"gap-resume30"));
    let last=epwflow::superconductivity::points::Run::load(&path).unwrap();assert_eq!(last.attempts.len(),3);assert!(last.execution_complete&&last.data_complete);id.verify().unwrap();assert_eq!(last.attempts[0].record.as_ref().unwrap().sha256,id.sha256);
    let calls:Vec<Value>=c.calls().2.lines().map(|l|serde_json::from_str(l).unwrap()).collect();assert_eq!(calls.iter().map(|v|v["temperature_k"].as_f64().unwrap()).collect::<Vec<_>>(),[20.,30.,30.,40.]);
}

#[path = "support/multitemperature.rs"]
mod multitemperature;

#[path = "support/stage_results.rs"]
mod stage_results;

#[path = "support/decay.rs"]
mod decay;

#[test]
fn numbered_preparation_keeps_clean_input_and_json_provenance() {
    let c = Case::numbered();
    c.accounting(None);
    let before = c.calls();
    success(c.numbered_cli(0, &["--stage", "scf", "--prepare-only"], None, "clean-prepare"));
    assert!(!c.root.join("run/epw-state/parameter-map.json").exists());
    assert!(c.root.join("run/epw-state/0000-config.toml").is_file());
    let input = c.root.join("run/1.qe-scf/scf.in");
    assert!(fs::read_to_string(&input).unwrap().starts_with("&CONTROL\n"));
    let receipt = c.direct_task();
    assert_eq!(receipt["state"], "prepared");
    assert!(receipt["job"].is_null());
    assert!(receipt["inputs"].as_array().unwrap().iter().any(|id| id["path"] == input.to_str().unwrap()));
    assert_eq!(c.calls(), before, "Prepare input not to call a scientific program or submit a job");
}

#[path = "support/phonon_import.rs"]
mod phonon_import_tests;

#[test]
fn unconverged_wannier_still_draws_bands_without_scientific_completion() {
    let c=Case::numbered();c.accounting(Some(201));
    c.numbered_change(|cfg| cfg.set_native(&[epwflow::foundation::native_config::WANNIER,"wannier90","bands_num_points"],json!(7)).unwrap());
    let file=c.root.join("epw_native_io.py");
    let body=fs::read_to_string(&file).unwrap().replace(
        "<<< Wannierisation convergence criteria satisfied >>>",
        "Wannierisation convergence was not reached");
    fs::write(&file,body).unwrap();
    let out=c.numbered_cli(1,&["--stage","wannier","-r"],Some(201),"unconverged-fit");
    assert!(!out.status.success());
    let dir=c.root.join("run/5.epw-wannier");
    assert!(!dir.join("completion.json").exists());
    assert!(!c.root.join("run/6.epw-coarse").exists());
    assert!(dir.join("diagnostics/wannier-fit.png").is_file(),"{}",String::from_utf8_lossy(&out.stderr));
    let report:Value=store::read(&dir.join("diagnostics/diagnostic.json")).unwrap();
    assert_eq!(report["converged"],false);
    assert_eq!(report["independent_native_sampling"],true);
    let csv=fs::read_to_string(dir.join("diagnostics/wannier-fit.csv")).unwrap();
    assert!(csv.lines().filter(|l|l.starts_with("Wannier,") && l.split(',').nth(3)==Some("0")).count()>csv.lines().filter(|l|l.starts_with("QE,") && l.split(',').nth(3)==Some("0")).count());
    assert_eq!(report["scientific_completion_published"],false);
    let calls=c.calls();
    let input=fs::read(dir.join("epw_wannier.in")).unwrap();
    success(c.numbered_cli(1,&["--plot"],None,"unconverged-replot"));
    assert_eq!(calls,c.calls(),"Diagnosis reprograms cannot perform scientific procedures");
    assert_eq!(input,fs::read(dir.join("epw_wannier.in")).unwrap());
}

#[test]
fn failed_native_wannier_still_draws_before_scheduler_error_returns() {
    let c=Case::numbered();c.stage_setup();
    let file=c.root.join("epw_native_io.py");
    let body=fs::read_to_string(&file).unwrap();
    fs::write(file,format!("{body}\nraise SystemExit(17)\n")).unwrap();
    success(c.stage_submit("wannier",false));
    let out=c.stage_execute();
    assert!(!out.status.success());
    let dir=c.root.join("run/5.epw-wannier");
    assert!(!dir.join("completion.json").exists());
    assert!(dir.join("diagnostics/wannier-fit.png").is_file(),"{}",String::from_utf8_lossy(&out.stderr));
    let report:Value=store::read(&dir.join("diagnostics/diagnostic.json")).unwrap();
    assert_eq!(report["scientific_completion_published"],false);
    assert!(!c.root.join("run/6.epw-coarse").exists());
}

#[path = "support/readable_state_0064.rs"]
mod readable_state_0064;

#[test]
fn public_scf_reuse_ignores_raw_density_and_wavefunction_changes() {
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["--stage", "scf", "-r"], Some(201), "scf-for-unchecked"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let snapshot = c.root.join("run/1.qe-scf/out").join(format!("{}.save", cfg.qe.prefix));
    fs::remove_file(snapshot.join("wfc1.dat")).unwrap();
    let density = snapshot.join("charge-density.dat");
    fs::set_permissions(&density, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(density, "").unwrap();
    let before = c.calls();
    c.accounting(None);
    let out = c.numbered_cli(0, &["--stage", "scf", "-s"], None, "unchecked-scf-reuse");
    assert!(String::from_utf8_lossy(&out.stdout).contains("reuse: 1.qe-scf"));
    success(out);
    assert_eq!(before, c.calls(), "No reruns due to original big data changes SCF");
}

#[test]
fn epw6_version_rejection_is_revalidated_without_refitting() {
    use epwflow::foundation::{readable_state as log, stages::Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "fit-before-version-upgrade"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let output = c.root.join("run/5.epw-wannier/epw_wannier.out");
    fs::write(&output, fs::read_to_string(&output).unwrap().replace("Program EPW v.5.9", "Program EPW v.6.0")).unwrap();
    let mark = |reason: &str| {
        log::update(&cfg.work_dir, Step::Wannier, |d| {
            d.set("Stage", "Status", "failed");
            d.set("Stage", "Scientific status", "validation pending");
            d.set("Problem", "Reason", reason);
            let label = d.sections.iter().find(|(_, fields)| fields.get("Output name").is_some_and(|s| s == "epw_wannier"))
                .map(|(name, _)| name.clone()).unwrap();
            d.set(&label, "Status", "failed");
            d.set(&label, "Exit code", "0");
            Ok(())
        }).unwrap();
    };
    c.accounting(None);
    let before = c.calls();
    mark("execution_failed: unrelated failure");
    assert!(!c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "unrelated-failure-stays-failed").status.success());
    assert_eq!(before, c.calls());
    mark("environment_incompatible: EPW 5.9 runtime evidence missing");
    let label = log::read(&cfg.work_dir, Step::Wannier).unwrap().sections.iter()
        .find(|(_, fields)| fields.get("Output name").is_some_and(|s| s == "epw_wannier"))
        .map(|(name, _)| name.clone()).unwrap();
    log::update(&cfg.work_dir, Step::Wannier, |d| {
        d.sections.get_mut(&label).unwrap().retain(|k, _| !k.starts_with("Scientific input"));
        Ok(())
    }).unwrap();
    assert!(!c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "missing-input-record-stays-failed").status.success());
    assert_eq!(before, c.calls());
    log::update(&cfg.work_dir, Step::Wannier, |d| {
        d.set(&label, "Scientific input 1", "5.epw-wannier/epw_wannier.in");
        Ok(())
    }).unwrap();
    c.stage_setup();
    c.put("job-201.state", "COMPLETED");
    c.put("submissions", "previous allocation\n");
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "revalidate-epw6"));
    success(c.stage_execute());
    assert_eq!(before, c.calls(), "The version wrongly refused to resume any scientific mission");
    assert_eq!(log::read(&cfg.work_dir, Step::Wannier).unwrap().get("Stage", "Scientific status"), Some("completed"));
}

#[test]
fn legacy_wannier_log_and_registered_task_resume_without_refitting() {
    use epwflow::foundation::{readable_state as log, stages::Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "create-fit"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let dir = cfg.work_dir.join("5.epw-wannier");
    // 重建旧版记录的形状，输入与原生数据保持不变。
    for ext in ["out"] {
        fs::rename(dir.join(format!("epw_wannier.{ext}")), dir.join(format!("epw.{ext}"))).unwrap();
    }
    let script = dir.join("epw_wannier.sh");
    fs::write(&script, fs::read_to_string(&script).unwrap().replace("epw_wannier.out", "epw.out")).unwrap();
    log::update(&cfg.work_dir, Step::Wannier, |d| {
        for fields in d.sections.values_mut() {
            for value in fields.values_mut() {
                *value = value.replace("epw_wannier.out", "epw.out")
                    .replace("epw_wannier.exitcode", "epw.exitcode");
            }
            if fields.get("Output name").is_some_and(|s| s == "epw_wannier") {
                fields.insert("Output name".into(), "epw".into());
            }
        }
        // 保留任务证据，要求从已退出的原生任务继续阶段校验。
        d.set("Stage", "Status", "validation pending");
        d.set("Stage", "Scientific status", "validation pending");
        Ok(())
    }).unwrap();
    let before = c.calls();
    let hash = store::hash_file(&dir.join("epw.out")).unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "--resume", "-r"], Some(202), "legacy-fit"));
    assert_eq!(before, c.calls(), "The old mission can't be reformulated");
    assert_eq!(hash, store::hash_file(&dir.join("epw.out")).unwrap());
    assert!(!dir.join("epw_wannier.out").exists(), "Do not copy old logs to create new evidence");
}

#[test]
fn public_scf_legacy_chinese_state_reuses_science_and_writes_english() {
    use epwflow::foundation::{readable_state as log, stages::Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["--stage", "scf", "-r"], Some(201), "legacy-language-scf"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let pairs: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../src/foundation/legacy-language.json")).unwrap();
    let old = |value: &str| -> String {
        if let Some((source, _)) = pairs.iter().find(|(_, en)| en.as_str() == value) {
            return source.clone();
        }
        for (source, en) in pairs.iter().filter(|(source, _)| source.ends_with(" · ")) {
            if let Some(rest) = value.strip_prefix(en) { return format!("{source}{rest}"); }
        }
        value.into()
    };
    let doc = log::read(&cfg.work_dir, Step::Scf).unwrap();
    let mut text = String::from("Legacy SCF state
");
    for (section, fields) in doc.sections {
        text.push_str(&format!("
【{}】
", old(&section)));
        for (key, value) in fields { text.push_str(&format!("  {}：{}
", old(&key), old(&value))); }
    }
    let footer = include_str!("fixtures/legacy-chinese-state.log").lines().last().unwrap();
    text.push_str(&format!("
{footer}
"));
    fs::write(log::path(&cfg.work_dir, Step::Scf).unwrap(), text).unwrap();
    let jobs = log::jobs(&cfg.work_dir, Step::Scf).unwrap();
    let header = include_str!("fixtures/legacy-chinese-jobs.txt").lines().next().unwrap();
    let mut table = format!("{header}
");
    for (name, job) in jobs { table.push_str(&format!("{}	{job}
", old(&name))); }
    fs::write(log::jobs_path(&cfg.work_dir, Step::Scf).unwrap(), table).unwrap();
    let before = c.calls();
    c.accounting(None);
    let out = c.numbered_cli(0, &["--stage", "scf", "-s"], None, "legacy-language-reuse");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("reuse: 1.qe-scf"), "{stdout}");
    assert!(!stdout.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)));
    success(out);
    assert_eq!(before, c.calls(), "language conversion must not trigger scientific calls");
    let status = c.numbered_cli(0, &["--status", "--json"], None, "legacy-language-status");
    assert!(!String::from_utf8_lossy(&status.stdout).chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)));
    success(status);
}

#[test]
fn electronic_reuse_combines_projection_once_and_new_commands_revalidate() {
    use epwflow::foundation::{readable_state as log, stages::Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(201), "reuse-once-initial"));
    let before = (c.calls(), c.post_calls(), fs::read(c.root.join("pdos-calls")).unwrap());
    let assert_combined = |output: Output| {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        success(output);
        assert_eq!(stdout.lines().filter(|line| *line == "reuse: 3.qe-electronic").count(), 1, "{stdout}");
        assert!(!stdout.contains("/projwfc"), "{stdout}");
    };
    c.accounting(Some(202));
    assert_combined(c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(202), "reuse-once-cached"));
    assert_eq!(before, (c.calls(), c.post_calls(), fs::read(c.root.join("pdos-calls")).unwrap()));

    // 新命令必须检查新状态；解析检查点缺失时复用原生投影补齐，不能被旧确认遮蔽。
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    log::update(&cfg.work_dir, Step::Electronic, |doc| {
        doc.remove("Electronic substep · projwfc");
        Ok(())
    }).unwrap();
    fs::remove_file(cfg.work_dir.join("results/3.qe-electronic/electronic.png")).unwrap();
    c.accounting(Some(203));
    assert_combined(c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(203), "reuse-once-repair"));
    assert_eq!(before, (c.calls(), c.post_calls(), fs::read(c.root.join("pdos-calls")).unwrap()));
    assert!(log::completed_files(&cfg, Step::Electronic, "Electronic substep · projwfc").unwrap().is_some());

    // 后续独立调用仍拒绝损坏的电子科学日志。
    fs::write(cfg.work_dir.join("3.qe-electronic/qe_eband/elebanddata.out"), "Error in routine fixture: corrupted").unwrap();
    c.accounting(None);
    let failed = c.numbered_cli(1, &["--stage", "coarse", "-s"], None, "reuse-once-damaged");
    assert!(!failed.status.success());
    assert_eq!(before, (c.calls(), c.post_calls(), fs::read(c.root.join("pdos-calls")).unwrap()));
}

#[test]
fn layout_0070_native_upstream_uses_single_sources_and_no_exitcode() {
    let c=Case::numbered();
    c.numbered_change(|cfg| {cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.gap_temperatures_k=vec![20.,30.];cfg.superconductivity.iso=None;cfg.superconductivity.aniso=None;});
    c.accounting(Some(201));
    for (program,stage) in [(0,"scf"),(0,"nscf"),(0,"electronic"),(0,"phonon"),(1,"wannier"),(1,"coarse")] {
        success(c.numbered_cli(program,&["--stage",stage,"-r"],Some(201),&format!("layout-0070-{stage}")));
    }
    let root=c.root.join("run");
    assert!(root.join("1.qe-scf/out/scf.save/data-file-schema.xml").is_file());
    assert!(!root.join("1.qe-scf/snapshot").exists());
    assert!(!root.join("1.qe-scf/snapshot-files.json").exists());
    assert!(!root.join("3.qe-electronic/qe_eledos/out").exists());
    assert!(!root.join("6.epw-coarse/save").exists());
    assert!(!root.join("6.epw-coarse/comparison").exists());
    assert!(!root.join("6.epw-coarse/comparison-native/epw").exists());
    assert!(!root.join("6.epw-coarse/decay").exists());
    for name in ["scf.ukk","scf.bvec","scf.mmn"] {
        assert!(fs::symlink_metadata(root.join("6.epw-coarse").join(name)).unwrap().file_type().is_symlink());
    }
    fn walk(dir:&std::path::Path){for entry in fs::read_dir(dir).unwrap(){let entry=entry.unwrap();if entry.file_type().unwrap().is_dir(){walk(&entry.path());}else{assert_ne!(entry.path().extension().and_then(|x|x.to_str()),Some("exitcode"));}}}
    walk(&root);
    assert!(!root.join("4.qe-phonon-nosplit/out").exists());
    assert!(!root.join("4.qe-phonon-nosplit/scf.dyn0").exists());
    for file in epwflow::qe_data::save::files(&root.join("results")).unwrap() {
        let name=file.relative_path.to_string_lossy();
        assert!(name.ends_with(".png") || name.ends_with(".svg") || name.ends_with(".pdf") || name.ends_with(".plot.json") || name.ends_with("-band-ranges.json"), "unneeded result file: {name}");
    }
}

#[test]
fn layout_0070_single_fine_grid_and_native_temperature_batch_share_inputs() {
    let c=Case::numbered();
    c.numbered_change(|cfg|{cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.mode=config::Mode::Both;cfg.superconductivity.gap_temperatures_k=vec![20.,30.];cfg.superconductivity.iso=None;cfg.superconductivity.aniso=None;});
    c.accounting(Some(201));
    for (program,stage) in [(0,"scf"),(0,"nscf"),(0,"electronic"),(0,"phonon"),(1,"wannier"),(1,"coarse"),(2,"fine")] {
        success(c.numbered_cli(program,&["--stage",stage,"-r"],Some(201),&format!("layout-0070-{stage}")));
    }
    let root=c.root.join("run");
    assert!(root.join("7.epw-fine/out/scf.ephmat/egnv").is_file());
    assert!(!root.join("7.epw-fine/smearing-0001").exists());
    assert!(fs::symlink_metadata(root.join("7.epw-fine/out/scf.epmatwp")).unwrap().file_type().is_symlink());
    let before=c.calls();
    c.accounting(Some(202));
    success(c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(202),"layout-0070-gap"));
    assert_eq!(before.0,c.calls().0);
    for method in ["iso","aniso"] {
        let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
        let mode=if method=="iso"{config::Mode::Iso}else{config::Mode::Aniso};
        let dir=root.join("8.epw-superconductivity").join(epwflow::superconductivity::batch::directory_name(&cfg,mode));
        assert!(fs::symlink_metadata(dir.join("source.a2f")).unwrap().file_type().is_symlink());
        assert!(fs::read_to_string(dir.join("epw.in")).unwrap().contains("nstemp = 2"));
    }
    assert!(!root.join("results/index.json").exists());
}

// 0052：直接文件接续，不使用历史阶段回执或全链补算。

// 0052：直接文件接续，不使用历史阶段回执或全链补算。
#[test]
fn reuse_0052_nscf_missing_direct_input_stops_without_running_scf() {
    let c=Case::numbered();c.accounting(Some(201));
    let out=c.numbered_cli(0,&["--stage","nscf","-r"],Some(201),"reuse-0052-missing");
    assert!(!out.status.success());
    assert!(c.calls().0.is_empty(),"must not calculate missing SCF automatically");
}

#[test]
fn reuse_0052_scf_then_nscf_reports_actual_files_without_electronic_work() {
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(0,&["--stage","scf","-r"],Some(201),"reuse-0052-scf"));
    let out=c.numbered_cli(0,&["--stage","nscf","-r"],Some(201),"reuse-0052-nscf");
    let stdout=String::from_utf8_lossy(&out.stdout).into_owned();success(out);
    assert!(stdout.contains("reuse: 1.qe-scf\n"),"{stdout}");
    assert!(stdout.contains("       1.qe-scf/out/scf.save/data-file-schema.xml"),"{stdout}");
    assert!(!stdout.contains("reuse: 3.qe-electronic"),"{stdout}");
    assert!(!c.root.join("run/3.qe-electronic/qe_eband/eband.out").exists());
    let log=fs::read_to_string(c.root.join("run/epw-state/1.qe-scf.log")).unwrap();
    assert!(!log.contains("Protected metadata"));
}

fn reuse_0052_seed_coarse(c:&Case) {
    let dir=c.root.join("run/6.epw-coarse");fs::create_dir_all(dir.join("out")).unwrap();
    let n=5;
    let ukk=format!("9 7\n{}\n{}{}{}{}",(8..=16).map(|i|i.to_string()).collect::<Vec<_>>().join("\n"),
        "(0,0)\n".repeat(9*n),"T\n".repeat(9),"T\n".repeat(7)+&"F\n".repeat(9),"0 0 0\n".repeat(n));
    fs::write(dir.join("scf.ukk"),ukk).unwrap();
    fs::write(dir.join("epwdata.fmt"),format!("0.2\n5 1 6 1 1\n{}{}","0\n".repeat(27),"(0,0)\n".repeat(n*n+36))).unwrap();
    fs::write(dir.join("crystal.fmt"),"2\n6\nfixture-IO\n").unwrap();
    fs::write(dir.join("wigner.fmt"),format!("1 1 1 1 1\n{}","0 0 0 0\n1\n".repeat(3))).unwrap();
    fs::write(dir.join("vmedata.fmt"),"(0,0)\n".repeat(6*n*n)).unwrap();
    fs::write(dir.join("dmedata.fmt"),"(0,0)\n".repeat(3*n*n)).unwrap();
    fs::write(dir.join("out/scf.epmatwp"),vec![0u8;16*n*n*6]).unwrap();
}

#[test]
fn reuse_0052_fine_reads_only_coarse_and_keeps_numbered_parameter_groups() {
    let c=Case::numbered();reuse_0052_seed_coarse(&c);c.accounting(Some(201));
    let out=c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"reuse-0052-fine");
    let stdout=String::from_utf8_lossy(&out.stdout).into_owned();success(out);
    assert!(stdout.contains("reuse: 6.epw-coarse\n"),"{stdout}");
    for stage in ["1.qe-scf","2.qe-nscf","3.qe-electronic","4.qe-phonon","5.epw-wannier"] {
        assert!(!stdout.contains(&format!("reuse: {stage}")),"{stdout}");
    }
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let first=epwflow::interpolation::fine::directory(&cfg).unwrap();
    assert!(first.file_name().unwrap().to_string_lossy().starts_with("01.k4x4x4-q2x2x2-fsthick"));
    let original=fs::read(first.join("out/scf.ephmat/egnv")).unwrap();
    let input=fs::read_to_string(first.join("epw_fine.in")).unwrap();
    assert!(!input.contains("dis_froz_max"));
    assert!(input.contains("efermi_read = .false."));
    fs::write(c.root.join("run/6.epw-coarse/manifest.json"),"obsolete garbage").unwrap();
    c.numbered_change(|cfg|cfg.interpolation.fsthick_ev=0.8);
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"reuse-0052-fine-second"));
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let second=epwflow::interpolation::fine::directory(&cfg).unwrap();
    assert!(second.file_name().unwrap().to_string_lossy().starts_with("02."));
    assert_eq!(original,fs::read(first.join("out/scf.ephmat/egnv")).unwrap());
    assert!(second.join("out/scf.ephmat/egnv").is_file());
}

#[test]
fn reuse_0052_fine_rejects_missing_direct_matrix_without_upstream_calculation() {
    let c=Case::numbered();reuse_0052_seed_coarse(&c);c.accounting(Some(201));
    fs::remove_file(c.root.join("run/6.epw-coarse/out/scf.epmatwp")).unwrap();
    let out=c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"reuse-0052-missing-matrix");
    assert!(!out.status.success());
    assert!(c.calls().0.is_empty() && c.calls().1.is_empty());
}

#[test]
fn reuse_0052_iso_spectrum_route_needs_no_matrix_or_earlier_stage() {
    let c=Case::numbered();
    c.numbered_change(|cfg| {cfg.superconductivity.mode=config::Mode::Iso;cfg.superconductivity.task=config::SuperconductivityTask::Tc;});
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    epwflow::foundation::layout::initialize(&cfg).unwrap();
    let dir=epwflow::interpolation::fine::directory(&cfg).unwrap();fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("scf.a2f"),"w[meV] a2f and integrated 2*a2f/w for 1 smearing values\n1 0.5 1\n2 1 2\n3 1.5 3\n4 2 4\nIntegrated el-ph coupling\n # 4\nSummed el-ph coupling 4\n").unwrap();
    let input=epwflow::superconductivity::source::selected(&cfg).unwrap();
    input.verify(config::Mode::Iso).unwrap();
    assert!(input.matrix_files.is_empty());
    assert!(input.states.is_empty());
    assert!(!dir.join("out/scf.ephmat").exists());
}

#[test]
fn reuse_0052_phonon_does_not_require_nscf_or_electronic_reference() {
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(0,&["--stage","scf","-r"],Some(201),"reuse-0052-ph-scf"));
    let out=c.numbered_cli(0,&["--stage","phonon","-r"],Some(201),"reuse-0052-ph");
    let stdout=String::from_utf8_lossy(&out.stdout).into_owned();success(out);
    assert!(stdout.contains("reuse: 1.qe-scf"),"{stdout}");
    assert!(!stdout.contains("reuse: 2.qe-nscf"),"{stdout}");
    assert!(!c.root.join("run/2.qe-nscf/nscf.out").exists());
    assert!(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2").is_file());
}

#[test]
fn reuse_0052_aniso_uses_per_file_links_to_selected_fine_group() {
    let c=Case::numbered();reuse_0052_seed_coarse(&c);c.accounting(Some(201));
    c.numbered_change(|cfg| {cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.mode=config::Mode::Aniso;cfg.superconductivity.iso=None;cfg.superconductivity.aniso=None;});
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"reuse-0052-link-fine"));
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let fine=epwflow::interpolation::fine::directory(&cfg).unwrap();
    fs::write(fine.join("out/scf.dos"),"fixture native DOS metadata\n").unwrap();
    let original=fs::read(fine.join("out/scf.ephmat/ephmat1")).unwrap();
    let out=c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"reuse-0052-links");
    let stdout=String::from_utf8_lossy(&out.stdout).into_owned();success(out);
    assert!(stdout.contains("reuse: 7.epw-fine"),"{stdout}");
    let dir=cfg.work_dir.join("8.epw-superconductivity").join(epwflow::superconductivity::batch::directory_name(&cfg,config::Mode::Aniso));
    for name in ["egnv","freq","ikmap","npool","ephmat1"] {
        let target=dir.join("out/scf.ephmat").join(name);
        assert!(fs::symlink_metadata(&target).unwrap().file_type().is_symlink());
        assert_eq!(fs::canonicalize(&target).unwrap(),fs::canonicalize(fine.join("out/scf.ephmat").join(name)).unwrap());
    }
    assert_eq!(original,fs::read(fine.join("out/scf.ephmat/ephmat1")).unwrap());
}

#[test]
fn reuse_0052_coarse_consumes_wannier_matrices_without_wout_or_electronic_plots() {
    let c=Case::numbered();c.accounting(Some(201));
    // 只准备粗网格真实需要的第1、2、4步数据，不执行第3、5步。
    for stage in ["scf","nscf","phonon"] {success(c.numbered_cli(0,&["--stage",stage,"-r"],Some(201),&format!("reuse-0052-coarse-{stage}")));}
    let target=c.root.join("run/5.epw-wannier");fs::create_dir_all(&target).unwrap();
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();let n=cfg.wannier.num_wann as usize;let nk=8;
    fs::write(target.join("scf.ukk"),format!("9 7\n{}\n{}{}{}{}",(8..=16).map(|i|i.to_string()).collect::<Vec<_>>().join("\n"),"(0,0)\n".repeat(9*n*nk),"T\n".repeat(9*nk),"T\n".repeat(7)+&"F\n".repeat(9),"0 0 0\n".repeat(n))).unwrap();
    fs::write(target.join("scf.bvec"),format!("fixture bvec\n{nk} 1\n{}","1 0 0 1\n".repeat(nk))).unwrap();
    fs::write(target.join("scf.mmn"),"(0,0)\n".repeat(9*9*nk)).unwrap();
    let out=c.numbered_cli(1,&["--stage","coarse","-r"],Some(201),"reuse-0052-coarse");
    let stdout=String::from_utf8_lossy(&out.stdout).into_owned();success(out);
    assert!(stdout.contains("reuse: 2.qe-nscf"),"{stdout}");
    assert!(stdout.contains("reuse: 4.qe-phonon-nosplit"),"{stdout}");
    assert!(stdout.contains("       5.epw-wannier/scf.mmn"),"{stdout}");
    assert!(!stdout.contains("reuse: 3.qe-electronic"),"{stdout}");
    assert!(!target.join("scf.wout").exists());
    assert!(cfg.work_dir.join("results/6.epw-coarse/phonon.png").is_file());
}

#[test]
fn reuse_0052_native_failure_is_never_marked_completed() {
    let c=Case::numbered();c.accounting(Some(201));
    c.script("pw.x","#!/bin/sh\nprintf 'native fixture failure\\n'\nexit 17\n");
    let out=c.numbered_cli(0,&["--stage","scf","-r"],Some(201),"reuse-0052-native-failure");
    assert!(!out.status.success());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let log=epwflow::foundation::readable_state::read(&cfg.work_dir,epwflow::foundation::stages::Step::Scf).unwrap();
    assert_ne!(log.get("Stage","Scientific status"),Some("completed"));
}

#[test]
fn reuse_0052_active_submission_cannot_be_submitted_twice() {
    let c=Case::numbered();c.accounting(None);
    success(c.numbered_cli(0,&["--stage","scf","-s"],None,"reuse-0052-submit-first"));
    let before=fs::read(c.root.join("submissions")).unwrap();
    c.accounting(Some(201));
    let out=c.numbered_cli(0,&["--stage","scf","-s"],None,"reuse-0052-submit-again");
    assert!(!out.status.success());
    assert_eq!(before,fs::read(c.root.join("submissions")).unwrap());
}

#[test]
fn reuse_0052_explicit_stage_sequence_runs_without_implicit_upstream_work() {
    let c=Case::numbered();c.accounting(Some(201));
    for (program,stage) in [(0,"scf"),(0,"nscf"),(0,"electronic"),(0,"phonon"),(1,"wannier"),(1,"coarse"),(2,"fine")] {
        success(c.numbered_cli(program,&["--stage",stage,"-r"],Some(201),&format!("reuse-0052-sequence-{stage}")));
    }
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let fine=epwflow::interpolation::fine::directory(&cfg).unwrap();
    assert!(fine.join("out/scf.ephmat/egnv").is_file());
    for stage in ["5.epw-wannier","6.epw-coarse"] {
        assert!(!cfg.work_dir.join(stage).join("manifest.json").exists());
    }
    assert!(!cfg.work_dir.join("6.epw-coarse/comparison-native/sources.json").exists());
}

#[test]
fn reuse_0052_coarse_submit_stops_on_missing_inputs_before_sbatch() {
    let c=Case::numbered();c.accounting(None);
    let out=c.numbered_cli(1,&["--stage","coarse","-s"],None,"reuse-0052-coarse-preflight");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("2.qe-nscf/out/scf.save/data-file-schema.xml"));
    assert!(!c.root.join("submissions").exists());
}

#[test]
fn reuse_0052_iso_matrix_route_regenerates_spectrum_with_stage_eight_width() {
    let c=Case::numbered();reuse_0052_seed_coarse(&c);c.accounting(Some(201));
    c.numbered_change(|cfg|{cfg.superconductivity.mode=config::Mode::Iso;cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.iso=None;cfg.superconductivity.aniso=None;});
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"reuse-0052-width-fine"));
    let file=c.root.join("input.toml");let mut cfg=config::Config::load(&file).unwrap();
    let fine=epwflow::interpolation::fine::directory(&cfg).unwrap();
    fs::write(fine.join("out/scf.dos"),"fixture native DOS\n").unwrap();
    let original=fs::read(fine.join("scf.a2f")).unwrap();
    cfg.set_native(&["8.epw-superconductivity","iso","epw","INPUTEPW","fila2f"],json!("")).unwrap();
    cfg.set_native(&["8.epw-superconductivity","iso","epw","INPUTEPW","degaussq"],json!(1.25)).unwrap();
    cfg.set_native(&["8.epw-superconductivity","iso","epw","INPUTEPW","nqstep"],json!(6)).unwrap();
    fs::write(&file,toml::to_string_pretty(&cfg).unwrap()).unwrap();
    let before=c.calls().1;
    success(c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"reuse-0052-width-solve"));
    assert_eq!(before,c.calls().1,"changing spectral width must not run fine interpolation");
    let dir=cfg.work_dir.join("8.epw-superconductivity").join(epwflow::superconductivity::batch::directory_name(&cfg,config::Mode::Iso));
    assert!(fs::symlink_metadata(dir.join("out/scf.ephmat/egnv")).unwrap().file_type().is_symlink());
    assert!(!dir.join("source.a2f").exists());
    let raw=epwflow::interpolation::native::NativeSpectrum::read(&dir.join("scf.a2f"),6).unwrap();
    assert_eq!(raw.lambda_sum,15.0,"synthetic width tag must reflect stage-eight degaussq");
    assert_eq!(original,fs::read(fine.join("scf.a2f")).unwrap());
}

#[test]
fn reuse_0052_linearized_matrix_route_links_qmap_metadata_without_fine_rerun() {
    let c=Case::numbered();reuse_0052_seed_coarse(&c);c.accounting(Some(201));
    c.numbered_change(|cfg|{cfg.superconductivity.mode=config::Mode::Iso;cfg.superconductivity.task=config::SuperconductivityTask::Tc;});
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"reuse-0052-linear-fine"));
    let file=c.root.join("input.toml");let mut cfg=config::Config::load(&file).unwrap();
    let fine=epwflow::interpolation::fine::directory(&cfg).unwrap();
    fs::write(fine.join("out/scf.dos"),"fixture native DOS\n").unwrap();
    cfg.set_native(&["8.epw-superconductivity","iso","epw","INPUTEPW","fila2f"],json!("")).unwrap();
    cfg.set_native(&["8.epw-superconductivity","iso","epw","INPUTEPW","degaussq"],json!(1.25)).unwrap();
    fs::write(&file,toml::to_string_pretty(&cfg).unwrap()).unwrap();
    let before=c.calls().1;
    success(c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"reuse-0052-linear-solve"));
    assert_eq!(before,c.calls().1);
    let dir=cfg.work_dir.join("8.epw-superconductivity").join(epwflow::superconductivity::batch::directory_name(&cfg,config::Mode::Iso));
    assert!(fs::symlink_metadata(dir.join("selecq.fmt")).unwrap().file_type().is_symlink());
    assert!(dir.join("scf.a2f").is_file());
    assert!(!dir.join("source.a2f").exists());
}

#[path = "support/stage_auto_plotting_0072.rs"]
mod stage_auto_plotting_0072;
#[path = "support/electronic_parallel_0073.rs"]
mod electronic_parallel_0073;
