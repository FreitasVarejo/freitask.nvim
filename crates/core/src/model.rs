//! Parser e serializer do bloco de callout de uma task. Puro: linhas entram,
//! `Model` sai, e vice-versa. O parser é TOLERANTE (formatos legados; callout
//! desconhecido vira status 0); quem é estrito é quem valida a entrada.

use crate::md;
use crate::path;
use crate::status::Vocab;
use crate::{config, trim};

/// O bloco de uma task, decomposto.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Model {
    /// 0 = callout não reconhecido.
    pub status_num: u32,
    /// O tipo digitado, preservado verbatim quando `status_num == 0`.
    pub raw_callout: String,
    pub title: String,
    /// == nome do arquivo == nome da branch.
    pub id: String,
    /// A linha em itálico; vazia quando ausente.
    pub desc: String,
    /// Notas e impedimentos, texto livre.
    pub extras: Vec<String>,
    /// Sem ele, o link sai no formato legado `[[id]]`.
    pub project: Option<String>,
    /// Tipo de arquivamento; move o alvo do link.
    pub archived: Option<String>,
}

/// Verdadeiro se `content` é um "texto de status" legado (`1 - Backlog`), e o
/// número da fase pela chave ATUAL do título casado, não pelo dígito da linha.
pub fn is_status_text(vocab: &Vocab, content: &str) -> (bool, Option<u32>) {
    // `^(%d+)%s*%-%s*(.+)$`
    let digitos = content.chars().take_while(char::is_ascii_digit).count();
    if digitos == 0 {
        return (false, None);
    }
    let resto = content[digitos..].trim_start_matches(crate::eh_espaco);
    let Some(resto) = resto.strip_prefix('-') else {
        return (false, None);
    };
    let titulo = resto.trim_start_matches(crate::eh_espaco);
    if titulo.is_empty() {
        return (false, None);
    }
    let titulo = trim(titulo);
    if let Some(n) = vocab.by_title(titulo) {
        return (true, Some(n));
    }
    // Vocabulário legado que já não existe no status.json: a linha é
    // reconhecida e descartada do mesmo jeito; sem o callout no vault, 0.
    if let Some(callout) = config::legacy_callout(titulo) {
        return (true, Some(vocab.by_callout(callout).unwrap_or(0)));
    }
    (false, None)
}

/// `%*%*(.-)%*%*`: o primeiro trecho em negrito.
fn negrito(s: &str) -> Option<&str> {
    let i = s.find("**")?;
    let resto = &s[i + 2..];
    let j = resto.find("**")?;
    Some(&resto[..j])
}

/// `%[%[(.-)%]%]`: o miolo do primeiro wikilink.
fn primeiro_wikilink(s: &str) -> Option<&str> {
    let i = s.find("[[")?;
    let resto = &s[i + 2..];
    let j = resto.find("]]")?;
    Some(&resto[..j])
}

/// Parseia o bloco `>` de uma task. O status vem do TIPO do callout da linha
/// 1; título da linha 1 (novo) ou de `**...**` (legado); id do primeiro
/// `[[...]]`. Linhas de status-texto, id-só e `Branch:` são consumidas; o
/// resto vira `extras`.
pub fn parse_block(vocab: &Vocab, block: &[String]) -> Model {
    let mut model = Model {
        status_num: 1,
        ..Model::default()
    };

    let first = block
        .first()
        .map(|l| md::strip_quote(l))
        .unwrap_or_default();
    // Match PERMISSIVO (`[^%]]*`): um tipo inválido pode ter acento ou hífen.
    let callout: Option<&str> = first
        .strip_prefix("[!")
        .and_then(|r| r.find(']').map(|j| &r[..j]));
    if let Some(c) = callout {
        model.raw_callout = c.to_string();
        model.status_num = vocab.by_callout(c).unwrap_or(0);
    }
    let rest: &str = match callout {
        Some(c) => first[2 + c.len() + 1..].trim_start_matches(crate::eh_espaco),
        None => &first,
    };
    if !rest.is_empty() && !is_status_text(vocab, rest).0 {
        model.title = trim(negrito(rest).unwrap_or(rest)).to_string();
    }

    let mut seen_note = false;
    for linha in block.iter().skip(1) {
        let content = md::strip_quote(linha);
        if model.id.is_empty()
            && let Some(lid) = primeiro_wikilink(&content)
        {
            let target = lid.split('|').next().unwrap_or("");
            model.id = path::basename_sem_extensao(trim(target));
        }
        let aparada = trim(&content);
        let is_only_link =
            aparada.starts_with("[[") && aparada.ends_with("]]") && aparada.len() >= 4 && {
                // `^%s*%[%[.-%]%]%s*$`: o primeiro `]]` fecha a linha.
                let miolo = &aparada[2..];
                miolo.find("]]").is_some_and(|j| j + 2 == miolo.len())
            };
        let (statusy, snum) = is_status_text(vocab, &content);
        let is_branch = content.starts_with("Branch:") || content.starts_with("branch:");
        let bold = negrito(&content);
        let italic = content.strip_prefix('_').and_then(|r| r.strip_suffix('_'));
        if is_only_link || statusy || is_branch {
            if statusy && callout.is_none() {
                model.status_num = snum.unwrap_or(0); // fallback: linha 1 sem callout
            }
        } else if let Some(b) = bold.filter(|_| model.title.is_empty()) {
            model.title = trim(b).to_string();
        } else if let Some(it) =
            italic.filter(|it| !it.is_empty() && model.desc.is_empty() && !seen_note)
        {
            model.desc = trim(it).to_string();
        } else {
            if !trim(&content).is_empty() {
                seen_note = true;
            }
            model.extras.push(content);
        }
    }
    model
}

