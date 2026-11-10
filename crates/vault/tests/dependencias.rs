//! I10 (spec §3.2): o grafo de dependências. `set --bloqueada-por` nunca grava
//! um ciclo, `bloqueia` é o inverso exato de `bloqueada_por` sobre as tasks
//! ativas, `rename` preserva as dependências pelo id novo e o `doctor` acha
//! exatamente as penduradas e os ciclos do vault. Sobre vaults do gerador com
//! `dependencias: true` gravados num diretório temporário, com o `core` e o
//! executor de verdade (`Vault::gravar`, `Vault::aplicar`).
//!
//! O oráculo é deste arquivo e não usa `deps::ciclo`: lê a linha
//! `bloqueada-por` do texto cru das tasks e faz alcançabilidade simples sobre
//! o grafo de ids ativos. O id é o nó (o gerador repete um id entre
//! projetos): as arestas de todas as ativas com o mesmo id valem juntas.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use freitask_core::date::Date;
use freitask_core::deps;
use freitask_core::deslocar::{self, Contexto, Deslocamento};
use freitask_core::doctor::{self, Nivel};
use freitask_core::operacao::{self, PedidoSet};
use freitask_core::path;
use freitask_core::plano::Efeito;
use freitask_vault::Vault;
use proptest::prelude::*;

fn hoje() -> Date {
    Date::parse("2026-11-09").unwrap()
}

fn vault_do_gerador(semente: u64) -> (tempfile::TempDir, Vault) {
    let dir = tempfile::tempdir().unwrap();
    let raiz = dir.path().join("ObsidianVault");
    let opcoes = gerador::Opcoes {
        dependencias: true,
        ..gerador::Opcoes::default()
    };
    gerador::sintetico(semente, &opcoes).gravar(&raiz).unwrap();
    (dir, Vault::em(raiz))
}

// ---------------------------------------------------------------------------
// O oráculo
// ---------------------------------------------------------------------------

/// Uma task como o oráculo a vê: o caminho, o id e a lista `bloqueada-por`
/// lida do texto cru.
#[derive(Debug, Clone)]
struct No {
    rel: String,
    id: String,
    ativa: bool,
    deps: Vec<String>,
}

