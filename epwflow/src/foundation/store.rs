use anyhow::{bail,ensure,Context,Result};
use serde::{de::DeserializeOwned,Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{fs::{self,File,OpenOptions},io::{Read,Write},path::{Path,PathBuf},sync::atomic::{AtomicU64,Ordering},time::{SystemTime,UNIX_EPOCH}};
static SEQUENCE:AtomicU64=AtomicU64::new(0);
pub fn token()->String {format!("{:x}-{:x}-{:x}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos(),std::process::id(),SEQUENCE.fetch_add(1,Ordering::Relaxed))}
pub fn hash_bytes(bytes:&[u8])->String {format!("{:x}",Sha256::digest(bytes))}
pub fn hash_file(path:&Path)->Result<String>{let mut f=File::open(path).with_context(||format!("data_invalid: 无法读取 {}",path.display()))?;let mut hash=Sha256::new();let mut buf=vec![0u8;1024*1024];loop{let n=f.read(&mut buf)?;if n==0{break}hash.update(&buf[..n]);}Ok(format!("{:x}",hash.finalize()))}
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
/// 仅文件确实不存在才返回不可用；权限、链接目标及内容冲突仍返回原错误。
pub fn identity_available(id: &Identity) -> Result<bool> {
    match fs::metadata(&id.path) {
        Ok(_) => { id.verify()?; Ok(true) }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match fs::symlink_metadata(&id.path) {
                Ok(_) => { id.verify()?; Ok(false) }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(e.into()),
            }
        }
        Err(e) => Err(e.into()),
    }
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

    /// 先移走完整的所属锁，再清理；清理失败不得留下空的公共锁路径。
    fn release_with(&self, cleanup: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
        if !self.path.exists() { return Ok(()); }
        ensure!(!fs::symlink_metadata(&self.path)?.file_type().is_symlink(), "scheduler_unknown: 锁目录是链接");
        let owner_path = self.path.join("owner.json");
        ensure!(!fs::symlink_metadata(&owner_path)?.file_type().is_symlink(), "scheduler_unknown: 锁 owner 是链接");
        let owner: Owner = read(&owner_path)?;
        if owner.token != self.token { return Ok(()); }
        let entries = fs::read_dir(&self.path)?.collect::<std::io::Result<Vec<_>>>()?;
        ensure!(entries.len() == 1 && entries[0].file_name() == "owner.json", "scheduler_unknown: 所属锁含未知现场，保留原锁");
        // Unix DirEntry 可持有目录句柄，移动和清理前先释放。
        drop(entries);
        let retired = self.path.parent().context("锁缺少父目录")?.join(format!("epw-released-{}", self.token));
        ensure!(!retired.try_exists()?, "scheduler_unknown: 锁退役路径已存在");
        fs::rename(&self.path, &retired)?;
        cleanup(&retired)
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
    ensure!(owner.job_id==Some(job),
        "scheduler_unknown: 工作区锁 {} 的所有者 host={}、pid={}、Slurm job={:?}，与旧作业 {job} 不同；另一条提交或生成命令可能仍在执行，请等待其结束。未核实的锁不能自动删除",
        path.display(),owner.host,owner.pid,owner.job_id);
    ensure!(fs::read_dir(&path)?.count()==1,"scheduler_unknown: 锁含未知现场");
    terminal()?;identity.verify()?;
    let recovered = Lock {path, token: owner.token};
    recovered.release_with(cleanup_retired_lock)
}

fn cleanup_retired_lock(retired: &Path) -> Result<()> {
    fs::remove_file(retired.join("owner.json"))?;
    fs::remove_dir(retired)?;
    Ok(())
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn released_lock_cleanup_failure_never_blocks_or_removes_the_next_owner() {
        let root=std::env::temp_dir().join(format!("epw-release-{}",token()));fs::create_dir(&root).unwrap();
        let target=root.join("execution");let first=Lock::acquire(&target).unwrap();
        let mut retired=None;
        assert!(first.release_with(|path| {retired=Some(path.to_path_buf());bail!("fixture: unlink failed")}).is_err());
        let retired=retired.unwrap();
        assert_eq!(read::<Owner>(&retired.join("owner.json")).unwrap().token,first.token);
        let second=Lock::acquire(&target).unwrap();
        drop(first);
        assert_eq!(read::<Owner>(&second.path.join("owner.json")).unwrap().token,second.token);
        drop(second);
        assert!(!root.join("execution.lock").exists());
        fs::remove_file(retired.join("owner.json")).unwrap();fs::remove_dir(retired).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn releasing_a_lock_preserves_unknown_contents_and_owner() {
        let root=std::env::temp_dir().join(format!("epw-release-unknown-{}",token()));fs::create_dir(&root).unwrap();
        let target=root.join("execution");let lock=Lock::acquire(&target).unwrap();
        let unknown=lock.path.join("unknown");fs::write(&unknown,"keep").unwrap();
        assert!(lock.release_with(|_|panic!("未知现场不得清理")).is_err());
        assert!(lock.path.join("owner.json").is_file() && unknown.is_file());
        fs::remove_file(unknown).unwrap();drop(lock);fs::remove_dir(root).unwrap();
    }
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
    if let Err(error) = self.release_with(cleanup_retired_lock) { eprintln!("lock_release: {}: {error:#}", self.path.display()); }
}}
pub fn hostname()->String{let mut name=[0u8;256];#[cfg(unix)] unsafe{if libc::gethostname(name.as_mut_ptr().cast(),name.len())==0 {return String::from_utf8_lossy(&name[..name.iter().position(|b|*b==0).unwrap_or(name.len())]).into_owned();}}"unknown".into()}
pub fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
/// 预算注册表默认放在可见的 ~/epwflow-state；保留显式环境覆盖。
pub fn state_root()->Result<PathBuf>{
    if let Ok(local)=std::env::var("EPWFLOW_LOCAL_STATE"){return Ok(PathBuf::from(local));}
    let base=match std::env::var("XDG_STATE_HOME"){Ok(v) if !v.is_empty()=>PathBuf::from(v),_=>{let home=std::env::var("HOME").unwrap_or_else(|_|".".to_string());return Ok(PathBuf::from(home).join("epwflow-state"))}};
    Ok(base.join("epwflow"))
}

#[cfg(test)]
mod hash_tests {
    use super::*;
    #[test]
    fn streaming_hash_keeps_full_content_and_non_aligned_tail_identity() {
        let path = std::env::temp_dir().join(format!("epw-stream-hash-{}", token()));
        let mut bytes: Vec<u8> = (0..3 * 1024 * 1024 + 17).map(|i| (i % 251) as u8).collect();
        fs::write(&path, &bytes).unwrap();
        assert_eq!(hash_file(&path).unwrap(), hash_bytes(&bytes));
        let id = Identity::capture(&path).unwrap();
        id.verify().unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&path, &bytes).unwrap();
        assert!(id.verify().is_err(), "最后一个非整块字节变化也必须拒绝");
        fs::remove_file(path).unwrap();
    }
}
