//! `freitask current` (spec §2.1, o shim de hoje): `exec nvim
//! "$HOME/ObsidianVault/CURRENT.md"`. Roda o binário de verdade, com um `nvim`
//! falso num `PATH` temporário e um HOME que o teste confere vazio no fim.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;

/// Escrever um executável e dar `exec` nele em outra thread que faz `fork` no
/// meio da escrita dá `ETXTBSY` (o filho herda o descritor aberto até o seu
/// `exec`). Quem escreve o script e quem dá `spawn` toma esta trava.
static TRAVA: Mutex<()> = Mutex::new(());

/// Roda `freitask current` com ambiente limpo (só `HOME` e `PATH`) e devolve o
/// pid do processo que o teste lançou e a saída dele. `antes` roda sob a
/// trava, antes do `spawn`, para montar o `nvim` falso.
fn roda_current(home: &Path, path: &Path, antes: impl FnOnce()) -> (u32, Output) {
    let filho = {
        let _trava = TRAVA.lock().unwrap_or_else(|e| e.into_inner());
        antes();
        Command::new(env!("CARGO_BIN_EXE_freitask"))
            .arg("current")
            .env_clear()
            .env("HOME", home)
            .env("PATH", path)
            .current_dir(home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let pid = filho.id();
    (pid, filho.wait_with_output().unwrap())
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn nada_em(dir: &Path) -> bool {
    fs::read_dir(dir).unwrap().next().is_none()
}

#[test]
fn current_executa_o_nvim_no_current_md_sem_criar_nada() {
    let home = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let args = bin.path().join("args.txt");
    let pid = bin.path().join("pid.txt");
    let (pid_do_freitask, saida) = roda_current(home.path(), bin.path(), || {
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$#\" \"$@\" > '{}'\necho $$ > '{}'\necho nvim falso\necho erro do nvim falso >&2\nexit 7\n",
            args.display(),
            pid.display()
        );
        let nvim = bin.path().join("nvim");
        fs::write(&nvim, script).unwrap();
        fs::set_permissions(&nvim, fs::Permissions::from_mode(0o755)).unwrap();
    });

    // Um só argumento, o CURRENT.md do vault do HOME, sem `--clean` nem nada.
    assert_eq!(
        fs::read_to_string(&args).unwrap(),
        format!("1\n{}/ObsidianVault/CURRENT.md\n", home.path().display())
    );
    // A saída e o código são os do `nvim` falso, e o `nvim` é o mesmo processo
    // que o teste lançou: `exec`, não filho.
    assert_eq!(saida.status.code(), Some(7));
    assert_eq!(texto(&saida.stdout), "nvim falso\n");
    assert_eq!(texto(&saida.stderr), "erro do nvim falso\n");
    assert_eq!(
        fs::read_to_string(&pid).unwrap().trim(),
        pid_do_freitask.to_string()
    );
    // Não confere se o arquivo existe, e não cria nem o vault.
    assert!(nada_em(home.path()), "o HOME tinha de continuar vazio");
}

#[test]
fn current_sem_nvim_no_path_sai_2_com_a_mensagem_do_shim() {
    let home = tempfile::tempdir().unwrap();
    let vazio = tempfile::tempdir().unwrap();
    let (_, saida) = roda_current(home.path(), vazio.path(), || {});
    assert_eq!(saida.status.code(), Some(2));
    assert_eq!(texto(&saida.stdout), "");
    assert_eq!(
        texto(&saida.stderr),
        "freitask: nvim não encontrado no PATH\n"
    );
    assert!(nada_em(home.path()));
}

#[test]
fn current_com_nvim_que_nao_executa_sai_2_e_diz_o_erro() {
    // Um `nvim` sem permissão de execução: o `exec` falha com outro erro que
    // não `NotFound`.
    let home = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let (_, saida) = roda_current(home.path(), bin.path(), || {
        let nvim = bin.path().join("nvim");
        fs::write(&nvim, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&nvim, fs::Permissions::from_mode(0o644)).unwrap();
    });
    assert_eq!(saida.status.code(), Some(2));
    assert_eq!(texto(&saida.stdout), "");
    assert_eq!(
        texto(&saida.stderr),
        "freitask: não consegui executar o nvim: Permission denied (os error 13)\n"
    );
    assert!(nada_em(home.path()));
}
