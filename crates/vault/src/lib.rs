//! freitask-vault — a fronteira com o disco.
//!
//! Varre e lê o vault, grava com CAS (temporário, fsync, rename), toma o lock
//! entre dois `freitask` na mesma máquina e executa os planos do `core`. O
//! vault é `$HOME/ObsidianVault`, como hoje; nenhuma variável o troca (decisão
//! de `09c4fba`): testes trocam `HOME`.

use std::path::{Path, PathBuf};

use freitask_core::config;
use freitask_core::doctor::{self, Fotografia};
use freitask_core::index::{self, Indice};
use freitask_core::operacao::Visao;
use freitask_core::plano::Efeito;
use freitask_core::status::Vocab;

pub mod disco;
pub mod escrita;
pub mod trava;

pub use escrita::ErroEscrita;

/// O vault em disco. Só guarda a raiz; cada operação lê o que precisa na hora.
#[derive(Debug, Clone)]
pub struct Vault {
    raiz: PathBuf,
}

impl Vault {
    /// `$HOME/ObsidianVault`. Erro sem `HOME`.
    pub fn do_home() -> Result<Vault, String> {
        let home = std::env::var_os("HOME").ok_or("HOME não está definido")?;
        Ok(Vault {
            raiz: PathBuf::from(home).join("ObsidianVault"),
        })
    }

    pub fn em(raiz: impl Into<PathBuf>) -> Vault {
        Vault { raiz: raiz.into() }
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }

    /// O caminho absoluto de um relativo ao vault, como string: é o `path` do
    /// JSON e o que os comandos imprimem.
    pub fn caminho(&self, rel: &str) -> PathBuf {
        self.raiz.join(rel)
    }

    pub fn caminho_str(&self, rel: &str) -> String {
        self.caminho(rel).to_string_lossy().into_owned()
    }

    /// Os bytes de um arquivo do vault, ou `None` se não existe.
    pub fn ler(&self, rel: &str) -> Option<Vec<u8>> {
        std::fs::read(self.caminho(rel)).ok()
    }

    pub fn existe(&self, rel: &str) -> bool {
        self.caminho(rel).exists()
    }

    /// O `.freitask/status.json`, se existe. Leitura pura: não cria nada.
    pub fn status_json(&self) -> Option<String> {
        std::fs::read_to_string(self.caminho(".freitask/status.json")).ok()
    }

    /// O vocabulário de fases, relido a cada execução: o arquivo do vault, ou
    /// o default embutido quando ele falta. Quando o arquivo existe e não
    /// parseia, cai no default e devolve o aviso que o Lua dava por
    /// `vim.notify` (`status.lua:62`).
    pub fn vocab(&self) -> (Vocab, Option<String>) {
        match self.status_json() {
            Some(texto) => match Vocab::from_json(&texto) {
                Ok(v) => (v, None),
                Err(_) => (
                    Vocab::default(),
                    Some("freitask: não consegui parsear status.json, usando defaults".into()),
                ),
            },
            None => (Vocab::default(), None),
        }
    }

    /// Os projetos: diretórios sob `projects/` com um `tasks/` dentro, fora
    /// os reservados, em ordem. A presença da pasta é o registro.
    pub fn projetos(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(entradas) = std::fs::read_dir(self.caminho("projects")) else {
            return out;
        };
        for e in entradas.flatten() {
            let nome = e.file_name().to_string_lossy().into_owned();
            if nome.starts_with('.') || config::is_reserved(&nome) {
                continue;
            }
            if e.path().join("tasks").is_dir() {
                out.push(nome);
            }
        }
        out.sort();
        out
    }

