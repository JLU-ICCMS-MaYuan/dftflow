//! 0070：衰减原始数据唯一保存，显式重绘不执行科学程序。
use super::*;

#[test]
fn decay_is_published_before_comparison_failure_and_explicit_redraw_uses_single_sources() {
    let c=Case::numbered();c.stage_setup();
    success(c.stage_submit("coarse",false));success(c.stage_execute());
    let native=c.root.join("run/6.epw-coarse");
    let result=c.root.join("run/results/6.epw-coarse");
    for name in epwflow::delivery::decay::NATIVE {
        assert!(native.join(name).is_file());assert!(!result.join(name).exists());
        assert!(!native.join("decay").join(name).exists());
    }
    assert!(!c.root.join("run/results/index.json").exists());
    assert!(!native.join("decay").exists());
    let calls=(c.calls(),c.post_calls());
    fs::remove_file(result.join("decay-overview.png")).unwrap();
    success(c.numbered_cli(1,&["--plot","--results",result.to_str().unwrap()],None,"decay-explicit-redraw"));
    assert!(fs::read(result.join("decay-overview.png")).unwrap().starts_with(b"\x89PNG"));
    assert_eq!(calls,(c.calls(),c.post_calls()));
    let pointer:Value=store::read(&result.join("decay.plot.json")).unwrap();
    assert!(pointer.get("panels").is_none());
    assert!(pointer["sources"].as_array().unwrap().iter().all(|p|!std::path::Path::new(p.as_str().unwrap()).is_absolute()));
}
