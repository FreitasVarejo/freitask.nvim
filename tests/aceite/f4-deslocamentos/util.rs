//! Helper da fatia 4 (cópia do da fatia 3, mais `sucesso` e
//! `arquivos_mudados`). O texto original da fatia 3 segue abaixo.
//!
//! Helper da fatia 3 (cópia do da fatia 2, mais o que a escrita precisa):
//! roda o binário `freitask` com HOME num diretório temporário e devolve
//! stdout, stderr e código já decodificados; monta vaults pequenos à mão;
//! fotografa a árvore para provar o que mudou e o que não mudou; segura o lock
//! de escrita de fora com o `flock` do util-linux.
//!
//! Regras de `docs/agents/aceite.md`: `env_clear()`, depois `HOME`, `PATH`,
//! `LANG=C.UTF-8`, `LC_ALL=C.UTF-8`, `FREITASK_TODAY`; `current_dir` no HOME.
//! Sem `XDG_RUNTIME_DIR` no ambiente, o lock mora em
//! `<HOME>/.local/state/freitask/lock` (spec §1.5), isto é, cada teste tem o
//! seu e nenhum espera pelo outro.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use tempfile::TempDir;

/// O dia fixo de todos os testes desta fatia (um domingo).
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

/// Grava um arquivo em `<HOME>/ObsidianVault/<rel>`, criando as pastas.
pub fn arquivo(home: &Path, rel: &str, conteudo: &str) {
    arquivo_bytes(home, rel, conteudo.as_bytes());
}

/// Grava bytes crus em `<HOME>/ObsidianVault/<rel>`, criando as pastas.
pub fn arquivo_bytes(home: &Path, rel: &str, conteudo: &[u8]) {
    let caminho = vault(home).join(rel);
    fs::create_dir_all(caminho.parent().expect("pai")).expect("criar pastas");
    fs::write(&caminho, conteudo).expect("gravar arquivo");
}

/// Cria a pasta `<HOME>/ObsidianVault/<rel>` (vazia, se nada mais a preencher).
pub fn pasta(home: &Path, rel: &str) {
    fs::create_dir_all(vault(home).join(rel)).expect("criar pasta");
}

/// Lê `<HOME>/ObsidianVault/<rel>` como texto.
pub fn le(home: &Path, rel: &str) -> String {
    let caminho = vault(home).join(rel);
    fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {}: {e}", caminho.display()))
}

/// Lê `<HOME>/ObsidianVault/<rel>` em bytes.
pub fn le_bytes(home: &Path, rel: &str) -> Vec<u8> {
    let caminho = vault(home).join(rel);
    fs::read(&caminho).unwrap_or_else(|e| panic!("não consegui ler {}: {e}", caminho.display()))
}

/// Verdadeiro se `<HOME>/ObsidianVault/<rel>` existe.
pub fn existe(home: &Path, rel: &str) -> bool {
    vault(home).join(rel).exists()
}

/// O caminho absoluto de `<HOME>/ObsidianVault/<rel>`, como string.
pub fn abs(home: &Path, rel: &str) -> String {
    vault(home).join(rel).to_str().expect("utf-8").to_owned()
}

/// Um HOME com `ObsidianVault/` e os arquivos dados (`rel` → conteúdo).
pub fn home_com(arquivos: &[(&str, &str)]) -> TempDir {
    let home = home_com_vault_vazio();
    for (rel, conteudo) in arquivos {
        arquivo(home.path(), rel, conteudo);
    }
    home
}

/// O mtime de `<HOME>/ObsidianVault/<rel>`.
pub fn mtime(home: &Path, rel: &str) -> SystemTime {
    fs::metadata(vault(home).join(rel))
        .expect("metadata")
        .modified()
        .expect("mtime")
}

/// Um instante bem no passado (2020-09-13), para "envelhecer" os arquivos do
/// fixture: qualquer regravação depois disso muda o mtime para agora, sem
/// depender de `sleep` nem da resolução do relógio do sistema de arquivos.
pub fn antigo() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000)
}

/// Põe o mtime de todo ARQUIVO sob `<HOME>/ObsidianVault` em `antigo()`.
pub fn envelhece(home: &Path) {
    let raiz = vault(home);
    if !raiz.exists() {
        return;
    }
    let mut pilha = vec![raiz];
    while let Some(dir) = pilha.pop() {
        for entrada in fs::read_dir(&dir).expect("read_dir") {
            let caminho = entrada.expect("entrada").path();
            let meta = fs::symlink_metadata(&caminho).expect("metadata");
            if meta.is_dir() {
                pilha.push(caminho);
            } else {
                let f = fs::OpenOptions::new()
                    .write(true)
                    .open(&caminho)
                    .expect("abrir para mudar o mtime");
                f.set_modified(antigo()).expect("set_modified");
            }
        }
    }
}