/// Reconstrói o bloco `>` a partir do modelo, na ordem canônica.
pub fn serialize_block(vocab: &Vocab, model: &Model) -> Vec<String> {
    // Status válido → o callout canônico; 0 → o tipo digitado verbatim, para o
    // erro sobreviver ao round-trip e seguir consertável.
    let callout = if model.status_num == 0 {
        let c = trim(&model.raw_callout);
        if c.is_empty() {
            "invalid".to_string()
        } else {
            c.to_string()
        }
    } else {
        let c = &vocab.meta(model.status_num).callout;
        if c.is_empty() {
            "note".to_string()
        } else {
            c.clone()
        }
    };
    let link = match &model.project {
        Some(p) => format!(
            "{}/{}|{}",
            path::vault_dir(p, model.archived.as_deref()),
            model.id,
            model.id
        ),
        None => model.id.clone(),
    };
    let mut out = vec![
        format!("> [!{callout}] {}", model.title),
        format!("> [[{link}]]"),
    ];
    // Omitida quando vazia: um `>` vazio no meio promoveria a primeira nota a
    // descrição no re-parse.
    let desc = trim(&model.desc);
    if !desc.is_empty() {
        out.push(format!("> _{desc}_"));
    }
    let mut extras = model.extras.clone();
    while extras.last().is_some_and(|e| trim(e).is_empty()) {
        extras.pop();
    }
    for e in &extras {
        out.push(if trim(e).is_empty() {
            ">".to_string()
        } else {
            format!("> {e}")
        });
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    fn fase(vocab: &Vocab, c: &str) -> u32 {
        vocab.by_callout(c).unwrap()
    }

    #[test]
    fn is_status_text_reconhece_um_titulo_vigente() {
        let vo = Vocab::default();
        assert_eq!(is_status_text(&vo, "1 - Não iniciada"), (true, Some(1)));
    }

    #[test]
    fn is_status_text_reconhece_o_vocabulario_legado() {
        let vo = Vocab::default();
        assert_eq!(is_status_text(&vo, "1 - Backlog"), (true, Some(1)));
        assert_eq!(
            is_status_text(&vo, "3 - Blocked"),
            (true, Some(fase(&vo, "warning")))
        );
        assert_eq!(
            is_status_text(&vo, "4 - In Progress"),
            (true, Some(fase(&vo, "example")))
        );
        assert_eq!(
            is_status_text(&vo, "5 - Review"),
            (true, Some(fase(&vo, "question")))
        );
    }

    #[test]
    fn o_legado_resolve_pelo_callout_do_vocabulario_do_vault() {
        let sete = Vocab::from_json(
            r#"{"1":{"title":"Não iniciada","callout":"todo"},"2":{"title":"Refinando","callout":"abstract"},
                "3":{"title":"Em implementação","callout":"example"},"4":{"title":"Em homologação","callout":"question"},
                "5":{"title":"Bloqueada","callout":"warning"},"6":{"title":"Pronta","callout":"check"},"7":{"title":"Arquivada","callout":"done"}}"#,
        )
        .unwrap();
        assert_eq!(is_status_text(&sete, "3 - Blocked").1, Some(5));
        assert_eq!(is_status_text(&sete, "4 - In Progress").1, Some(3));
        assert_eq!(is_status_text(&sete, "5 - Review").1, Some(4));
        let sem_warning = Vocab::from_json(
            r#"{"1":{"title":"Não iniciada","callout":"todo"},"6":{"title":"Pronta","callout":"check"}}"#,
        )
        .unwrap();
        assert_eq!(
            is_status_text(&sem_warning, "3 - Blocked"),
            (true, Some(0)),
            "reconhecida, e vira Sem status"
        );
    }

    #[test]
    fn is_status_text_usa_a_chave_atual_do_titulo() {
        let vo = Vocab::default();
        assert_eq!(
            is_status_text(&vo, "99 - Arquivada"),
            (true, Some(fase(&vo, "done")))
        );
    }

    #[test]
    fn is_status_text_nao_come_nota_que_so_parece_status() {
        let vo = Vocab::default();
        assert_eq!(is_status_text(&vo, "3 - comprar leite"), (false, None));
        assert_eq!(is_status_text(&vo, "nota qualquer"), (false, None));
        assert_eq!(is_status_text(&vo, "3 -"), (false, None));
    }

    #[test]
    fn parse_block_le_o_formato_atual_completo() {
        let vo = Vocab::default();
        let m = parse_block(
            &vo,
            &v(&[
                "> [!todo] Título da task",
                "> [[projects/bjju-web/tasks/fazer-x|fazer-x]]",
                "> _Em análise do Fábio_",
                "> impedimento: falta VPN",
            ]),
        );
        assert_eq!(m.status_num, 1);
        assert_eq!(m.raw_callout, "todo");
        assert_eq!(m.title, "Título da task");
        assert_eq!(m.id, "fazer-x");
        assert_eq!(m.desc, "Em análise do Fábio");
        assert_eq!(m.extras, v(&["impedimento: falta VPN"]));
    }

    #[test]
    fn parse_block_omite_a_descricao_quando_nao_esta_em_italico() {
        let m = parse_block(
            &Vocab::default(),
            &v(&[
                "> [!todo] T",
                "> [[projects/p/tasks/x|x]]",
                "> uma nota qualquer",
            ]),
        );
        assert_eq!(m.desc, "");
        assert_eq!(m.extras, v(&["uma nota qualquer"]));
    }

    #[test]
    fn parse_block_nao_promove_italico_depois_de_nota() {
        let m = parse_block(
            &Vocab::default(),
            &v(&[
                "> [!todo] T",
                "> [[projects/p/tasks/x|x]]",
                "> nota primeiro",
                "> _isso continua nota_",
            ]),
        );
        assert_eq!(m.desc, "");
        assert_eq!(m.extras, v(&["nota primeiro", "_isso continua nota_"]));
    }

    #[test]
    fn parse_block_da_status_0_e_preserva_o_tipo_desconhecido() {
        let m = parse_block(
            &Vocab::default(),
            &v(&[
                "> [!questão] Título preservado",
                "> [[projects/p/tasks/x|x]]",
            ]),
        );
        assert_eq!(m.status_num, 0);
        assert_eq!(m.raw_callout, "questão");
        assert_eq!(m.title, "Título preservado");
    }

    #[test]
    fn parse_block_le_o_formato_legado() {
        let m = parse_block(
            &Vocab::default(),
            &v(&[
                "> **Minha task**",
                "> [[minha-task]]",
                "> 1 - Backlog",
                "> Branch: minha-task",
                "> alguma nota",
            ]),
        );
        assert_eq!(m.status_num, 1);
        assert_eq!(m.title, "Minha task");
        assert_eq!(m.id, "minha-task");
        assert_eq!(m.extras, v(&["alguma nota"]));
    }

    #[test]
    fn parse_block_extrai_o_id_do_alvo_nao_do_alias() {
        let m = parse_block(
            &Vocab::default(),
            &v(&[
                "> [!todo] T",
                "> [[projects/p/tasks/archived/done/x|rótulo qualquer]]",
            ]),
        );
        assert_eq!(m.id, "x");
    }

    #[test]
    fn parse_block_de_bloco_vazio_e_sem_callout() {
        let m = parse_block(&Vocab::default(), &[]);
        assert_eq!(m.status_num, 1);
        assert_eq!(m.title, "");
        let m = parse_block(&Vocab::default(), &v(&["> só texto", "> [[x]]"]));
        assert_eq!(m.title, "só texto");
        assert_eq!(m.id, "x");
    }

    #[test]
    fn serialize_block_emite_o_link_path_qualified() {
        let m = Model {
            status_num: 1,
            title: "Título".into(),
            id: "x".into(),
            project: Some("p".into()),
            ..Model::default()
        };
        assert_eq!(
            serialize_block(&Vocab::default(), &m),
            v(&["> [!todo] Título", "> [[projects/p/tasks/x|x]]"])
        );
    }

    #[test]
    fn serialize_block_aponta_para_archived_quando_arquivada() {
        let vo = Vocab::default();
        let m = Model {
            status_num: fase(&vo, "done"),
            title: "Título".into(),
            id: "x".into(),
            project: Some("p".into()),
            archived: Some("done".into()),
            ..Model::default()
        };
        assert_eq!(
            serialize_block(&vo, &m),
            v(&[
                "> [!done] Título",
                "> [[projects/p/tasks/archived/done/x|x]]"
            ])
        );
    }

    #[test]
    fn serialize_block_omite_a_descricao_vazia() {
        let m = Model {
            status_num: 1,
            title: "T".into(),
            id: "x".into(),
            project: Some("p".into()),
            desc: "  ".into(),
            ..Model::default()
        };
        assert_eq!(serialize_block(&Vocab::default(), &m).len(), 2);
    }

    #[test]
    fn serialize_block_preserva_o_tipo_invalido_do_status_0() {
        let m = Model {
            status_num: 0,
            raw_callout: "questão".into(),
            title: "T".into(),
            id: "x".into(),
            project: Some("p".into()),
            ..Model::default()
        };
        assert_eq!(serialize_block(&Vocab::default(), &m)[0], "> [!questão] T");
    }

    #[test]
    fn serialize_block_cai_em_invalid_sem_tipo_digitado() {
        let m = Model {
            status_num: 0,
            title: "T".into(),
            id: "x".into(),
            project: Some("p".into()),
            ..Model::default()
        };
        assert_eq!(serialize_block(&Vocab::default(), &m)[0], "> [!invalid] T");
    }

    #[test]
    fn serialize_block_apara_linhas_em_branco_no_fim_das_notas() {
        let m = Model {
            status_num: 1,
            title: "T".into(),
            id: "x".into(),
            project: Some("p".into()),
            extras: v(&["nota", "", "  "]),
            ..Model::default()
        };
        assert_eq!(
            serialize_block(&Vocab::default(), &m),
            v(&["> [!todo] T", "> [[projects/p/tasks/x|x]]", "> nota"])
        );
    }

    #[test]
    fn serialize_block_sem_project_emite_o_link_curto_e_nota_vazia_no_meio_vira_maior() {
        let m = Model {
            status_num: 1,
            title: "T".into(),
            id: "x".into(),
            extras: v(&["a", "", "b"]),
            ..Model::default()
        };
        assert_eq!(
            serialize_block(&Vocab::default(), &m),
            v(&["> [!todo] T", "> [[x]]", "> a", ">", "> b"])
        );
    }

    #[test]
    fn round_trip_e_idempotente_para_um_bloco_canonico() {
        let vo = Vocab::default();
        let original = v(&[
            "> [!example] Título da task",
            "> [[projects/bjju-web/tasks/fazer-x|fazer-x]]",
            "> _aguardando revisão_",
            "> impedimento: VPN",
            "> segunda nota",
        ]);
        let mut m = parse_block(&vo, &original);
        m.project = Some("bjju-web".into());
        assert_eq!(serialize_block(&vo, &m), original);
    }

    #[test]
    fn round_trip_normaliza_o_legado_para_o_atual() {
        let vo = Vocab::default();
        let mut m = parse_block(
            &vo,
            &v(&[
                "> **Minha task**",
                "> [[minha-task]]",
                "> 1 - Backlog",
                "> Branch: minha-task",
            ]),
        );
        m.project = Some("p".into());
        assert_eq!(
            serialize_block(&vo, &m),
            v(&[
                "> [!todo] Minha task",
                "> [[projects/p/tasks/minha-task|minha-task]]"
            ])
        );
    }

    #[test]
    fn round_trip_sobrevive_com_status_0() {
        let vo = Vocab::default();
        let blk = v(&["> [!questão] T", "> [[projects/p/tasks/x|x]]"]);
        let mut m = parse_block(&vo, &blk);
        m.project = Some("p".into());
        let out = serialize_block(&vo, &m);
        assert_eq!(out, blk);
        assert_eq!(parse_block(&vo, &out).status_num, 0);
    }
}
