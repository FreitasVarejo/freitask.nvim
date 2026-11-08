//! Rodar um passo num lado (A ou B) e tirar o manifesto da árvore.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::NaiveDate;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub enum Runner {
    /// O freitask de hoje: `nvim --clean --cmd "set runtimepath+=<repo>" -l <repo>/lua/freitask/cli.lua …`.
    Oraculo { repo: PathBuf },
    /// O binário novo.
    Binario(PathBuf),
}

pub struct Lado {
    pub nome: &'static str,
    pub home: PathBuf,
    pub runner: Runner,
    pub hoje: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saida {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub codigo: i32,
}

impl Lado {
    pub fn rodar(&self, args: &[String]) -> Saida {
        let mut cmd = match &self.runner {
            Runner::Oraculo { repo } => {
                let mut c = Command::new("nvim");
                c.arg("--clean")
                    .arg("--cmd")
                    .arg(format!("set runtimepath+={}", repo.display()))
                    .arg("-l")
                    .arg(repo.join("lua/freitask/cli.lua"));
                c
            }
            Runner::Binario(bin) => Command::new(bin),
        };
        cmd.args(args);
        cmd.current_dir(&self.home);
        // O mesmo ambiente dos dois lados: só HOME muda entre A e B. TZ é
        // herdado (igual para ambos); LANG fixo para o `%A` do painel sair em
        // inglês nos dois; FREITASK_TODAY fixa o dia no binário novo (o oráculo
        // o ignora e lê o relógio, que o harness confere não ter virado).
        cmd.env_clear();
        cmd.env("HOME", &self.home);
        cmd.env("PATH", std::env::var_os("PATH").unwrap_or_default());
        if let Some(tz) = std::env::var_os("TZ") {
            cmd.env("TZ", tz);
        }
        cmd.env("LANG", "C.UTF-8");
        cmd.env("LC_ALL", "C.UTF-8");
        cmd.env("FREITASK_TODAY", self.hoje.format("%Y-%m-%d").to_string());
        cmd.env("XDG_RUNTIME_DIR", self.home.join(".runtime"));
        let _ = std::fs::create_dir_all(self.home.join(".runtime"));
        cmd.stdin(std::process::Stdio::null());
        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("rodar {:?} no lado {}: {e}", self.runner, self.nome));
        Saida {
            stdout: out.stdout,
            stderr: out.stderr,
            codigo: out.status.code().unwrap_or(-1),
        }
    }
}

/// `caminho relativo → sha256(bytes)` da árvore inteira; diretórios entram como
/// `"dir"` para que uma pasta criada vazia (D7) apareça na comparação. Sem o
/// vault, o manifesto é vazio.
pub fn manifesto(vault: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if !vault.exists() {
        return out;
    }
    andar(vault, vault, &mut out);
    out
}

fn andar(raiz: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entradas.flatten() {
        let p = e.path();
        let rel = p
            .strip_prefix(raiz)
            .expect("rel")
            .to_string_lossy()
            .to_string();
        let Ok(tipo) = e.file_type() else { continue };
        if tipo.is_dir() {
            out.insert(rel, "dir".into());
            andar(raiz, &p, out);
        } else if tipo.is_file() {
            let bytes = std::fs::read(&p).unwrap_or_default();
            let hash = Sha256::digest(&bytes);
            out.insert(rel, format!("{hash:x}"));
        }
    }
}
