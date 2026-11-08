//! Helper da fatia 1: roda o binário `freitask` com HOME num diretório
//! temporário e devolve stdout, stderr e código já decodificados.
//!
//! Regras de `docs/agents/aceite.md`: `env_clear()`, depois `HOME`, `PATH`,
//! `LANG=C.UTF-8`, `LC_ALL=C.UTF-8`, `FREITASK_TODAY`; `current_dir` no HOME.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use tempfile::TempDir;

/// O dia fixo de todos os testes desta fatia.
pub const HOJE: &str = "2026-11-08";

/// Raiz do repo: `FREITASK_REPO_RAIZ` no rascunho; congelado, dois níveis
/// acima do manifesto do pacote `aceite`.
pub fn raiz_do_repo() -> String {
    std::env::var("FREITASK_REPO_RAIZ")
        .unwrap_or_else(|_| format!("{}/../..", env!("CARGO_MANIFEST_DIR")))
}

/// O binário sob teste, compilado antes por `make aceite`/`rascunho.sh`.
pub fn binario() -> PathBuf {
    PathBuf::from(format!("{}/target/debug/freitask", raiz_do_repo()))
}

/// Um HOME temporário sem `ObsidianVault/`.
pub fn home_sem_vault() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// Um HOME temporário com `ObsidianVault/` vazio (sem `.freitask/`, sem
/// `projects/`).
pub fn home_com_vault_vazio() -> TempDir {
    let home = home_sem_vault();
    fs::create_dir_all(vault(home.path())).expect("criar ObsidianVault");
    home
}

/// Um HOME temporário com `ObsidianVault/.freitask/status.json` dado.
pub fn home_com_status_json(conteudo: &str) -> TempDir {
    let home = home_com_vault_vazio();
    escreve_status_json(home.path(), conteudo);
    home
}

/// Grava (ou regrava) `ObsidianVault/.freitask/status.json`.
pub fn escreve_status_json(home: &Path, conteudo: &str) {
    let dir = vault(home).join(".freitask");
    fs::create_dir_all(&dir).expect("criar .freitask");
    fs::write(dir.join("status.json"), conteudo).expect("gravar status.json");
}

/// `<HOME>/ObsidianVault`.
pub fn vault(home: &Path) -> PathBuf {
    home.join("ObsidianVault")
}

/// O que um comando devolveu, já em texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saida {
    pub codigo: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Monta o comando com o ambiente limpo que o doc manda.
pub fn comando(home: &Path) -> Command {
    let mut cmd = Command::new(binario());
    cmd.env_clear()
        .env("HOME", home)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("FREITASK_TODAY", HOJE)
        .current_dir(home);
    cmd
}

/// Roda `freitask <args>` com HOME em `home`.
pub fn roda(home: &Path, args: &[&str]) -> Saida {
    let saida = comando(home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("não consegui rodar {}: {e}", binario().display()));
    Saida {
        codigo: saida.status.code(),
        stdout: String::from_utf8(saida.stdout).expect("stdout não é UTF-8"),
        stderr: String::from_utf8(saida.stderr).expect("stderr não é UTF-8"),
    }
}

/// Decodifica o stdout como JSON; o teste compara o valor, nunca o texto.
pub fn json(saida: &Saida) -> serde_json::Value {
    serde_json::from_str(&saida.stdout)
        .unwrap_or_else(|e| panic!("stdout não é JSON ({e}):\n{}", saida.stdout))
}

/// Foto de uma árvore: caminho relativo → (é diretório, bytes, mtime). Serve
/// para provar que uma leitura não mudou nenhum byte nem mtime (I6).
pub type Foto = BTreeMap<PathBuf, (bool, Vec<u8>, SystemTime)>;

pub fn fotografa(raiz: &Path) -> Foto {
    let mut foto = Foto::new();
    if !raiz.exists() {
        return foto;
    }
    let mut pilha = vec![raiz.to_path_buf()];
    while let Some(dir) = pilha.pop() {
        for entrada in fs::read_dir(&dir).expect("read_dir") {
            let entrada = entrada.expect("entrada");
            let caminho = entrada.path();
            let meta = fs::metadata(&caminho).expect("metadata");
            let rel = caminho.strip_prefix(raiz).expect("prefixo").to_path_buf();
            let mtime = meta.modified().expect("mtime");
            if meta.is_dir() {
                foto.insert(rel, (true, Vec::new(), mtime));
                pilha.push(caminho);
            } else {
                let bytes = fs::read(&caminho).expect("ler arquivo");
                foto.insert(rel, (false, bytes, mtime));
            }
        }
    }
    foto
}

/// Os nomes das entradas diretas de um diretório, em ordem.
pub fn entradas(dir: &Path) -> Vec<String> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut nomes: Vec<String> = fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| {
            e.expect("entrada")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    nomes.sort();
    nomes
}
