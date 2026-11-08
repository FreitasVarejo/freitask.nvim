//! Propriedades dos deslocamentos (spec §3.2): I1 (links íntegros), I2
//! (preservação) e I5 (ida e volta), sobre vaults do gerador gravados num
//! diretório temporário. Os planos saem do `core` (`deslocar`) e o executor é
//! o de verdade (`Vault::aplicar`), como na CLI; só o lock e o painel ficam de
//! fora.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use freitask_core::config;
use freitask_core::date::Date;
use freitask_core::deslocar::{self, Contexto, Deslocamento};
use freitask_core::links;
use freitask_core::md;
use freitask_core::operacao::Visao;
use freitask_core::path;
use freitask_core::texto::Doc;
use freitask_vault::Vault;
use proptest::prelude::*;

/// Todos os arquivos da árvore (relativo → bytes), ocultos inclusive.
fn arvore(raiz: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut pilha = vec![raiz.to_path_buf()];
    while let Some(dir) = pilha.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                pilha.push(p);
            } else {
                let rel = p.strip_prefix(raiz).unwrap().to_string_lossy().into_owned();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    out
}

/// As tasks do vault (o que o `split_task_path` aceita; `*.sync-conflict-*`
/// fica de fora), em ordem.
fn tasks(v: &Vault) -> Vec<String> {
    v.tasks()
        .into_iter()
        .map(|(r, _)| r)
        .filter(|r| path::split_task_path(r).is_some())
        .collect()
}

fn hoje() -> Date {
    Date::parse("2026-11-08").unwrap()
}

/// Um deslocamento pedido sobre a task de índice `i` (módulo o número de
/// tasks): 0 archive, 1 unarchive, 2 rename, 3 move. `p` escolhe o tipo, o id
/// novo ou o projeto.
#[derive(Debug, Clone)]
struct Op {
    tipo: u8,
    i: usize,
    p: usize,
}

fn op() -> impl Strategy<Value = Op> {
    (0u8..4, any::<usize>(), any::<usize>()).prop_map(|(tipo, i, p)| Op { tipo, i, p })
}

/// Executa `op` no vault e devolve (de, para) se algo se moveu. Recusa do
/// `core` (destino ocupado, mesmo projeto…) não é falha da propriedade: o
/// vault fica como estava.
fn executa(v: &Vault, op: &Op, n: usize) -> Option<(String, String)> {
    let (vocab, _) = v.vocab();
    let tasks = tasks(v);
    if tasks.is_empty() {
        return None;
    }
    let rel = tasks[op.i % tasks.len()].clone();
    let t = path::split_task_path(&rel).unwrap();
    let bytes = v.ler(&rel).unwrap();
    let notas = v.notas();
    let raiz = v.raiz().to_string_lossy().into_owned();
    let cx = Contexto {
        vocab: &vocab,
        visao: v,
        raiz: &raiz,
        notas: &notas,
        hoje: hoje(),
    };
    let abs = v.caminho_str(&rel);
    let r: Result<Option<Deslocamento>, String> = match op.tipo {
        0 => {
            let tipo = config::ARCHIVED_TYPES[op.p % 3];
            deslocar::arquivar(&cx, &rel, &abs, &bytes, Some(tipo))
        }
        1 => deslocar::arquivar(&cx, &rel, &abs, &bytes, None),
        2 => deslocar::renomear(&cx, &rel, &abs, &bytes, &format!("{}-r{n}", t.id)).map(Some),
        _ => {
            let projetos = v.projetos();
            let destino = &projetos[op.p % projetos.len()];
            deslocar::mover(&cx, &rel, &abs, &bytes, destino, &projetos).map(Some)
        }
    };
    let d = r.ok()??;
    v.aplicar(&d.plano).unwrap();
    Some((rel, d.novo_rel))
}

/// Os links path-qualificados para `projects/*/tasks/` fora de código, nas
/// notas em que um deslocamento procura referências, menos as que estão em
/// `fora`: (resolvem, não resolvem).
fn links_de_task(v: &Vault, fora: &[&str]) -> (usize, usize) {
    let (mut sim, mut nao) = (0, 0);
    for (rel, bytes) in v.notas() {
        if fora.contains(&rel.as_str()) {
            continue;
        }
        let Ok(texto) = std::str::from_utf8(&bytes) else {
            continue;
        };
        for l in links::scan(texto) {
            let alvo = &texto[l.target.clone()];
            if alvo.starts_with("projects/") && alvo.contains("/tasks/") {
                if v.arquivo_existe(&format!("{alvo}.md")) {
                    sim += 1;
                } else {
                    nao += 1;
                }
            }
        }
    }
    (sim, nao)
}

/// O texto com o miolo de cada wikilink (fora de código) apagado: o que I2
/// manda preservar byte a byte numa nota que cita a task.
fn sem_miolos(texto: &str) -> String {
    let mut out = String::new();
    let mut de = 0;
    for l in links::scan(texto) {
        out.push_str(&texto[de..l.inner.start]);
        de = l.inner.end;
    }
    out.push_str(&texto[de..]);
    out
}

