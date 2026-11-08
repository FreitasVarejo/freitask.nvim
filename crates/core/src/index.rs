//! O índice em memória das tasks: o que o `list`, o painel e a TUI leem. Cada
//! arquivo de task é lido UMA vez (`scan_task`) e dali saem o bloco, a fase,
//! o frontmatter, as notas, o histórico e o checklist. Puro: recebe bytes e
//! caminhos relativos, nunca abre arquivo.

use crate::date::Date;
use crate::md;
use crate::meta::{self, Meta};
use crate::model::{self, Model};
use crate::path::{self, TaskRef};
use crate::status::Vocab;
use crate::texto::Doc;

/// Uma task já lida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub project: String,
    pub task_id: String,
    pub archived: Option<String>,
    /// Caminho relativo ao vault.
    pub rel: String,
    pub status_num: u32,
    /// As linhas cruas do primeiro bloco `>`; vazio sem callout.
    pub block: Vec<String>,
    pub model: Model,
    pub meta: Meta,
    /// As linhas `- AAAA-MM-DD — …` do `## Histórico`, inteiras.
    pub historico: Vec<String>,
    /// `(feitos, total)` dos `- [ ]`/`- [x]` do corpo, fora do bloco.
    pub checklist: (u32, u32),
}

/// Lê um arquivo de task. `None` quando o caminho não é de task ou o arquivo
/// não é UTF-8 (não é nota do vault).
pub fn scan_task(vocab: &Vocab, rel: &str, bytes: &[u8]) -> Option<Entry> {
    let t = path::split_task_path(rel)?;
    let doc = Doc::parse(bytes).ok()?;
    Some(scan_linhas(vocab, t, rel, &doc.lines))
}

fn scan_linhas(vocab: &Vocab, t: TaskRef, rel: &str, lines: &[String]) -> Entry {
    let bloco = md::first_block(lines);
    let (block, fim_do_bloco) = match &bloco {
        Some((_, e, b)) => (b.clone(), e + 1),
        None => (Vec::new(), md::content_start(lines)),
    };
    // Arquivo sem callout é um arquivo quebrado: 0 o deixa no topo do projeto
    // em vez de escondê-lo no meio do Backlog.
    let model = if block.is_empty() {
        Model {
            status_num: 0,
            ..Model::default()
        }
    } else {
        model::parse_block(vocab, &block)
    };
    let corpo = lines.get(fim_do_bloco..).unwrap_or(&[]);
    Entry {
        status_num: model.status_num,
        historico: historico(corpo),
        checklist: checklist(corpo),
        meta: meta::read(lines),
        project: t.project,
        task_id: t.id,
        archived: t.archived,
        rel: rel.to_string(),
        block,
        model,
    }
}

/// As linhas do `## Histórico` (a última seção com esse nome, como o
/// `append_entry`), até a próxima `## seção`, só as que são entradas datadas.
fn historico(corpo: &[String]) -> Vec<String> {
    let Some(inicio) = corpo.iter().rposition(|l| eh_titulo_historico(l)) else {
        return Vec::new();
    };
    corpo[inicio + 1..]
        .iter()
        .take_while(|l| !eh_secao(l))
        .filter(|l| eh_entrada_datada(l))
        .cloned()
        .collect()
}

fn eh_titulo_historico(l: &str) -> bool {
    let Some(resto) = l.strip_prefix("##") else {
        return false;
    };
    let sem = resto.trim_start_matches(crate::eh_espaco);
    sem.len() != resto.len()
        && sem
            .strip_prefix("Histórico")
            .is_some_and(|f| f.chars().all(crate::eh_espaco))
}

fn eh_secao(l: &str) -> bool {
    l.strip_prefix("##")
        .is_some_and(|r| r.chars().next().is_some_and(crate::eh_espaco))
}

/// `- AAAA-MM-DD — …`
fn eh_entrada_datada(l: &str) -> bool {
    l.starts_with("- ")
        && l.get(2..12).and_then(Date::parse).is_some()
        && l.get(12..).is_some_and(|r| r.starts_with(" — "))
}

/// `(feitos, total)` dos itens `- [ ]` e `- [x]`/`- [X]` do corpo.
fn checklist(corpo: &[String]) -> (u32, u32) {
    let mut feitos = 0;
    let mut total = 0;
    for l in corpo {
        let s = l.trim_start_matches(crate::eh_espaco);
        if let Some(r) = s.strip_prefix("- [") {
            match r.get(..3) {
                Some(" ] ") | Some(" ]") if r.len() == 2 || r.get(..3) == Some(" ] ") => total += 1,
                Some("x] ") | Some("X] ") => {
                    feitos += 1;
                    total += 1;
                }
                _ if r == " ]" || r == "x]" || r == "X]" => {
                    total += 1;
                    if r != " ]" {
                        feitos += 1;
                    }
                }
                _ => {}
            }
        }
    }
    (feitos, total)
}