    /// Os `.md` de uma pasta, como (relativo, bytes), em ordem de nome. Nomes
    /// que começam com `.` ficam de fora, como no `glob()` do Vim.
    fn mds_de(&self, rel_dir: &str) -> Vec<(String, Vec<u8>)> {
        let mut nomes: Vec<String> = match std::fs::read_dir(self.caminho(rel_dir)) {
            Ok(it) => it
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".md") && !n.starts_with('.'))
                .collect(),
            Err(_) => return Vec::new(),
        };
        nomes.sort();
        nomes
            .into_iter()
            .filter_map(|n| {
                let rel = format!("{rel_dir}/{n}");
                self.ler(&rel).map(|b| (rel, b))
            })
            .collect()
    }

    /// Todos os arquivos de task, ativos e arquivados, já lidos: os
    /// `projects/*/tasks/*.md` e `projects/*/tasks/archived/*/*.md` que o
    /// `split_task_path` aceita.
    pub fn tasks(&self) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for p in self.projetos() {
            out.extend(self.mds_de(&format!("projects/{p}/tasks")));
            for t in config::ARCHIVED_TYPES {
                out.extend(self.mds_de(&format!("projects/{p}/tasks/archived/{t}")));
            }
        }
        out
    }

    /// As notas em que um deslocamento procura referências, já lidas: todo
    /// `.md` do vault, como o `glob("**/*.md")` de hoje (nome com `.` na
    /// frente fica de fora, pasta ou arquivo), menos `daily/` e o
    /// `CURRENT.md` (o `ref_candidates` de hoje) e menos `*.sync-conflict-*`
    /// (I7: a cópia de conflito é informação a resolver, não se reescreve).
    /// Symlink também fica de fora: gravar por cima dele com `rename` o
    /// trocaria por um arquivo comum.
    pub fn notas(&self) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut pilha = vec![String::new()];
        while let Some(rel_dir) = pilha.pop() {
            let Ok(it) = std::fs::read_dir(self.caminho(&rel_dir)) else {
                continue;
            };
            for e in it.flatten() {
                let nome = e.file_name().to_string_lossy().into_owned();
                if nome.starts_with('.') {
                    continue;
                }
                let rel = if rel_dir.is_empty() {
                    nome.clone()
                } else {
                    format!("{rel_dir}/{nome}")
                };
                let Ok(tipo) = e.file_type() else { continue };
                if tipo.is_dir() {
                    if rel != config::DAILY_DIR {
                        pilha.push(rel);
                    }
                } else if tipo.is_file()
                    && nome.ends_with(".md")
                    && rel != config::CURRENT_FILE
                    && !nome.contains(".sync-conflict-")
                    && let Some(b) = self.ler(&rel)
                {
                    out.push((rel, b));
                }
            }
        }
        out.sort();
        out
    }

    /// O que o `doctor` lê, numa varredura só (decisões 40 e 41 do log):
    /// - todo `.md` fora de nome oculto, como o `glob("**/*.md")` de hoje
    ///   (inclusive `daily/`, `CURRENT.md` e as cópias de conflito);
    /// - todo arquivo sob `projects/` com `.sync-conflict-` no nome;
    /// - os `.freitask.<nome>.<pid>.tmp` das pastas em que o freitask grava:
    ///   as não ocultas e a `.freitask/` da raiz (o `status.json`).
    ///
    /// Symlink de arquivo é lido pelo alvo, como o `glob()`; symlink de pasta
    /// não é seguido (um laço travaria a varredura). Só lê.
    pub fn fotografia_do_doctor(&self) -> Fotografia {
        let mut foto = Fotografia::default();
        let mut pilha = vec![String::new()];
        while let Some(rel_dir) = pilha.pop() {
            let Ok(it) = std::fs::read_dir(self.caminho(&rel_dir)) else {
                continue;
            };
            for e in it.flatten() {
                let nome = e.file_name().to_string_lossy().into_owned();
                let rel = juntar(&rel_dir, &nome);
                let Ok(tipo) = e.file_type() else { continue };
                if tipo.is_dir() {
                    if !nome.starts_with('.') || rel == ".freitask" {
                        pilha.push(rel);
                    }
                    continue;
                }
                if doctor::eh_temporario(&nome) {
                    if let Some(b) = self.ler(&rel) {
                        foto.temporarios.push((rel, b));
                    }
                    continue;
                }
                if nome.starts_with('.') || rel_dir.starts_with('.') {
                    continue;
                }
                if rel.starts_with("projects/") && doctor::eh_conflito(&rel) {
                    foto.conflitos.push(rel.clone());
                }
                if nome.ends_with(".md")
                    && self.caminho(&rel).is_file()
                    && let Some(b) = self.ler(&rel)
                {
                    foto.notas.push((rel, b));
                }
            }
        }
        foto.notas.sort();
        foto.conflitos.sort();
        foto.temporarios.sort();
        foto
    }

    /// Os caminhos (relativos ao vault) de todos os arquivos sob `rel_dir`,
    /// inclusive os ocultos, em ordem. Symlink de pasta não é seguido.
    pub fn arquivos_sob(&self, rel_dir: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut pilha = vec![rel_dir.to_string()];
        while let Some(dir) = pilha.pop() {
            let Ok(it) = std::fs::read_dir(self.caminho(&dir)) else {
                continue;
            };
            for e in it.flatten() {
                let rel = juntar(&dir, &e.file_name().to_string_lossy());
                if e.file_type().is_ok_and(|t| t.is_dir()) {
                    pilha.push(rel);
                } else {
                    out.push(rel);
                }
            }
        }
        out.sort();
        out
    }

    /// O índice inteiro: projetos e tasks lidas.
    pub fn indice(&self, vocab: &Vocab) -> Indice {
        let entradas = self
            .tasks()
            .into_iter()
            .filter_map(|(rel, bytes)| index::scan_task(vocab, &rel, &bytes))
            .collect();
        Indice::new(self.projetos(), entradas)
    }

    /// Traduz um caminho dado pelo usuário (absoluto ou relativo ao cwd) num
    /// caminho relativo ao vault, se o arquivo existe e está dentro dele.
    pub fn rel_de(&self, caminho: &str) -> Option<String> {
        let p = Path::new(caminho);
        if !p.is_file() {
            return None;
        }
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(p)
        };
        let abs = normaliza(&abs);
        let raiz = normaliza(&self.raiz);
        let rel = abs.strip_prefix(&raiz).ok()?;
        Some(rel.to_string_lossy().replace('\\', "/"))
    }

    /// Resolve `<id|caminho>` no arquivo da task (relativo ao vault), como o
    /// `find_task` de hoje: caminho que existe primeiro; senão o id, único
    /// entre ativas e arquivadas.
    pub fn find_task(&self, indice: &Indice, referencia: &str) -> Result<String, String> {
        if let Some(rel) = self.rel_de(referencia) {
            return Ok(rel);
        }
        let hits = indice.por_id(referencia);
        match hits.len() {
            1 => Ok(hits[0].rel.clone()),
            0 => Err(format!("task não encontrada: {referencia}")),
            n => Err(format!(
                "id \"{referencia}\" é ambíguo ({n} arquivos); passe o caminho"
            )),
        }
    }

    /// Garante `projects/` e `.freitask/status.json`, como o `ensure_root` de
    /// hoje: só os comandos que escrevem chamam isto.
    pub fn garantir_raiz(&self) -> Result<(), ErroEscrita> {
        let projects = self.caminho("projects");
        std::fs::create_dir_all(&projects).map_err(|e| ErroEscrita::Io(projects, e))?;
        if !self.existe(".freitask/status.json") {
            let dir = self.caminho(".freitask");
            std::fs::create_dir_all(&dir).map_err(|e| ErroEscrita::Io(dir, e))?;
            // Com `\n` final, como o `writefile` de hoje grava (uma linha por elemento).
            let mut bytes = config::DEFAULT_STATUS_JSON.as_bytes().to_vec();
            bytes.push(b'\n');
            self.gravar(".freitask/status.json", None, &bytes)?;
        }
        Ok(())
    }

    /// Grava `rel` com CAS contra `antes` (o que foi lido; `None` se não
    /// existia). Sem mudança, não grava nem toca o mtime (§1.5). Devolve se
    /// gravou.
    pub fn gravar(
        &self,
        rel: &str,
        antes: Option<&[u8]>,
        depois: &[u8],
    ) -> Result<bool, ErroEscrita> {
        let destino = self.caminho(rel);
        // A pasta que falta nasce junto (o `daily/` do primeiro snapshot).
        if antes != Some(depois)
            && let Some(pai) = destino.parent()
            && !pai.is_dir()
        {
            std::fs::create_dir_all(pai).map_err(|e| ErroEscrita::Io(pai.to_path_buf(), e))?;
        }
        escrita::gravar(&disco::Real, &destino, antes, depois)
    }

    /// Executa um plano do `core` no disco de verdade.
    pub fn aplicar(&self, plano: &[Efeito]) -> Result<(), ErroEscrita> {
        escrita::aplicar(&disco::Real, &self.raiz, plano)
    }
}

