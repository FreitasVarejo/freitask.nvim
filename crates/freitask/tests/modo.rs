//! A gravação preserva o modo do arquivo que substitui: o vault real tem tasks
//! em 0600 e o Syncthing sincroniza permissão, então reescrever uma task não
//! pode trocar o 0600 pelo 0644 do umask. Roda o binário de verdade, com um
//! HOME temporário.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn modo(p: &Path) -> u32 {
    fs::metadata(p).unwrap().permissions().mode() & 0o7777
}

fn freitask(home: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_freitask"))
        .args(args)
        .env_clear()
        .env("HOME", home)
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("FREITASK_TODAY", "2026-11-10")
        .current_dir(home)
        .output()
        .unwrap()
}

fn home_com_task(m: u32) -> (tempfile::TempDir, std::path::PathBuf) {
    let home = tempfile::tempdir().unwrap();
    let task = home.path().join("ObsidianVault/projects/a/tasks/um.md");
    fs::create_dir_all(task.parent().unwrap()).unwrap();
    fs::write(&task, "> [!todo] Um\n> [[um]]\n").unwrap();
    fs::set_permissions(&task, fs::Permissions::from_mode(m)).unwrap();
    (home, task)
}

#[test]
fn set_numa_task_0600_continua_0600() {
    let (home, task) = home_com_task(0o600);
    let s = freitask(home.path(), &["set", "um", "--title", "Novo"]);
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    assert!(fs::read_to_string(&task).unwrap().contains("Novo"));
    assert_eq!(modo(&task), 0o600);
}

#[test]
fn set_numa_task_0644_continua_0644() {
    let (home, task) = home_com_task(0o644);
    let s = freitask(home.path(), &["set", "um", "--title", "Novo"]);
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    assert_eq!(modo(&task), 0o644);
}

#[test]
fn rename_leva_o_modo_junto() {
    let (home, task) = home_com_task(0o600);
    let s = freitask(home.path(), &["rename", "um", "dois"]);
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    let nova = task.with_file_name("dois.md");
    assert!(nova.is_file() && !task.exists());
    assert_eq!(
        modo(&nova),
        0o600,
        "rename regrava o arquivo no endereço novo"
    );
}

#[test]
fn task_nova_nasce_com_o_modo_do_umask() {
    let (home, task) = home_com_task(0o600);
    let s = freitask(home.path(), &["new", "a", "dois", "Dois"]);
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    let nova = task.with_file_name("dois.md");
    // O umask decide: o esperado é o de um arquivo qualquer criado agora.
    let referencia = home.path().join("referencia");
    fs::write(&referencia, b"x").unwrap();
    assert_eq!(modo(&nova), modo(&referencia));
    assert_eq!(modo(&task), 0o600, "a outra task não é tocada");
}
