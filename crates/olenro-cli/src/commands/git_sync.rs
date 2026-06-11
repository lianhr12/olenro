//! Git sync commands (backed by `olenro_core::git_sync`).

use crate::errors::Result;

pub async fn status() -> Result<()> {
    let s = olenro_core::git_sync::status();
    if !s.is_repo {
        println!("Git sync: not configured (run `olenro git-sync configure`)");
        return Ok(());
    }
    println!("Git sync status:");
    println!("  branch: {}", s.branch.as_deref().unwrap_or("-"));
    println!("  remote: {}", s.remote.as_deref().unwrap_or("(none)"));
    println!("  state:  {}", if s.dirty { "dirty" } else { "clean" });
    Ok(())
}

pub async fn push() -> Result<()> {
    match olenro_core::git_sync::push("olenro: sync config") {
        Ok(msg) => println!("✓ {msg}"),
        Err(e) => println!("✗ push failed: {e}"),
    }
    Ok(())
}

pub async fn pull() -> Result<()> {
    match olenro_core::git_sync::pull() {
        Ok(msg) => println!("✓ pulled\n{msg}"),
        Err(e) => println!("✗ pull failed: {e}"),
    }
    Ok(())
}

pub async fn configure(repo: Option<String>, branch: &str) -> Result<()> {
    match olenro_core::git_sync::init(repo.as_deref(), branch) {
        Ok(_) => println!("✓ Git sync configured (branch {branch})"),
        Err(e) => println!("✗ configure failed: {e}"),
    }
    Ok(())
}