/// Afirma saída 0 e stderr vazio: é o que todo comando que deu certo devolve.
pub fn ok(saida: &Saida) {
    assert_eq!(
        saida.codigo,
        Some(0),
        "código de saída; stderr: {:?}",
        saida.stderr
    );
    assert_eq!(
        saida.stderr, "",
        "stderr tem de estar vazio num comando que deu certo"
    );
}

/// Afirma uma recusa: código dado, stdout vazio e stderr exatamente
/// `freitask: <msg>\n`.
pub fn recusa(saida: &Saida, codigo: i32, msg: &str) {
    assert_eq!(
        saida.codigo,
        Some(codigo),
        "código de saída; stderr: {:?}",
        saida.stderr
    );
    assert_eq!(saida.stderr, format!("freitask: {msg}\n"), "stderr");
    assert_eq!(saida.stdout, "", "stdout tem de ficar vazio numa recusa");
}

/// Afirma um comando que deu certo: saída 0, stdout e stderr exatamente os
/// dados. O stderr de `archive`/`unarchive`/`rename`/`move` traz o aviso de
/// quantas referências foram reescritas (ou nada, se nenhuma).
pub fn sucesso(saida: &Saida, stdout: &str, stderr: &str) {
    assert_eq!(
        saida.codigo,
        Some(0),
        "código de saída; stderr: {:?}",
        saida.stderr
    );
    assert_eq!(saida.stderr, stderr, "stderr");
    assert_eq!(saida.stdout, stdout, "stdout");
}

/// O aviso de stderr de uma reescrita de referências (`docs/freitask.md`,
/// "Referências do Obsidian"; texto medido no oráculo).
pub fn aviso_refs(refs: usize, arquivos: usize) -> String {
    format!("freitask: {refs} referência(s) atualizada(s) em {arquivos} arquivo(s)\n")
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

fn saida_de(o: std::process::Output) -> Saida {
    Saida {
        codigo: o.status.code(),
        stdout: String::from_utf8(o.stdout).expect("stdout não é UTF-8"),
        stderr: String::from_utf8(o.stderr).expect("stderr não é UTF-8"),
    }
}

/// Roda `freitask <args>` com HOME em `home`.
pub fn roda(home: &Path, args: &[&str]) -> Saida {
    let o = comando(home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("não consegui rodar {}: {e}", binario().display()));
    saida_de(o)
}

/// Roda `freitask <args>` com HOME em `home` e `XDG_RUNTIME_DIR` em `xdg`.
pub fn roda_com_xdg(home: &Path, xdg: &Path, args: &[&str]) -> Saida {
    let o = comando(home)
        .env("XDG_RUNTIME_DIR", xdg)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("não consegui rodar {}: {e}", binario().display()));
    saida_de(o)
}

/// Decodifica o stdout como JSON; o teste compara o valor, nunca o texto.
pub fn json(saida: &Saida) -> serde_json::Value {
    serde_json::from_str(&saida.stdout)
        .unwrap_or_else(|e| panic!("stdout não é JSON ({e}):\n{}", saida.stdout))
}

// ---------------------------------------------------------------------------
// Fotografia da árvore
// ---------------------------------------------------------------------------

/// Uma entrada da foto: pasta, ou arquivo com bytes e mtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entrada {
    Pasta,
    Arquivo(Vec<u8>, SystemTime),
}

/// Foto de uma árvore: caminho relativo (com `/`) → entrada. O mtime das
/// pastas não entra: o que se afirma é que nenhum arquivo mudou de bytes nem de
/// mtime e que nenhuma entrada apareceu ou sumiu.
pub type Foto = BTreeMap<String, Entrada>;

