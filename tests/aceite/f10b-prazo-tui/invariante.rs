//! I11 na TUI (spec §10.2, "Ordem e visibilidade"): o prazo não muda a ordem
//! da lista nem esconde ou mostra task alguma. O mesmo vault com e sem
//! `prazo:` nas tasks (vencido, de hoje, futuro, de outro ano, inválido,
//! vazio; em task de prioridade alta, adiada, de Algum dia, que já voltou e
//! arquivada) dá, em cada projeto e nos modos normal, `S`, `A` e `S`+`A`, a
//! mesma lista de ids na mesma ordem, e o mesmo painel de projetos.

use std::collections::{BTreeMap, BTreeSet};

use crate::util::*;

/// Uma task do vault da I11: caminho relativo ao projeto, frontmatter sem o
/// prazo, callout, título, e o prazo que a versão "com prazo" acrescenta.
struct T {
    projeto: &'static str,
    /// `<id>` ou `archived/<tipo>/<id>`.
    onde: &'static str,
    fm: &'static [&'static str],
    callout: &'static str,
    titulo: &'static str,
    prazo: Option<&'static str>,
}

/// Os ids não se contêm uns aos outros, e nenhum título contém um id: um
/// item da lista se identifica pelo id que contém. Em `alfa`, as três de
/// `todo` sem prioridade têm prazos na ordem inversa da dos ids: uma lista
/// que ordenasse pelo prazo as trocaria.
const VAULT: [T; 18] = [
    T {
        projeto: "alfa",
        onde: "a-dez",
        fm: &[],
        callout: "todo",
        titulo: "Alfa dez",
        prazo: Some("2027-01-15"),
    },
    T {
        projeto: "alfa",
        onde: "a-um",
        fm: &[],
        callout: "todo",
        titulo: "Alfa um",
        prazo: Some("2026-11-10"),
    },
    T {
        projeto: "alfa",
        onde: "a-zeta",
        fm: &[],
        callout: "todo",
        titulo: "Alfa zeta",
        prazo: Some("2026-11-01"),
    },
    T {
        projeto: "alfa",
        onde: "a-dois",
        fm: &["prioridade: alta"],
        callout: "example",
        titulo: "Alfa dois",
        prazo: Some("2026-02-30"),
    },
    T {
        projeto: "alfa",
        onde: "a-tres",
        fm: &[],
        callout: "check",
        titulo: "Alfa três",
        prazo: Some(""),
    },
    T {
        projeto: "alfa",
        onde: "a-quatro",
        fm: &[],
        callout: "quote",
        titulo: "Alfa quatro",
        prazo: Some("2026-11-01"),
    },
    T {
        projeto: "alfa",
        onde: "a-cinco",
        fm: &["volta-em: 2026-12-01"],
        callout: "tip",
        titulo: "Alfa cinco",
        prazo: Some("2026-11-05"),
    },
    T {
        projeto: "alfa",
        onde: "a-seis",
        fm: &["volta-em: 2026-11-01"],
        callout: "todo",
        titulo: "Alfa seis",
        prazo: Some("2026-12-24"),
    },
    T {
        projeto: "alfa",
        onde: "archived/done/a-sete",
        fm: &[],
        callout: "done",
        titulo: "Alfa sete",
        prazo: Some("2026-11-01"),
    },
    T {
        projeto: "alfa",
        onde: "archived/dropped/a-oito",
        fm: &[],
        callout: "todo",
        titulo: "Alfa oito",
        prazo: None,
    },
    T {
        projeto: "beta",
        onde: "b-um",
        fm: &[],
        callout: "todo",
        titulo: "Beta um",
        prazo: None,
    },
    T {
        projeto: "beta",
        onde: "b-dois",
        fm: &[],
        callout: "todo",
        titulo: "Beta dois",
        prazo: Some("2026-10-01"),
    },
    T {
        projeto: "beta",
        onde: "b-tres",
        fm: &[],
        callout: "example",
        titulo: "Beta três",
        prazo: Some("2026-11-10"),
    },
    T {
        projeto: "beta",
        onde: "b-quatro",
        fm: &["volta-em: 2026-11-20"],
        callout: "tip",
        titulo: "Beta quatro",
        prazo: Some("2026-11-10"),
    },
    T {
        projeto: "beta",
        onde: "archived/failed/b-cinco",
        fm: &[],
        callout: "example",
        titulo: "Beta cinco",
        prazo: Some("2026-02-30"),
    },
    T {
        projeto: "gama",
        onde: "g-um",
        fm: &[],
        callout: "quote",
        titulo: "Gama um",
        prazo: Some("2026-11-09"),
    },
    T {
        projeto: "gama",
        onde: "g-dois",
        fm: &["volta-em: 2026-12-01"],
        callout: "todo",
        titulo: "Gama dois",
        prazo: Some("2026-11-01"),
    },
    T {
        projeto: "gama",
        onde: "archived/dropped/g-tres",
        fm: &[],
        callout: "todo",
        titulo: "Gama três",
        prazo: Some("2026-12-01"),
    },
];

const PROJETOS: [&str; 3] = ["alfa", "beta", "gama"];

fn id_de(t: &T) -> &'static str {
    t.onde.rsplit('/').next().expect("id")
}

