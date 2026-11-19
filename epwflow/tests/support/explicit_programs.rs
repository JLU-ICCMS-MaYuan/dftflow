//! 执行真实生成脚本的 argv 契约；科学求解器为隔离替身。
use super::*;

#[test]
fn explicit_programs_in_separate_directories_receive_only_their_own_args() {
    let c = Case::numbered();
    let mut cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for (index, name) in ["pw", "ph", "epw", "bands", "dos", "projwfc", "q2r", "matdyn", "postw90"].into_iter().enumerate() {
        let dir = c.root.join(format!("external programs/{name}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.x"));
        // 交替使用非空和空列表；空列表不得出现自动 pencil 或占位参数。
        let mut args = if index % 2 == 0 { vec![format!("--only-{name}"), "value with spaces".into()] } else { vec![] };
        let expected = args.clone();
        if name == "epw" { args.extend(["--io-contract".into(), "-npool".into(), "1".into()]); }
        let script = format!(r#"#!/usr/bin/python3
import json,os,sys
expected={expected}
assert sys.argv[1:1+len(expected)]==expected, sys.argv
rest=sys.argv[1+len(expected):]
assert '-pd' not in rest, sys.argv
with open({record},'a') as f: f.write(json.dumps([{name},sys.argv[1:]])+'\n')
original={original}
os.execv(original,[original]+rest)
"#, expected=serde_json::to_string(&expected).unwrap(), record=serde_json::to_string(&c.root.join("explicit-argv.jsonl")).unwrap(), name=serde_json::to_string(name).unwrap(), original=serde_json::to_string(&c.root.join(format!("{name}.x"))).unwrap());
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let program = config::Program { path, args };
        match name {
            "pw" => cfg.programs.pw = program,
            "ph" => cfg.programs.ph = program,
            "epw" => cfg.programs.epw = program,
            "bands" => cfg.programs.bands = Some(program),
            "dos" => cfg.programs.dos = Some(program),
            "projwfc" => cfg.programs.projwfc = Some(program),
            "q2r" => cfg.programs.q2r = Some(program),
            "matdyn" => cfg.programs.matdyn = Some(program),
            "postw90" => cfg.programs.postw90 = Some(program),
            _ => unreachable!(),
        }
    }
    c.put("input.toml", &toml::to_string_pretty(&cfg).unwrap());
    for (bin, stage, job) in [(0, "phonon", 201), (1, "wannier", 202), (1, "coarse", 203), (2, "fine", 204)] {
        c.accounting(Some(job));
        success(c.numbered_cli(bin, &["--stage", stage, "--resume", "-r"], Some(job), &format!("explicit-{stage}")));
    }
    let rows = fs::read_to_string(c.root.join("explicit-argv.jsonl")).unwrap();
    for name in ["pw", "ph", "epw", "bands", "dos", "projwfc", "q2r", "matdyn", "postw90"] {
        assert!(rows.lines().any(|line| serde_json::from_str::<Value>(line).unwrap()[0] == name), "missing execution: {name}");
    }
}