/// A lista da linha `bloqueada-por: [a, b]` do frontmatter, como escrita.
fn deps_do_texto(bytes: &[u8]) -> Vec<String> {
    let texto = String::from_utf8_lossy(bytes);
    let mut linhas = texto.trim_start_matches('\u{feff}').lines();
    if linhas.next().map(str::trim) != Some("---") {
        return Vec::new();
    }
    for l in linhas {
        if l.trim() == "---" {
            break;
        }
        if let Some(v) = l.strip_prefix("bloqueada-por:") {
            return v
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    Vec::new()
}

/// Todas as tasks do vault, ativas e arquivadas, em ordem de caminho.
fn nos(v: &Vault) -> Vec<No> {
    let mut out: Vec<No> = v
        .tasks()
        .into_iter()
        .filter_map(|(rel, bytes)| {
            let t = path::split_task_path(&rel)?;
            Some(No {
                id: t.id,
                ativa: t.archived.is_none(),
                deps: deps_do_texto(&bytes),
                rel,
            })
        })
        .collect();
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

type Grafo = BTreeMap<String, BTreeSet<String>>;

/// As arestas entre ids ATIVOS (quem espera → bloqueadora ativa). Com
/// `troca = (rel, lista)`, a lista da task `rel` é a dada.
fn grafo(nos: &[No], troca: Option<(&str, &[String])>) -> Grafo {
    let ativos: BTreeSet<&str> = nos
        .iter()
        .filter(|n| n.ativa)
        .map(|n| n.id.as_str())
        .collect();
    let mut g = Grafo::new();
    for n in nos.iter().filter(|n| n.ativa) {
        let lista = match troca {
            Some((rel, nova)) if rel == n.rel => nova,
            _ => &n.deps,
        };
        g.entry(n.id.clone()).or_default().extend(
            lista
                .iter()
                .filter(|d| ativos.contains(d.as_str()))
                .cloned(),
        );
    }
    g
}

/// Há um caminho de uma ou mais arestas de `de` até `ate`.
fn alcanca(g: &Grafo, de: &str, ate: &str) -> bool {
    let mut vistos = BTreeSet::new();
    let mut pilha: Vec<&str> = g
        .get(de)
        .map(|s| s.iter().map(String::as_str).collect())
        .unwrap_or_default();
    while let Some(u) = pilha.pop() {
        if u == ate {
            return true;
        }
        if vistos.insert(u)
            && let Some(s) = g.get(u)
        {
            pilha.extend(s.iter().map(String::as_str));
        }
    }
    false
}

/// Os ids que estão em algum ciclo.
fn em_ciclo(g: &Grafo) -> BTreeSet<String> {
    g.keys().filter(|u| alcanca(g, u, u)).cloned().collect()
}

fn sem_repetidos(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// As operações do core, como a CLI as chama
// ---------------------------------------------------------------------------

/// O pedido `set <rel> --bloqueada-por <texto>`: a validação do `core` e, se
/// aceita, a gravação de verdade. `Err` é a recusa (nada gravado).
fn set_bloqueada_por(v: &Vault, rel: &str, texto: &str) -> Result<Vec<String>, String> {
    let (vocab, _) = v.vocab();
    let indice = v.indice(&vocab);
    let entrada = indice.por_rel(rel).unwrap();
    let ids = deps::validar_bloqueada_por(&indice, entrada, texto)?;
    let t = path::split_task_path(rel).unwrap();
    let pedido = PedidoSet {
        bloqueada_por: Some(texto.to_string()),
        ..PedidoSet::default()
    };
    let validado = operacao::validar_set(&vocab, &pedido, t.archived.as_deref())
        .unwrap()
        .com_bloqueada_por(ids.clone());
    let bytes = v.ler(rel).unwrap();
    let depois = operacao::editar_set(&vocab, &t, &v.caminho_str(rel), &bytes, &validado).unwrap();
    v.gravar(rel, Some(&bytes), &depois).unwrap();
    Ok(ids)
}

/// O plano de `rename <rel> <id>-renomeada`; `None` se o `core` recusa (task
/// sem callout, por exemplo).
fn planeja_rename(v: &Vault, rel: &str) -> Option<(String, Deslocamento)> {
    let (vocab, _) = v.vocab();
    let t = path::split_task_path(rel).unwrap();
    let novo_id = format!("{}-renomeada", t.id);
    let bytes = v.ler(rel).unwrap();
    let notas = v.notas();
    let raiz = v.raiz().to_string_lossy().into_owned();
    let cx = Contexto {
        vocab: &vocab,
        visao: v,
        raiz: &raiz,
        notas: &notas,
        hoje: hoje(),
    };
    let d = deslocar::renomear(&cx, rel, &v.caminho_str(rel), &bytes, &novo_id).ok()?;
    Some((novo_id, d))
}

// ---------------------------------------------------------------------------
// Propriedades
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// `set --bloqueada-por` aceita ⇔ a lista nova não fecha ciclo pelo alvo
    /// (e os ids existem, são únicos e não são o alvo); quando aceita e
    /// gravada, o vault relido não tem ciclo passando pelo alvo.
    #[test]
    fn i10_set_bloqueada_por_nunca_grava_ciclo(
        semente in 0u64..1_000,
        alvo in any::<usize>(),
        dependentes in any::<bool>(),
        sorteio in proptest::collection::vec(any::<usize>(), 0..5),
    ) {
        let (_dir, v) = vault_do_gerador(semente);
        let todos = nos(&v);
        let ativas: Vec<&No> = todos.iter().filter(|n| n.ativa).collect();
        prop_assume!(!ativas.is_empty());
        let alvo = ativas[alvo % ativas.len()];

        // Metade dos casos sorteia entre quem já espera pelo alvo, que é de
        // onde o ciclo vem; a outra metade, entre todos os ids do vault.
        let g = grafo(&todos, None);
        let mut pool: Vec<String> = if dependentes {
            g.keys().filter(|u| alcanca(&g, u, &alvo.id)).cloned().collect()
        } else {
            Vec::new()
        };
        if pool.is_empty() {
            pool = todos.iter().map(|n| n.id.clone()).collect::<BTreeSet<_>>().into_iter().collect();
        }
        pool.push("inexistente-zz".to_string());
        let pedidos: Vec<String> = sorteio.iter().map(|i| pool[i % pool.len()].clone()).collect();
        let texto = pedidos.join(" , ");

        let lista = sem_repetidos(&pedidos);
        let ids_validos = lista.iter().all(|id| {
            *id != alvo.id && todos.iter().filter(|n| n.id == *id).count() == 1
        });
        let fecha_ciclo = alcanca(&grafo(&todos, Some((&alvo.rel, &lista))), &alvo.id, &alvo.id);
        // Limpar é sempre aceito, mesmo numa task que já está em ciclo.
        let aceita = lista.is_empty() || (ids_validos && !fecha_ciclo);

        let antes: BTreeMap<String, Vec<u8>> =
            v.tasks().into_iter().collect();
        match set_bloqueada_por(&v, &alvo.rel, &texto) {
            Ok(gravada) => {
                prop_assert!(aceita, "aceitou {texto:?} em {}: ids_validos={ids_validos} ciclo={fecha_ciclo}", alvo.rel);
                prop_assert_eq!(&gravada, &lista);
                let depois = nos(&v);
                let lida = depois.iter().find(|n| n.rel == alvo.rel).unwrap();
                prop_assert_eq!(&lida.deps, &lista, "a lista gravada é a validada");
                if !lista.is_empty() {
                    prop_assert!(!alcanca(&grafo(&depois, None), &alvo.id, &alvo.id),
                        "ciclo por {} depois de gravar {lista:?}", alvo.id);
                    let (vocab, _) = v.vocab();
                    prop_assert!(deps::ciclo(&v.indice(&vocab), &alvo.id).is_none());
                }
                // Só a task do alvo mudou.
                for (rel, bytes) in v.tasks() {
                    if rel != alvo.rel {
                        prop_assert_eq!(Some(&bytes), antes.get(&rel), "{} mudou", rel);
                    }
                }
            }
            Err(msg) => {
                prop_assert!(!aceita, "recusou {texto:?} em {}: {msg}", alvo.rel);
                if ids_validos && fecha_ciclo {
                    prop_assert!(msg.starts_with("ciclo de dependência: "), "{}", msg);
                }
                prop_assert_eq!(v.tasks().into_iter().collect::<BTreeMap<_, _>>(), antes,
                    "recusa não grava nada");
            }
        }
    }

    /// `bloqueia` é o inverso exato de `bloqueada_por` sobre as ativas, e
    /// `bloqueada` é "ao menos uma bloqueadora ativa".
    #[test]
    fn i10_bloqueia_e_o_inverso_exato_de_bloqueada_por(semente in 0u64..1_000) {
        let (_dir, v) = vault_do_gerador(semente);
        let (vocab, _) = v.vocab();
        let indice = v.indice(&vocab);
        let todos = nos(&v);
        let ativos: BTreeSet<&str> = todos.iter().filter(|n| n.ativa).map(|n| n.id.as_str()).collect();

        let mut inverso: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for n in todos.iter().filter(|n| n.ativa) {
            for d in &n.deps {
                inverso.entry(d.as_str()).or_default().insert(n.id.as_str());
            }
        }
        prop_assert_eq!(indice.entradas.len(), todos.len());
        for e in &indice.entradas {
            let n = todos.iter().find(|n| n.rel == e.rel).unwrap();
            prop_assert_eq!(&e.meta.bloqueada_por, &n.deps, "a lista como está escrita: {}", e.rel);
            let d = deps::dependencias(&indice, e);
            let esperado: Vec<&str> = inverso
                .get(e.task_id.as_str())
                .map(|s| s.iter().copied().collect())
                .unwrap_or_default();
            let tem: Vec<&str> = d.bloqueia.iter().map(String::as_str).collect();
            prop_assert_eq!(tem, esperado, "bloqueia de {}", e.rel);
            prop_assert_eq!(
                d.bloqueada,
                n.deps.iter().any(|b| ativos.contains(b.as_str())),
                "bloqueada de {}", e.rel
            );
        }
        // O outro sentido: quem lista `w` aparece no `bloqueia` de toda
        // entrada de `w`.
        for n in todos.iter().filter(|n| n.ativa) {
            for w in &n.deps {
                for e in indice.por_id(w) {
                    prop_assert!(deps::dependencias(&indice, e).bloqueia.contains(&n.id),
                        "{} lista {w} e não está no bloqueia dele", n.id);
                }
            }
        }
    }

    /// `rename` preserva todas as dependências pelo id novo: as listas de
    /// todas as tasks, depois, são as de antes com o id trocado.
    #[test]
    fn i10_rename_preserva_as_dependencias_pelo_id_novo(
        semente in 0u64..1_000,
        escolha in any::<usize>(),
    ) {
        let (_dir, v) = vault_do_gerador(semente);
        let antes = nos(&v);
        prop_assume!(!antes.is_empty());
        // A primeira task, a partir da sorteada, que o `core` aceita renomear.
        let Some((rel, novo_id, d)) = (0..antes.len()).find_map(|k| {
            let rel = &antes[(escolha + k) % antes.len()].rel;
            planeja_rename(&v, rel).map(|(id, d)| (rel.clone(), id, d))
        }) else {
            return Ok(());
        };
        let velho_id = path::split_task_path(&rel).unwrap().id;

        // Cada arquivo, uma escrita só: duas falhariam o CAS da segunda.
        let mut escritos = BTreeSet::new();
        for e in &d.plano {
            if let Efeito::Escrever { rel, .. } = e {
                prop_assert!(escritos.insert(rel.clone()), "duas escritas em {rel}");
            }
        }
        v.aplicar(&d.plano).unwrap();

        let depois = nos(&v);
        let por_rel: BTreeMap<&str, &No> = depois.iter().map(|n| (n.rel.as_str(), n)).collect();
        prop_assert_eq!(depois.len(), antes.len());
        for n in &antes {
            let rel_novo = if n.rel == rel { d.novo_rel.as_str() } else { n.rel.as_str() };
            let esperada = sem_repetidos(
                &n.deps
                    .iter()
                    .map(|i| if *i == velho_id { novo_id.clone() } else { i.clone() })
                    .collect::<Vec<_>>(),
            );
            let lida = por_rel.get(rel_novo).unwrap_or_else(|| panic!("{rel_novo} sumiu"));
            prop_assert_eq!(&lida.deps, &esperada, "lista de {} depois de renomear {}", n.rel, rel);
        }
        // Ninguém ficou citando o id antigo (ele deixou de existir).
        prop_assert!(depois.iter().all(|n| !n.deps.contains(&velho_id)));
    }

    /// O `doctor` acha exatamente as dependências penduradas (ativa que cita
    /// id que não existe nem entre as arquivadas) e as tasks que estão em
    /// algum ciclo, nunca reparados.
    #[test]
    fn i10_doctor_acha_exatamente_as_penduradas_e_os_ciclos(semente in 0u64..1_000) {
        let (_dir, v) = vault_do_gerador(semente);
        let (vocab, _) = v.vocab();
        let todos = nos(&v);
        let existentes: BTreeSet<&str> = todos.iter().map(|n| n.id.as_str()).collect();
        let g = grafo(&todos, None);

        let mut penduradas_esperadas: BTreeSet<(String, String)> = BTreeSet::new();
        for n in todos.iter().filter(|n| n.ativa) {
            for d in &n.deps {
                if !existentes.contains(d.as_str()) {
                    penduradas_esperadas.insert((n.rel.clone(), d.clone()));
                }
            }
        }
        let ciclos_esperados = em_ciclo(&g);

        for fix in [false, true] {
            let diag = doctor::diagnosticar(&vocab, &v.fotografia_do_doctor(), fix, hoje());
            let mut penduradas = BTreeSet::new();
            let mut n_penduradas = 0;
            let mut ciclos = BTreeSet::new();
            for a in &diag.achados {
                match a.kind {
                    "dependencia-pendurada" => {
                        prop_assert_eq!(a.nivel, Nivel::Warn);
                        prop_assert!(!a.fixed);
                        // `bloqueada-por cita "id", que não existe — …`
                        let id = a.msg.split('"').nth(1).unwrap().to_string();
                        penduradas.insert((a.path.clone(), id));
                        n_penduradas += 1;
                    }
                    "dependencia-ciclica" => {
                        prop_assert_eq!(a.nivel, Nivel::Error);
                        prop_assert!(!a.fixed);
                        // `ciclo de dependência: a → b → a`
                        let caminho: Vec<&str> = a
                            .msg
                            .strip_prefix("ciclo de dependência: ")
                            .unwrap()
                            .split(" → ")
                            .collect();
                        prop_assert!(caminho.len() >= 2 && caminho.first() == caminho.last(), "{}", a.msg);
                        for par in caminho.windows(2) {
                            prop_assert!(g.get(par[0]).is_some_and(|s| s.contains(par[1])),
                                "{} → {} não é dependência", par[0], par[1]);
                        }
                        let primeiro = todos.iter().find(|n| n.rel == a.path).unwrap();
                        prop_assert!(primeiro.ativa && primeiro.id == caminho[0],
                            "o path é a task da primeira posição: {}", a.path);
                        ciclos.extend(caminho.iter().map(|s| s.to_string()));
                    }
                    _ => {}
                }
            }
            prop_assert_eq!(n_penduradas, penduradas.len(), "um achado por (task, id)");
            prop_assert_eq!(&penduradas, &penduradas_esperadas);
            prop_assert_eq!(&ciclos, &ciclos_esperados);
        }
    }
}

/// Sem isto as propriedades passariam num gerador que não produz grafo
/// nenhum: o gerador tem de dar ciclo, dependência pendurada, task com quem a
/// espere (para o `rename` arrastar) e ciclo que o `set` recusa.
#[test]
fn o_gerador_exercita_o_grafo() {
    let (mut com_ciclo, mut com_pendurada, mut arrastou, mut recusou_ciclo) = (0, 0, 0, 0);
    for semente in 0..60 {
        let (_dir, v) = vault_do_gerador(semente);
        let todos = nos(&v);
        let g = grafo(&todos, None);
        if !em_ciclo(&g).is_empty() {
            com_ciclo += 1;
        }
        let existentes: BTreeSet<&str> = todos.iter().map(|n| n.id.as_str()).collect();
        if todos
            .iter()
            .any(|n| n.ativa && n.deps.iter().any(|d| !existentes.contains(d.as_str())))
        {
            com_pendurada += 1;
        }
        // Alvo com dependente ativo e id único: o `rename` arrasta a lista do
        // dependente, e o `set` do alvo para esperar por ele fecha ciclo.
        let alvo = todos.iter().find(|a| {
            a.ativa
                && todos.iter().filter(|n| n.id == a.id).count() == 1
                && todos
                    .iter()
                    .any(|n| n.ativa && n.rel != a.rel && n.deps.contains(&a.id))
        });
        let Some(alvo) = alvo else { continue };
        if let Some((_, d)) = planeja_rename(&v, &alvo.rel) {
            let dependente = todos
                .iter()
                .find(|n| n.ativa && n.rel != alvo.rel && n.deps.contains(&alvo.id))
                .unwrap();
            if d.plano
                .iter()
                .any(|e| matches!(e, Efeito::Escrever { rel, .. } if *rel == dependente.rel))
            {
                arrastou += 1;
            }
        }
        let dependente = todos
            .iter()
            .find(|n| {
                n.ativa
                    && n.rel != alvo.rel
                    && n.id != alvo.id
                    && todos.iter().filter(|m| m.id == n.id).count() == 1
                    && n.deps.contains(&alvo.id)
            })
            .map(|n| n.id.clone());
        if let Some(dep) = dependente
            && set_bloqueada_por(&v, &alvo.rel, &dep)
                .is_err_and(|m| m.starts_with("ciclo de dependência: "))
        {
            recusou_ciclo += 1;
        }
    }
    for (nome, n) in [
        ("vaults com ciclo", com_ciclo),
        ("vaults com dependência pendurada", com_pendurada),
        ("renames que arrastam a lista de outra task", arrastou),
        ("sets recusados por ciclo", recusou_ciclo),
    ] {
        assert!(n >= 3, "o gerador deu só {n} {nome} em 60 sementes");
    }
}
