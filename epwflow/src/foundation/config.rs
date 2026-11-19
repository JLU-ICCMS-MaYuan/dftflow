use anyhow::{ensure,Context,Result};
use serde::{Deserialize,Serialize};
use std::{collections::{BTreeMap,BTreeSet},fs,path::{Path,PathBuf}};
use super::paths;
pub use super::native_config::Config;
macro_rules! data {($name:ident {$($f:ident:$t:ty),* $(,)?})=>{
    #[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
    pub struct $name {$(pub $f:$t),*}
};}
macro_rules! enumeration {($name:ident {$($v:ident),*})=>{
    #[derive(Debug,Clone,Copy,Serialize,Deserialize,PartialEq,Eq)] #[serde(rename_all="snake_case")]
    pub enum $name {$($v),*}
};}
enumeration!(PhMode{Nosplit,Index,Coords});
impl Default for PhMode{fn default()->Self{Self::Index}}
enumeration!(Mode{Iso,Aniso,Both});
enumeration!(SuperconductivityTask{Tc,Gap});
impl Default for SuperconductivityTask {fn default()->Self{Self::Tc}}
impl SuperconductivityTask {pub fn is_tc(&self)->bool{*self==Self::Tc}}

enumeration!(Occupations{Fixed,Smearing});
enumeration!(Smearing{Gaussian,Mp,Mv,Fd});
enumeration!(EnergyReference{QeZero,F2Fermi});
enumeration!(FermiSource{F2,FineFixedElectrons});
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default)] pub plot: PlotConfig,
    #[serde(default, skip_serializing_if="Option::is_none")] pub layout:Option<String>,
    pub schema_version:u32,pub work_dir:PathBuf,
    #[serde(default="default_outdir")] pub outdir:PathBuf,
    pub structure:PathBuf,pub pseudos:BTreeMap<String,PathBuf>,pub programs:Programs,
    #[serde(default, skip_serializing_if="Option::is_none")] pub kpath:Option<PathBuf>,
    #[serde(default, skip_serializing_if="Option::is_none")] pub comparison:Option<Comparison>,
    #[serde(default)] pub atomic_masses:BTreeMap<String,f64>,
    pub slurm:qf_config::SlurmSection,pub qe:Qe,pub phonon:Phonon,
    pub wannier:Wannier,pub interpolation:Interpolation,pub superconductivity:Superconductivity,
}
/// 仅控制显示，不参与任何科学阶段身份。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct PlotConfig {
    pub formats: Vec<String>,
    pub electronic_energy_min_ev: f64,
    pub electronic_energy_max_ev: f64,
}
impl Default for PlotConfig {
    fn default() -> Self { Self { formats: vec!["svg".into()], electronic_energy_min_ev: -3.0, electronic_energy_max_ev: 3.0 } }
}
impl PlotConfig {
    pub fn bounds(&self) -> [f64; 2] { [self.electronic_energy_min_ev, self.electronic_energy_max_ev] }
    pub fn formats(&self) -> Result<Vec<qf_config::PlotFormat>> { qf_config::PlotSection { formats: self.formats.clone() }.validate_formats() }
    pub fn validate(&self) -> Result<()> { self.formats()?; interval(self.bounds(), "plot.electronic_energy_min_ev/electronic_energy_max_ev") }
}
fn default_outdir()->PathBuf{PathBuf::from("./tmp")}
data!(Program{path:PathBuf,args:Vec<String>});
pub use super::programs::Programs;
data!(Qe{prefix:String,ecutwfc_ry:f64,ecutrho_ry:f64,occupations:Occupations,smearing:Option<Smearing>,degauss_ry:Option<f64>,scf:Scf,nscf:Nscf});
data!(Scf{k_grid:[u32;3],k_shift:[u32;3],conv_thr_ry:f64});
data!(Nscf{k_grid:[u32;3],nbnd:u32,conv_thr_ry:f64});
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Phonon {#[serde(default)] pub mode:PhMode,pub q_grid:[u32;3],pub tr2_ph:f64}
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Wannier {
    #[serde(default, skip_serializing_if="qf_wannier::SelectionMode::is_legacy")]
    pub selection_mode:qf_wannier::SelectionMode,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub guiding_centres:Option<bool>,
    pub num_wann:u32,pub projections:Vec<String>,pub exclude_bands:Vec<u32>,
    pub energy_reference:EnergyReference,pub outer_window_ev:Option<[f64;2]>,
    pub frozen_window_ev:Option<[f64;2]>,pub frozen_max_ev:Option<f64>,
    pub localization:Convergence,pub disentanglement:Option<Convergence>,
}
data!(Convergence{max_iterations:u32,tolerance_angstrom2:f64,window_iterations:u32});
data!(Comparison{points_per_edge:u32,electronic_sigma_ev:f64,phonon_sigma_cm1:f64});
impl Default for Comparison {fn default()->Self{Self{points_per_edge:20,electronic_sigma_ev:0.1,phonon_sigma_cm1:10.0}}}
data!(Interpolation{k_grid:[u32;3],q_grid:[u32;3],electron_smearing:Smearing,degaussw_ev:f64,phonon_smearings_mev:Vec<f64>,spectral_points:u32,fsthick_ev:f64,eps_acoustic_cm1:f64,fermi_source:FermiSource});
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Superconductivity {
    #[serde(default,skip_serializing_if="SuperconductivityTask::is_tc")]
    pub task:SuperconductivityTask,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub initial_gap_mev:Option<f64>,
    pub mode:Mode,
    #[serde(default="default_mu_star")]
    pub mu_star:f64,
    pub frequency_cutoff_ev:f64,pub max_iterations:u32,
    pub convergence_relative:f64,pub mixing_beta:f64,pub mixing_history:u32,
    #[serde(default)] pub initial_gaps_mev:[f64;2],
    pub initial_shape:f64,pub gap_temperatures_k:Vec<f64>,pub spectrum_index:u32,
    pub iso:Option<Tc>,pub aniso:Option<Tc>,
}
fn default_mu_star()->f64{0.13}
impl Superconductivity {pub fn initial_gap(&self)->f64{self.initial_gap_mev.unwrap_or(10.0)}}
data!(Tc{min_k:f64,max_k:f64,precision_k:f64,max_solves:u32,eigen_margin:f64,normal_gap_mev:f64,super_gap_mev:f64});

pub fn positive(x:f64,field:&str)->Result<()> {ensure!(x.is_finite()&&x>0.0,"configuration: {field}: must be finite and positive");Ok(())}
fn finite_nonnegative(x:f64,f:&str)->Result<()>{ensure!(x.is_finite()&&x>=0.0,"configuration: {f}: must be finite and nonnegative");Ok(())}
fn grid(g:[u32;3],f:&str)->Result<()>{ensure!(g.iter().all(|x|*x>0)&&g.iter().try_fold(1u64,|p,x|p.checked_mul(*x as u64)).is_some(),"configuration: {f}: invalid grid");Ok(())}
fn interval(w:[f64;2],f:&str)->Result<()>{ensure!(w.iter().all(|x|x.is_finite())&&w[0]<w[1],"configuration: {f}: must be finite and increasing");Ok(())}
pub fn temperature(x:f64,field:&str)->Result<i64>{
    positive(x,field)?;
    ensure!(x<=99999.99 && (x*100.0-(x*100.0).round()).abs()<1e-7,"configuration: {field}: temperature must use 0.01 K increments and fit the output range");Ok((x*100.0).round() as i64)
}
impl Convergence {fn validate(&self,f:&str,dis:bool)->Result<()>{let (tol,window,iterations)=if dis {("dis_conv_tol","dis_conv_window","dis_num_iter")}else{("conv_tol","conv_window","num_iter")};positive(self.tolerance_angstrom2,&format!("{f}.{tol}"))?;ensure!(self.window_iterations>0&&self.window_iterations<=self.max_iterations,"configuration: {f}.{window}/{iterations}: invalid iteration window");Ok(())}}

impl Config {
    pub fn load(path:&Path)->Result<Self>{Self::load_impl(path)}
    /// 数据导入与普通加载使用同一版本 2 协议，不接受旧科学配置。
    pub(crate) fn load_imported(path:&Path)->Result<Self>{Self::load_impl(path)}
    fn load_impl(path:&Path)->Result<Self>{
        let path=paths::absolute(&std::env::current_dir()?,path)?;
        let raw=fs::read_to_string(&path).with_context(||format!("configuration: cannot read {}",path.display()))?;
        let mut c:Self=toml::from_str(&raw).map_err(|e: toml::de::Error| {
            let migration = if e.to_string().contains("unknown field `resources`") {
                "; legacy [resources] was removed; use [slurm].header and programs.launcher"
            } else if ["pw", "ph", "epw", "bands", "dos", "projwfc", "q2r", "matdyn", "postw90", "wannier90", "pw2wannier90"].iter().any(|n| e.to_string().contains(&format!("unknown field `{n}`"))) {
                "; legacy [programs.xx] tables were removed; use flat programs.xx_x and programs.xx_args fields with explicit executable paths"
            } else { "" };
            anyhow::anyhow!("configuration: {}: TOML field/type error{migration}: {e}", path.display())
        })?;
        c.validate()?;
        c.resolve(&super::config_versions::base(&path)?)?;
        Ok(c)
    }
    pub fn validate(&self)->Result<()>{
        self.plot.validate()?;
        self.validate_impl().map_err(|e| self.config_error(e))
    }
    fn validate_impl(&self)->Result<()>{
        self.validate_native()?;
        let ph=self.phonon_stage();
        let owner=if self.superconductivity.mode==Mode::Aniso {"aniso"}else{"iso"};
        ensure!(self.layout.as_deref().is_none_or(|v|v=="numbered"),"configuration: layout supports only numbered");
        ensure!(self.schema_version==2,"unsupported_version: only schema_version = 2 is supported; legacy configuration is incompatible");ensure!(!self.slurm.header.as_deref().unwrap_or("").contains('\0'),"configuration: slurm.header: NUL");
        if let Some(c)=&self.comparison {
            ensure!((1..=1000).contains(&c.points_per_edge),"configuration: stages.3.qe-electronic.workflow.points_per_edge: must be in 1..1000");
            positive(c.electronic_sigma_ev,"stages.5.epw-wannier.electron_dos.postw90.dos_smr_fixed_en_width")?;
            positive(c.phonon_sigma_cm1,&format!("stages.{ph}.workflow.phonon_sigma_cm1"))?;
        }
        ensure!(!self.pseudos.is_empty()&&self.pseudos.keys().all(|s|!s.is_empty()&&s.chars().all(|c|c.is_ascii_alphabetic())),"configuration: pseudos: requires a nonempty element-to-file mapping");
        for (el,mass) in &self.atomic_masses {ensure!(self.pseudos.contains_key(el),"configuration: stages.1.qe-scf.pw.ATOMIC_SPECIES: {el} has no declared pseudopotential element");positive(*mass,&format!("stages.1.qe-scf.pw.ATOMIC_SPECIES.{el}"))?;}
        for (n,p) in self.programs.iter() {
            ensure!(p.args.iter().all(|s|!s.contains('\0')),"configuration: programs.{n}_args: NUL");
        }
        super::parallelism::check_epw_images(&self.programs.epw.args)?;
        ensure!(self.programs.launcher.iter().all(|s|!s.contains('\0')),"configuration: programs.launcher: NUL");
        let q=&self.qe;
        ensure!(!q.prefix.is_empty()&&q.prefix.chars().all(|c|c.is_ascii_alphanumeric()||"_-".contains(c)),"configuration: stages.1.qe-scf.pw.CONTROL.prefix: invalid identifier");
        positive(q.ecutwfc_ry,"stages.1.qe-scf.pw.SYSTEM.ecutwfc")?;positive(q.ecutrho_ry,"stages.1.qe-scf.pw.SYSTEM.ecutrho")?;
        ensure!(q.ecutrho_ry>=q.ecutwfc_ry,"configuration: stages.1.qe-scf.pw.SYSTEM.ecutrho: less than ecutwfc");
        match q.occupations {Occupations::Smearing=>{ensure!(q.smearing.is_some(),"configuration: stages.1.qe-scf.pw.SYSTEM.smearing: missing smearing type");positive(q.degauss_ry.context("configuration: stages.1.qe-scf.pw.SYSTEM.degauss: missing")?,"stages.1.qe-scf.pw.SYSTEM.degauss")?;},Occupations::Fixed=>ensure!(q.smearing.is_none()&&q.degauss_ry.is_none(),"configuration: stages.1.qe-scf.pw.SYSTEM.occupations: fixed conflicts with smearing fields")}
        grid(q.scf.k_grid,"stages.1.qe-scf.pw.K_POINTS")?;grid(q.nscf.k_grid,"stages.2.qe-nscf.pw.K_POINTS")?;grid(self.phonon.q_grid,&format!("stages.{ph}.ph.INPUTPH.nq1/nq2/nq3"))?;
        ensure!(q.scf.k_shift.iter().all(|v|*v<=1),"configuration: stages.1.qe-scf.pw.K_POINTS: only 0/1 allowed");
        ensure!(q.nscf.nbnd>0,"configuration: stages.2.qe-nscf.pw.SYSTEM.nbnd: must be positive");
        for (k,p) in q.nscf.k_grid.iter().zip(self.phonon.q_grid){ensure!(k%p==0,"configuration: stages.2.qe-nscf.pw.K_POINTS: incompatible with coarse q grid");}
        positive(q.scf.conv_thr_ry,"stages.1.qe-scf.pw.ELECTRONS.conv_thr")?;positive(q.nscf.conv_thr_ry,"stages.2.qe-nscf.pw.ELECTRONS.conv_thr")?;positive(self.phonon.tr2_ph,&format!("stages.{ph}.ph.INPUTPH.tr2_ph"))?;
        let w=&self.wannier;
        ensure!(w.num_wann>0 && w.num_wann<=q.nscf.nbnd,"configuration: stages.5.epw-wannier.wannier90.num_wann: exceeds band count");
        ensure!(!w.projections.is_empty()&&w.projections.iter().all(|p|!p.trim().is_empty()&&!p.contains(['\n','\r','\0'])&&p.trim()!="random"),"configuration: stages.5.epw-wannier.wannier90.projections: explicit nonrandom projections required");
        let exc:BTreeSet<_>=w.exclude_bands.iter().collect();ensure!(exc.len()==w.exclude_bands.len()&&w.exclude_bands.iter().all(|x|*x>0&&*x<=q.nscf.nbnd),"configuration: stages.5.epw-wannier.wannier90.exclude_bands: duplicate or out-of-range bands");
        ensure!(q.nscf.nbnd as usize-w.exclude_bands.len()>=w.num_wann as usize,"configuration: stages.5.epw-wannier.wannier90.exclude_bands: insufficient remaining bands");
        w.localization.validate("stages.5.epw-wannier.wannier90",false)?;
        if let Some(win)=w.outer_window_ev {interval(win,"stages.5.epw-wannier.wannier90.dis_win_min/dis_win_max")?;w.disentanglement.as_ref().context("configuration: stages.5.epw-wannier.wannier90: missing")?;}
        if let Some(conv)=&w.disentanglement {conv.validate("stages.5.epw-wannier.wannier90",true)?;}
        if let Some(max)=w.frozen_max_ev {
            ensure!(max.is_finite(),"configuration: stages.5.epw-wannier.wannier90.dis_froz_max: nonfinite energy window");
            ensure!(w.frozen_window_ev.is_none(),"configuration: stages.5.epw-wannier.wannier90.dis_froz_max: conflicts with dis_froz_min/dis_froz_max");
            if let Some(outer)=w.outer_window_ev {ensure!(max>=outer[0]&&max<=outer[1],"configuration: stages.5.epw-wannier.wannier90.dis_froz_max: outside outer window");}
        }
        if w.selection_mode==qf_wannier::SelectionMode::EnergyOnly {w.disentanglement.as_ref().context("configuration: stages.5.epw-wannier.wannier90: convergence criterion missing for energy-only mode")?;}
        if let Some(win)=w.frozen_window_ev {
            interval(win,"stages.5.epw-wannier.wannier90.dis_froz_min/dis_froz_max")?;
            if let Some(outer)=w.outer_window_ev {ensure!(win[0]>=outer[0]&&win[1]<=outer[1],"configuration: stages.5.epw-wannier.wannier90.dis_froz_min/dis_froz_max: outside outer window");}
            else {ensure!(w.selection_mode==qf_wannier::SelectionMode::EnergyOnly,"configuration: stages.5.epw-wannier.wannier90.dis_froz_min/dis_froz_max: outer window missing");}
        }
        let i=&self.interpolation;
        grid(i.k_grid,"stages.7.epw-fine.epw.INPUTEPW.nkf1/nkf2/nkf3")?;grid(i.q_grid,"stages.7.epw-fine.epw.INPUTEPW.nqf1/nqf2/nqf3")?;
        ensure!(i.k_grid.iter().zip(i.q_grid).all(|(k,q)|k%q==0),"configuration: stages.7.epw-fine.epw.INPUTEPW.nkf1/nkf2/nkf3: not divisible by fine q grid");
        positive(i.degaussw_ev,"stages.7.epw-fine.epw.INPUTEPW.degaussw")?;positive(i.fsthick_ev,"stages.7.epw-fine.epw.INPUTEPW.fsthick")?;finite_nonnegative(i.eps_acoustic_cm1,"stages.7.epw-fine.epw.INPUTEPW.eps_acoustic")?;
        ensure!(i.spectral_points>=2&&!i.phonon_smearings_mev.is_empty(),"configuration: stages.7.epw-fine.epw.INPUTEPW.nqstep/degaussq: spectral grid or smearing missing");
        for (j,x) in i.phonon_smearings_mev.iter().enumerate(){positive(*x,"stages.7.epw-fine.epw.INPUTEPW.degaussq")?;ensure!(!i.phonon_smearings_mev[..j].contains(x),"configuration: stages.7.epw-fine.epw.INPUTEPW.degaussq: duplicate");}
        let s=&self.superconductivity;
        finite_nonnegative(s.mu_star,&format!("stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.muc"))?;
        for (n,x) in [("wscut",s.frequency_cutoff_ev),("conv_thr_iaxis",s.convergence_relative),("a_gap0",s.initial_shape)]{positive(x,&format!("stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.{n}"))?;}
        ensure!(s.mixing_beta.is_finite()&&s.mixing_beta.abs()>0.0&&s.mixing_beta.abs()<=1.0&&s.max_iterations>0&&(1..=8).contains(&s.mixing_history),"configuration: stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.nsiter/broyden_beta/broyden_ndim: iteration/mixing parameters out of range (target EPW Broyden history length must be in 1..8)");
        ensure!((s.spectrum_index as usize)<i.phonon_smearings_mev.len(),"configuration: stages.8.epw-superconductivity.workflow.spectrum_index: spectrum does not exist");
        if s.task==SuperconductivityTask::Gap {positive(s.initial_gap(),&format!("stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.gap_edge"))?;}
        ensure!(!s.gap_temperatures_k.is_empty(),format!("configuration: stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.temps: empty"));
        ensure!(s.task!=SuperconductivityTask::Gap || s.gap_temperatures_k.len()<=50,"configuration: native multitemperature solver supports at most 50 points");
        let mut seen=BTreeSet::new();for t in &s.gap_temperatures_k {ensure!(seen.insert(temperature(*t,&format!("stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.temps"))?),format!("configuration: stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.temps: duplicate temperature"));}
        if s.task==SuperconductivityTask::Gap {
            for &t in &s.gap_temperatures_k {
                let temperature=temperature(t,&format!("stages.8.epw-superconductivity.{owner}.epw.INPUTEPW.temps"))?;
                let mode=if s.mode==Mode::Aniso {Mode::Aniso}else{Mode::Iso};
                crate::superconductivity::attempt::Request::from_config(self,mode,crate::superconductivity::attempt::SolveKind::Gap,temperature,Some(s.initial_gap()))?;
            }
        }
        Ok(())
    }
    pub(crate) fn resolve(&mut self,base:&Path)->Result<()>{
        self.work_dir=paths::destination(&paths::absolute(base,&self.work_dir)?)?;
        self.outdir=paths::destination(&paths::absolute(base,&self.outdir)?)?;
        self.structure=paths::existing(base,&self.structure,"structure",false)?;
        if self.kpath.is_none() && base.join("KPATH.in").is_file() { self.kpath=Some(base.join("KPATH.in")); }
        if let Some(p)=&mut self.kpath { *p=paths::existing(base,p,"kpath",false)?; }
        for (el,p) in &mut self.pseudos {*p=paths::existing(base,p,&format!("pseudos.{el}"),false)?;}
        self.runtime.programs.resolve(base)?;
        // mpirun 在作业加载环境后定位；保留配置原文，避免重写历史请求身份。
        if let Some(p)=self.programs.launcher.first_mut(){
            if !qf_runner::is_mpirun(p) {*p=paths::command(base,p)?.to_string_lossy().into_owned();}
        }
        if let Some(p)=&mut self.runtime.programs.environment_script {*p=paths::existing(base,p,"programs.environment_script",false)?;}
        // 输入不得在后续要由本运行拥有的工作树内；防止准备与复制相互覆盖。
        for p in std::iter::once(&self.structure).chain(self.pseudos.values()) {
            if p.starts_with(&self.work_dir) && self.layout.as_deref()!=Some("numbered") { // 已绑定配置中结构快照例外由 snapshot 校验。
                ensure!(p.file_name().is_some_and(|n|n.to_string_lossy().ends_with("-init")),"configuration: input is inside the working directory");
            }
        }
        ensure!(self.work_dir!=self.outdir,"configuration: work_dir and outdir base must differ");
        Ok(())
    }
}
