//! Helper da fatia 9a (cópia enxuta do da fatia 5: o binário com HOME
//! temporário, os vaults montados à mão, a fotografia da árvore, o lock
//! segurado de fora e a validação contra `schema/<comando>.json`; sai o que
//! esta fatia não usa). O texto original da fatia 5 resume o resto:
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

/// Um HOME temporário com `ObsidianVault/` vazio.
pub fn home_com_vault_vazio() -> TempDir {
    let home = tempfile::tempdir().expect("tempdir");
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
    let caminho = vault(home).join(rel);
    fs::create_dir_all(caminho.parent().expect("pai")).expect("criar pastas");
    fs::write(&caminho, conteudo).expect("gravar arquivo");
}

/// Cria a pasta `<HOME>/ObsidianVault/<rel>`.
pub fn pasta(home: &Path, rel: &str) {
    fs::create_dir_all(vault(home).join(rel)).expect("criar pasta");
}

/// Lê `<HOME>/ObsidianVault/<rel>` como texto.
pub fn le(home: &Path, rel: &str) -> String {
    let caminho = vault(home).join(rel);
    fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {}: {e}", caminho.display()))
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

/// Um instante bem no passado (2020-09-13), para "envelhecer" os arquivos do
/// fixture: qualquer regravação depois disso muda o mtime para agora.
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

/// Foto de uma árvore: caminho relativo (com `/`) → entrada.
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
/// que apareceram ou sumiram; pastas ficam de fora.
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

// ---------------------------------------------------------------------------
// O lock segurado de fora
// ---------------------------------------------------------------------------

/// Um processo que segura `flock -x` sobre um arquivo até morrer.
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

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

/// Diretório dos arquivos da raiz de `tests/aceite/` (`schema/`):
/// `FREITASK_ACEITE_RAIZ` no rascunho; congelado, `<raiz>/tests/aceite`.
pub fn aceite_raiz() -> String {
    std::env::var("FREITASK_ACEITE_RAIZ")
        .unwrap_or_else(|_| format!("{}/tests/aceite", raiz_do_repo()))
}

/// O schema `schema/<comando>.json`, decodificado.
pub fn schema(comando: &str) -> serde_json::Value {
    let caminho = format!("{}/schema/{comando}.json", aceite_raiz());
    let texto = fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler o schema {caminho}: {e}"));
    serde_json::from_str(&texto).unwrap_or_else(|e| panic!("schema {caminho} não é JSON: {e}"))
}

/// Valida `instancia` contra `schema/<comando>.json` (draft 2020-12); pânico
/// com a lista de erros quando não passa.
pub fn valida(comando: &str, instancia: &serde_json::Value) {
    let esquema = schema(comando);
    assert_eq!(
        esquema["$schema"],
        serde_json::json!("https://json-schema.org/draft/2020-12/schema"),
        "schema/{comando}.json tem de ser draft 2020-12"
    );
    let validador = jsonschema::validator_for(&esquema)
        .unwrap_or_else(|e| panic!("schema/{comando}.json inválido: {e}"));
    let erros: Vec<String> = validador
        .iter_errors(instancia)
        .map(|e| format!("{} em {}", e, e.instance_path()))
        .collect();
    assert!(
        erros.is_empty(),
        "a saída de `{comando}` não passa em schema/{comando}.json:\n{}\n\nsaída:\n{instancia}",
        erros.join("\n")
    );
}

/// O que o `doctor` imprime num vault sem achado.
pub const CONSISTENTE: &str = "freitask: vault consistente.\n";
