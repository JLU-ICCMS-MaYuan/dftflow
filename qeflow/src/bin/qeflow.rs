//! qeflow 命令入口；实现由可复用库维护。
fn main() -> anyhow::Result<()> {
    qeflow::run_cli()
}
