//! A linha de uma task como o `list --json`, o `show --json` e a TUI a veem,
//! e o texto do `list`. Uma função só: o JSON e a tela saem do mesmo lugar.

use freitask_core::date::Date;
use freitask_core::index::{self, Entry, Indice};
use freitask_core::meta;
use freitask_core::path;
use freitask_core::status::Vocab;
use freitask_vault::Vault;
use serde_json::{Value, json};

/// O que a TUI e o `list --json` sabem de uma task, calculado num lugar só
/// (spec §1.7): a TUI desenha este `Resumo` e o JSON o serializa, então as
/// duas telas não divergem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resumo {
    pub project: String,
    pub id: String,
    /// O título da fase ("Não iniciada"), não o da task.
    pub status: String,
    pub status_num: u32,
    pub callout: String,
    pub title: String,
    pub desc: String,
    pub archived: Option<String>,
    pub suggested_archive: &'static str,
    /// O que o painel esconde por padrão: a fase Algum dia ou a task adiada.
    /// Nunca vale para arquivada, que tem o próprio toggle.
    pub hidden: bool,
    /// `"alta"` ou nada: só o valor que conta.
    pub prioridade: Option<&'static str>,
    /// O dia em que a task adiada volta; só para quem está adiada agora.
    pub volta_em: Option<String>,
    /// O caminho absoluto.
    pub path: String,
}

/// O `Resumo` de uma task, no dia `today`.
pub fn resumo(vault: &Vault, vocab: &Vocab, e: &Entry, today: Date) -> Resumo {
    let st = vocab.meta(e.status_num);
    let ativa = e.archived.is_none();
    // O tipo digitado, mesmo inválido: é o que o usuário precisa ver para
    // consertar; sem bloco, o callout do sentinela.
    let callout = if e.model.raw_callout.is_empty() {
        st.callout.clone()
    } else {
        e.model.raw_callout.clone()
    };
    Resumo {
        project: e.project.clone(),
        id: e.task_id.clone(),
        status: st.title.clone(),
        status_num: e.status_num,
        callout,
        title: e.model.title.clone(),
        desc: e.model.desc.clone(),
        archived: e.archived.clone(),
        suggested_archive: path::suggest_archive_type(vocab, e.status_num),
        hidden: ativa && index::is_hidden(vocab, e, today),
        prioridade: if ativa && meta::is_high(&e.meta) {
            Some(meta::HIGH)
        } else {
            None
        },
        volta_em: if ativa && meta::is_postponed(&e.meta, today) {
            e.meta.volta_em.clone()
        } else {
            None
        },
        path: vault.caminho_str(&e.rel),
    }
}

/// A linha do `list --json` (spec §2.2): os 13 campos de hoje mais `notes`,
/// `historico`, `checklist`, `bloqueada_por`, `bloqueada`, `bloqueia`,
/// `prazo` e `dias_para_o_prazo`. As chaves saem em ordem alfabética.
pub fn linha(vault: &Vault, vocab: &Vocab, indice: &Indice, e: &Entry, today: Date) -> Value {
    let r = resumo(vault, vocab, e, today);
    let deps = freitask_core::deps::dependencias(indice, e);
    json!({
        "project": r.project,
        "id": r.id,
        "status": r.status,
        "status_num": r.status_num,
        "callout": r.callout,
        "title": r.title,
        "desc": r.desc,
        "archived": r.archived,
        "suggested_archive": r.suggested_archive,
        "hidden": r.hidden,
        "prioridade": r.prioridade,
        "volta_em": r.volta_em,
        "path": r.path,
        "notes": e.model.extras,
        "historico": e.historico,
        "checklist": { "feitos": e.checklist.0, "total": e.checklist.1 },
        "bloqueada_por": e.meta.bloqueada_por,
        "bloqueada": deps.bloqueada,
        "bloqueia": deps.bloqueia,
        "prazo": e.meta.prazo,
        "dias_para_o_prazo": meta::dias_para_o_prazo(&e.meta, today),
    })
}

/// Os projetos e quantas tasks o painel mostra de cada um (o `active` do
/// `projects --json`).
pub fn projetos(vocab: &Vocab, indice: &Indice, today: Date) -> Vec<(String, usize)> {
    indice
        .projetos
        .iter()
        .map(|p| (p.clone(), indice.mostradas(vocab, p, today)))
        .collect()
}

/// A linha do `list` em texto: `%-14s %-34s %-12s %s`, com padding por bytes.
pub fn texto(vocab: &Vocab, e: &Entry) -> String {
    let st = vocab.meta(e.status_num);
    let arquivada = e
        .archived
        .as_ref()
        .map(|a| format!("[{a}]"))
        .unwrap_or_default();
    format!(
        "{} {} {} {}",
        freitask_core::pad_bytes(&e.project, 14),
        freitask_core::pad_bytes(&e.task_id, 34),
        freitask_core::pad_bytes(&st.title, 12),
        arquivada
    )
}

/// As linhas do `list`, na ordem do painel: por projeto, as ativas e, com
/// `archived`, as arquivadas do projeto logo depois.
pub fn todas(indice: &Indice, archived: bool) -> Vec<&Entry> {
    let mut out = Vec::new();
    for p in &indice.projetos {
        out.extend(indice.entries_for(p));
        if archived {
            out.extend(indice.archived_for(p));
        }
    }
    out
}
