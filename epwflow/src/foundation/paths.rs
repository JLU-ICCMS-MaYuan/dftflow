use anyhow::{bail, Context, Result};
use std::{env, fs, path::{Component, Path, PathBuf}};

/// 输入路径按配置位置解析；保留软链接路径以便每次校验其目标。
pub fn absolute(base: &Path, input: &Path) -> Result<PathBuf> {
    let s=input.to_str().context("configuration: 路径不是 UTF-8")?;
    if s.is_empty() || s.contains('\0') { bail!("configuration: 空路径或 NUL"); }
    let expanded=if s=="~" || s.starts_with("~/") {
        let home=env::var_os("HOME").context("configuration: HOME 未定义")?;
        PathBuf::from(home).join(s.strip_prefix("~/").unwrap_or(""))
    } else {input.to_path_buf()};
    let joined=if expanded.is_absolute(){expanded}else{base.join(expanded)};
    let mut out=PathBuf::new();
    for c in joined.components(){match c {
        Component::CurDir=>{},
        Component::ParentDir=>{ // 不在软链接前错误地消除 ..
            out=fs::canonicalize(&out).with_context(||format!("configuration: 无法解析 {} 的父目录",out.display()))?;
            if !out.pop(){bail!("configuration: 路径越过根目录");}
        },
        _=>out.push(c.as_os_str()),
    }}
    Ok(out)
}
/// 非现存输出路径也以真实现存祖先确定身份，防止别名创建第二账本。
pub fn destination(path:&Path)->Result<PathBuf>{
    if path.exists(){return fs::canonicalize(path).with_context(||format!("configuration: {}",path.display()));}
    if fs::symlink_metadata(path).is_ok(){bail!("configuration: 悬空链接 {}",path.display());}
    let parent=path.parent().context("configuration: 输出缺少父目录")?;
    Ok(destination(parent)?.join(path.file_name().context("configuration: 输出缺少文件名")?))
}
pub fn existing(base:&Path, path:&Path, field:&str, executable:bool)->Result<PathBuf>{
    let p=absolute(base,path)?;
    let m=fs::metadata(&p).with_context(||format!("configuration: {field}: 文件不可读 {}",p.display()))?;
    if !m.is_file(){bail!("configuration: {field}: 不是普通文件");}
    fs::File::open(&p).with_context(||format!("configuration: {field}: 文件不可读"))?;
    #[cfg(unix)] if executable {
        use std::os::unix::fs::PermissionsExt;
        if m.permissions().mode()&0o111==0 {bail!("environment: {field}: 不可执行");}
    }
    Ok(p)
}
pub fn command(base:&Path, value:&str)->Result<PathBuf>{
    if value.contains('/') || value.starts_with('~'){return existing(base,Path::new(value),"programs.launcher",true);}
    for p in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
        let p=absolute(base,&p)?.join(value);
        if p.is_file(){return existing(base,&p,"programs.launcher",true);}
    }
    bail!("environment: programs.launcher: PATH 中找不到 {value}")
}
pub fn state_root()->Result<PathBuf>{
    let root=if let Some(v)=env::var_os("XDG_STATE_HOME"){PathBuf::from(v)}else{PathBuf::from(env::var_os("HOME").context("environment: HOME 未定义")?).join(".local/state")};
    if !root.is_absolute(){bail!("configuration: XDG_STATE_HOME 必须是绝对路径");}
    destination(&root.join("epwflow"))
}
