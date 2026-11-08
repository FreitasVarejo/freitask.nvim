//! A linha de uma task como o `list --json`, o `show --json` e a TUI a veem,
//! e o texto do `list`. Uma função só: o JSON e a tela saem do mesmo lugar.

use freitask_core::date::Date;
use freitask_core::index::{self, Entry, Indice};
use freitask_core::meta;
use freitask_core::path;
use freitask_core::status::Vocab;
use freitask_vault::Vault;
use serde_json::{Value, json};

/// A linha do `list --json` (spec §2.2): os 13 campos de hoje mais `notes`,
/// `historico`, `checklist`, `bloqueada_por`, `bloqueada`, `bloqueia`,
/// `prazo` e `dias_para_o_prazo`. As chaves saem em ordem alfabética.
pub fn linha(vault: &Vault, vocab: &Vocab, indice: &Indice, e: &Entry, today: Date) -> Value {
    let st = vocab.meta(e.status_num);
    let ativa = e.archived.is_none();
    // O tipo digitado, mesmo inválido: é o que o usuário precisa ver para
    // consertar; sem bloco, o callout do sentinela.
    let callout = if e.model.raw_callout.is_empty() {
        st.callout.clone()
    } else {
        e.model.raw_callout.clone()
    };
    let hidden = ativa && index::is_hidden(vocab, e, today);
    let prioridade = if ativa && meta::is_high(&e.meta) {
        Some(meta::HIGH)
    } else {
        None
    };
    let volta_em = if ativa && meta::is_postponed(&e.meta, today) {
        e.meta.volta_em.clone()
    } else {
        None
    };
    let deps = freitask_core::deps::dependencias(indice, e);
    json!({
        "project": e.project,
        "id": e.task_id,
        "status": st.title,
        "status_num": e.status_num,
        "callout": callout,
        "title": e.model.title,
        "desc": e.model.desc,
        "archived": e.archived,
        "suggested_archive": path::suggest_archive_type(vocab, e.status_num),
        "hidden": hidden,
        "prioridade": prioridade,
        "volta_em": volta_em,
        "path": vault.caminho_str(&e.rel),
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
