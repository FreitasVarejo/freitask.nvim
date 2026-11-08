//! O que o freitask lê do FRONTMATTER de uma task: prioridade, data de volta
//! (a task adiada), prazo e dependências. Tudo ortogonal ao status: o callout
//! diz em que FASE o trabalho está. Cada campo tem o próprio escritor, e uma
//! gravação nunca apaga outro campo.

use std::collections::BTreeMap;

use crate::date::Date;
use crate::md;
use crate::trim;

pub const HIGH: &str = "alta";
pub const KEY_PRIORIDADE: &str = "prioridade";
pub const KEY_VOLTA_EM: &str = "volta-em";
pub const KEY_PRAZO: &str = "prazo";
pub const KEY_BLOQUEADA_POR: &str = "bloqueada-por";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Meta {
    /// "alta" é o único valor que conta; o texto vem como está para o doctor.
    pub prioridade: Option<String>,
    /// O texto de `volta-em`, aparado; só adia se `Date::parse` o lê.
    pub volta_em: Option<String>,
    /// O texto de `prazo`, aparado; só conta se `Date::parse` o lê.
    pub prazo: Option<String>,
    /// Os ids de `bloqueada-por: [a, b]`, como escritos.
    pub bloqueada_por: Vec<String>,
}

fn nao_vazio(v: Option<&String>) -> Option<String> {
    v.map(|s| trim(s))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Lê os campos do frontmatter. Valor vazio conta como ausente.
pub fn read(lines: &[String]) -> Meta {
    let (fm, _) = md::frontmatter(lines);
    Meta {
        prioridade: nao_vazio(fm.get(KEY_PRIORIDADE)),
        volta_em: nao_vazio(fm.get(KEY_VOLTA_EM)),
        prazo: nao_vazio(fm.get(KEY_PRAZO)),
        bloqueada_por: fm
            .get(KEY_BLOQUEADA_POR)
            .map(|v| parse_lista(v))
            .unwrap_or_default(),
    }
}

/// `[a, b]` ou `a, b` → ids aparados, sem vazios.
pub fn parse_lista(v: &str) -> Vec<String> {
    let s = trim(v);
    let s = s
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .unwrap_or(s);
    s.split(',')
        .map(trim)
        .filter(|x| !x.is_empty())
        .map(md::yaml_unquote)
        .collect()
}

/// `[a, b]`, a forma em uma linha que o parser de frontmatter entende.
pub fn serializa_lista(ids: &[String]) -> String {
    format!("[{}]", ids.join(", "))
}

pub fn is_high(meta: &Meta) -> bool {
    meta.prioridade.as_deref() == Some(HIGH)
}

/// Verdadeiro se a task está ADIADA em `today`: `volta-em` é uma data de um
/// dia depois de hoje. `volta-em` é o primeiro dia visível. Data inválida NÃO
/// adia: a task fica à vista e o doctor acusa.
pub fn is_postponed(meta: &Meta, today: Date) -> bool {
    meta.volta_em
        .as_deref()
        .and_then(Date::parse)
        .is_some_and(|d| d > today)
}

/// `prazo - hoje` em dias, ou `None` sem prazo ou com prazo inválido.
pub fn dias_para_o_prazo(meta: &Meta, today: Date) -> Option<i64> {
    meta.prazo
        .as_deref()
        .and_then(Date::parse)
        .map(|p| today.dias_ate(p))
}

fn escreve_chave(lines: &mut Vec<String>, chave: &str, value: Option<&str>) -> bool {
    let mut up = BTreeMap::new();
    up.insert(chave.to_string(), value.map(str::to_string));
    md::set_frontmatter(lines, &up)
}

/// Grava a prioridade (`None` remove). Só esta chave.
pub fn write_prioridade(lines: &mut Vec<String>, value: Option<&str>) -> bool {
    escreve_chave(lines, KEY_PRIORIDADE, value)
}

/// Grava a data de volta (`None` remove, e o bloco inteiro se esvazia).
pub fn write_volta_em(lines: &mut Vec<String>, value: Option<&str>) -> bool {
    escreve_chave(lines, KEY_VOLTA_EM, value)
}

/// Grava o prazo (`None` remove).
pub fn write_prazo(lines: &mut Vec<String>, value: Option<&str>) -> bool {
    escreve_chave(lines, KEY_PRAZO, value)
}

/// Grava `bloqueada-por: [a, b]`; lista vazia remove a chave.
pub fn write_bloqueada_por(lines: &mut Vec<String>, ids: &[String]) -> bool {
    let mut up = BTreeMap::new();
    up.insert(
        KEY_BLOQUEADA_POR.to_string(),
        if ids.is_empty() {
            None
        } else {
            Some(serializa_lista(ids))
        },
    );
    // Cru, não citado: `[a, b]` é uma lista YAML, e entre aspas viraria texto.
    md::set_frontmatter_com(lines, &up, false)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    #[test]
    fn le_so_a_prioridade_ignorando_o_resto() {
        let m = read(&v(&[
            "---",
            "id: x",
            "prioridade: alta",
            "tags: [p]",
            "---",
        ]));
        assert_eq!(m.prioridade.as_deref(), Some("alta"));
    }

    #[test]
    fn campo_vazio_ou_ausente_e_sem_prioridade() {
        assert_eq!(read(&v(&["---", "prioridade:", "---"])).prioridade, None);
        assert_eq!(read(&v(&["---", "id: x", "---"])).prioridade, None);
        assert_eq!(read(&v(&["> [!todo] sem frontmatter"])).prioridade, None);
    }

    #[test]
    fn escreve_o_estado_inteiro_da_prioridade() {
        let mut l = v(&["---", "id: x", "prioridade: alta", "---", "", "> x"]);
        write_prioridade(&mut l, None);
        assert_eq!(read(&l).prioridade, None);
        assert_eq!(
            md::frontmatter(&l).0["id"],
            "x",
            "o que não é do freitask fica"
        );
        write_prioridade(&mut l, Some("alta"));
        assert_eq!(read(&l).prioridade.as_deref(), Some("alta"));
    }

    #[test]
    fn so_alta_exato_conta() {
        assert!(is_high(&Meta {
            prioridade: Some("alta".into()),
            ..Meta::default()
        }));
        assert!(!is_high(&Meta {
            prioridade: Some("Alta".into()),
            ..Meta::default()
        }));
        assert!(!is_high(&Meta {
            prioridade: Some("urgente".into()),
            ..Meta::default()
        }));
        assert!(!is_high(&Meta::default()));
    }

    #[test]
    fn le_volta_em_aparado_sem_aspas_e_deixa_a_prioridade() {
        let m = read(&v(&[
            "---",
            "prioridade: alta",
            "volta-em: 2026-12-01",
            "---",
        ]));
        assert_eq!(m.volta_em.as_deref(), Some("2026-12-01"));
        assert_eq!(m.prioridade.as_deref(), Some("alta"));
        assert_eq!(
            read(&v(&["---", "volta-em: \"2026-12-01\"", "---"]))
                .volta_em
                .as_deref(),
            Some("2026-12-01")
        );
        assert_eq!(
            read(&v(&["---", "volta-em:   2026-12-01  ", "---"]))
                .volta_em
                .as_deref(),
            Some("2026-12-01")
        );
    }

    #[test]
    fn volta_em_vazio_ou_ausente_e_sem_data() {
        assert_eq!(read(&v(&["---", "volta-em:", "---"])).volta_em, None);
        assert_eq!(read(&v(&["---", "volta-em: \"\"", "---"])).volta_em, None);
        assert_eq!(read(&v(&["---", "prioridade: alta", "---"])).volta_em, None);
        assert_eq!(read(&v(&["> [!todo] x"])).volta_em, None);
    }

    #[test]
    fn texto_que_nao_e_data_chega_cru() {
        assert_eq!(
            read(&v(&["---", "volta-em: 2026-12-1", "---"]))
                .volta_em
                .as_deref(),
            Some("2026-12-1")
        );
    }

    #[test]
    fn volta_em_e_o_primeiro_dia_visivel() {
        let m = Meta {
            volta_em: Some("2026-12-01".into()),
            ..Meta::default()
        };
        assert!(is_postponed(&m, d("2026-11-30")), "véspera");
        assert!(is_postponed(&m, d("2026-11-01")));
        assert!(!is_postponed(&m, d("2026-12-01")), "no dia a task volta");
        assert!(!is_postponed(&m, d("2026-12-02")));
        assert!(!is_postponed(&m, d("2027-06-01")));
    }

    #[test]
    fn a_comparacao_e_de_data_nao_de_texto() {
        let m = |s: &str| Meta {
            volta_em: Some(s.into()),
            ..Meta::default()
        };
        assert!(!is_postponed(&m("2026-12-1"), d("2026-12-05")));
        assert!(!is_postponed(&m("2026-02-30"), d("2026-01-01")));
        assert!(!is_postponed(&m("01/12/2026"), d("2026-11-01")));
        assert!(!is_postponed(&m("amanhã"), d("2026-11-01")));
        assert!(is_postponed(&m("2026-12-10"), d("2026-12-05")));
    }

    #[test]
    fn sem_data_de_volta_nao_esta_adiada() {
        assert!(!is_postponed(&Meta::default(), d("2026-11-01")));
        assert!(!is_postponed(
            &Meta {
                prioridade: Some("alta".into()),
                ..Meta::default()
            },
            d("2026-11-01")
        ));
    }

    #[test]
    fn write_volta_em_cria_troca_e_remove() {
        let mut l = v(&["> [!todo] x", "> [[x]]"]);
        assert!(write_volta_em(&mut l, Some("2026-12-01")));
        assert_eq!(
            l,
            v(&[
                "---",
                "volta-em: 2026-12-01",
                "---",
                "",
                "> [!todo] x",
                "> [[x]]"
            ])
        );
        assert_eq!(read(&l).volta_em.as_deref(), Some("2026-12-01"));
        assert!(write_volta_em(&mut l, Some("2027-01-15")));
        assert_eq!(read(&l).volta_em.as_deref(), Some("2027-01-15"));
        assert!(
            !write_volta_em(&mut l, Some("2027-01-15")),
            "mesma data: nada a gravar"
        );
        assert!(write_volta_em(&mut l, None));
        assert_eq!(l, v(&["> [!todo] x", "> [[x]]"]));
        assert!(!write_volta_em(&mut l, None));
        assert_eq!(l, v(&["> [!todo] x", "> [[x]]"]));
    }

    #[test]
    fn write_volta_em_so_toca_na_propria_chave() {
        let mut l = v(&[
            "---",
            "id: x",
            "prioridade: alta",
            "tags: [p]",
            "---",
            "",
            "> [!todo] x",
        ]);
        write_volta_em(&mut l, Some("2026-12-01"));
        assert_eq!(
            l,
            v(&[
                "---",
                "id: x",
                "prioridade: alta",
                "tags: [p]",
                "volta-em: 2026-12-01",
                "---",
                "",
                "> [!todo] x"
            ])
        );
        write_volta_em(&mut l, None);
        assert_eq!(
            l,
            v(&[
                "---",
                "id: x",
                "prioridade: alta",
                "tags: [p]",
                "---",
                "",
                "> [!todo] x"
            ])
        );
    }

    #[test]
    fn a_prioridade_nao_apaga_a_data_nem_o_contrario() {
        let mut l = v(&["---", "volta-em: 2026-12-01", "---", "", "> [!todo] x"]);
        write_prioridade(&mut l, Some("alta"));
        assert_eq!(read(&l).volta_em.as_deref(), Some("2026-12-01"));
        write_prioridade(&mut l, None);
        assert_eq!(read(&l).volta_em.as_deref(), Some("2026-12-01"));
        write_prioridade(&mut l, Some("alta"));
        write_volta_em(&mut l, None);
        assert_eq!(read(&l).prioridade.as_deref(), Some("alta"));
    }

    #[test]
    fn prazo_e_dias_para_o_prazo() {
        let m = read(&v(&["---", "prazo: 2026-11-18", "---"]));
        assert_eq!(m.prazo.as_deref(), Some("2026-11-18"));
        assert_eq!(dias_para_o_prazo(&m, d("2026-11-08")), Some(10));
        assert_eq!(dias_para_o_prazo(&m, d("2026-11-20")), Some(-2));
        assert_eq!(
            dias_para_o_prazo(&read(&v(&["---", "prazo: amanha", "---"])), d("2026-11-08")),
            None
        );
        assert_eq!(dias_para_o_prazo(&Meta::default(), d("2026-11-08")), None);
        let mut l = v(&["> [!todo] x"]);
        assert!(write_prazo(&mut l, Some("2026-11-18")));
        assert_eq!(read(&l).prazo.as_deref(), Some("2026-11-18"));
        assert!(write_prazo(&mut l, None));
        assert_eq!(l, v(&["> [!todo] x"]));
    }

    #[test]
    fn bloqueada_por_le_e_escreve_a_lista_em_uma_linha() {
        let m = read(&v(&["---", "bloqueada-por: [a, b]", "---"]));
        assert_eq!(m.bloqueada_por, v(&["a", "b"]));
        assert_eq!(
            read(&v(&["---", "bloqueada-por: a,b , c", "---"])).bloqueada_por,
            v(&["a", "b", "c"])
        );
        assert_eq!(
            read(&v(&["---", "bloqueada-por: []", "---"])).bloqueada_por,
            Vec::<String>::new()
        );
        assert_eq!(
            read(&v(&["---", "bloqueada-por:", "---"])).bloqueada_por,
            Vec::<String>::new()
        );
        let mut l = v(&["> [!todo] x"]);
        assert!(write_bloqueada_por(&mut l, &v(&["a", "b"])));
        assert_eq!(l[1], "bloqueada-por: [a, b]");
        assert_eq!(read(&l).bloqueada_por, v(&["a", "b"]));
        assert!(write_bloqueada_por(&mut l, &[]));
        assert_eq!(l, v(&["> [!todo] x"]));
    }
}