/// Fotografa `<HOME>/ObsidianVault` (vazia se o vault não existe).
pub fn fotografa(home: &Path) -> Foto {
    let raiz = vault(home);
    let mut foto = Foto::new();
    if !raiz.exists() {
        return foto;
    }
    let mut pilha = vec![raiz.clone()];
    while let Some(dir) = pilha.pop() {
        for entrada in fs::read_dir(&dir).expect("read_dir") {
            let caminho = entrada.expect("entrada").path();
            let meta = fs::symlink_metadata(&caminho).expect("metadata");
            let rel = caminho
                .strip_prefix(&raiz)
                .expect("prefixo")
                .to_str()
                .expect("utf-8")
                .to_owned();
            if meta.is_dir() {
                foto.insert(rel, Entrada::Pasta);
                pilha.push(caminho);
            } else {
                let bytes = fs::read(&caminho).expect("ler arquivo");
                foto.insert(
                    rel,
                    Entrada::Arquivo(bytes, meta.modified().expect("mtime")),
                );
            }
        }
    }
    foto
}

/// Os caminhos (relativos ao vault) que diferem entre duas fotos: arquivo com
/// bytes ou mtime diferentes, e entrada que apareceu ou sumiu.
pub fn mudancas(antes: &Foto, depois: &Foto) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (k, v) in antes {
        match depois.get(k) {
            Some(w) if w == v => {}
            _ => {
                out.insert(k.clone());
            }
        }
    }
    for k in depois.keys() {
        if !antes.contains_key(k) {
            out.insert(k.clone());
        }
    }
    out
}

/// Só os ARQUIVOS que diferem entre duas fotos (bytes ou mtime), mais os
/// que apareceram ou sumiram; pastas ficam de fora (uma pasta vazia que sobra
/// ou nasce não é afirmada).
pub fn arquivos_mudados(antes: &Foto, depois: &Foto) -> BTreeSet<String> {
    mudancas(antes, depois)
        .into_iter()
        .filter(|k| {
            !matches!(antes.get(k), Some(Entrada::Pasta))
                && !matches!(depois.get(k), Some(Entrada::Pasta))
        })
        .collect()
}

/// Um conjunto de caminhos a partir de literais.
pub fn conjunto(itens: &[&str]) -> BTreeSet<String> {
    itens.iter().map(|s| s.to_string()).collect()
}

/// Afirma que nada mudou sob o vault entre as duas fotos.
pub fn nada_mudou(antes: &Foto, depois: &Foto) {
    let m = mudancas(antes, depois);
    assert!(
        m.is_empty(),
        "nada devia ter sido gravado, mas mudou: {m:?}"
    );
}

/// Os temporários de gravação (`.freitask.*.tmp`) em qualquer lugar do vault.
pub fn temporarios(home: &Path) -> Vec<String> {
    fotografa(home)
        .keys()
        .filter(|k| {
            let nome = k.rsplit('/').next().unwrap_or(k);
            nome.starts_with(".freitask.") && nome.ends_with(".tmp")
        })
        .cloned()
        .collect()
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

// ---------------------------------------------------------------------------
// O lock segurado de fora
// ---------------------------------------------------------------------------

/// Um processo que segura `flock -x` sobre um arquivo até morrer. O `sh` abre
/// o arquivo no fd 9, toma o lock, avisa `pronto` e vira `sleep` (exec): o
/// lock fica com um processo só, que o `Drop` mata.
pub struct Lock {
    filho: Child,
}

impl Lock {
    /// Toma o lock em `arquivo` (criando-o) e só devolve depois que ele está
    /// tomado. Solta sozinho depois de `segundos`.
    pub fn segura(arquivo: &Path, segundos: u32) -> Lock {
        if let Some(pai) = arquivo.parent() {
            fs::create_dir_all(pai).expect("criar a pasta do lock");
        }
        let mut filho = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "exec 9>\"$1\" && flock -x 9 && echo pronto && exec sleep {segundos}"
            ))
            .arg("sh")
            .arg(arquivo)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("rodar sh + flock (util-linux)");
        let stdout = filho.stdout.take().expect("stdout do flock");
        let mut linha = String::new();
        BufReader::new(stdout)
            .read_line(&mut linha)
            .expect("ler o aviso do flock");
        assert_eq!(linha, "pronto\n", "o flock de fora não tomou o lock");
        Lock { filho }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.filho.kill();
        let _ = self.filho.wait();
    }
}

/// O lock quando `XDG_RUNTIME_DIR` está definido (spec §1.5).
pub fn lock_em_xdg(xdg: &Path) -> PathBuf {
    xdg.join("freitask.lock")
}

/// O lock quando `XDG_RUNTIME_DIR` falta (spec §1.5).
pub fn lock_sem_xdg(home: &Path) -> PathBuf {
    home.join(".local/state/freitask/lock")
}
