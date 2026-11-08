//! O `CURRENT.md`, painel do dia. Gerado do zero a cada regen, exceto a seção
//! `## Notas Avulsas`, que é autoral e por isso é extraída e recolocada.
//! Antes de sobrescrever um painel de outro dia, ele vira `daily/<data>.md`:
//! aquilo é história, e história não se reescreve. Puro: recebe o índice e o
//! painel anterior, devolve os bytes novos e, se for o caso, o snapshot.

use crate::date::Date;
use crate::index::{self, Indice};
use crate::status::Vocab;
use crate::texto::Doc;

/// O que `rebuild` produz: o painel e, quando o anterior era de outro dia, o
/// snapshot a gravar em `daily/<data>.md` (se ainda não existir).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Painel {
    pub bytes: Vec<u8>,
    pub daily: Option<(String, Vec<u8>)>,
}

/// Extrai o corpo autoral de `## Notas Avulsas` do painel anterior.
pub fn extract_loose_notes(lines: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut capturing = false;
    for l in lines {
        if eh_secao_chamada(l, "Notas Avulsas") {
            capturing = true;
        } else if capturing
            && l.strip_prefix("##")
                .is_some_and(|r| r.chars().next().is_some_and(crate::eh_espaco))
        {
            break;
        } else if capturing {
            out.push(l.clone());
        }
    }
    while out.first().is_some_and(|l| l.chars().all(crate::eh_espaco)) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.chars().all(crate::eh_espaco)) {
        out.pop();
    }
    // O placeholder solitário (`- `) não é nota.
    if out.len() == 1
        && out[0]
            .strip_prefix('-')
            .is_some_and(|r| r.chars().all(crate::eh_espaco))
    {
        out.clear();
    }
    out
}

/// `^##%s+Notas Avulsas`
fn eh_secao_chamada(l: &str, nome: &str) -> bool {
    let Some(resto) = l.strip_prefix("##") else {
        return false;
    };
    let sem = resto.trim_start_matches(crate::eh_espaco);
    sem.len() != resto.len() && sem.starts_with(nome)
}

/// A `date:` do frontmatter do painel anterior (`^date:%s*(AAAA-MM-DD)`).
pub fn frontmatter_date(lines: &[String]) -> Option<String> {
    if lines.first().map(String::as_str) != Some("---") {
        return None;
    }
    for l in lines.iter().skip(1) {
        if l == "---" {
            break;
        }
        if let Some(resto) = l.strip_prefix("date:") {
            let v = resto.trim_start_matches(crate::eh_espaco);
            if v.len() >= 10 && Date::parse(&v[..10]).is_some() {
                return Some(v[..10].to_string());
            }
        }
    }
    None
}

