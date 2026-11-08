//! Tudo que se deriva de um CAMINHO: a que projeto uma task pertence, se está
//! arquivada, e o `kebab` que vira id. O caminho é a fonte de verdade das duas
//! primeiras; não há flag de frontmatter para nenhuma.
//!
//! Caminhos aqui são sempre relativos ao vault (`projects/p/tasks/x.md`): a
//! crate `vault` traduz os absolutos antes de chamar.

use crate::config;
use crate::status::Vocab;

/// Dobra de acentos para ids kebab-case (títulos em PT-BR). A mesma tabela do
/// Lua; o que não está nela e não é ASCII alfanumérico cai fora.
fn dobra_acento(c: char) -> Option<char> {
    Some(match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'ê' | 'è' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ô' | 'õ' | 'ò' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        _ => return None,
    })
}

/// Converte um texto qualquer num identificador kebab-case: minúscula
/// (Unicode, como `vim.fn.tolower`, para `Á` virar `á` e dobrar em `a` em vez
/// de sumir), acentos dobrados, só `[a-z0-9]`, espaço e `_` viram `-`, `-`
/// repetido colapsa e as pontas são aparadas.
pub fn kebab(s: &str) -> String {
    let mut saida = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        let c = dobra_acento(c).unwrap_or(c);
        if c.is_ascii_alphanumeric() {
            saida.push(c);
        } else if c == '-' || c == '_' || crate::eh_espaco(c) {
            // Espaço e `_` viram `-`; `-` repetido colapsa logo abaixo.
            saida.push('-');
        }
        // O resto (pontuação, símbolo, letra fora da tabela) é descartado.
    }
    let mut out = String::with_capacity(saida.len());
    for c in saida.chars() {
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches('-').to_string()
}

/// Um arquivo de task decomposto pelo caminho.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskRef {
    pub project: String,
    pub id: String,
    pub archived: Option<String>,
}

/// Decompõe o caminho (relativo ao vault) de um arquivo de task:
///   `projects/<p>/tasks/<id>.md`                    → (p, id, None)
///   `projects/<p>/tasks/archived/<tipo>/<id>.md`    → (p, id, Some(tipo))
/// Qualquer outra coisa (projeto reservado, `*.sync-conflict-*`, tipo de
/// archived desconhecido, subpasta) é `None`: é o guarda único que impede um
/// arquivo fora de padrão de entrar no índice ou no painel.
pub fn split_task_path(rel: &str) -> Option<TaskRef> {
    let rest = rel.strip_prefix("projects/")?;
    let (project, rest) = rest.split_once('/')?;
    let rest = rest.strip_prefix("tasks/")?;
    let rest = rest.strip_suffix(".md")?;
    if project.is_empty() || rest.is_empty() || config::is_reserved(project) {
        return None;
    }
    // Conflito do Syncthing ao lado da task real: nunca é uma task; o doctor o
    // acusa (é informação a resolver, não lixo a varrer para baixo do tapete).
    if rest.contains(".sync-conflict-") {
        return None;
    }
    if let Some(sub) = rest.strip_prefix("archived/") {
        let (tipo, id) = sub.split_once('/')?;
        if id.is_empty() || id.contains('/') || !config::is_archived_type(tipo) {
            return None;
        }
        return Some(TaskRef {
            project: project.into(),
            id: id.into(),
            archived: Some(tipo.into()),
        });
    }
    if rest.contains('/') {
        return None;
    }
    Some(TaskRef {
        project: project.into(),
        id: rest.into(),
        archived: None,
    })
}

/// Caminho (relativo ao vault) do arquivo de uma task.
pub fn task_rel(project: &str, id: &str, archived: Option<&str>) -> String {
    match archived {
        Some(t) => format!("projects/{project}/tasks/archived/{t}/{id}.md"),
        None => format!("projects/{project}/tasks/{id}.md"),
    }
}

/// Diretório vault-relative de uma task, alvo do wikilink da linha 2.
pub fn vault_dir(project: &str, archived: Option<&str>) -> String {
    match archived {
        Some(t) => format!("projects/{project}/tasks/archived/{t}"),
        None => format!("projects/{project}/tasks"),
    }
}

/// Pasta de um projeto, viva ou encerrada, e o prefixo (com barra final) dos
/// wikilinks que apontam para dentro dela.
pub fn project_dir(project: &str, closed: bool) -> (String, String) {
    let rel = if closed {
        format!("projects/{}/{project}/", config::CLOSED_DIR)
    } else {
        format!("projects/{project}/")
    };
    (rel[..rel.len() - 1].to_string(), rel)
}

pub fn is_archived(rel: &str) -> bool {
    split_task_path(rel).is_some_and(|t| t.archived.is_some())
}

/// Tipo de arquivamento sugerido para uma fase: só o DEFAULT do prompt.
pub fn suggest_archive_type(vocab: &Vocab, status_num: u32) -> &'static str {
    let callout = &vocab.meta(status_num).callout;
    if config::DONE_CALLOUTS.contains(&callout.as_str()) {
        "done"
    } else {
        "dropped"
    }
}

