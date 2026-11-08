//! Os cenários (spec §3.4): cada subcomando sozinho e sequências de escrita,
//! com `doctor` entre os passos e `rebuild` no fim, sobre cada corpus.
//!
//! Passos com `{task1}`, `{task2}`, `{proj1}`, `{proj2}` são resolvidos pelo
//! oráculo sobre uma cópia do corpus (`descobrir`): o id de uma task ativa sem
//! homônimo, outra de outro projeto, e os dois projetos. Sem valor, o cenário
//! é pulado e o relatório diz.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::executar::Lado;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Corpus {
    Real,
    Sintetico(u64),
    Minimo,
    Patologico(String),
}

pub struct Cenario {
    pub nome: String,
    pub corpus: Corpus,
    pub passos: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct Descobertos {
    pub valores: BTreeMap<&'static str, String>,
}

fn p(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

const SEMENTES: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

pub fn todos(filtro: &str) -> Vec<Cenario> {
    let mut corpora: Vec<Corpus> = Vec::new();
    if matches!(filtro, "todos" | "real") {
        if crate::corpus::vault_real().is_dir() {
            corpora.push(Corpus::Real);
        } else {
            eprintln!("diferencial: ~/ObsidianVault não existe; o corpus real fica de fora");
        }
    }
    if matches!(filtro, "todos" | "sintetico") {
        corpora.push(Corpus::Minimo);
        corpora.extend(SEMENTES.iter().map(|s| Corpus::Sintetico(*s)));
    }
    if matches!(filtro, "todos" | "patologico") {
        corpora.extend(
            gerador::patologicos()
                .into_iter()
                .map(|(n, _)| Corpus::Patologico(n.to_string())),
        );
    }

    let mut out = Vec::new();
    for c in &corpora {
        for (nome, passos) in comuns() {
            out.push(Cenario {
                nome: format!("{}::{nome}", rotulo(c)),
                corpus: c.clone(),
                passos,
            });
        }
        if let Corpus::Patologico(n) = c {
            for (nome, passos) in especificos(n) {
                out.push(Cenario {
                    nome: format!("{}::{nome}", rotulo(c)),
                    corpus: c.clone(),
                    passos,
                });
            }
        }
    }
    out
}

fn rotulo(c: &Corpus) -> String {
    match c {
        Corpus::Real => "real".into(),
        Corpus::Sintetico(s) => format!("sintetico-{s}"),
        Corpus::Minimo => "minimo".into(),
        Corpus::Patologico(n) => format!("patologico-{n}"),
    }
}

/// Os cenários que rodam em todo corpus.
fn comuns() -> Vec<(&'static str, Vec<Vec<String>>)> {
    vec![
        (
            "leituras",
            vec![
                p(&["list"]),
                p(&["list", "--json"]),
                p(&["list", "--archived"]),
                p(&["list", "--json", "--archived"]),
                p(&["projects"]),
                p(&["projects", "--json"]),
                p(&["statuses"]),
                p(&["statuses", "--json"]),
                p(&["kebab", "Ávida Revisão_final"]),
                p(&["kebab", "  foo   bar  "]),
                p(&["kebab", "!!!"]),
                p(&["help"]),
                p(&[]),
                p(&["comando-desconhecido"]),
            ],
        ),
        (
            "doctor",
            vec![
                p(&["doctor"]),
                p(&["doctor", "--json"]),
                p(&["doctor", "--quiet"]),
                p(&["doctor"]),
            ],
        ),
        (
            "rebuild",
            vec![
                p(&["rebuild"]),
                p(&["rebuild"]),
                p(&["doctor", "--json"]),
                p(&["list", "--json"]),
            ],
        ),
        (
            "doctor-fix",
            vec![
                p(&["doctor", "--json"]),
                p(&["doctor", "--fix"]),
                p(&["doctor", "--json"]),
                p(&["doctor", "--fix", "--json"]),
                p(&["list", "--json", "--archived"]),
            ],
        ),
        (
            "escrita-nova",
            vec![
                p(&["new-project", "Diff Novo"]),
                p(&[
                    "new",
                    "diff-novo",
                    "diff-a",
                    "Tarefa Ávida",
                    "--desc",
                    "começando",
                ]),
                p(&["new", "diff-novo", "diff-b", "Segunda"]),
                p(&["new", "diff-novo", "diff-b", "Repetida"]),
                p(&["new", "diff-novo", "Id Ruim", "Ruim"]),
                p(&["list", "--json"]),
                p(&[
                    "set",
                    "diff-a",
                    "--status",
                    "example",
                    "--title",
                    "Título novo",
                ]),
                p(&["set", "diff-a", "--desc", ""]),
                p(&["set", "diff-b", "--volta-em", "2099-01-02"]),
                p(&["set", "diff-b", "--volta-em=2099-02-03"]),
                p(&["set", "diff-b", "--volta-em", ""]),
                p(&["set", "diff-b", "--status", "quote"]),
                p(&["doctor", "--json"]),
                p(&["archive", "diff-a", "done"]),
                p(&["set", "diff-a", "--status", "todo"]),
                p(&["list", "--json", "--archived"]),
                p(&["unarchive", "diff-a"]),
                p(&["rename", "diff-a", "diff-a2"]),
                p(&["rename", "diff-a2", "diff-a2"]),
                p(&["new-project", "diff-outro"]),
                p(&["move", "diff-a2", "diff-outro"]),
                p(&["move", "diff-a2", "diff-outro"]),
                p(&["encerrar", "diff-novo"]),
                p(&["archive", "diff-b", "dropped"]),
                p(&["encerrar", "diff-novo"]),
                p(&["new", "diff-novo", "diff-c", "No encerrado"]),
                p(&["reabrir", "diff-novo"]),
                p(&["doctor", "--json"]),
                p(&["delete", "diff-b"]),
                p(&["delete", "diff-b"]),
                p(&["projects", "--json"]),
                p(&["rebuild"]),
            ],
        ),
        (
            "escrita-existente",
            vec![
                p(&["set", "{task1}", "--desc", "tocada pelo diferencial"]),
                p(&["doctor", "--json"]),
                p(&["archive", "{task1}", "dropped"]),
                p(&["doctor", "--json"]),
                p(&["unarchive", "{task1}"]),
                p(&["rename", "{task1}", "{task1}-dif"]),
                p(&["doctor", "--json"]),
                p(&["rename", "{task1}-dif", "{task1}"]),
                p(&["move", "{task2}", "{proj1}"]),
                p(&["doctor", "--json"]),
                p(&["move", "{task2}", "{proj2}"]),
                p(&["doctor", "--json"]),
                p(&["list", "--json", "--archived"]),
                p(&["rebuild"]),
            ],
        ),
        (
            "recusas",
            vec![
                p(&["set", "nao-existe", "--status", "todo"]),
                p(&["set", "{task1}", "--status", "done"]),
                p(&["set", "{task1}", "--status", "bogus"]),
                p(&["set", "{task1}"]),
                p(&["set", "{task1}", "--title", "  "]),
                p(&["set", "{task1}", "--volta-em", "2026-13-01"]),
                p(&["set", "{task1}", "--volta-em"]),
                p(&["new", "daily", "x", "X"]),
                p(&["new", "{proj1}", "{task1}", "Repetida"]),
                p(&["new-project", "daily"]),
                p(&["new-project", "{proj1}"]),
                p(&["move", "{task1}", "nao-existe"]),
                p(&["move", "{task1}", "daily"]),
                p(&["move", "{task1}", "{proj1}"]),
                p(&["encerrar", "nao-existe"]),
                p(&["encerrar", "{proj1}"]),
                p(&["reabrir", "nao-existe"]),
                p(&["reabrir", "{proj1}"]),
                p(&["archive", "{task1}"]),
                p(&["archive", "{task1}", "talvez"]),
                p(&["unarchive", "nao-existe"]),
                p(&["rename", "{task1}", "Id Ruim"]),
                p(&["rename", "{task1}"]),
                p(&["delete", "nao-existe"]),
                p(&["new"]),
                p(&["new", "{proj1}"]),
                p(&["new", "{proj1}", "id-x"]),
                p(&["move"]),
                p(&["encerrar"]),
                p(&["reabrir"]),
                p(&["delete"]),
                p(&["new-project"]),
                p(&["kebab"]),
                p(&["doctor", "--json"]),
            ],
        ),
        (
            "help-por-subcomando",
            vec![
                p(&["doctor", "--help"]),
                p(&["new", "--help"]),
                p(&["list", "--help"]),
                p(&["set", "-h"]),
                p(&["archive", "--help"]),
                p(&["help", "--json"]),
                p(&["version"]),
                p(&["version", "--json"]),
                p(&["show", "{task1}"]),
                p(&["show", "{task1}", "--json"]),
                p(&["new", "{proj1}", "diff-json", "Com json", "--json"]),
                p(&["doctor", "--json"]),
            ],
        ),
        (
            "ponto-e-ponto-ponto",
            vec![
                p(&["new", "..", "diff-y", "Y"]),
                p(&["new", ".", "diff-z", "Z"]),
                p(&["new-project", ".."]),
                p(&["move", "{task1}", ".."]),
                p(&["doctor", "--json"]),
                p(&["list", "--json"]),
            ],
        ),
    ]
}

/// Cenários que só fazem sentido num patológico específico.
fn especificos(nome: &str) -> Vec<(&'static str, Vec<Vec<String>>)> {
    match nome {
        "link-em-tabela" => vec![(
            "tabela",
            vec![
                p(&["archive", "t1", "dropped"]),
                p(&["doctor", "--json"]),
                p(&["unarchive", "t1"]),
                p(&["rename", "t1", "t1b"]),
                p(&["doctor", "--json"]),
                p(&["move", "t1b", "q"]),
                p(&["doctor", "--json"]),
                p(&["rename", "t2", "t2b"]),
                p(&["doctor", "--json"]),
            ],
        )],
        "sync-conflict" => vec![(
            "encerrar-com-conflito",
            vec![
                p(&["doctor", "--json"]),
                p(&["encerrar", "c"]),
                p(&["doctor", "--json"]),
                p(&["list", "--json", "--archived"]),
            ],
        )],
        "crlf-bom" => vec![(
            "set-preserva-bytes",
            vec![
                p(&["set", "crlf", "--status", "example"]),
                p(&["set", "bom", "--desc", "com bom"]),
                p(&["set", "sem-fim", "--title", "Título trocado"]),
                p(&["set", "crlf", "--volta-em", "2099-01-01"]),
                p(&["archive", "crlf", "dropped"]),
                p(&["doctor", "--json"]),
                p(&["list", "--json", "--archived"]),
            ],
        )],
        "link-em-codigo" => vec![(
            "codigo",
            vec![
                p(&["archive", "t1", "done"]),
                p(&["rename", "t2", "t2b"]),
                p(&["doctor", "--json"]),
            ],
        )],
        "id-duplicado" => vec![(
            "ambiguo",
            vec![
                p(&["doctor", "--json"]),
                p(&["set", "t1", "--desc", "ambígua"]),
                p(&[
                    "set",
                    "ObsidianVault/projects/p/tasks/t1.md",
                    "--desc",
                    "pelo caminho",
                ]),
                p(&["archive", "ObsidianVault/projects/q/tasks/t1.md", "dropped"]),
                p(&["doctor", "--json"]),
            ],
        )],
        "doctor-variado"
        | "link-pendurado"
        | "volta-em-invalido"
        | "sem-callout"
        | "archived-true-legado" => vec![(
            "fix-idempotente",
            vec![
                p(&["doctor"]),
                p(&["doctor", "--fix"]),
                p(&["doctor", "--json"]),
                p(&["doctor", "--fix"]),
                p(&["list", "--json", "--archived"]),
                p(&["rebuild"]),
            ],
        )],
        "vault-sem-projects" | "home-vazio" => vec![
            (
                "leitura-pura-sem-vault",
                vec![
                    p(&["list", "--json"]),
                    p(&["projects", "--json"]),
                    p(&["statuses", "--json"]),
                    p(&["doctor", "--json"]),
                    p(&["doctor"]),
                    p(&["kebab", "x y"]),
                    p(&["help"]),
                ],
            ),
            (
                "nasce-do-nada",
                vec![
                    p(&["new", "primeiro", "t", "Primeira task"]),
                    p(&["list", "--json"]),
                    p(&["doctor", "--json"]),
                    p(&["rebuild"]),
                ],
            ),
        ],
        _ => vec![],
    }
}

/// Resolve os placeholders com o que o oráculo enxerga no corpus.
pub fn descobrir(lado: &Lado) -> Option<Descobertos> {
    let tasks = lado.rodar(&p(&["list", "--json"]));
    let projetos = lado.rodar(&p(&["projects", "--json"]));
    let tasks: Vec<Value> = serde_json::from_slice(&tasks.stdout).ok()?;
    let projetos: Vec<Value> = serde_json::from_slice(&projetos.stdout).ok()?;

    let mut contagem: BTreeMap<String, usize> = BTreeMap::new();
    for t in &tasks {
        if let Some(id) = t.get("id").and_then(Value::as_str) {
            *contagem.entry(id.to_string()).or_default() += 1;
        }
    }
    // Ids ativos sem homônimo em lugar nenhum, inclusive arquivado: o oráculo
    // recusa id ambíguo, e o `rename` para `<id>-dif` não pode colidir.
    let arquivadas = lado.rodar(&p(&["list", "--json", "--archived"]));
    let arquivadas: Vec<Value> = serde_json::from_slice(&arquivadas.stdout).unwrap_or_default();
    for t in &arquivadas {
        if t.get("archived").and_then(Value::as_str).is_some()
            && let Some(id) = t.get("id").and_then(Value::as_str)
        {
            *contagem.entry(id.to_string()).or_default() += 1;
        }
    }
    let unicas: Vec<(&str, &str)> = tasks
        .iter()
        .filter_map(|t| {
            let id = t.get("id")?.as_str()?;
            let pr = t.get("project")?.as_str()?;
            (contagem.get(id) == Some(&1)).then_some((pr, id))
        })
        .collect();

    let mut d = Descobertos::default();
    let (proj1, task1) = *unicas.first()?;
    d.valores.insert("task1", task1.to_string());
    d.valores.insert("proj1", proj1.to_string());
    if let Some((_, t2)) = unicas.iter().find(|(pr, _)| *pr != proj1) {
        d.valores.insert("task2", t2.to_string());
    } else if let Some((_, t2)) = unicas.get(1) {
        d.valores.insert("task2", t2.to_string());
    }
    let proj2 = projetos
        .iter()
        .filter_map(|pr| pr.get("project").and_then(Value::as_str))
        .find(|pr| *pr != proj1);
    if let Some(p2) = proj2 {
        d.valores.insert("proj2", p2.to_string());
    }
    Some(d)
}

/// Substitui os placeholders; `None` se algum não tem valor.
pub fn resolver(passos: &[Vec<String>], d: Option<&Descobertos>) -> Option<Vec<Vec<String>>> {
    let mut out = Vec::new();
    for passo in passos {
        let mut novo = Vec::new();
        for arg in passo {
            let mut a = arg.clone();
            for chave in ["task1", "task2", "proj1", "proj2"] {
                let marca = format!("{{{chave}}}");
                if a.contains(&marca) {
                    let v = d?.valores.get(chave)?;
                    a = a.replace(&marca, v);
                }
            }
            novo.push(a);
        }
        out.push(novo);
    }
    Some(out)
}
