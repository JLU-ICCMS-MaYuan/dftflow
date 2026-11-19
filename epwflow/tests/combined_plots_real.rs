//! 只读已有真实数据，向新目录绘制全部阶段组合图，不执行科学程序。
use epwflow::foundation::{combined_plot, config::Config, layout, stages::Step, store};
use serde_json::Value;
use std::{fs,path::{Path,PathBuf}};

fn copy_tree(source:&Path,target:&Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry=entry.unwrap();
        if entry.file_type().unwrap().is_dir() {copy_tree(&entry.path(),&target.join(entry.file_name()));}
        else {fs::copy(entry.path(),target.join(entry.file_name())).unwrap();}
    }
}
#[test]
#[ignore = "requires DFTFLOW_COMBINED_SOURCE_CONFIG and a new DFTFLOW_COMBINED_OUTPUT directory"]
fn existing_real_data_generates_six_combined_figures_without_science() {
    let input=PathBuf::from(std::env::var_os("DFTFLOW_COMBINED_SOURCE_CONFIG").unwrap());
    let output=PathBuf::from(std::env::var_os("DFTFLOW_COMBINED_OUTPUT").unwrap());
    assert!(!output.exists());
    let mut cfg=Config::load(&input).unwrap();
    let source=cfg.work_dir.clone();
    let mut identities=vec![];
    cfg.work_dir=output.clone();
    layout::initialize(&cfg).unwrap();
    for (step,band,dos) in [
        (Step::Electronic,source.join("3.qe-electronic/qe_eband/plots/electronic-bands.plot.json"),
            source.join("3.qe-electronic/qe_eledos/plots/electronic-dos.plot.json")),
        (Step::Wannier,source.join("5.epw-wannier/comparison-native/electronic-bands.plot.json"),
            source.join("5.epw-wannier/comparison-native/electronic-dos.plot.json")),
    ] {
        let dir=step.directory(&cfg).join("plot");
        fs::create_dir_all(&dir).unwrap();
        for (source,name) in [(band,"electronic-bands.json"),(dos,"electronic-dos.json")] {
            identities.push(store::Identity::capture(&source).unwrap());
            fs::copy(source,dir.join(name)).unwrap();
        }
        combined_plot::electronic(&cfg,&dir,&dir).unwrap();
        let full:Value=store::read(&dir.join("electronic-from-lowest-to-highest.plot.json")).unwrap();
        let xml=source.join(format!("3.qe-electronic/qe_eband/out/{}.save/data-file-schema.xml",cfg.qe.prefix));
        let mesh=epwflow::qe_data::nscf::parse_xml(&xml).unwrap();
        let nscf=epwflow::qe_data::nscf::parse_xml(&source.join(format!("2.qe-nscf/out/{}.save/data-file-schema.xml",cfg.qe.prefix))).unwrap();
        let lo=mesh.eigenvalues.iter().flatten().copied().fold(f64::INFINITY,f64::min)-nscf.fermi;
        let hi=mesh.eigenvalues.iter().flatten().copied().fold(f64::NEG_INFINITY,f64::max)-nscf.fermi;
        assert!((full["bounds"][0].as_f64().unwrap()-lo).abs()<1e-7);
        assert!((full["bounds"][1].as_f64().unwrap()-hi).abs()<1e-7);
        identities.push(store::Identity::capture(&xml).unwrap());
    }
    let phonon=Step::Phonon.directory(&cfg);
    fs::create_dir_all(&phonon).unwrap();
    let original=source.join(phonon.file_name().unwrap());
    for name in [format!("{}.phband.freq",cfg.qe.prefix),format!("{}_band.labelinfo.dat",cfg.qe.prefix),format!("{}.phdos.dat",cfg.qe.prefix)] {
        identities.push(store::Identity::capture(&original.join(&name)).unwrap());
        fs::copy(original.join(&name),phonon.join(&name)).unwrap();
    }
    let plot=phonon.join("plot");fs::create_dir_all(&plot).unwrap();
    for name in [format!("{}.phband.freq",cfg.qe.prefix),format!("{}_band.labelinfo.dat",cfg.qe.prefix),format!("{}.phdos.dat",cfg.qe.prefix)] {
        fs::copy(phonon.join(&name),plot.join(name)).unwrap();
    }
    let crystal=epwflow::qe_data::qe_config::QeSettings::from_config(&cfg).unwrap().cell;
    combined_plot::qe_phonon(&cfg,&plot,&crystal.elements,&crystal.counts,cfg.comparison.clone().unwrap().phonon_sigma_cm1).unwrap();
    let results=output.join("results/6.epw-coarse");
    copy_tree(&source.join("results/6.epw-coarse/comparison"),&results.join("comparison"));
    combined_plot::coarse(&cfg,&results).unwrap();
    let mut images=vec![];
    for step in [Step::Electronic,Step::Phonon,Step::Wannier,Step::Coarse] {
        for entry in fs::read_dir(step.directory(&cfg).join("plot")).unwrap() {
            let path=entry.unwrap().path();
            if path.extension().is_some_and(|e|e=="png") {
                let mut decoder=png::Decoder::new(fs::File::open(&path).unwrap()).read_info().unwrap();
                let info=decoder.info();
                assert_eq!((info.width,info.height),(3600,2400));
                assert!(info.pixel_dims.unwrap().xppu>=11811);
                let _=&mut decoder;
                images.push(path);
            }
        }
    }
    assert_eq!(images.len(),6);
    for id in &identities { id.verify().unwrap(); }
    store::write(&output.join("verification.json"),&serde_json::json!({
        "images":images,"sources":identities,"source_data_unchanged":true,
        "scientific_program_calls":0,"slurm_submissions":0
    })).unwrap();
    println!("combined_real_output: {}",output.display());
}