/// O `:t:r` do Vim: nome do arquivo sem a última extensão (um `.` na primeira
/// posição não conta como extensão).
pub fn basename_sem_extensao(p: &str) -> String {
    let tail = p.rsplit('/').next().unwrap_or(p);
    match tail.rfind('.') {
        Some(0) | None => tail.to_string(),
        Some(i) => tail[..i].to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn kebab_dobra_acentos_multibyte() {
        assert_eq!(kebab("Configuração da API"), "configuracao-da-api");
        assert_eq!(kebab("Ávida Tarefa"), "avida-tarefa");
    }

    #[test]
    fn kebab_descarta_pontuacao_e_colapsa_separadores() {
        assert_eq!(kebab("Fix: the thing!"), "fix-the-thing");
        assert_eq!(kebab("  foo   bar  "), "foo-bar");
        assert_eq!(kebab("--foo--bar--"), "foo-bar");
    }

    #[test]
    fn kebab_converte_underscore_em_dash() {
        assert_eq!(kebab("foo_bar"), "foo-bar");
        assert_eq!(kebab("foo __ bar"), "foo-bar");
    }

    #[test]
    fn kebab_devolve_vazio_quando_nao_sobra_nada() {
        assert_eq!(kebab("!!!"), "");
        assert_eq!(kebab(""), "");
    }

    #[test]
    fn split_task_path_decompoe_uma_task_ativa() {
        let t = split_task_path("projects/bjju-web/tasks/fazer-x.md").unwrap();
        assert_eq!(
            (t.project.as_str(), t.id.as_str(), t.archived),
            ("bjju-web", "fazer-x", None)
        );
    }

    #[test]
    fn split_task_path_decompoe_uma_task_arquivada() {
        let t = split_task_path("projects/bjju-web/tasks/archived/done/fazer-x.md").unwrap();
        assert_eq!(t.archived.as_deref(), Some("done"));
        assert_eq!(t.id, "fazer-x");
    }

    #[test]
    fn split_task_path_recusa_tipo_de_arquivamento_desconhecido() {
        assert!(split_task_path("projects/bjju-web/tasks/archived/talvez/fazer-x.md").is_none());
    }

    #[test]
    fn split_task_path_recusa_diretorios_reservados() {
        assert!(split_task_path("projects/daily/tasks/2026-08-09.md").is_none());
        assert!(split_task_path("projects/templates/tasks/task.md").is_none());
        assert!(split_task_path("projects/_encerrados/tasks/x.md").is_none());
    }

    #[test]
    fn split_task_path_nao_enxerga_projeto_encerrado() {
        assert!(split_task_path("projects/_encerrados/p/tasks/x.md").is_none());
        assert!(split_task_path("projects/_encerrados/p/tasks/archived/done/x.md").is_none());
    }

    #[test]
    fn split_task_path_recusa_conflito_do_syncthing() {
        assert!(
            split_task_path("projects/bjju-web/tasks/x.sync-conflict-20260809-123456-ABCDEFG.md")
                .is_none()
        );
    }

    #[test]
    fn split_task_path_recusa_subdiretorio_que_nao_e_archived() {
        assert!(split_task_path("projects/bjju-web/tasks/notas/x.md").is_none());
        assert!(split_task_path("projects/p/tasks/archived/done/sub/x.md").is_none());
        assert!(split_task_path("projects/p/tasks/.md").is_none());
        assert!(split_task_path("notas/x.md").is_none());
    }

    #[test]
    fn project_dir_devolve_a_pasta_e_o_prefixo_viva_e_encerrada() {
        assert_eq!(
            project_dir("p", false),
            ("projects/p".into(), "projects/p/".into())
        );
        assert_eq!(
            project_dir("p", true),
            (
                "projects/_encerrados/p".into(),
                "projects/_encerrados/p/".into()
            )
        );
    }

    #[test]
    fn suggest_archive_type_deriva_done_dos_callouts_de_conclusao() {
        let v = Vocab::default();
        assert_eq!(
            suggest_archive_type(&v, v.by_callout("done").unwrap()),
            "done"
        );
        assert_eq!(
            suggest_archive_type(&v, v.by_callout("check").unwrap()),
            "done"
        );
    }

    #[test]
    fn suggest_archive_type_nao_deriva_failed_de_fase_nenhuma() {
        let v = Vocab::default();
        for n in 1..=5 {
            assert_eq!(suggest_archive_type(&v, n), "dropped");
        }
    }

    #[test]
    fn suggest_archive_type_cai_em_dropped_inclusive_status_0() {
        let v = Vocab::default();
        assert_eq!(suggest_archive_type(&v, 1), "dropped");
        assert_eq!(suggest_archive_type(&v, 0), "dropped");
    }

    #[test]
    fn is_archived_le_o_estado_do_caminho() {
        assert!(is_archived("projects/p/tasks/archived/done/x.md"));
        assert!(!is_archived("projects/p/tasks/x.md"));
    }

    #[test]
    fn task_rel_e_vault_dir_montam_os_dois_enderecos() {
        assert_eq!(task_rel("p", "x", None), "projects/p/tasks/x.md");
        assert_eq!(
            task_rel("p", "x", Some("done")),
            "projects/p/tasks/archived/done/x.md"
        );
        assert_eq!(vault_dir("p", None), "projects/p/tasks");
        assert_eq!(
            vault_dir("p", Some("dropped")),
            "projects/p/tasks/archived/dropped"
        );
    }

    #[test]
    fn basename_sem_extensao_e_o_t_r_do_vim() {
        assert_eq!(basename_sem_extensao("projects/p/tasks/x"), "x");
        assert_eq!(basename_sem_extensao("x.md"), "x");
        assert_eq!(basename_sem_extensao("a/b/foo.bar"), "foo");
        assert_eq!(basename_sem_extensao(".hidden"), ".hidden");
    }
}
