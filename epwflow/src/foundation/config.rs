use anyhow::{bail,ensure,Context,Result};
use serde::{Deserialize,Serialize};
use std::{collections::{BTreeMap,BTreeSet},fs,path::{Path,PathBuf}};
use super::paths;
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
enumeration!(Occupations{Fixed,Smearing});
enumeration!(Smearing{Gaussian,Mp,Mv,Fd});
enumeration!(EnergyReference{QeZero,F2Fermi});
enumeration!(FermiSource{F2,FineFixedElectrons});
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version:u32,pub work_dir:PathBuf,
    #[serde(default="default_outdir")] pub outdir:PathBuf,
    pub structure:PathBuf,pub pseudos:BTreeMap<String,PathBuf>,pub programs:Programs,
    #[serde(default, skip_serializing_if="Option::is_none")] pub kpath:Option<PathBuf>,
    #[serde(default, skip_serializing_if="Option::is_none")] pub comparison:Option<Comparison>,
    #[serde(default)] pub atomic_masses:BTreeMap<String,f64>,
    pub resources:Resources,pub budget:BudgetConfig,pub qe:Qe,pub phonon:Phonon,
    pub wannier:Wannier,pub interpolation:Interpolation,pub superconductivity:Superconductivity,
}
fn default_outdir()->PathBuf{PathBuf::from("./tmp")}
data!(Program{path:PathBuf,args:Vec<String>});
data!(Programs{pw:Program,ph:Program,epw:Program,launcher:Vec<String>,environment_script:Option<PathBuf>,wannier90:Option<Program>,pw2wannier90:Option<Program>});
data!(Resources{partition:String,nodes:u32,mpi:u32,wall_seconds:u64,storage_bytes:u64});
data!(BudgetConfig{campaign_id:String,ledger:PathBuf});
data!(Qe{prefix:String,ecutwfc_ry:f64,ecutrho_ry:f64,occupations:Occupations,smearing:Option<Smearing>,degauss_ry:Option<f64>,scf:Scf,nscf:Nscf});
data!(Scf{k_grid:[u32;3],k_shift:[u32;3],conv_thr_ry:f64});
data!(Nscf{k_grid:[u32;3],nbnd:u32,conv_thr_ry:f64});
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Phonon {#[serde(default)] pub mode:PhMode,pub q_grid:[u32;3],pub tr2_ph:f64}
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Wannier {
    #[serde(default, skip_serializing_if="qf_wannier::SelectionMode::is_legacy")]
    pub selection_mode:qf_wannier::SelectionMode,
    pub num_wann:u32,pub projections:Vec<String>,pub exclude_bands:Vec<u32>,
    pub energy_reference:EnergyReference,pub outer_window_ev:Option<[f64;2]>,
    pub frozen_window_ev:Option<[f64;2]>,pub frozen_max_ev:Option<f64>,
    pub localization:Convergence,pub disentanglement:Option<Convergence>,
}
data!(Convergence{max_iterations:u32,tolerance_angstrom2:f64,window_iterations:u32});
data!(Comparison{points_per_edge:u32,electronic_sigma_ev:f64,phonon_sigma_cm1:f64});
impl Default for Comparison {fn default()->Self{Self{points_per_edge:20,electronic_sigma_ev:0.1,phonon_sigma_cm1:10.0}}}
data!(Interpolation{k_grid:[u32;3],q_grid:[u32;3],electron_smearing:Smearing,degaussw_ev:f64,phonon_smearings_mev:Vec<f64>,spectral_points:u32,fsthick_ev:f64,eps_acoustic_cm1:f64,fermi_source:FermiSource,quality:Quality});
data!(Quality{k_points_crystal:Vec<[f64;3]>,q_points_crystal:Vec<[f64;3]>,energy_window_ev:[f64;2],electron_max_ev:f64,electron_rms_ev:f64,phonon_max_cm1:f64,phonon_rms_cm1:f64,reference_manifest:Option<PathBuf>});
data!(Superconductivity{mode:Mode,mu_star:f64,frequency_cutoff_ev:f64,max_iterations:u32,convergence_relative:f64,mixing_beta:f64,mixing_history:u32,initial_gaps_mev:[f64;2],initial_shape:f64,gap_temperatures_k:Vec<f64>,spectrum_index:u32,iso:Option<Tc>,aniso:Option<Tc>});
data!(Tc{min_k:f64,max_k:f64,precision_k:f64,max_solves:u32,eigen_margin:f64,normal_gap_mev:f64,super_gap_mev:f64});

pub fn positive(x:f64,field:&str)->Result<()> {ensure!(x.is_finite()&&x>0.0,"configuration: {field}: 必须是有限正数");Ok(())}
fn finite_nonnegative(x:f64,f:&str)->Result<()>{ensure!(x.is_finite()&&x>=0.0,"configuration: {f}: 必须是有限非负数");Ok(())}
fn grid(g:[u32;3],f:&str)->Result<()>{ensure!(g.iter().all(|x|*x>0)&&g.iter().try_fold(1u64,|p,x|p.checked_mul(*x as u64)).is_some(),"configuration: {f}: 无效网格");Ok(())}
fn interval(w:[f64;2],f:&str)->Result<()>{ensure!(w.iter().all(|x|x.is_finite())&&w[0]<w[1],"configuration: {f}: 必须有限且递增");Ok(())}
pub fn temperature(x:f64,field:&str)->Result<i64>{
    positive(x,field)?;
    ensure!(x<=99999.99 && (x*100.0-(x*100.0).round()).abs()<1e-7,"configuration: {field}: 温度必须为 0.01 K 刻度且适配输出范围");Ok((x*100.0).round() as i64)
}
impl Resources {
    pub fn validate(&self)->Result<()>{
        ensure!(self.nodes==1,"configuration: resources.nodes: 只允许 1 节点");
        ensure!((1..=64).contains(&self.mpi),"configuration: resources.mpi: 必须在 1..64");
        ensure!((1..=7200).contains(&self.wall_seconds),"configuration: resources.wall_seconds: 必须在 1..7200");
        ensure!((1..=107374182400).contains(&self.storage_bytes),"configuration: resources.storage_bytes: 超出 100 GiB");
        ensure!(!self.partition.is_empty() && self.partition.chars().all(|c| c.is_ascii_alphanumeric()||"_-".contains(c)),"configuration: resources.partition: 非法分区");Ok(())
    }
}
impl Convergence {fn validate(&self,f:&str)->Result<()>{positive(self.tolerance_angstrom2,&format!("{f}.tolerance_angstrom2"))?;ensure!(self.window_iterations>0&&self.window_iterations<=self.max_iterations,"configuration: {f}: 无效迭代窗口");Ok(())}}
impl Tc {fn validate(&self,f:&str)->Result<()>{
    let lo=temperature(self.min_k,&format!("{f}.min_k"))?;let hi=temperature(self.max_k,&format!("{f}.max_k"))?;
    let p=temperature(self.precision_k,&format!("{f}.precision_k"))?;
    ensure!(lo<hi && p<=hi-lo,"configuration: {f}: 温度范围/精度冲突");
    ensure!(self.max_solves>0,"configuration: {f}.max_solves: 必须为正");
    ensure!(self.eigen_margin.is_finite()&&self.eigen_margin>0.5e-7,"configuration: {f}.eigen_margin: 不足以区分输出舍入");
    positive(self.normal_gap_mev,&format!("{f}.normal_gap_mev"))?;positive(self.super_gap_mev,&format!("{f}.super_gap_mev"))?;
    ensure!(self.normal_gap_mev<self.super_gap_mev,"configuration: {f}: 能隙阈值必须分开");Ok(())
}}
impl Config {
    pub fn load(path:&Path)->Result<Self>{
        let path=paths::absolute(&std::env::current_dir()?,path)?;
        let raw=fs::read_to_string(&path).with_context(||format!("configuration: 无法读取 {}",path.display()))?;
        let mut c:Self=toml::from_str(&raw).context("configuration: TOML 字段/类型错误")?;
        c.validate()?;
        c.resolve(path.parent().context("configuration: 配置缺少父目录")?)?;
        Ok(c)
    }
    pub fn validate(&self)->Result<()>{
        ensure!(self.schema_version==1,"unsupported_version: schema_version 必须为 1");self.resources.validate()?;
        if let Some(c)=&self.comparison {
            ensure!((1..=1000).contains(&c.points_per_edge),"configuration: comparison.points_per_edge: 必须在 1..1000");
            positive(c.electronic_sigma_ev,"comparison.electronic_sigma_ev")?;
            positive(c.phonon_sigma_cm1,"comparison.phonon_sigma_cm1")?;
        }
        ensure!(self.budget.campaign_id=="epic-0007","configuration: budget.campaign_id: 本次必须共用 epic-0007");
        ensure!(!self.pseudos.is_empty()&&self.pseudos.keys().all(|s|!s.is_empty()&&s.chars().all(|c|c.is_ascii_alphabetic())),"configuration: pseudos: 必须为元素到文件的非空映射");
        for (el,mass) in &self.atomic_masses {ensure!(self.pseudos.contains_key(el),"configuration: atomic_masses.{el}: 未声明对应赝势元素");positive(*mass,&format!("atomic_masses.{el}"))?;}
        for (n,p) in [("pw",&self.programs.pw),("ph",&self.programs.ph),("epw",&self.programs.epw)] {
            ensure!(p.args.iter().all(|s|!s.contains('\0')),"configuration: programs.{n}.args: NUL");
        }
        ensure!(self.programs.launcher.iter().all(|s|!s.contains('\0')),"configuration: programs.launcher: NUL");
        let q=&self.qe;
        ensure!(!q.prefix.is_empty()&&q.prefix.chars().all(|c|c.is_ascii_alphanumeric()||"_-".contains(c)),"configuration: qe.prefix: 非法标识");
        positive(q.ecutwfc_ry,"qe.ecutwfc_ry")?;positive(q.ecutrho_ry,"qe.ecutrho_ry")?;
        ensure!(q.ecutrho_ry>=q.ecutwfc_ry,"configuration: qe.ecutrho_ry: 小于 ecutwfc");
        match q.occupations {Occupations::Smearing=>{ensure!(q.smearing.is_some(),"configuration: qe.smearing: 缺少展宽类型");positive(q.degauss_ry.context("configuration: qe.degauss_ry: 缺失")?,"qe.degauss_ry")?;},Occupations::Fixed=>ensure!(q.smearing.is_none()&&q.degauss_ry.is_none(),"configuration: qe.occupations: fixed 与展宽字段冲突")}
        grid(q.scf.k_grid,"qe.scf.k_grid")?;grid(q.nscf.k_grid,"qe.nscf.k_grid")?;grid(self.phonon.q_grid,"phonon.q_grid")?;
        ensure!(q.scf.k_shift.iter().all(|v|*v<=1),"configuration: qe.scf.k_shift: 仅允许 0/1");
        ensure!(q.nscf.nbnd>0,"configuration: qe.nscf.nbnd: 必须为正");
        for (k,p) in q.nscf.k_grid.iter().zip(self.phonon.q_grid){ensure!(k%p==0,"configuration: qe.nscf.k_grid: 不与粗 q 网格相容");}
        positive(q.scf.conv_thr_ry,"qe.scf.conv_thr_ry")?;positive(q.nscf.conv_thr_ry,"qe.nscf.conv_thr_ry")?;positive(self.phonon.tr2_ph,"phonon.tr2_ph")?;
        let w=&self.wannier;
        ensure!(w.num_wann>0 && w.num_wann<=q.nscf.nbnd,"configuration: wannier.num_wann: 超出能带数量");
        ensure!(!w.projections.is_empty()&&w.projections.iter().all(|p|!p.trim().is_empty()&&!p.contains(['\n','\r','\0'])&&p.trim()!="random"),"configuration: wannier.projections: 需要显式非随机投影");
        let exc:BTreeSet<_>=w.exclude_bands.iter().collect();ensure!(exc.len()==w.exclude_bands.len()&&w.exclude_bands.iter().all(|x|*x>0&&*x<=q.nscf.nbnd),"configuration: wannier.exclude_bands: 重复或超范围");
        ensure!(q.nscf.nbnd as usize-w.exclude_bands.len()>=w.num_wann as usize,"configuration: wannier.exclude_bands: 剩余能带不足");
        w.localization.validate("wannier.localization")?;
        if let Some(win)=w.outer_window_ev {interval(win,"wannier.outer_window_ev")?;w.disentanglement.as_ref().context("configuration: wannier.disentanglement: 缺失")?;}
        if let Some(conv)=&w.disentanglement {conv.validate("wannier.disentanglement")?;}
        if let Some(max)=w.frozen_max_ev {
            ensure!(max.is_finite(),"configuration: wannier.frozen_max_ev: 非有限能窗");
            ensure!(w.frozen_window_ev.is_none(),"configuration: wannier.frozen_max_ev: 与 frozen_window_ev 冲突");
            if let Some(outer)=w.outer_window_ev {ensure!(max>=outer[0]&&max<=outer[1],"configuration: wannier.frozen_max_ev: 超出外窗");}
        }
        if w.selection_mode==qf_wannier::SelectionMode::EnergyOnly {w.disentanglement.as_ref().context("configuration: wannier.disentanglement: 纯能量模式缺失收敛标准")?;}
        if let Some(win)=w.frozen_window_ev {
            interval(win,"wannier.frozen_window_ev")?;
            if let Some(outer)=w.outer_window_ev {ensure!(win[0]>=outer[0]&&win[1]<=outer[1],"configuration: wannier.frozen_window_ev: 超出外窗");}
            else {ensure!(w.selection_mode==qf_wannier::SelectionMode::EnergyOnly,"configuration: wannier.frozen_window_ev: 缺少外窗");}
        }
        let i=&self.interpolation;
        grid(i.k_grid,"interpolation.k_grid")?;grid(i.q_grid,"interpolation.q_grid")?;
        ensure!(i.k_grid.iter().zip(i.q_grid).all(|(k,q)|k%q==0),"configuration: interpolation.k_grid: 不与细 q 网格整除相容");
        positive(i.degaussw_ev,"interpolation.degaussw_ev")?;positive(i.fsthick_ev,"interpolation.fsthick_ev")?;finite_nonnegative(i.eps_acoustic_cm1,"interpolation.eps_acoustic_cm1")?;
        ensure!(i.spectral_points>=2&&!i.phonon_smearings_mev.is_empty(),"configuration: interpolation.spectral_points/phonon_smearings_mev: 谱网格或展宽缺失");
        for (j,x) in i.phonon_smearings_mev.iter().enumerate(){positive(*x,"interpolation.phonon_smearings_mev")?;ensure!(!i.phonon_smearings_mev[..j].contains(x),"configuration: interpolation.phonon_smearings_mev: 重复");}
        let quality=&i.quality;
        interval(quality.energy_window_ev,"interpolation.quality.energy_window_ev")?;
        ensure!(quality.energy_window_ev[0]<=-i.fsthick_ev&&quality.energy_window_ev[1]>=i.fsthick_ev,"configuration: interpolation.quality.energy_window_ev: 未覆盖 Fermi 窗口");
        for (n,pts) in [("k_points_crystal",&quality.k_points_crystal),("q_points_crystal",&quality.q_points_crystal)] {
            ensure!(!pts.is_empty()&&pts.iter().flatten().all(|v|v.is_finite()),"configuration: interpolation.quality.{n}: 验证点缺失或非有限");
        }
        for (n,x) in [("electron_max_ev",quality.electron_max_ev),("electron_rms_ev",quality.electron_rms_ev),("phonon_max_cm1",quality.phonon_max_cm1),("phonon_rms_cm1",quality.phonon_rms_cm1)]{positive(x,&format!("interpolation.quality.{n}"))?;}
        ensure!(quality.electron_rms_ev<=quality.electron_max_ev&&quality.phonon_rms_cm1<=quality.phonon_max_cm1,"configuration: interpolation.quality: RMS 超出最大阈值");
        let s=&self.superconductivity;
        finite_nonnegative(s.mu_star,"superconductivity.mu_star")?;
        for (n,x) in [("frequency_cutoff_ev",s.frequency_cutoff_ev),("convergence_relative",s.convergence_relative),("mixing_beta",s.mixing_beta),("initial_shape",s.initial_shape)]{positive(x,&format!("superconductivity.{n}"))?;}
        ensure!(s.mixing_beta<=1.0&&s.max_iterations>0&&(1..=8).contains(&s.mixing_history),"configuration: superconductivity: 迭代/混合参数越界（目标 EPW Broyden 历史长度须为 1..8）");
        ensure!((s.spectrum_index as usize)<i.phonon_smearings_mev.len(),"configuration: superconductivity.spectrum_index: 谱不存在");
        for x in s.initial_gaps_mev {positive(x,"superconductivity.initial_gaps_mev")?;}
        ensure!(s.initial_gaps_mev[0]!=s.initial_gaps_mev[1],"configuration: superconductivity.initial_gaps_mev: 需要不同初值");
        ensure!(!s.gap_temperatures_k.is_empty(),"configuration: superconductivity.gap_temperatures_k: 为空");
        let mut seen=BTreeSet::new();for t in &s.gap_temperatures_k {ensure!(seen.insert(temperature(*t,"superconductivity.gap_temperatures_k")?),"configuration: superconductivity.gap_temperatures_k: 重复温度");}
        for (name,tc,required) in [("iso",&s.iso,s.mode!=Mode::Aniso),("aniso",&s.aniso,s.mode!=Mode::Iso)]{
            if required && tc.is_none(){bail!("configuration: superconductivity.{name}: 缺失");}
            if let Some(t)=tc {t.validate(&format!("superconductivity.{name}"))?;if required {ensure!(s.gap_temperatures_k.iter().all(|x|*x>=t.min_k&&*x<=t.max_k),"configuration: superconductivity.{name}: 能隙温度超出范围");}}
        }
        Ok(())
    }
    fn resolve(&mut self,base:&Path)->Result<()>{
        self.work_dir=paths::destination(&paths::absolute(base,&self.work_dir)?)?;
        self.outdir=paths::destination(&paths::absolute(base,&self.outdir)?)?;
        self.structure=paths::existing(base,&self.structure,"structure",false)?;
        if self.kpath.is_none() && base.join("KPATH.in").is_file() { self.kpath=Some(base.join("KPATH.in")); }
        if let Some(p)=&mut self.kpath { *p=paths::existing(base,p,"kpath",false)?; }
        for (el,p) in &mut self.pseudos {*p=paths::existing(base,p,&format!("pseudos.{el}"),false)?;}
        for (n,p) in [("pw",&mut self.programs.pw),("ph",&mut self.programs.ph),("epw",&mut self.programs.epw)] {p.path=paths::existing(base,&p.path,&format!("programs.{n}.path"),true)?;}
        for (n,p) in [("wannier90",&mut self.programs.wannier90),("pw2wannier90",&mut self.programs.pw2wannier90)] {
            if let Some(pr)=p { pr.path=paths::existing(base,&pr.path,&format!("programs.{n}.path"),true)?; }
        }
        if let Some(p)=self.programs.launcher.first_mut(){*p=paths::command(base,p)?.to_string_lossy().into_owned();}
        if let Some(p)=&mut self.programs.environment_script {*p=paths::existing(base,p,"programs.environment_script",false)?;}
        if let Some(p)=&mut self.interpolation.quality.reference_manifest {*p=paths::existing(base,p,"interpolation.quality.reference_manifest",false)?;}
        self.budget.ledger=paths::destination(&paths::absolute(base,&self.budget.ledger)?)?;
        // 输入不得在后续要由本运行拥有的工作树内；防止准备与复制相互覆盖。
        for p in std::iter::once(&self.structure).chain(self.pseudos.values()) {
            if p.starts_with(&self.work_dir) { // 已绑定配置中结构快照例外由 snapshot 校验。
                ensure!(p.file_name().is_some_and(|n|n.to_string_lossy().ends_with("-init")),"configuration: 输入位于工作目录内部");
            }
        }
        ensure!(self.work_dir!=self.outdir,"configuration: work_dir 与 outdir 基址不能相同");
        ensure!(!self.budget.ledger.starts_with(&self.work_dir),"configuration: budget.ledger 必须独立于运行目录");
        Ok(())
    }
}
