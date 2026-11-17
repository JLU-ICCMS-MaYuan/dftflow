use anyhow::{bail, Context, Result};
use std::{env, fs, path::{Component, Path, PathBuf}};

/// 输入路径按配置位置解析；保留软链接路径以便每次校验其目标。
pub fn absolute(base: &Path, input: &Path) -> Result<PathBuf> {
    let s=input.to_str().context("configuration: path is not UTF-8")?;
    if s.is_empty() || s.contains('\0') { bail!("configuration: empty path or NUL character"); }
    let expanded=if s=="~" || s.starts_with("~/") {
        let home=env::var_os("HOME").context("configuration: HOME is undefined")?;
        PathBuf::from(home).join(s.strip_prefix("~/").unwrap_or(""))
    } else {input.to_path_buf()};
    let joined=if expanded.is_absolute(){expanded}else{base.join(expanded)};
    let mut out=PathBuf::new();
    for c in joined.components(){match c {
        Component::CurDir=>{},
        Component::ParentDir=>{ // 不在软链接前错误地消除 ..
            out=fs::canonicalize(&out).with_context(||format!("configuration: cannot resolve parent directory of {}",out.display()))?;
            if !out.pop(){bail!("configuration: path escapes the root directory");}
        },
        _=>out.push(c.as_os_str()),
    }}
    Ok(out)
}
/// 非现存输出路径也以真实现存祖先确定身份，防止输出目录通过别名绕过身份核对。
pub fn destination(path:&Path)->Result<PathBuf>{
    if path.exists(){return fs::canonicalize(path).with_context(||format!("configuration: {}",path.display()));}
    if fs::symlink_metadata(path).is_ok(){bail!("configuration: dangling symlink {}",path.display());}
    let parent=path.parent().context("configuration: output path has no parent directory")?;
    Ok(destination(parent)?.join(path.file_name().context("configuration: output path has no filename")?))
}
pub fn existing(base:&Path, path:&Path, field:&str, executable:bool)->Result<PathBuf>{
    let p=absolute(base,path)?;
    let m=fs::metadata(&p).with_context(||format!("configuration: {field}: file unreadable {}",p.display()))?;
    if !m.is_file(){bail!("configuration: {field}: not a regular file");}
    fs::File::open(&p).with_context(||format!("configuration: {field}: file unreadable"))?;
    #[cfg(unix)] if executable {
        use std::os::unix::fs::PermissionsExt;
        if m.permissions().mode()&0o111==0 {bail!("environment: {field}: not executable");}
    }
    Ok(p)
}
pub fn command(base:&Path, value:&str)->Result<PathBuf>{
    if value.contains('/') || value.starts_with('~'){return existing(base,Path::new(value),"programs.launcher",true);}
    for p in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
        let p=absolute(base,&p)?.join(value);
        if p.is_file(){return existing(base,&p,"programs.launcher",true);}
    }
    bail!("environment: programs.launcher: {value} not found in PATH")
}