/// Monta o painel de hoje. `anterior` são os bytes do `CURRENT.md` atual, se
/// houver: dele saem as notas avulsas e a data do snapshot.
pub fn render(vocab: &Vocab, indice: &Indice, today: Date, anterior: Option<&[u8]>) -> Painel {
    let linhas_anteriores: Vec<String> = anterior
        .and_then(|b| Doc::parse(b).ok())
        .map(|d| d.lines)
        .unwrap_or_default();
    let hoje = today.to_string();
    let daily = match (anterior, frontmatter_date(&linhas_anteriores)) {
        (Some(bytes), Some(prev)) if prev < hoje => {
            Some((format!("daily/{prev}.md"), bytes.to_vec()))
        }
        _ => None,
    };
    let notes = extract_loose_notes(&linhas_anteriores);

    let mut shown: Vec<(&str, Vec<&index::Entry>)> = Vec::new();
    let mut total = 0;
    for p in &indice.projetos {
        let es: Vec<&index::Entry> = indice
            .entries_for(p)
            .into_iter()
            .filter(|e| !index::is_hidden(vocab, e, today))
            .collect();
        total += es.len();
        shown.push((p, es));
    }

    let mut lines: Vec<String> = vec![
        "---".into(),
        format!("date: {hoje}"),
        format!("weekday: {}", today.weekday()),
        "---".into(),
        String::new(),
        format!("# Planning diário {}", today.curto()),
        String::new(),
        format!(
            "> Painel gerado automaticamente — {total} tasks em {} projetos.",
            indice.projetos.len()
        ),
        String::new(),
        "## Notas Avulsas".into(),
        String::new(),
    ];
    if notes.is_empty() {
        lines.push("- ".into());
    } else {
        lines.extend(notes);
    }
    lines.push(String::new());
    for (p, es) in &shown {
        lines.push(format!("## {p}"));
        lines.push(String::new());
        if es.is_empty() {
            lines.push("_(sem tasks)_".into());
            lines.push(String::new());
        } else {
            for e in es {
                lines.extend(e.block.iter().cloned());
                lines.push(String::new());
            }
        }
    }
    Painel {
        bytes: Doc::novo(lines).to_bytes(),
        daily,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::index::scan_task;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn extract_loose_notes_pega_so_a_secao_e_descarta_o_placeholder() {
        assert_eq!(
            extract_loose_notes(&v(&["## Notas Avulsas", "", "- a", "- b", "", "## p", "x"])),
            v(&["- a", "- b"])
        );
        assert_eq!(
            extract_loose_notes(&v(&["## Notas Avulsas", "", "- ", ""])),
            Vec::<String>::new()
        );
        assert_eq!(
            extract_loose_notes(&v(&["# x", "## p"])),
            Vec::<String>::new()
        );
    }

    #[test]
    fn frontmatter_date_le_a_data_do_painel() {
        assert_eq!(
            frontmatter_date(&v(&["---", "date: 2026-11-07", "weekday: Saturday", "---"]))
                .as_deref(),
            Some("2026-11-07")
        );
        assert_eq!(frontmatter_date(&v(&["# sem frontmatter"])), None);
        assert_eq!(
            frontmatter_date(&v(&["---", "weekday: x", "---", "date: 2026-11-07"])),
            None
        );
    }

    #[test]
    fn render_monta_o_painel_e_o_snapshot_do_dia_anterior() {
        let vo = Vocab::default();
        let idx = Indice::new(
            vec!["b".into(), "a".into()],
            vec![
                scan_task(
                    &vo,
                    "projects/a/tasks/x.md",
                    b"> [!todo] X\n> [[projects/a/tasks/x|x]]\n> nota\n",
                )
                .unwrap(),
                scan_task(&vo, "projects/a/tasks/q.md", b"> [!quote] Q\n> [[q]]\n").unwrap(),
                scan_task(
                    &vo,
                    "projects/a/tasks/archived/done/z.md",
                    b"> [!done] Z\n> [[z]]\n",
                )
                .unwrap(),
            ],
        );
        let anterior = b"---\ndate: 2026-11-07\nweekday: Saturday\n---\n\n# Planning di\xc3\xa1rio 07/11/26\n\n## Notas Avulsas\n\n- lembrar\n\n## a\n\nvelho\n";
        let p = render(
            &vo,
            &idx,
            Date::parse("2026-11-08").unwrap(),
            Some(anterior),
        );
        let texto = String::from_utf8(p.bytes).unwrap();
        assert_eq!(
            texto,
            "---\ndate: 2026-11-08\nweekday: Sunday\n---\n\n# Planning diário 08/11/26\n\n> Painel gerado automaticamente — 1 tasks em 2 projetos.\n\n## Notas Avulsas\n\n- lembrar\n\n## a\n\n> [!todo] X\n> [[projects/a/tasks/x|x]]\n> nota\n\n## b\n\n_(sem tasks)_\n\n"
        );
        assert_eq!(
            p.daily,
            Some(("daily/2026-11-07.md".to_string(), anterior.to_vec()))
        );
    }

    #[test]
    fn render_sem_anterior_ou_do_mesmo_dia_nao_faz_snapshot_e_poe_o_placeholder() {
        let vo = Vocab::default();
        let idx = Indice::new(vec![], vec![]);
        let p = render(&vo, &idx, Date::parse("2026-11-08").unwrap(), None);
        assert!(p.daily.is_none());
        assert!(
            String::from_utf8(p.bytes)
                .unwrap()
                .contains("## Notas Avulsas\n\n- \n")
        );
        let hoje = b"---\ndate: 2026-11-08\n---\n## Notas Avulsas\n\n- x\n";
        let p = render(&vo, &idx, Date::parse("2026-11-08").unwrap(), Some(hoje));
        assert!(p.daily.is_none());
        assert!(
            String::from_utf8(p.bytes)
                .unwrap()
                .contains("## Notas Avulsas\n\n- x\n")
        );
    }
}
