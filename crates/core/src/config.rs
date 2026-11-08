//! Constantes do formato. Sem lógica, sem estado: é o vocabulário fixo, de
//! onde todo o resto importa sem risco de ciclo. O significado de cada uma no
//! formato das tasks está em `docs/freitask.md`.

/// Pasta dos projetos encerrados, um nível abaixo do glob `projects/*/tasks`.
pub const CLOSED_DIR: &str = "_encerrados";

/// Nomes que não podem virar projeto. `daily` e `templates` já não colidiriam
/// com nada, mas seguem vetados para não reintroduzir a ambiguidade que a
/// mudança de layout removeu; `_encerrados` é a pasta dos encerrados.
pub const RESERVED: [&str; 3] = ["daily", "templates", CLOSED_DIR];

pub fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

/// Tipos de arquivamento, subdiretórios de `tasks/archived/`. São três de
/// propósito: `done` é derivável do callout, `dropped` é a única decisão que o
/// status não carrega, `failed` se escolhe à mão.
pub const ARCHIVED_TYPES: [&str; 3] = ["done", "dropped", "failed"];

pub fn is_archived_type(t: &str) -> bool {
    ARCHIVED_TYPES.contains(&t)
}

/// Glosa em PT-BR de cada tipo, só para a linha do `## Histórico`.
pub fn archived_label(t: &str) -> &str {
    match t {
        "done" => "feito",
        "dropped" => "abandonado",
        "failed" => "falhou",
        outro => outro,
    }
}

/// Callouts que sugerem `done` como tipo de arquivamento. Não há callout que
/// sugira `failed`: nenhuma fase significa "falhou".
pub const DONE_CALLOUTS: [&str; 2] = ["done", "check"];

/// Callouts das fases que o painel e a TUI escondem por padrão (Algum dia).
pub const HIDDEN_CALLOUTS: [&str; 1] = ["quote"];

/// O vocabulário default, espelhado em `.freitask/status.json` quando ele
/// falta. As nove fases na ordem da narrativa; ver `docs/freitask.md`.
pub const DEFAULT_STATUS_JSON: &str = r#"{
  "1": { "title": "Não iniciada", "callout": "todo", "icon": "󰗡", "hl_group": "DiagnosticInfo" },
  "2": { "title": "Refinando", "callout": "abstract", "icon": "󰨸", "hl_group": "DiagnosticInfo" },
  "3": { "title": "Refinada", "callout": "tip", "icon": "󰌶", "hl_group": "DiagnosticHint" },
  "4": { "title": "Em implementação", "callout": "example", "icon": "󰉹", "hl_group": "DiagnosticHint" },
  "5": { "title": "Em homologação", "callout": "question", "icon": "󰘥", "hl_group": "DiagnosticWarn" },
  "6": { "title": "Bloqueada", "callout": "warning", "icon": "󰀪", "hl_group": "DiagnosticWarn" },
  "7": { "title": "Pronta", "callout": "check", "icon": "󰄬", "hl_group": "DiagnosticOk" },
  "8": { "title": "Algum dia", "callout": "quote", "icon": "󱆨", "hl_group": "Comment" },
  "9": { "title": "Arquivada", "callout": "done", "icon": "󰄬", "hl_group": "DiagnosticOk" }
}"#;

/// Títulos do `status.json` antigo (pré-callouts) e o callout equivalente
/// hoje. Usado só por `model::is_status_text`, para reconhecer e descartar as
/// linhas `> N - Título` que a migração deixou pelo vault.
pub const LEGACY_STATUS_TITLES: [(&str, &str); 4] = [
    ("Backlog", "todo"),
    ("In Progress", "example"),
    ("Blocked", "warning"),
    ("Review", "question"),
];

pub fn legacy_callout(title: &str) -> Option<&'static str> {
    LEGACY_STATUS_TITLES
        .iter()
        .find(|(t, _)| *t == title)
        .map(|(_, c)| *c)
}

/// Versão da CLI e do contrato JSON (`version --json`).
pub const VERSAO: &str = "2.0.0";
pub const JSON_VERSAO: u32 = 1;