/// O vault da I11, com ou sem as linhas `prazo:`.
fn home_i11(com_prazo: bool) -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("tempdir");
    let h = home.path();
    arquivo(h, ".freitask/status.json", gerador::STATUS_JSON);
    for t in &VAULT {
        let mut fm: Vec<String> = t.fm.iter().map(|l| l.to_string()).collect();
        if com_prazo && let Some(p) = t.prazo {
            if p.is_empty() {
                fm.push("prazo:".to_owned());
            } else {
                fm.push(format!("prazo: {p}"));
            }
        }
        let fm: Vec<&str> = fm.iter().map(String::as_str).collect();
        let mut corpo = format!(
            "> [!{}] {}\n> [[projects/{}/tasks/{}|{}]]\n",
            t.callout,
            t.titulo,
            t.projeto,
            t.onde,
            id_de(t)
        );
        if let Some(resto) = t.onde.strip_prefix("archived/") {
            let tipo = resto.split('/').next().expect("tipo");
            let rotulo = match tipo {
                "done" => "feito",
                "dropped" => "abandonado",
                _ => "falhou",
            };
            corpo.push_str(&format!(
                "\n## Histórico\n\n- 2026-10-20 — arquivada em `archived/{tipo}` ({rotulo})\n"
            ));
        }
        arquivo(
            h,
            &format!("projects/{}/tasks/{}.md", t.projeto, t.onde),
            &com_frontmatter(&fm, &corpo),
        );
    }
    envelhece(h);
    home
}

/// O que a TUI mostra num modo: projeto → ids na ordem da lista, e os itens
/// do painel de projetos.
type Vista = (BTreeMap<String, Vec<String>>, Vec<String>);

/// Abre a TUI, liga os modos (`S`, `A`) e lê a lista de cada projeto.
fn vista(home: &std::path::Path, modos: &str) -> Vista {
    let ids: Vec<&str> = VAULT.iter().map(id_de).collect();
    let mut s = abre(home);
    aperta(&mut s, tecla('l'));
    aperta_todas(&mut s, modos);
    aperta(&mut s, tecla('h'));
    let mut listas = BTreeMap::new();
    let mut projetos = Vec::new();
    for passo in 0..PROJETOS.len() {
        if passo > 0 {
            aperta(&mut s, tecla('j'));
        }
        let t = tela_larga(&s);
        let titulo = t.titulo_das_tasks();
        if modos.contains('S') {
            assert!(titulo.contains("+algum-dia/adiadas"), "{titulo}");
        }
        if modos.contains('A') {
            assert!(titulo.contains("+arquivadas"), "{titulo}");
        }
        let projeto = PROJETOS
            .iter()
            .find(|p| titulo.contains(&format!("Tasks: {p}")))
            .unwrap_or_else(|| panic!("projeto do título {titulo:?}"));
        let lista: Vec<String> = itens_das_tasks(&t)
            .iter()
            .map(|item| {
                let achados: Vec<&&str> = ids.iter().filter(|id| item.contains(**id)).collect();
                assert_eq!(achados.len(), 1, "um id por item: {item:?}");
                achados[0].to_string()
            })
            .collect();
        listas.insert(projeto.to_string(), lista);
        projetos = t.itens_do(t.projetos());
    }
    (listas, projetos)
}

/// O conjunto de ids de cada projeto em cada modo, pela spec e pelo
/// `docs/freitask.md`: Algum dia e adiada só com `S`; arquivada só com `A`;
/// a `volta-em` passada não esconde.
fn esperado(modos: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut m: BTreeMap<String, BTreeSet<String>> = PROJETOS
        .iter()
        .map(|p| (p.to_string(), BTreeSet::new()))
        .collect();
    let escondidas = ["a-quatro", "a-cinco", "b-quatro", "g-um", "g-dois"];
    for t in &VAULT {
        let id = id_de(t);
        let mostra = if t.onde.starts_with("archived/") {
            modos.contains('A')
        } else if escondidas.contains(&id) {
            modos.contains('S')
        } else {
            true
        };
        if mostra {
            m.get_mut(t.projeto).expect("projeto").insert(id.to_owned());
        }
    }
    m
}

#[test]
fn prazo_nao_muda_a_ordem_nem_a_visibilidade_na_tui() {
    let sem = home_i11(false);
    let com = home_i11(true);
    for modos in ["", "S", "A", "SA"] {
        let (listas_sem, projetos_sem) = vista(sem.path(), modos);
        let (listas_com, projetos_com) = vista(com.path(), modos);
        let conjuntos: BTreeMap<String, BTreeSet<String>> = listas_sem
            .iter()
            .map(|(p, l)| (p.clone(), l.iter().cloned().collect()))
            .collect();
        assert_eq!(
            conjuntos,
            esperado(modos),
            "modo {modos:?}: o vault sem prazo já mostra o esperado"
        );
        assert_eq!(
            listas_com, listas_sem,
            "modo {modos:?}: a lista de cada projeto, com e sem prazo"
        );
        assert_eq!(
            projetos_com, projetos_sem,
            "modo {modos:?}: o painel de projetos, com e sem prazo"
        );
    }
}
