use anyhow::{bail,ensure,Context,Result};
use serde::{de::DeserializeOwned,Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{fs::{self,File,OpenOptions},io::{Read,Write},path::{Path,PathBuf},sync::atomic::{AtomicU64,Ordering},time::{SystemTime,UNIX_EPOCH}};
static SEQUENCE:AtomicU64=AtomicU64::new(0);
pub fn token()->String {format!("{:x}-{:x}-{:x}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos(),std::process::id(),SEQUENCE.fetch_add(1,Ordering::Relaxed))}
pub fn hash_bytes(bytes:&[u8])->String {format!("{:x}",Sha256::digest(bytes))}
pub fn hash_file(path:&Path)->Result<String>{let mut f=File::open(path).with_context(||format!("data_invalid: 无法读取 {}",path.display()))?;let mut hash=Sha256::new();let mut buf=[0u8;65536];loop{let n=f.read(&mut buf)?;if n==0{break}hash.update(&buf[..n]);}Ok(format!("{:x}",hash.finalize()))}
pub fn read<T:DeserializeOwned>(path:&Path)->Result<T>{serde_json::from_slice(&fs::read(path).with_context(||format!("data_invalid: {} 缺失或不可读",path.display()))?).with_context(||format!("data_invalid: {} 损坏/不支持",path.display()))}
pub fn atomic_bytes(path:&Path,bytes:&[u8])->Result<()>{
    let parent=path.parent().context("data_invalid: 文件没有父目录")?;fs::create_dir_all(parent)?;
    ensure!(!fs::symlink_metadata(path).is_ok_and(|m|m.file_type().is_symlink()),"input_conflict: 拒绝替换符号链接 {}",path.display());
    let tmp=parent.join(format!(".epw-{}.tmp",token()));
    let mut file=OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    file.write_all(bytes)?;file.sync_all()?;fs::rename(&tmp,path)?;File::open(parent)?.sync_all()?;Ok(())
}
pub fn write<T:Serialize>(path:&Path,data:&T)->Result<()> {let mut bytes=serde_json::to_vec_pretty(data)?;bytes.push(b'\n');atomic_bytes(path,&bytes)}
#[derive(Debug,Clone,Serialize,Deserialize)] #[serde(deny_unknown_fields)]
pub struct Identity {pub path:PathBuf,pub target:PathBuf,pub sha256:String,pub bytes:u64}
impl Identity {
    pub fn capture(path:&Path)->Result<Self>{let target=fs::canonicalize(path).with_context(||format!("input_conflict: 无法定位 {}",path.display()))?;let bytes=fs::metadata(&target)?.len();let sha256=hash_file(path)?;Ok(Self{path:path.into(),target,sha256,bytes})}
    pub fn verify(&self)->Result<()>{let now=Self::capture(&self.path)?;ensure!(now.target==self.target && now.sha256==self.sha256&&now.bytes==self.bytes,"input_conflict: 文件或链接目标已改变 {}",self.path.display());Ok(())}
}
#[derive(Serialize,Deserialize)] #[serde(deny_unknown_fields)]
struct Owner {host:String,pid:u32,token:String,created_unix:u64,#[serde(default)]job_id:Option<u64>}
pub struct Lock {path:PathBuf,token:String}
impl Lock {
    pub fn acquire(target:&Path)->Result<Self>{
        let path=target.with_file_name(format!("{}.lock",target.file_name().context("data_invalid: 锁对象缺失名称")?.to_string_lossy()));
        fs::create_dir_all(path.parent().context("data_invalid: 锁路径")?)?;
        if let Err(e)=fs::create_dir(&path){
            if e.kind()==std::io::ErrorKind::AlreadyExists {bail!("scheduler_unknown: 锁 {} 已存在；核对 owner 和未决提交后恢复，不能删除未知锁",path.display());}
            return Err(e.into())
        }
        let t=token(); let owner=Owner{host:hostname(),pid:std::process::id(),token:t.clone(),created_unix:now(),job_id:std::env::var("SLURM_JOB_ID").ok().and_then(|s|s.parse().ok())};
        write(&path.join("owner.json"),&owner)?;Ok(Self{path,token:t})
    }
}
/// 仅显式恢复使用：所有者必须属于指定旧分配，且该分配有明确终态证据。
pub fn recover_finished_lock(target:&Path,job:u64)->Result<()> {
    recover_lock_with(target,job,||{
        ensure!(super::slurm::accounting(job)?.terminal,"scheduler_unknown: 原锁所属分配仍活动");Ok(())
    })
}
fn recover_lock_with(target:&Path,job:u64,terminal:impl FnOnce()->Result<()>)->Result<()> {
    let path=target.with_file_name(format!("{}.lock",target.file_name().context("锁路径")?.to_string_lossy()));
    if !path.exists(){return Ok(());}
    ensure!(!fs::symlink_metadata(&path)?.file_type().is_symlink(),"scheduler_unknown: 锁目录是链接");
    let owner_path=path.join("owner.json");let identity=Identity::capture(&owner_path)?;
    let owner:Owner=read(&owner_path)?;
    ensure!(owner.job_id==Some(job),"scheduler_unknown: 锁不属于原分配，不能清除未知锁");
    ensure!(fs::read_dir(&path)?.count()==1,"scheduler_unknown: 锁含未知现场");
    terminal()?;identity.verify()?;
    fs::remove_file(owner_path)?;fs::remove_dir(path)?;Ok(())
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn lock_recovery_requires_matching_allocation_and_confirmed_terminal(){
        let root=std::env::temp_dir().join(format!("epw-lock-{}",token()));fs::create_dir(&root).unwrap();
        let target=root.join("execution");let lock=root.join("execution.lock");fs::create_dir(&lock).unwrap();
        let owner=Owner{host:"fixture".into(),pid:1,token:"fixture".into(),created_unix:0,job_id:Some(42)};
        write(&lock.join("owner.json"),&owner).unwrap();
        assert!(recover_lock_with(&target,41,||panic!("错误分配不查询或清锁")).is_err());
        assert!(recover_lock_with(&target,42,||bail!("scheduler_unknown: active")).is_err());
        assert!(lock.exists());recover_lock_with(&target,42,||Ok(())).unwrap();assert!(!lock.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
impl Drop for Lock {fn drop(&mut self){
    // 仅移除自己持有且内容完整的锁；异常现场保留。
    if let Ok(owner)=read::<Owner>(&self.path.join("owner.json")){if owner.token==self.token {let _=fs::remove_file(self.path.join("owner.json"));let _=fs::remove_dir(&self.path);}}
}}
pub fn hostname()->String{let mut name=[0u8;256];#[cfg(unix)] unsafe{if libc::gethostname(name.as_mut_ptr().cast(),name.len())==0 {return String::from_utf8_lossy(&name[..name.iter().position(|b|*b==0).unwrap_or(name.len())]).into_owned();}}"unknown".into()}
pub fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
/// 状态根：$XDG_STATE_HOME/epwflow；仓库本地用 EPWFLOW_LOCAL_STATE 覆盖（gitignored）。
pub fn state_root()->Result<PathBuf>{
    if let Ok(local)=std::env::var("EPWFLOW_LOCAL_STATE"){return Ok(PathBuf::from(local));}
    let base=match std::env::var("XDG_STATE_HOME"){Ok(v) if !v.is_empty()=>PathBuf::from(v),_=>{let home=std::env::var("HOME").unwrap_or_else(|_|".".to_string());PathBuf::from(home).join(".local").join("state")}};
    Ok(base.join("epwflow"))
}