/// As linhas (com o próprio fim de linha) da task fora do primeiro bloco e da
/// linha `id:` do frontmatter, sem as linhas em branco do fim.
fn corpo(bytes: &[u8]) -> Vec<String> {
    let doc = Doc::parse(bytes).unwrap();
    let bloco = md::first_block_range(&doc.lines);
    let (_, fm) = md::frontmatter(&doc.lines);
    let mut out: Vec<String> = doc
        .lines
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            !bloco.is_some_and(|(s, e)| (s..=e).contains(i))
                && !(fm.is_some_and(|(s, e)| (s..=e).contains(i)) && l.starts_with("id:"))
        })
        .map(|(_, l)| l.clone())
        .collect();
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out
}

fn eh_subsequencia(a: &[String], b: &[String]) -> bool {
    let mut it = b.iter();
    a.iter().all(|x| it.any(|y| y == x))
}

/// I2 sobre um deslocamento `de → para`: nenhum arquivo some nem se
/// duplica; fora da task movida, cada arquivo que mudou é uma nota elegível
/// e só mudou dentro de `[[…]]`; a task movida guarda o corpo (em ordem) e o
/// fim de linha dominante.
fn confere_i2(
    antes: &BTreeMap<String, Vec<u8>>,
    depois: &BTreeMap<String, Vec<u8>>,
    de: &str,
    para: &str,
    elegiveis: &BTreeSet<String>,
) -> Result<(), TestCaseError> {
    let mut esperado: BTreeSet<&String> = antes.keys().collect();
    esperado.remove(&de.to_string());
    let para_s = para.to_string();
    esperado.insert(&para_s);
    let agora: BTreeSet<&String> = depois.keys().collect();
    prop_assert_eq!(agora, esperado, "arquivo sumiu ou se duplicou");
    for (rel, b) in depois {
        if rel == para {
            continue;
        }
        let a = &antes[rel];
        if a == b {
            continue;
        }
        prop_assert!(elegiveis.contains(rel), "{rel} mudou sem ser nota elegível");
        let (ta, tb) = (
            std::str::from_utf8(a).unwrap(),
            std::str::from_utf8(b).unwrap(),
        );
        prop_assert_eq!(
            sem_miolos(ta),
            sem_miolos(tb),
            "{} mudou fora dos links",
            rel
        );
    }
    let (a, b) = (&antes[de], &depois[para]);
    prop_assert!(
        eh_subsequencia(&corpo(a), &corpo(b)),
        "corpo de {de} não sobreviveu"
    );
    prop_assert_eq!(
        Doc::parse(a).unwrap().eol_dominante(),
        Doc::parse(b).unwrap().eol_dominante()
    );
    Ok(())
}

fn vault_do_gerador(semente: u64) -> (tempfile::TempDir, Vault) {
    let dir = tempfile::tempdir().unwrap();
    let raiz = dir.path().join("ObsidianVault");
    gerador::sintetico(semente, &gerador::Opcoes::default())
        .gravar(&raiz)
        .unwrap();
    (dir, Vault::em(raiz))
}

/// As tasks cujo arquivo é UTF-8 (as outras o `core` recusa).
fn utf8(v: &Vault, rel: &str) -> bool {
    v.ler(rel).is_some_and(|b| std::str::from_utf8(&b).is_ok())
}

/// A propriedade de I1/I2 aceita recusa (o vault fica como estava); isto
/// garante que o gerador e as operações deslocam de fato, para ela não
/// passar sem testar nada.
#[test]
fn as_sequencias_deslocam_de_verdade() {
    let mut movidos = [0usize; 4];
    for semente in 0..10 {
        let (_dir, v) = vault_do_gerador(semente);
        for (n, tipo) in [0u8, 1, 2, 3, 1, 0].into_iter().enumerate() {
            let o = Op {
                tipo,
                i: n * 7 + 3,
                p: n,
            };
            if executa(&v, &o, n).is_some() {
                movidos[usize::from(tipo)] += 1;
            }
        }
    }
    assert!(movidos.iter().all(|&m| m > 0), "{movidos:?}");
}

