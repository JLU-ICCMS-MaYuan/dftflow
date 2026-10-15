use anyhow::{bail,ensure,Context,Result};
use serde::{de::DeserializeOwned,Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{fs::{self,File,OpenOptions},io::{Read,Write},path::{Path,PathBuf},sync::atomic::{AtomicU64,Ordering},time::{SystemTime,UNIX_EPOCH}};
static SEQUENCE:AtomicU64=AtomicU64::new(0);
pub fn token()->String {format!("{:x}-{:x}-{:x}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos(),std::process::id(),SEQUENCE.fetch_add(1,Ordering::Relaxed))}
pub fn hash_bytes(bytes:&[u8])->String {format!("{:x}",Sha256::digest(bytes))}
pub fn hash_file(path:&Path)->Result<String>{let mut f=File::open(path).with_context(||format!("data_invalid: 无法读取 {}",path.display()))?;let mut hash=Sha256::new();let mut buf=[0u8;65536];loop{let n=f.read(&mut buf)?;if n==0{break}hash.update(&buf[..n]);}Ok(format!("{hash:x}"))}
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
struct Owner {host:String,pid:u32,token:String,created_unix:u64}
pub struct Lock {path:PathBuf,token:String}
impl Lock {
    pub fn acquire(target:&Path)->Result<Self>{
        let path=target.with_file_name(format!("{}.lock",target.file_name().context("data_invalid: 锁对象缺失名称")?.to_string_lossy()));
        fs::create_dir_all(path.parent().context("data_invalid: 锁路径")?)?;
        if let Err(e)=fs::create_dir(&path){
            if e.kind()==std::io::ErrorKind::AlreadyExists {bail!("scheduler_unknown: 锁 {} 已存在；核对 owner 和未决提交后恢复，不能删除未知锁",path.display());}
            return Err(e.into())
        }
        let t=token(); let owner=Owner{host:hostname(),pid:std::process::id(),token:t.clone(),created_unix:now()};
        write(&path.join("owner.json"),&owner)?;Ok(Self{path,token:t})
    }
}
impl Drop for Lock {fn drop(&mut self){
    // 仅移除自己持有且内容完整的锁；异常现场保留。
    if let Ok(owner)=read::<Owner>(&self.path.join("owner.json")){if owner.token==self.token {let _=fs::remove_file(self.path.join("owner.json"));let _=fs::remove_dir(&self.path);}}
}}
pub fn hostname()->String{let mut name=[0u8;256];#[cfg(unix)] unsafe{if libc::gethostname(name.as_mut_ptr().cast(),name.len())==0 {return String::from_utf8_lossy(&name[..name.iter().position(|b|*b==0).unwrap_or(name.len())]).into_owned();}}"unknown".into()}
pub fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