/// Verdadeiro se o painel e a TUI escondem a task por padrão: a fase é de
/// Algum dia, ou ela está adiada. Vale para qualquer fase ativa e vence
/// `prioridade: alta`, que só decide a ordem.
pub fn is_hidden(vocab: &Vocab, e: &Entry, today: Date) -> bool {
    vocab.is_hidden(e.status_num) || meta::is_postponed(&e.meta, today)
}

/// O índice: os projetos (pastas com `tasks/`, em ordem) e as tasks lidas.
#[derive(Debug, Clone, Default)]
pub struct Indice {
    pub projetos: Vec<String>,
    pub entradas: Vec<Entry>,
}

impl Indice {
    pub fn new(mut projetos: Vec<String>, entradas: Vec<Entry>) -> Indice {
        projetos.retain(|p| !crate::config::is_reserved(p));
        projetos.sort();
        projetos.dedup();
        Indice { projetos, entradas }
    }

    /// As tasks ATIVAS de um projeto: as altas primeiro, depois por fase e id.
    /// É a única fonte da ordem do `CURRENT.md`, do `list` e da TUI.
    pub fn entries_for(&self, project: &str) -> Vec<&Entry> {
        let mut out: Vec<&Entry> = self
            .entradas
            .iter()
            .filter(|e| e.project == project && e.archived.is_none())
            .collect();
        out.sort_by(|a, b| {
            let (ha, hb) = (meta::is_high(&a.meta), meta::is_high(&b.meta));
            hb.cmp(&ha)
                .then(a.status_num.cmp(&b.status_num))
                .then(a.task_id.cmp(&b.task_id))
        });
        out
    }

    /// As arquivadas de um projeto: por tipo e depois por id.
    pub fn archived_for(&self, project: &str) -> Vec<&Entry> {
        let mut out: Vec<&Entry> = self
            .entradas
            .iter()
            .filter(|e| e.project == project && e.archived.is_some())
            .collect();
        out.sort_by(|a, b| a.archived.cmp(&b.archived).then(a.task_id.cmp(&b.task_id)));
        out
    }

    /// Todas as tasks, ativas e arquivadas, com este id.
    pub fn por_id(&self, id: &str) -> Vec<&Entry> {
        self.entradas.iter().filter(|e| e.task_id == id).collect()
    }

    pub fn por_rel(&self, rel: &str) -> Option<&Entry> {
        self.entradas.iter().find(|e| e.rel == rel)
    }