impl Visao for Vault {
    fn arquivo_existe(&self, rel: &str) -> bool {
        self.caminho(rel).is_file()
    }

    fn dir_existe(&self, rel: &str) -> bool {
        self.caminho(rel).is_dir()
    }
}

fn juntar(dir: &str, nome: &str) -> String {
    if dir.is_empty() {
        nome.to_string()
    } else {
        format!("{dir}/{nome}")
    }
}

/// Remove `.` e resolve `..` lexicamente, sem tocar o disco (symlinks ficam
/// como estão: o HOME dos testes é um caminho literal).
fn normaliza(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            outro => out.push(outro),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn vocab_cai_no_default_sem_arquivo_e_avisa_com_arquivo_quebrado() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::em(dir.path());
        let (vocab, aviso) = v.vocab();
        assert_eq!(vocab, Vocab::default());
        assert!(aviso.is_none());
        std::fs::create_dir_all(dir.path().join(".freitask")).unwrap();
        std::fs::write(dir.path().join(".freitask/status.json"), "{ quebrado").unwrap();
        let (vocab, aviso) = v.vocab();
        assert_eq!(vocab, Vocab::default());
        assert!(aviso.unwrap().contains("usando defaults"));
        std::fs::write(
            dir.path().join(".freitask/status.json"),
            r#"{"1":{"title":"A","callout":"todo"}}"#,
        )
        .unwrap();
        assert_eq!(v.vocab().0.meta(1).title, "A");
    }

    #[test]
    fn varre_projetos_e_tasks_ignorando_o_que_nao_e_task() {
        let dir = tempfile::tempdir().unwrap();
        gerador::minimo().gravar(dir.path()).unwrap();
        std::fs::create_dir_all(dir.path().join("projects/daily/tasks")).unwrap();
        std::fs::create_dir_all(dir.path().join("projects/so-spec")).unwrap();
        std::fs::write(dir.path().join("projects/p/tasks/.oculta.md"), "x").unwrap();
        std::fs::write(dir.path().join("projects/p/tasks/nota.txt"), "x").unwrap();
        let v = Vault::em(dir.path());
        assert_eq!(v.projetos(), vec!["p", "q"]);
        let rels: Vec<String> = v.tasks().into_iter().map(|(r, _)| r).collect();
        assert_eq!(
            rels,
            vec![
                "projects/p/tasks/t1.md",
                "projects/p/tasks/t2.md",
                "projects/p/tasks/t3.md",
                "projects/p/tasks/archived/done/t0.md"
            ]
        );
        let idx = v.indice(&Vocab::default());
        assert_eq!(idx.entradas.len(), 4);
        assert!(
            Vault::em(dir.path().join("nao-existe"))
                .projetos()
                .is_empty()
        );
    }

    #[test]
    fn find_task_por_id_por_caminho_e_as_recusas() {
        let dir = tempfile::tempdir().unwrap();
        gerador::minimo().gravar(dir.path()).unwrap();
        std::fs::write(
            dir.path().join("projects/q/tasks/t1.md"),
            "> [!todo] Outra\n> [[t1]]\n",
        )
        .unwrap();
        let v = Vault::em(dir.path());
        let idx = v.indice(&Vocab::default());
        assert_eq!(v.find_task(&idx, "t2").unwrap(), "projects/p/tasks/t2.md");
        assert_eq!(
            v.find_task(&idx, "t0").unwrap(),
            "projects/p/tasks/archived/done/t0.md"
        );
        assert_eq!(
            v.find_task(&idx, "t1").unwrap_err(),
            "id \"t1\" é ambíguo (2 arquivos); passe o caminho"
        );
        assert_eq!(
            v.find_task(&idx, "nao").unwrap_err(),
            "task não encontrada: nao"
        );
        let abs = dir.path().join("projects/q/tasks/t1.md");
        assert_eq!(
            v.find_task(&idx, abs.to_str().unwrap()).unwrap(),
            "projects/q/tasks/t1.md"
        );
        let fora = tempfile::NamedTempFile::new().unwrap();
        assert!(
            v.find_task(&idx, fora.path().to_str().unwrap()).is_err(),
            "arquivo fora do vault não é task"
        );
    }

    #[test]
    fn gravar_nao_toca_o_mtime_quando_igual_e_aplicar_executa_o_plano() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::em(dir.path());
        assert!(
            v.gravar("a/b.md", None, b"x").unwrap(),
            "cria a pasta que falta"
        );
        v.aplicar(&[Efeito::CriarDir { rel: "c/d".into() }])
            .unwrap();
        assert!(dir.path().join("c/d").is_dir());
        let m1 = std::fs::metadata(dir.path().join("a/b.md"))
            .unwrap()
            .modified()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!v.gravar("a/b.md", Some(b"x"), b"x").unwrap());
        assert_eq!(
            std::fs::metadata(dir.path().join("a/b.md"))
                .unwrap()
                .modified()
                .unwrap(),
            m1
        );
        assert!(v.gravar("a/b.md", Some(b"x"), b"y").unwrap());
        assert_eq!(std::fs::read(dir.path().join("a/b.md")).unwrap(), b"y");
        assert!(
            std::fs::read_dir(dir.path().join("a")).unwrap().count() == 1,
            "sem temporário sobrando"
        );
        assert!(matches!(
            v.gravar("a/b.md", Some(b"x"), b"z").unwrap_err(),
            ErroEscrita::Conflito(_)
        ));
    }

    #[test]
    fn visao_distingue_arquivo_e_pasta() {
        let dir = tempfile::tempdir().unwrap();
        gerador::minimo().gravar(dir.path()).unwrap();
        let v = Vault::em(dir.path());
        assert!(v.arquivo_existe("projects/p/tasks/t1.md"));
        assert!(!v.arquivo_existe("projects/p/tasks"));
        assert!(v.dir_existe("projects/p/tasks"));
        assert!(!v.dir_existe("projects/p/tasks/t1.md"));
        assert!(!v.dir_existe("projects/nao"));
    }

    #[test]
    fn fotografia_do_doctor_le_notas_conflitos_e_temporarios() {
        let dir = tempfile::tempdir().unwrap();
        let w = |rel: &str, b: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b).unwrap();
        };
        w("CURRENT.md", "c");
        w("daily/d.md", "d");
        w("projects/p/tasks/t.md", "t");
        w("projects/p/tasks/t.sync-conflict-1.md", "tc");
        w("projects/p/anexos/f.sync-conflict-1.pdf", "pdf");
        w("projects/p/anexos/f.pdf", "pdf");
        w("inbox/n.sync-conflict-1.md", "nc");
        w("inbox/.oculta.md", "o");
        w(".obsidian/x.md", "o");
        w(".obsidian/.freitask.x.md.1.tmp", "o");
        w(".freitask/status.json", "{}");
        w(".freitask/.freitask.status.json.7.tmp", "s");
        w(".freitask/nota.md", "nao");
        w("projects/p/tasks/.freitask.t.md.12.tmp", "meio");
        w(".freitask.CURRENT.md.3.tmp", "r");
        w("inbox/.freitask.n.md.x.tmp", "nao e temporario");
        std::fs::create_dir_all(dir.path().join("vazia")).unwrap();
        std::os::unix::fs::symlink(
            dir.path().join("inbox/n.sync-conflict-1.md"),
            dir.path().join("link.md"),
        )
        .unwrap();
        std::os::unix::fs::symlink(dir.path().join("projects"), dir.path().join("laco")).unwrap();
        let foto = Vault::em(dir.path()).fotografia_do_doctor();
        let rels: Vec<&str> = foto.notas.iter().map(|n| n.0.as_str()).collect();
        assert_eq!(
            rels,
            vec![
                "CURRENT.md",
                "daily/d.md",
                "inbox/n.sync-conflict-1.md",
                "link.md",
                "projects/p/tasks/t.md",
                "projects/p/tasks/t.sync-conflict-1.md",
            ]
        );
        assert_eq!(
            foto.notas[3].1, b"nc",
            "symlink de arquivo é lido pelo alvo"
        );
        assert_eq!(
            foto.conflitos,
            vec![
                "projects/p/anexos/f.sync-conflict-1.pdf",
                "projects/p/tasks/t.sync-conflict-1.md"
            ]
        );
        assert_eq!(
            foto.temporarios,
            vec![
                (".freitask.CURRENT.md.3.tmp".to_string(), b"r".to_vec()),
                (
                    ".freitask/.freitask.status.json.7.tmp".to_string(),
                    b"s".to_vec()
                ),
                (
                    "projects/p/tasks/.freitask.t.md.12.tmp".to_string(),
                    b"meio".to_vec()
                ),
            ]
        );
        assert_eq!(
            Vault::em(dir.path().join("nao")).fotografia_do_doctor(),
            Fotografia::default()
        );
    }

    #[test]
    fn arquivos_sob_lista_tudo_inclusive_oculto() {
        let dir = tempfile::tempdir().unwrap();
        for rel in ["p/a.md", "p/.b", "p/s/c.pdf", "q/d.md"] {
            let f = dir.path().join(rel);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, "x").unwrap();
        }
        std::fs::create_dir_all(dir.path().join("p/vazia")).unwrap();
        assert_eq!(
            Vault::em(dir.path()).arquivos_sob("p"),
            vec!["p/.b", "p/a.md", "p/s/c.pdf"]
        );
        assert!(Vault::em(dir.path()).arquivos_sob("nao").is_empty());
    }

    #[test]
    fn garantir_raiz_cria_projects_e_status_json_uma_vez() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::em(dir.path());
        v.garantir_raiz().unwrap();
        assert!(dir.path().join("projects").is_dir());
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".freitask/status.json")).unwrap(),
            format!("{}\n", config::DEFAULT_STATUS_JSON)
        );
        std::fs::write(dir.path().join(".freitask/status.json"), "{}").unwrap();
        v.garantir_raiz().unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".freitask/status.json")).unwrap(),
            "{}",
            "não sobrescreve"
        );
    }
}