proptest! {
    /// I1 e I2 depois de cada passo de uma sequência de deslocamentos.
    #[test]
    fn i1_i2_links_integros_e_bytes_preservados(
        semente in 0u64..1_000,
        ops in proptest::collection::vec(op(), 1..8),
    ) {
        let (_dir, v) = vault_do_gerador(semente);
        for (n, o) in ops.iter().enumerate() {
            let antes = arvore(v.raiz());
            let elegiveis: BTreeSet<String> = v.notas().into_iter().map(|(r, _)| r).collect();
            let tasks = tasks(&v);
            let alvo = tasks.get(o.i % tasks.len().max(1)).cloned().unwrap_or_default();
            let i1_antes = links_de_task(&v, &[&alvo]);
            let Some((de, para)) = executa(&v, o, n) else {
                prop_assert_eq!(arvore(v.raiz()), antes, "recusa gravou algo");
                continue;
            };
            prop_assert_eq!(&de, &alvo);
            let i1_depois = links_de_task(&v, &[&para]);
            // I1: nenhum link que resolvia deixa de resolver, e nenhum
            // pendurado é "consertado".
            prop_assert_eq!(i1_antes, i1_depois, "{:?}: {} → {}", o, de, para);
            confere_i2(&antes, &arvore(v.raiz()), &de, &para, &elegiveis)?;
        }
    }

    /// I5: `archive; unarchive` (dropped/failed, que não mexem na fase) e
    /// `rename a→b; rename b→a` devolvem a árvore original, menos o
    /// `## Histórico` da task. Vale para task de bloco canônico: o primeiro
    /// deslocamento reserializa o bloco, e um bloco fora do canônico volta
    /// canônico (é o `bloco-dessincronizado` que o `doctor --fix` faria), e
    /// sem `id:` de frontmatter diferente do nome do arquivo (o rename o
    /// alinha ao id novo, como hoje).
    #[test]
    fn i5_ida_e_volta(semente in 0u64..1_000, i in any::<usize>(), tipo in 0usize..2, rename in any::<bool>()) {
        let (_dir, v) = vault_do_gerador(semente);
        let (vocab, _) = v.vocab();
        let ativas: Vec<String> = v
            .tasks()
            .into_iter()
            .map(|(r, _)| r)
            .filter(|r| path::split_task_path(r).is_some_and(|t| t.archived.is_none()) && utf8(&v, r))
            .filter(|r| {
                let doc = Doc::parse(&v.ler(r).unwrap()).unwrap();
                let (fm, _) = md::frontmatter(&doc.lines);
                let t = path::split_task_path(r).unwrap();
                if fm.get("id").is_some_and(|id| id != &t.id) {
                    return false;
                }
                md::first_block(&doc.lines).is_some_and(|(_, _, b)| {
                    let mut m = freitask_core::model::parse_block(&vocab, &b);
                    let t = path::split_task_path(r).unwrap();
                    m.project = Some(t.project);
                    freitask_core::model::serialize_block(&vocab, &m) == b
                })
            })
            .collect();
        prop_assume!(!ativas.is_empty());
        let rel = ativas[i % ativas.len()].clone();
        let t = path::split_task_path(&rel).unwrap();
        let antes = arvore(v.raiz());
        let (ida, volta) = if rename {
            (Op { tipo: 2, i: 0, p: 0 }, Op { tipo: 2, i: 0, p: 0 })
        } else {
            (Op { tipo: 0, i: 0, p: tipo + 1 }, Op { tipo: 1, i: 0, p: 0 })
        };
        // `executa` escolhe pela posição: refaz o índice para apontar a task.
        let idx = |v: &Vault, rel: &str| {
            tasks(v).iter().position(|r| r == rel).unwrap()
        };
        let (_, meio) = executa(&v, &Op { i: idx(&v, &rel), ..ida }, 0).unwrap();
        let (_, fim) = if rename {
            // O id novo é `<id>-r0`; a volta renomeia para o id original.
            let (vocab, _) = v.vocab();
            let bytes = v.ler(&meio).unwrap();
            let notas = v.notas();
            let raiz = v.raiz().to_string_lossy().into_owned();
            let cx = Contexto { vocab: &vocab, visao: &v, raiz: &raiz, notas: &notas, hoje: hoje() };
            let d = deslocar::renomear(&cx, &meio, "/a", &bytes, &t.id).unwrap();
            v.aplicar(&d.plano).unwrap();
            (meio.clone(), d.novo_rel)
        } else {
            executa(&v, &Op { i: idx(&v, &meio), ..volta }, 1).unwrap()
        };
        prop_assert_eq!(&fim, &rel);
        let depois = arvore(v.raiz());
        for (r, b) in &depois {
            if r != &rel {
                prop_assert_eq!(Some(b), antes.get(r), "{} não voltou", r);
            }
        }
        prop_assert_eq!(depois.len(), antes.len());
        // A task: o que havia antes, e depois só o `## Histórico`.
        let a = Doc::parse(&antes[&rel]).unwrap();
        let b = Doc::parse(&depois[&rel]).unwrap();
        let novas: Vec<&String> = b
            .lines
            .iter()
            .filter(|l| l.starts_with("- 2026-11-08 — ") && !a.lines.contains(l))
            .collect();
        prop_assert_eq!(novas.len(), 2, "duas entradas de histórico: {:?}", novas);
        let sem_novas: Vec<String> =
            b.lines.iter().filter(|l| !novas.contains(l)).cloned().collect();
        if a.lines.iter().any(|l| l == "## Histórico") {
            prop_assert_eq!(&sem_novas, &a.lines);
        } else {
            let mut base = a.lines.clone();
            while base.last().is_some_and(|l| l.trim().is_empty()) {
                base.pop();
            }
            base.extend(["".to_string(), "## Histórico".to_string(), "".to_string()]);
            prop_assert_eq!(&sem_novas, &base);
        }
    }
}