    /// Quantas tasks de um projeto o painel mostra.
    pub fn mostradas(&self, vocab: &Vocab, project: &str, today: Date) -> usize {
        self.entries_for(project)
            .into_iter()
            .filter(|e| !is_hidden(vocab, e, today))
            .count()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn entry(
        vocab: &Vocab,
        callout: &str,
        volta_em: Option<&str>,
        prioridade: Option<&str>,
    ) -> Entry {
        let mut texto = String::new();
        if volta_em.is_some() || prioridade.is_some() {
            texto.push_str("---\n");
            if let Some(p) = prioridade {
                texto.push_str(&format!("prioridade: {p}\n"));
            }
            if let Some(v) = volta_em {
                texto.push_str(&format!("volta-em: {v}\n"));
            }
            texto.push_str("---\n\n");
        }
        texto.push_str(&format!("> [!{callout}] T\n> [[projects/p/tasks/x|x]]\n"));
        scan_task(vocab, "projects/p/tasks/x.md", texto.as_bytes()).unwrap()
    }

    const TODAY: &str = "2026-11-10";

    #[test]
    fn algum_dia_segue_escondida_com_ou_sem_data() {
        let v = Vocab::default();
        assert!(is_hidden(&v, &entry(&v, "quote", None, None), d(TODAY)));
        assert!(is_hidden(
            &v,
            &entry(&v, "quote", Some("2020-01-01"), None),
            d(TODAY)
        ));
    }

    #[test]
    fn volta_em_no_futuro_some_em_qualquer_fase_ativa() {
        let v = Vocab::default();
        for c in [
            "todo", "abstract", "tip", "example", "question", "warning", "check",
        ] {
            assert!(
                is_hidden(&v, &entry(&v, c, Some("2026-12-01"), None), d(TODAY)),
                "{c} adiada some"
            );
            assert!(
                !is_hidden(&v, &entry(&v, c, None, None), d(TODAY)),
                "{c} sem data aparece"
            );
        }
    }

    #[test]
    fn a_data_vence_prioridade_alta_enquanto_nao_chega() {
        let v = Vocab::default();
        let e = entry(&v, "example", Some("2026-12-01"), Some("alta"));
        assert!(is_hidden(&v, &e, d(TODAY)));
        assert!(!is_hidden(&v, &e, d("2026-12-01")));
    }

    #[test]
    fn no_dia_e_depois_a_task_aparece_sem_nada_gravado() {
        let v = Vocab::default();
        let e = entry(&v, "tip", Some("2026-12-01"), None);
        assert!(is_hidden(&v, &e, d("2026-11-30")));
        assert!(!is_hidden(&v, &e, d("2026-12-01")));
        assert!(!is_hidden(&v, &e, d("2026-12-02")));
        assert_eq!(e.meta.volta_em.as_deref(), Some("2026-12-01"));
    }

    #[test]
    fn data_invalida_nao_esconde() {
        let v = Vocab::default();
        for ruim in ["2026-12-1", "2026-02-30", "01/12/2026", "amanhã"] {
            assert!(
                !is_hidden(&v, &entry(&v, "todo", Some(ruim), None), d("2026-01-01")),
                "{ruim}"
            );
        }
    }

    #[test]
    fn o_sentinela_tambem_e_adiavel() {
        let v = Vocab::default();
        assert!(!is_hidden(&v, &entry(&v, "xyz", None, None), d(TODAY)));
        assert!(is_hidden(
            &v,
            &entry(&v, "xyz", Some("2026-12-01"), None),
            d(TODAY)
        ));
    }

    #[test]
    fn scan_task_le_bloco_meta_historico_e_checklist() {
        let v = Vocab::default();
        let texto = "---\nprioridade: alta\n---\n\n> [!example] T\n> [[projects/p/tasks/x|x]]\n> _d_\n> - [ ] nota, não item\n\n## Passos\n\n- [x] um\n- [ ] dois\n  - [X] três\n- [ ]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\nnão conta\n- 2026-01-02 — desarquivada, de volta ao board\n\n## Depois\n\n- 2026-01-03 — nem isto\n";
        let e = scan_task(&v, "projects/p/tasks/x.md", texto.as_bytes()).unwrap();
        assert_eq!(e.status_num, 4);
        assert_eq!(e.model.desc, "d");
        assert_eq!(e.model.extras, vec!["- [ ] nota, não item"]);
        assert!(meta::is_high(&e.meta));
        assert_eq!(e.checklist, (2, 4));
        assert_eq!(
            e.historico,
            vec![
                "- 2026-01-01 — arquivada em `archived/done` (feito)",
                "- 2026-01-02 — desarquivada, de volta ao board"
            ]
        );
        assert_eq!(e.block.len(), 4);
    }

    #[test]
    fn scan_task_sem_callout_e_status_0_e_recusa_caminho_que_nao_e_task() {
        let v = Vocab::default();
        let e = scan_task(&v, "projects/p/tasks/x.md", b"# so titulo\n\n- [x] a\n").unwrap();
        assert_eq!(e.status_num, 0);
        assert!(e.block.is_empty());
        assert_eq!(e.model.title, "");
        assert_eq!(e.checklist, (1, 1));
        assert!(scan_task(&v, "notas/x.md", b"> [!todo] T\n").is_none());
        assert!(scan_task(&v, "projects/p/tasks/x.md", &[0xFF, 0xFE]).is_none());
    }

    #[test]
    fn indice_ordena_altas_fase_id_e_arquivadas_por_tipo_e_id() {
        let v = Vocab::default();
        let mk = |rel: &str, texto: &str| scan_task(&v, rel, texto.as_bytes()).unwrap();
        let idx = Indice::new(
            vec!["p".into(), "daily".into()],
            vec![
                mk("projects/p/tasks/b.md", "> [!todo] B\n> [[b]]\n"),
                mk("projects/p/tasks/a.md", "> [!todo] A\n> [[a]]\n"),
                mk(
                    "projects/p/tasks/z.md",
                    "---\nprioridade: alta\n---\n> [!check] Z\n> [[z]]\n",
                ),
                mk("projects/p/tasks/c.md", "> [!xyz] C\n> [[c]]\n"),
                mk(
                    "projects/p/tasks/archived/failed/f.md",
                    "> [!todo] F\n> [[f]]\n",
                ),
                mk(
                    "projects/p/tasks/archived/done/d2.md",
                    "> [!done] D\n> [[d]]\n",
                ),
                mk(
                    "projects/p/tasks/archived/done/d1.md",
                    "> [!done] D\n> [[d]]\n",
                ),
            ],
        );
        assert_eq!(idx.projetos, vec!["p"]);
        let ids: Vec<&str> = idx
            .entries_for("p")
            .iter()
            .map(|e| e.task_id.as_str())
            .collect();
        assert_eq!(ids, ["z", "c", "a", "b"]);
        let arq: Vec<&str> = idx
            .archived_for("p")
            .iter()
            .map(|e| e.task_id.as_str())
            .collect();
        assert_eq!(arq, ["d1", "d2", "f"]);
        assert_eq!(idx.por_id("d1").len(), 1);
        assert_eq!(idx.mostradas(&v, "p", d(TODAY)), 4);
    }
}
