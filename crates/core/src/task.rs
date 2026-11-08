//! As regras puras das operações sobre uma task: validar projeto e id, a fase
//! pedida ao `set`, a data de volta, a fase depois de (des)arquivar, o
//! template de uma task nova. O disco entra nas fatias seguintes (planos).

use crate::config;
use crate::date::Date;
use crate::model::{self, Model};
use crate::path;
use crate::status::Vocab;
use crate::trim;

/// Valida o NOME de um projeto: não vazio, sem `/`, não reservado, nem `.`/`..`
/// (D2: o Lua os aceitava e gravava `tasks/<id>.md` na raiz do vault).
pub fn validate_project(project: &str) -> Result<(), String> {
    let p = trim(project);
    if p.is_empty() {
        return Err("falta o projeto".into());
    }
    if p.contains('/') {
        return Err(format!("projeto não pode conter '/': {p}"));
    }
    if config::is_reserved(p) || p == "." || p == ".." {
        return Err(format!("\"{p}\" é um diretório reservado, não um projeto"));
    }
    Ok(())
}

/// Valida projeto e id de uma task nova, sem tocar o disco. `kebab(id) == id`
/// em vez de corrigir: o id é o nome do arquivo e o da branch git.
pub fn validate_new(project: &str, id: &str) -> Result<(), String> {
    let id = trim(id);
    validate_project(project)?;
    if id.is_empty() {
        return Err("falta o id".into());
    }
    let k = path::kebab(id);
    if k != id {
        return Err(format!(
            "id precisa ser kebab-case: \"{id}\" (seria \"{k}\")"
        ));
    }
    Ok(())
}

/// O número da fase pedida ao `set`. `done` fica de fora (quem grava `done` é
/// `archive <id> done`), e uma task em `archived/done/` não troca de fase sem
/// sair de lá. Erro com `Some(num)` é fase que existe mas foi recusada (CLI
/// sai 1); com `None`, fase desconhecida (CLI sai 2).
pub fn check_status_change(
    vocab: &Vocab,
    callout: &str,
    archived: Option<&str>,
) -> Result<u32, (String, Option<u32>)> {
    let Some(num) = vocab.by_callout(callout) else {
        return Err((format!("fase desconhecida: {callout}"), None));
    };
    if callout == "done" {
        return Err((
            "`done` é gravado por `freitask archive <id> done`, não por `set`".into(),
            Some(num),
        ));
    }
    if archived == Some("done") {
        return Err((
            "task em archived/done/ fica `done`; desarquive antes (`freitask unarchive <id>`)"
                .into(),
            Some(num),
        ));
    }
    Ok(num)
}

/// Valida `set --volta-em`: `Some(data)` para gravar, `None` para limpar (texto
/// vazio), erro para o que não é uma data de um dia que existe.
pub fn check_volta_em(text: &str) -> Result<Option<String>, String> {
    check_data("volta-em", text)
}

/// Idem para `set --prazo`.
pub fn check_prazo(text: &str) -> Result<Option<String>, String> {
    check_data("prazo", text)
}

fn check_data(_campo: &str, text: &str) -> Result<Option<String>, String> {
    let t = trim(text);
    if t.is_empty() {
        return Ok(None);
    }
    if Date::parse(t).is_none() {
        return Err(format!(
            "data inválida: \"{t}\" (use AAAA-MM-DD, de um dia que existe; \"\" limpa)"
        ));
    }
    Ok(Some(t.to_string()))
}

/// A fase depois de (des)arquivar: `archive done` grava `done`; desarquivar
/// `done` devolve `check`; o resto não muda.
pub fn status_after_move(vocab: &Vocab, status_num: u32, dest: Option<&str>) -> u32 {
    if dest == Some("done") {
        return vocab.by_callout("done").unwrap_or(status_num);
    }
    if dest.is_none() && Some(status_num) == vocab.by_callout("done") {
        return vocab.by_callout("check").unwrap_or(status_num);
    }
    status_num
}

