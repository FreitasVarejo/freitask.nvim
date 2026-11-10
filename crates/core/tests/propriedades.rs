//! Propriedades do núcleo (spec §3.2): round-trips e invariantes das regras
//! puras, sobre entradas geradas. `PROPTEST_CASES` controla o número de casos
//! (256 no `make test`, 4096 no `make test-longo`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use freitask_core::board;
use freitask_core::date::Date;
use freitask_core::index::{self, Indice};
use freitask_core::links::{self, Endereco};
use freitask_core::md;
use freitask_core::meta;
use freitask_core::model::{self, Model};
use freitask_core::path;
use freitask_core::status::Vocab;
use freitask_core::texto::Doc;
use proptest::prelude::*;

fn vocab() -> Vocab {
    Vocab::default()
}

/// Texto de uma linha sem quebra: letras, acentos, pontuação e espaço.
fn linha() -> impl Strategy<Value = String> {
    "[A-Za-zÁ-ú0-9 _:#*|\\[\\]`>!.,;-]{0,40}"
}

fn kebab_id() -> impl Strategy<Value = String> {
    "[a-z0-9]{1,8}(-[a-z0-9]{1,8}){0,3}"
}

proptest! {
    /// I2 em miniatura: o que não é tocado volta byte a byte.
    #[test]
    fn doc_round_trip_preserva_os_bytes(
        linhas in prop::collection::vec(linha(), 0..12),
        eols in prop::collection::vec(prop::bool::ANY, 0..12),
        bom in prop::bool::ANY,
        fim in prop::bool::ANY,
    ) {
        let mut bytes = Vec::new();
        if bom { bytes.extend([0xEF, 0xBB, 0xBF]); }
        let n = linhas.len();
        for (i, l) in linhas.iter().enumerate() {
            bytes.extend(l.as_bytes());
            if i + 1 < n || fim {
                if eols.get(i).copied().unwrap_or(false) { bytes.extend(b"\r\n"); } else { bytes.push(b'\n'); }
            }
        }
        let doc = Doc::parse(&bytes).unwrap();
        prop_assert_eq!(doc.to_bytes(), bytes);
        // E com as mesmas linhas de volta, o mesmo resultado.
        let mesmas = doc.lines.clone();
        prop_assert_eq!(doc.com_linhas(mesmas).to_bytes(), doc.to_bytes());
    }

    #[test]
    fn kebab_e_idempotente_e_so_tem_kebab(s in "\\PC{0,30}") {
        let k = path::kebab(&s);
        prop_assert_eq!(path::kebab(&k), k.clone());
        prop_assert!(k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        prop_assert!(!k.starts_with('-') && !k.ends_with('-') && !k.contains("--"));
    }

    #[test]
    fn split_task_path_e_o_inverso_de_task_rel(
        project in kebab_id(), id in kebab_id(), archived in prop::option::of(prop::sample::select(vec!["done", "dropped", "failed"]))
    ) {
        prop_assume!(!freitask_core::config::is_reserved(&project));
        let rel = path::task_rel(&project, &id, archived);
        let t = path::split_task_path(&rel).unwrap();
        prop_assert_eq!(t.project, project);
        prop_assert_eq!(t.id, id);
        prop_assert_eq!(t.archived.as_deref(), archived);
    }

    /// Um bloco canônico sobrevive ao round-trip parse → serialize.
    #[test]
    fn bloco_canonico_faz_round_trip(
        status in 1u32..=9,
        title in "[A-Za-zÁ-ú0-9 ]{1,30}",
        id in kebab_id(),
        project in kebab_id(),
        desc in "[A-Za-zÁ-ú0-9 ]{0,20}",
        extras in prop::collection::vec("[A-Za-z0-9 ]{1,20}", 0..3),
        archived in prop::option::of(prop::sample::select(vec!["done", "dropped", "failed"])),
    ) {
        let vo = vocab();
        // Notas que parecem status-texto, link-só ou Branch: seriam consumidas; o gerador não as produz.
        let m = Model {
            status_num: status,
            raw_callout: vo.meta(status).callout.clone(),
            title: title.trim().to_string(),
            id,
            desc: desc.trim().to_string(),
            extras,
            project: Some(project),
            archived: archived.map(str::to_string),
        };
        let bloco = model::serialize_block(&vo, &m);
        let mut back = model::parse_block(&vo, &bloco);
        back.project = m.project.clone();
        back.archived = m.archived.clone();
        prop_assert_eq!(model::serialize_block(&vo, &back), bloco);
        prop_assert_eq!(back.status_num, m.status_num);
        prop_assert_eq!(back.title, m.title);
        prop_assert_eq!(back.id, m.id);
        prop_assert_eq!(back.desc, m.desc);
    }

    /// `set_frontmatter` grava o que `frontmatter` lê de volta, e chaves alheias ficam.
    #[test]
    fn frontmatter_round_trip_e_preserva_alheias(
        pares in prop::collection::btree_map("[a-z][a-z0-9-]{0,8}", "[A-Za-zÁ-ú0-9 :#\\[\\]\"']{0,20}", 0..4),
        alheia in "[a-z]{3,6}",
    ) {
        prop_assume!(!pares.contains_key(&alheia));
        let mut lines = vec!["---".to_string(), format!("{alheia}: valor alheio"), "---".to_string(), "".to_string(), "> [!todo] x".to_string()];
        let updates: BTreeMap<String, Option<String>> = pares.iter().map(|(k, v)| (k.clone(), Some(v.clone()))).collect();
        md::set_frontmatter(&mut lines, &updates);
        let (map, _) = md::frontmatter(&lines);
        for (k, v) in &pares {
            prop_assert_eq!(map.get(k).map(|s| s.as_str()), Some(v.as_str()), "chave {}", k);
        }
        prop_assert_eq!(map.get(&alheia).map(String::as_str), Some("valor alheio"));
        prop_assert_eq!(lines.last().map(String::as_str), Some("> [!todo] x"));
        // Remover tudo que foi gravado devolve o bloco só com a alheia.
        let remover: BTreeMap<String, Option<String>> = pares.keys().map(|k| (k.clone(), None)).collect();
        md::set_frontmatter(&mut lines, &remover);
        prop_assert_eq!(lines, vec!["---".to_string(), format!("{alheia}: valor alheio"), "---".to_string(), "".to_string(), "> [!todo] x".to_string()]);
    }

    /// I5 nos links: renomear a→b e b→a devolve o texto original, exceto pelo
    /// alias da exceção da I5 (decisão 64).
    #[test]
    fn rename_ida_e_volta_devolve_os_links(
        a in kebab_id(), b in kebab_id(), p in kebab_id(),
        alias in prop::option::of("[A-Za-z ]{1,10}"),
        sub in prop::option::of(prop::sample::select(vec!["#Seção", "^bloco"])),
        tabela in prop::bool::ANY,
    ) {
        prop_assume!(a != b);
        let sep = if tabela { "\\|" } else { "|" };
        let al = alias.as_deref().map(|x| format!("{sep}{x}")).unwrap_or_default();
        let texto = format!(
            "ver [[{a}]] e [[projects/{p}/tasks/{a}{}{al}]] e `[[{a}]]`\n```\n[[{a}]]\n```\n",
            sub.unwrap_or("")
        );
        let ea = Endereco::de_rel(&format!("projects/{p}/tasks/{a}.md"));
        let eb = Endereco::de_rel(&format!("projects/{p}/tasks/{b}.md"));
        let (ida, n) = links::rewrite_text(&texto, |t, al| links::rewrite_link(t, al, &ea, &eb)).unwrap();
        prop_assert_eq!(n, 2, "o curto e o path-qualified; os de código não");
        let em_codigo = format!("`[[{}]]`", a);
        prop_assert!(ida.contains(&em_codigo));
        let (volta, _) = links::rewrite_text(&ida, |t, al| links::rewrite_link(t, al, &eb, &ea)).unwrap();
        // Exceção da I5 (decisão 64): o alias cujo trim é o id de destino não
        // se distingue, depois da ida, de um que acompanhou o id; o que é o id
        // de origem com espaço em volta perde o espaço na ida. Nos dois, a
        // volta deixa o alias igual ao id de origem, e só ele muda.
        let esperado = match alias.as_deref() {
            Some(x) if freitask_core::trim(x) == b || (freitask_core::trim(x) == a && x != a) => {
                texto.replacen(&format!("{sep}{x}]]"), &format!("{sep}{a}]]"), 1)
            }
            _ => texto.clone(),
        };
        prop_assert_eq!(volta, esperado);
    }

    #[test]
    fn encerrar_e_reabrir_devolvem_o_prefixo(inner in "projects/p/[a-z/]{0,20}[a-z]([#^][a-z]+)?(\\|[a-z ]+)?") {
        let ida = links::rewrite_prefix(&inner, "projects/p/", "projects/_encerrados/p/").unwrap();
        let volta = links::rewrite_prefix(&ida, "projects/_encerrados/p/", "projects/p/");
        prop_assert_eq!(volta.as_deref(), Some(inner.as_str()));
    }

    /// `parse_date` aceita exatamente as datas que o calendário tem.
    #[test]
    fn parse_date_e_display_fazem_round_trip(ano in 1900i32..2200, mes in 1u32..=12, dia in 1u32..=31) {
        match Date::new(ano, mes, dia) {
            Some(d) => {
                prop_assert_eq!(Date::parse(&d.to_string()), Some(d));
                prop_assert_eq!(d.dias_ate(d), 0);
                let amanha = Date::parse(&format!("{:04}-{:02}-{:02}", ano, mes, dia)).unwrap();
                prop_assert_eq!(amanha, d);
            }
            None => {
                let texto = format!("{:04}-{:02}-{:02}", ano, mes, dia);
                prop_assert!(Date::parse(&texto).is_none());
            }
        }
    }

    /// Dias civis são estritamente crescentes com a data.
    #[test]
    fn dias_civis_crescem_com_a_data(a in 1900i32..2200, b in 1900i32..2200, m1 in 1u32..=12, m2 in 1u32..=12, d1 in 1u32..=28, d2 in 1u32..=28) {
        let x = Date::new(a, m1, d1).unwrap();
        let y = Date::new(b, m2, d2).unwrap();
        prop_assert_eq!(x < y, x.dias_civis() < y.dias_civis());
        prop_assert_eq!(x.dias_ate(y), -y.dias_ate(x));
    }

    /// I11 em miniatura: `dias_para_o_prazo` é `prazo - hoje`, e `None` fora de data.
    #[test]
    fn dias_para_o_prazo_e_a_diferenca(prazo in "[0-9]{4}-[0-9]{2}-[0-9]{2}|amanha|", hoje in 2000i32..2100) {
        let today = Date::new(hoje, 6, 15).unwrap();
        let m = meta::Meta { prazo: Some(prazo.clone()).filter(|s| !s.is_empty()), ..meta::Meta::default() };
        match Date::parse(&prazo) {
            Some(p) => prop_assert_eq!(meta::dias_para_o_prazo(&m, today), Some(today.dias_ate(p))),
            None => prop_assert_eq!(meta::dias_para_o_prazo(&m, today), None),
        }
    }

    /// A adiada volta exatamente no dia: antes escondida, no dia e depois à vista.
    #[test]
    fn is_postponed_so_antes_da_data(ano in 2000i32..2100, mes in 1u32..=12, dia in 1u32..=28, delta in -3i64..=3) {
        let volta = Date::new(ano, mes, dia).unwrap();
        let hoje_dias = volta.dias_civis() + delta;
        // Reconstrói o dia de hoje a partir dos dias civis andando dia a dia.
        let mut hoje = volta;
        let mut passo = delta;
        while passo != 0 {
            let alvo = hoje.dias_civis() + passo.signum();
            hoje = (1..=31).flat_map(|d| (1..=12).map(move |m| (m, d))).flat_map(|(m, d)| {
                [hoje.ano - 1, hoje.ano, hoje.ano + 1].into_iter().filter_map(move |y| Date::new(y, m, d))
            }).find(|c| c.dias_civis() == alvo).unwrap();
            passo -= passo.signum();
        }
        prop_assert_eq!(hoje.dias_civis(), hoje_dias);
        let m = meta::Meta { volta_em: Some(volta.to_string()), ..meta::Meta::default() };
        prop_assert_eq!(meta::is_postponed(&m, hoje), delta < 0);
    }
}

/// Os `prazo:` que a I11 põe nas tasks: vencido, futuro, de hoje, com mês e dia
/// que não existem, em forma errada, em texto e vazio.
const PRAZOS: [&str; 7] = [
    "2026-11-01",
    "2027-03-01",
    "2026-11-08",
    "2026-02-30",
    "2026-12-1",
    "amanha",
    "",
];

/// Um projeto como o `list` o ordena: o nome, as ativas `(id, hidden)` e os ids
/// das arquivadas.
type OrdemDoProjeto = (String, Vec<(String, bool)>, Vec<String>);

/// O vault lido: o índice de todas as tasks (ativas e arquivadas) de `v`, com o
/// `prazo:` que `prazos` manda pôr em cada arquivo de task (na ordem dos
/// caminhos; `0` deixa a task como está).
fn indice_com_prazos(v: &gerador::Vault, prazos: &[u8]) -> Indice {
    let vocab = vocab();
    let mut projetos = Vec::new();
    let mut entradas = Vec::new();
    for (i, (rel, bytes)) in v.arquivos.iter().enumerate() {
        let Some(t) = path::split_task_path(rel) else {
            continue;
        };
        projetos.push(t.project);
        let escolha = prazos.get(i % prazos.len().max(1)).copied().unwrap_or(0) as usize;
        let bytes = match (escolha, Doc::parse(bytes)) {
            (0, _) | (_, Err(_)) => bytes.clone(),
            (n, Ok(doc)) => {
                let linha = format!("prazo: {}", PRAZOS[(n - 1) % PRAZOS.len()]);
                let mut lines = doc.lines.clone();
                if lines.first().map(String::as_str) == Some("---") {
                    lines.insert(1, linha);
                } else {
                    let mut novas = vec!["---".to_string(), linha, "---".into(), String::new()];
                    novas.append(&mut lines);
                    lines = novas;
                }
                doc.com_linhas(lines).to_bytes()
            }
        };
        if let Some(e) = index::scan_task(&vocab, rel, &bytes) {
            entradas.push(e);
        }
    }
    Indice::new(projetos, entradas)
}

proptest! {
    /// I11: pôr, trocar ou tirar `prazo:` (válido, inválido ou vazio) em
    /// qualquer subconjunto das tasks não muda o conjunto, a ordem nem o
    /// `hidden` que o painel e o `list` produzem, nem os bytes do `CURRENT.md`.
    #[test]
    fn prazo_nao_muda_ordem_conjunto_hidden_nem_painel(
        semente in 0u64..4096,
        prazos in prop::collection::vec(0u8..=7, 1..48),
        dia in prop::sample::select(vec!["2026-11-08", "2026-11-09", "2027-01-01"]),
    ) {
        let v = gerador::sintetico(semente, &gerador::Opcoes::default());
        let vocab = vocab();
        let hoje = Date::parse(dia).unwrap();
        let sem = indice_com_prazos(&v, &[0]);
        let com = indice_com_prazos(&v, &prazos);

        // O conjunto, e cada task igual a menos do `prazo` lido.
        prop_assert_eq!(&com.projetos, &sem.projetos);
        prop_assert_eq!(com.entradas.len(), sem.entradas.len());
        for (a, b) in com.entradas.iter().zip(&sem.entradas) {
            let mut a = a.clone();
            a.meta.prazo = None;
            prop_assert_eq!(a, b.clone());
        }

        // A ordem e o `hidden` do que o `list` produz, por projeto.
        let linhas = |i: &Indice| -> Vec<OrdemDoProjeto> {
            i.projetos.iter().map(|p| (
                p.clone(),
                i.entries_for(p).into_iter()
                    .map(|e| (e.task_id.clone(), index::is_hidden(&vocab, e, hoje)))
                    .collect(),
                i.archived_for(p).into_iter().map(|e| e.task_id.clone()).collect(),
            )).collect()
        };
        prop_assert_eq!(linhas(&com), linhas(&sem));

        // E o `CURRENT.md`, byte a byte.
        prop_assert_eq!(
            board::render(&vocab, &com, hoje, None),
            board::render(&vocab, &sem, hoje, None)
        );
    }
}

/// A I11 acima só prova algo se o `prazo:` injetado é lido: com a escolha `1`
/// (uma data válida) em todas as tasks, toda entrada do índice tem esse prazo.
#[test]
fn a_injecao_de_prazo_da_i11_e_lida_em_toda_task() {
    let mut lidas = 0;
    for semente in 0..256 {
        let v = gerador::sintetico(semente, &gerador::Opcoes::default());
        let com = indice_com_prazos(&v, &[1]);
        lidas += com.entradas.len();
        for e in &com.entradas {
            assert_eq!(
                e.meta.prazo.as_deref(),
                Some(PRAZOS[0]),
                "semente {semente}: {}",
                e.task_id
            );
        }
    }
    assert!(lidas > 1000, "só {lidas} tasks em 256 vaults");
}