/// O arquivo de uma task nova: o bloco e uma linha em branco, nada mais.
pub fn template(vocab: &Vocab, model: &Model) -> Vec<String> {
    let mut out = model::serialize_block(vocab, model);
    out.push(String::new());
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn fase(v: &Vocab, c: &str) -> u32 {
        v.by_callout(c).unwrap()
    }

    #[test]
    fn validate_new_recusa_projeto_reservado() {
        let err = validate_new("daily", "x").unwrap_err();
        assert!(err.contains("reservado"));
        assert!(validate_new("templates", "x").is_err());
        assert!(validate_new("_encerrados", "x").is_err());
    }

    #[test]
    fn validate_new_recusa_projeto_vazio_ou_com_barra() {
        assert!(validate_new("", "x").is_err());
        assert!(validate_new("a/b", "x").is_err());
    }

    #[test]
    fn validate_new_recusa_id_fora_de_kebab_sem_corrigir() {
        let err = validate_new("dotfiles", "meu id").unwrap_err();
        assert!(err.contains("kebab"));
        assert!(err.contains("meu-id"), "a mensagem sugere o id corrigido");
        assert!(validate_new("dotfiles", "Maiúscula").is_err());
        assert!(validate_new("dotfiles", "").is_err());
    }

    #[test]
    fn validate_new_aceita_kebab_com_digito_e_acento_dobrado() {
        assert!(validate_new("dotfiles", "vault-lint-2").is_ok());
        assert!(validate_new("dotfiles", &path::kebab("Configuração da API")).is_ok());
    }

    #[test]
    fn validate_project_recusa_vazio_barra_e_reservado_com_a_mesma_mensagem() {
        assert!(validate_project("").is_err());
        assert!(validate_project("   ").is_err());
        assert!(validate_project("a/b").is_err());
        for r in ["daily", "templates", "_encerrados"] {
            let err = validate_project(r).unwrap_err();
            assert!(err.contains("reservado"), "{err}");
            assert_eq!(
                validate_new(r, "x").unwrap_err(),
                err,
                "uma noção de reservado só"
            );
        }
    }

    #[test]
    fn validate_project_recusa_ponto_e_ponto_ponto() {
        assert!(validate_project(".").unwrap_err().contains("reservado"));
        assert!(validate_project("..").unwrap_err().contains("reservado"));
    }

    #[test]
    fn validate_project_aceita_um_nome_aparado() {
        assert!(validate_project("dotfiles").is_ok());
        assert!(validate_project("  dotfiles  ").is_ok());
    }

    #[test]
    fn template_e_o_bloco_e_uma_linha_em_branco() {
        let m = Model {
            status_num: 1,
            title: "Exemplo".into(),
            id: "exemplo".into(),
            project: Some("dotfiles".into()),
            ..Model::default()
        };
        assert_eq!(
            template(&Vocab::default(), &m),
            vec![
                "> [!todo] Exemplo",
                "> [[projects/dotfiles/tasks/exemplo|exemplo]]",
                ""
            ]
        );
    }

    #[test]
    fn template_volta_pelo_parser_com_o_mesmo_modelo() {
        let vo = Vocab::default();
        let m = Model {
            status_num: 1,
            title: "Exemplo".into(),
            id: "exemplo".into(),
            desc: "estado".into(),
            project: Some("dotfiles".into()),
            ..Model::default()
        };
        let out = template(&vo, &m);
        let back = model::parse_block(&vo, &out[..3]);
        assert_eq!(back.title, "Exemplo");
        assert_eq!(back.id, "exemplo");
        assert_eq!(back.desc, "estado");
        assert_eq!(back.status_num, 1);
    }

    #[test]
    fn status_after_move_arquivar_como_done_grava_done() {
        let v = Vocab::default();
        assert_eq!(
            status_after_move(&v, fase(&v, "todo"), Some("done")),
            fase(&v, "done")
        );
        assert_eq!(
            status_after_move(&v, fase(&v, "check"), Some("done")),
            fase(&v, "done")
        );
    }

    #[test]
    fn status_after_move_dropped_e_failed_nao_mexem() {
        let v = Vocab::default();
        assert_eq!(
            status_after_move(&v, fase(&v, "todo"), Some("dropped")),
            fase(&v, "todo")
        );
        assert_eq!(
            status_after_move(&v, fase(&v, "todo"), Some("failed")),
            fase(&v, "todo")
        );
        assert_eq!(
            status_after_move(&v, fase(&v, "done"), Some("dropped")),
            fase(&v, "done")
        );
    }

    #[test]
    fn status_after_move_desarquivar_devolve_done_como_check() {
        let v = Vocab::default();
        assert_eq!(
            status_after_move(&v, fase(&v, "done"), None),
            fase(&v, "check")
        );
    }

    #[test]
    fn status_after_move_desarquivar_nao_mexe_em_outra_fase() {
        let v = Vocab::default();
        assert_eq!(
            status_after_move(&v, fase(&v, "todo"), None),
            fase(&v, "todo")
        );
        assert_eq!(
            status_after_move(&v, fase(&v, "check"), None),
            fase(&v, "check")
        );
    }

    #[test]
    fn check_status_change_devolve_o_numero_da_fase() {
        let v = Vocab::default();
        assert_eq!(
            check_status_change(&v, "example", None),
            Ok(fase(&v, "example"))
        );
        assert_eq!(
            check_status_change(&v, "warning", Some("dropped")),
            Ok(fase(&v, "warning"))
        );
    }

    #[test]
    fn check_status_change_recusa_fase_desconhecida() {
        let (msg, num) = check_status_change(&Vocab::default(), "bogus", None).unwrap_err();
        assert!(msg.contains("desconhecida"));
        assert_eq!(num, None);
    }

    #[test]
    fn check_status_change_recusa_done() {
        let (_, num) = check_status_change(&Vocab::default(), "done", None).unwrap_err();
        assert!(num.is_some());
    }

    #[test]
    fn check_status_change_recusa_trocar_quem_esta_em_archived_done() {
        assert!(check_status_change(&Vocab::default(), "check", Some("done")).is_err());
    }

    #[test]
    fn check_volta_em_devolve_a_data_como_esta() {
        assert_eq!(check_volta_em("2026-12-01"), Ok(Some("2026-12-01".into())));
        assert_eq!(
            check_volta_em("  2026-12-01  "),
            Ok(Some("2026-12-01".into()))
        );
        assert_eq!(check_volta_em("2024-02-29"), Ok(Some("2024-02-29".into())));
    }

    #[test]
    fn check_volta_em_vazio_pede_para_limpar() {
        assert_eq!(check_volta_em(""), Ok(None));
        assert_eq!(check_volta_em("   "), Ok(None));
    }

    #[test]
    fn check_volta_em_recusa_o_que_nao_e_data_dizendo_o_valor_e_o_formato() {
        for ruim in [
            "2026-12-1",
            "2026-02-30",
            "2025-02-29",
            "01/12/2026",
            "amanhã",
            "2026-12-01T10:00",
            "hoje",
        ] {
            let err = check_volta_em(ruim).unwrap_err();
            assert!(
                err.contains(ruim) && err.contains("AAAA-MM-DD"),
                "{ruim}: {err}"
            );
        }
    }

    #[test]
    fn check_volta_em_aceita_data_passada() {
        assert_eq!(check_volta_em("2020-01-01"), Ok(Some("2020-01-01".into())));
    }

    #[test]
    fn check_prazo_tem_a_mesma_regra() {
        assert_eq!(check_prazo("2026-12-01"), Ok(Some("2026-12-01".into())));
        assert_eq!(check_prazo(""), Ok(None));
        assert!(check_prazo("amanhã").is_err());
    }
}
