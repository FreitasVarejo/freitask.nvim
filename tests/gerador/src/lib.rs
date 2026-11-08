//! Gerador de vaults sintéticos (spec §3.2 e §3.4, corpus b e c).
//!
//! Determinístico: a mesma semente produz os mesmos bytes, em qualquer máquina,
//! porque o PRNG é um xorshift próprio e nada aqui lê relógio ou ambiente. Os
//! testes de propriedade do `core` e o harness do diferencial partilham este
//! gerador para que "vault sintético" signifique a mesma coisa nos dois.
//!
//! O que ele cobre, de propósito: ids com acento dobrado, notas com `[[links]]`
//! curtos e path-qualificados, alias com `|` e `\|` (tabela), links em code
//! span e em bloco cercado, frontmatter com chaves alheias, CRLF em alguns
//! arquivos, `*.sync-conflict-*` em alguns, projeto sem `tasks/`, projeto
//! encerrado, formato legado, `archived: true` legado, `volta-em` válido e
//! inválido, prioridade certa e com typo. Grafos de `bloqueada-por` e `prazo`
//! só entram com as opções ligadas (fatias 9 e 10): sem elas, o vault é algo
//! que o oráculo Lua lê igual.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;

/// Um vault em memória: caminho relativo → bytes, mais os diretórios que
/// existem mesmo vazios (um projeto sem task ainda é um projeto).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vault {
    pub arquivos: BTreeMap<String, Vec<u8>>,
    pub dirs: BTreeSet<String>,
}

impl Vault {
    pub fn arquivo(&mut self, rel: &str, conteudo: impl Into<Vec<u8>>) {
        self.arquivos.insert(rel.to_string(), conteudo.into());
        if let Some((dir, _)) = rel.rsplit_once('/') {
            self.dir(dir);
        }
    }

    pub fn dir(&mut self, rel: &str) {
        let mut acc = String::new();
        for parte in rel.split('/') {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(parte);
            self.dirs.insert(acc.clone());
        }
    }

    /// Grava o vault em `raiz` (que vira `<HOME>/ObsidianVault` nos testes).
    pub fn gravar(&self, raiz: &Path) -> io::Result<()> {
        std::fs::create_dir_all(raiz)?;
        for d in &self.dirs {
            std::fs::create_dir_all(raiz.join(d))?;
        }
        for (rel, bytes) in &self.arquivos {
            if let Some(p) = Path::new(rel).parent() {
                std::fs::create_dir_all(raiz.join(p))?;
            }
            std::fs::write(raiz.join(rel), bytes)?;
        }
        Ok(())
    }

    /// Os ids das tasks ativas por projeto, lidos dos caminhos.
    pub fn tasks_ativas(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for rel in self.arquivos.keys() {
            let partes: Vec<&str> = rel.split('/').collect();
            if partes.len() == 4
                && partes[0] == "projects"
                && partes[2] == "tasks"
                && let Some(id) = partes[3].strip_suffix(".md")
                && !id.contains(".sync-conflict-")
            {
                out.push((partes[1].to_string(), id.to_string()));
            }
        }
        out
    }
}

/// xorshift64*: pequeno, determinístico, sem dependência.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(semente: u64) -> Self {
        Rng(semente.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn proximo(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Inteiro em `0..n` (n > 0).
    pub fn ate(&mut self, n: usize) -> usize {
        (self.proximo() % n as u64) as usize
    }

    /// Inteiro em `lo..=hi`.
    pub fn entre(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.ate(hi - lo + 1)
    }

    pub fn chance(&mut self, por_cento: usize) -> bool {
        self.ate(100) < por_cento
    }

    pub fn escolhe<'a, T>(&mut self, itens: &'a [T]) -> &'a T {
        &itens[self.ate(itens.len())]
    }
}

#[derive(Debug, Clone)]
pub struct Opcoes {
    pub max_projetos: usize,
    pub max_tasks: usize,
    /// Gera `bloqueada-por:` (fatia 9). Desligado por padrão: o oráculo não o conhece.
    pub dependencias: bool,
    /// Gera `prazo:` (fatia 10). Idem.
    pub prazo: bool,
}

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes {
            max_projetos: 6,
            max_tasks: 60,
            dependencias: false,
            prazo: false,
        }
    }
}

pub const STATUS_JSON: &str = r#"{
  "1": { "title": "Não iniciada", "callout": "todo", "icon": "󰗡", "hl_group": "DiagnosticInfo" },
  "2": { "title": "Refinando", "callout": "abstract", "icon": "󰨸", "hl_group": "DiagnosticInfo" },
  "3": { "title": "Refinada", "callout": "tip", "icon": "󰌶", "hl_group": "DiagnosticHint" },
  "4": { "title": "Em implementação", "callout": "example", "icon": "󰉹", "hl_group": "DiagnosticHint" },
  "5": { "title": "Em homologação", "callout": "question", "icon": "󰘥", "hl_group": "DiagnosticWarn" },
  "6": { "title": "Bloqueada", "callout": "warning", "icon": "󰀪", "hl_group": "DiagnosticWarn" },
  "7": { "title": "Pronta", "callout": "check", "icon": "󰄬", "hl_group": "DiagnosticOk" },
  "8": { "title": "Algum dia", "callout": "quote", "icon": "󱆨", "hl_group": "Comment" },
  "9": { "title": "Arquivada", "callout": "done", "icon": "󰄬", "hl_group": "DiagnosticOk" }
}"#;

const PROJETOS: &[&str] = &["alfa", "beta", "gama", "delta", "epsilon", "zeta"];
const VERBOS: &[&str] = &[
    "criar", "revisar", "medir", "mover", "apagar", "ligar", "testar", "cortar",
];
const OBJETOS: &[&str] = &[
    "painel",
    "painel-de-repasse",
    "aviso-de-rede",
    "galeria",
    "inventario",
    "modulo",
    "esteira",
    "recorte",
];
const TITULOS: &[&str] = &[
    "Configuração da API",
    "Ávida tarefa de revisão",
    "Fix: the thing!",
    "Mover o painel para o lugar certo",
    "Medição do tempo de partida",
    "Coisa com | pipe no título",
    "Título com `crase` e [colchete]",
];
const DESCS: &[&str] = &[
    "aguardando revisão",
    "no meio do caminho",
    "esperando dezembro",
    "falta a VPN",
];
const NOTAS: &[&str] = &[
    "impedimento: falta acesso",
    "ver também o MOC",
    "nota com acento: ação",
    "3 - comprar leite",
];
const CALLOUTS_ATIVOS: &[&str] = &[
    "todo", "abstract", "tip", "example", "question", "warning", "check", "quote",
];

/// O vault sintético da semente `semente`.
pub fn sintetico(semente: u64, opts: &Opcoes) -> Vault {
    let mut rng = Rng::new(semente);
    let mut v = Vault::default();

    // O vocabulário quase sempre está lá; às vezes falta, para a leitura pura
    // (I6) e o D7 terem corpus.
    if !rng.chance(15) {
        v.arquivo(".freitask/status.json", STATUS_JSON);
    }
    v.dir("projects");

    let n_proj = rng.entre(1, opts.max_projetos.clamp(1, PROJETOS.len()));
    let projetos: Vec<&str> = PROJETOS[..n_proj].to_vec();
    for p in &projetos {
        v.dir(&format!("projects/{p}/tasks"));
    }
    // Projeto com pasta mas sem tasks/: não é projeto para o freitask.
    if rng.chance(40) {
        v.arquivo(
            "projects/so-spec/so-spec.md",
            "# Só spec\n\nSem frente aberta.\n",
        );
    }
    // Projeto encerrado, com uma task arquivada dentro.
    if rng.chance(40) {
        v.arquivo(
            "projects/_encerrados/omega/tasks/archived/done/omega-feita.md",
            "> [!done] Feita\n> [[projects/_encerrados/omega/tasks/archived/done/omega-feita|omega-feita]]\n\n## Histórico\n\n- 2026-01-03 — arquivada em `archived/done` (feito)\n",
        );
        v.arquivo(
            "projects/_encerrados/omega/omega.md",
            "# omega\n\n[[omega-feita]]\n",
        );
    }

    let n_tasks = rng.ate(opts.max_tasks + 1);
    let mut ids: Vec<(String, String, Option<&'static str>)> = Vec::new(); // (projeto, id, archived)
    let mut usados: BTreeSet<String> = BTreeSet::new();
    for n in 0..n_tasks {
        let p = *rng.escolhe(&projetos);
        let mut id = format!("{}-{}", rng.escolhe(VERBOS), rng.escolhe(OBJETOS));
        if usados.contains(&id) || rng.chance(30) {
            id = format!("{id}-{n}");
        }
        if !usados.insert(id.clone()) {
            continue;
        }
        let archived = if rng.chance(25) {
            Some(*rng.escolhe(&["done", "dropped", "failed"]))
        } else {
            None
        };
        ids.push((p.to_string(), id, archived));
    }

    // Um id duplicado entre projetos, às vezes (id-duplicado no doctor).
    if projetos.len() > 1 && !ids.is_empty() && rng.chance(30) {
        let (p0, id0, _) = ids[0].clone();
        let outro = projetos
            .iter()
            .find(|p| **p != p0)
            .copied()
            .unwrap_or(projetos[0]);
        ids.push((outro.to_string(), id0, None));
    }

    let todos: Vec<(String, String, Option<&'static str>)> = ids.clone();
    for (p, id, archived) in &ids {
        let conteudo = task_sintetica(&mut rng, p, id, *archived, &todos, opts);
        let rel = match archived {
            Some(t) => format!("projects/{p}/tasks/archived/{t}/{id}.md"),
            None => format!("projects/{p}/tasks/{id}.md"),
        };
        v.arquivo(&rel, conteudo);
    }

    // Conflito do Syncthing ao lado de uma task.
    if let Some((p, id, None)) = ids.iter().find(|(_, _, a)| a.is_none())
        && rng.chance(30)
    {
        v.arquivo(
            &format!("projects/{p}/tasks/{id}.sync-conflict-20260809-123456-ABCDEFG.md"),
            format!("> [!todo] Cópia em conflito\n> [[projects/{p}/tasks/{id}|{id}]]\n"),
        );
    }
    // Arquivo fora do padrão em tasks/ (subpasta que não é archived/<tipo>).
    if rng.chance(25) {
        let p = projetos[0];
        v.arquivo(
            &format!("projects/{p}/tasks/notas/solta.md"),
            "# nota solta\n",
        );
        v.arquivo(
            &format!("projects/{p}/tasks/archived/talvez/estranha.md"),
            "> [!todo] Estranha\n> [[estranha]]\n",
        );
    }

    // MOC de cada projeto, com links para as tasks dele (curtos e com caminho).
    for p in &projetos {
        let mut moc = format!("# {p}\n\nFrente de trabalho {p}.\n\n");
        for (tp, id, archived) in &ids {
            if tp != p {
                continue;
            }
            let dir = match archived {
                Some(t) => format!("projects/{p}/tasks/archived/{t}"),
                None => format!("projects/{p}/tasks"),
            };
            match rng.ate(3) {
                0 => moc.push_str(&format!("- [[{id}]]\n")),
                1 => moc.push_str(&format!("- [[{dir}/{id}|{id}]]\n")),
                _ => moc.push_str(&format!("- [[{dir}/{id}#Notas|ver notas]]\n")),
            }
        }
        v.arquivo(&format!("projects/{p}/{p}.md"), moc);
    }

    // Notas soltas com referências de todo tipo, inclusive as que não se
    // reescrevem (código) e as de tabela.
    let n_notas = rng.ate(4);
    for i in 0..n_notas {
        let mut nota = format!("# Nota {i}\n\n");
        for (p, id, archived) in ids.iter().take(6) {
            let dir = match archived {
                Some(t) => format!("projects/{p}/tasks/archived/{t}"),
                None => format!("projects/{p}/tasks"),
            };
            match rng.ate(7) {
                0 => nota.push_str(&format!("ref curta [[{id}]]\n")),
                1 => nota.push_str(&format!("ref path [[{dir}/{id}|{id}]]\n")),
                2 => nota.push_str(&format!("| tabela | [[{dir}/{id}\\|{id}]] | fim |\n")),
                3 => nota.push_str(&format!(
                    "code span `[[{dir}/{id}|{id}]]` não se reescreve\n"
                )),
                4 => nota.push_str(&format!("```\n[[{dir}/{id}|{id}]] em bloco cercado\n```\n")),
                5 => nota.push_str(&format!("embed ![[{dir}/{id}#Seção]]\n")),
                _ => nota.push_str(&format!(
                    "alias [[{dir}/{id}|Meu Título]] e sufixo [[{dir}/{id}^bloco]]\n"
                )),
            }
        }
        // Link pendurado de propósito, às vezes: fica pendurado (I1).
        if rng.chance(30) {
            nota.push_str("pendurado [[projects/alfa/tasks/nao-existe|nao-existe]]\n");
        }
        let rel = format!("notas/nota-{i}.md");
        if rng.chance(30) {
            v.arquivo(&rel, crlf(&nota));
        } else {
            v.arquivo(&rel, nota);
        }
    }

    // daily/: snapshot que nunca se reescreve.
    if rng.chance(50)
        && let Some((p, id, _)) = ids.first()
    {
        v.arquivo(
            "daily/2026-01-01.md",
            format!("# Planning diário 01/01/26\n\n> [[projects/{p}/tasks/{id}|{id}]]\n"),
        );
    }
    // CURRENT.md de um dia anterior, com notas avulsas: o rebuild o arquiva em daily/.
    if rng.chance(60) {
        v.arquivo(
            "CURRENT.md",
            "---\ndate: 2026-01-02\nweekday: Friday\n---\n\n# Planning diário 02/01/26\n\n> Painel gerado automaticamente — 0 tasks em 0 projetos.\n\n## Notas Avulsas\n\n- lembrar do leite\n- [[nota-0]]\n\n",
        );
    }
    v
}

fn crlf(s: &str) -> Vec<u8> {
    s.replace('\n', "\r\n").into_bytes()
}

fn task_sintetica(
    rng: &mut Rng,
    projeto: &str,
    id: &str,
    archived: Option<&str>,
    todos: &[(String, String, Option<&'static str>)],
    opts: &Opcoes,
) -> Vec<u8> {
    let mut linhas: Vec<String> = Vec::new();

    // Frontmatter: chaves alheias, prioridade, volta-em, id (certo ou errado).
    let mut fm: Vec<String> = Vec::new();
    if rng.chance(30) {
        fm.push("tags: [projeto]".into());
    }
    if rng.chance(20) {
        fm.push(if rng.chance(20) {
            format!("id: {id}-velho")
        } else {
            format!("id: {id}")
        });
    }
    if rng.chance(25) {
        fm.push(if rng.chance(15) {
            "prioridade: Alta".into()
        } else {
            "prioridade: alta".into()
        });
    }
    if rng.chance(25) {
        let v = rng.escolhe(&[
            "2099-12-01",
            "2020-01-01",
            "2026-12-1",
            "01/12/2026",
            "\"2099-06-30\"",
            "",
        ]);
        fm.push(format!("volta-em: {v}"));
    }
    if opts.prazo && rng.chance(40) {
        let v = rng.escolhe(&[
            "2099-01-15",
            "2020-05-05",
            "2026-02-30",
            "amanha",
            "2027-03-01",
        ]);
        fm.push(format!("prazo: {v}"));
    }
    if opts.dependencias && rng.chance(40) {
        let candidatos: Vec<&str> = todos
            .iter()
            .map(|(_, i, _)| i.as_str())
            .filter(|i| *i != id)
            .collect();
        if !candidatos.is_empty() {
            let n = rng.entre(1, 3.min(candidatos.len()));
            let mut deps: Vec<&str> = Vec::new();
            for _ in 0..n {
                let d = *rng.escolhe(&candidatos);
                if !deps.contains(&d) {
                    deps.push(d);
                }
            }
            if rng.chance(15) {
                deps.push("inexistente");
            }
            fm.push(format!("bloqueada-por: [{}]", deps.join(", ")));
        }
    }
    if !fm.is_empty() {
        linhas.push("---".into());
        linhas.extend(fm);
        linhas.push("---".into());
        linhas.push(String::new());
    }

    // Bloco: formato atual (a maioria), legado ou status 0.
    let titulo = *rng.escolhe(TITULOS);
    let callout = match archived {
        Some("done") if !rng.chance(20) => "done",
        _ => {
            if rng.chance(5) {
                "questão"
            } else {
                rng.escolhe(CALLOUTS_ATIVOS)
            }
        }
    };
    let dir = match archived {
        Some(t) => format!("projects/{projeto}/tasks/archived/{t}"),
        None => format!("projects/{projeto}/tasks"),
    };
    match rng.ate(10) {
        0 => {
            // Legado: negrito, id curto, status-texto, Branch:.
            linhas.push(format!("> **{titulo}**"));
            linhas.push(format!("> [[{id}]]"));
            linhas.push("> 1 - Backlog".into());
            linhas.push(format!("> Branch: {id}"));
        }
        1 => {
            // Link da linha 2 dessincronizado do caminho (bloco-dessincronizado).
            linhas.push(format!("> [!{callout}] {titulo}"));
            linhas.push(format!("> [[projects/outro/tasks/{id}|{id}]]"));
        }
        _ => {
            linhas.push(format!("> [!{callout}] {titulo}"));
            linhas.push(format!("> [[{dir}/{id}|{id}]]"));
        }
    }
    if rng.chance(50) {
        linhas.push(format!("> _{}_", rng.escolhe(DESCS)));
    }
    let n_notas = rng.ate(3);
    for _ in 0..n_notas {
        linhas.push(format!("> {}", rng.escolhe(NOTAS)));
    }
    linhas.push(String::new());

    // Corpo: notas, checklist, links para outras tasks.
    if rng.chance(60) {
        linhas.push("## Notas".into());
        linhas.push(String::new());
        linhas.push("Texto livre que a ferramenta nunca reescreve.".into());
        if rng.chance(50) {
            let feitos = rng.ate(3);
            let total = feitos + rng.ate(3);
            for i in 0..total {
                linhas.push(format!(
                    "- [{}] item {i}",
                    if i < feitos { "x" } else { " " }
                ));
            }
        }
        if let Some((op, oid, oarch)) = todos.iter().find(|(_, oid, _)| oid != id) {
            let odir = match oarch {
                Some(t) => format!("projects/{op}/tasks/archived/{t}"),
                None => format!("projects/{op}/tasks"),
            };
            match rng.ate(4) {
                0 => linhas.push(format!("depende de [[{oid}]]")),
                1 => linhas.push(format!("depende de [[{odir}/{oid}|{oid}]]")),
                2 => linhas.push(format!("| dep | [[{odir}/{oid}\\|{oid}]] |")),
                _ => linhas.push(format!("`[[{odir}/{oid}|{oid}]]` em código")),
            }
        }
        linhas.push(String::new());
    }

    // Histórico: a arquivada quase sempre o tem; a ativa, às vezes (desarquivada).
    if archived.is_some() && !rng.chance(20) {
        linhas.push("## Histórico".into());
        linhas.push(String::new());
        linhas.push(format!(
            "- 2026-01-05 — arquivada em `archived/{}` ({})",
            archived.unwrap_or("done"),
            match archived {
                Some("done") => "feito",
                Some("failed") => "falhou",
                _ => "abandonado",
            }
        ));
        if rng.chance(30) {
            linhas.push(String::new());
            linhas.push("## Depois do histórico".into());
            linhas.push(String::new());
            linhas.push("Uma seção que o log não pode ultrapassar.".into());
        }
    } else if archived.is_none() && rng.chance(15) {
        linhas.push("## Histórico".into());
        linhas.push(String::new());
        linhas.push("- 2026-01-05 — arquivada em `archived/dropped` (abandonado)".into());
        linhas.push("- 2026-01-06 — desarquivada, de volta ao board".into());
    }

    let mut texto = linhas.join("\n");
    // Sem `\n` final às vezes; CRLF às vezes; BOM raramente.
    if rng.chance(70) {
        texto.push('\n');
    }
    let mut bytes = if rng.chance(15) {
        crlf(&texto)
    } else {
        texto.into_bytes()
    };
    if rng.chance(5) {
        let mut com_bom = vec![0xEF, 0xBB, 0xBF];
        com_bom.extend(bytes);
        bytes = com_bom;
    }
    bytes
}

/// Um vault mínimo e previsível: um projeto, três tasks ativas, uma arquivada.
pub fn minimo() -> Vault {
    let mut v = Vault::default();
    v.arquivo(".freitask/status.json", STATUS_JSON);
    v.arquivo(
        "projects/p/tasks/t1.md",
        "> [!todo] Primeira\n> [[projects/p/tasks/t1|t1]]\n> _começando_\n",
    );
    v.arquivo("projects/p/tasks/t2.md", "> [!example] Segunda\n> [[projects/p/tasks/t2|t2]]\n> nota\n\n## Notas\n\nver [[t1]] e [[projects/p/tasks/t1|t1]]\n");
    v.arquivo(
        "projects/p/tasks/t3.md",
        "---\nprioridade: alta\n---\n\n> [!check] Terceira\n> [[projects/p/tasks/t3|t3]]\n",
    );
    v.arquivo(
        "projects/p/tasks/archived/done/t0.md",
        "> [!done] Zero\n> [[projects/p/tasks/archived/done/t0|t0]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n",
    );
    v.arquivo("projects/q/tasks/.keep", "");
    v.dirs.insert("projects/q/tasks".into());
    v.arquivos.remove("projects/q/tasks/.keep");
    v.arquivo(
        "notas/ref.md",
        "path [[projects/p/tasks/t1|t1]]\ncurto [[t1]]\n| t | [[projects/p/tasks/t1\\|t1]] |\n",
    );
    v
}

/// Um HOME sem vault nenhum.
pub fn vazio() -> Vault {
    Vault::default()
}

/// Os casos patológicos (spec §3.4, corpus c), cada um com nome.
pub fn patologicos() -> Vec<(&'static str, Vault)> {
    let mut out = Vec::new();

    // D1: link em tabela com `\|`.
    let mut v = minimo();
    v.arquivo("notas/tabela.md", "| task | nota |\n| --- | --- |\n| [[projects/p/tasks/t1\\|t1]] | em tabela |\n| [[projects/p/tasks/t2\\|segunda]] | alias outro |\n");
    out.push(("link-em-tabela", v));

    // D2: `.` e `..` como projeto — o vault é só o palco.
    out.push(("ponto-e-ponto-ponto", minimo()));

    // D3: encerrar com sync-conflict e arquivo fora do padrão em tasks/.
    let mut v = minimo();
    v.arquivo("projects/c/tasks/archived/done/c-feita.md", "> [!done] Feita\n> [[projects/c/tasks/archived/done/c-feita|c-feita]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n");
    v.arquivo(
        "projects/c/tasks/c-feita.sync-conflict-20260809-123456-ABCDEFG.md",
        "> [!todo] Conflito\n> [[projects/c/tasks/c-feita|c-feita]]\n",
    );
    v.arquivo("projects/c/tasks/rascunho/ideia.md", "# ideia solta\n");
    out.push(("sync-conflict", v));

    // D4: CRLF e BOM.
    let mut v = minimo();
    v.arquivo("projects/p/tasks/crlf.md", crlf("> [!todo] Em CRLF\n> [[projects/p/tasks/crlf|crlf]]\n> _estado_\n\n## Notas\n\n- [ ] item\n"));
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend(b"> [!todo] Com BOM\n> [[projects/p/tasks/bom|bom]]\n");
    v.arquivo("projects/p/tasks/bom.md", bom);
    v.arquivo(
        "projects/p/tasks/sem-fim.md",
        "> [!todo] Sem newline final\n> [[projects/p/tasks/sem-fim|sem-fim]]",
    );
    out.push(("crlf-bom", v));

    // D5: link em código.
    let mut v = minimo();
    v.arquivo("notas/codigo.md", "Em span: `[[projects/p/tasks/t1|t1]]` fica.\n\n```md\n[[projects/p/tasks/t1|t1]] em bloco fica\n```\n\nFora: [[projects/p/tasks/t1|t1]] muda.\n");
    out.push(("link-em-codigo", v));

    // sem-callout.
    let mut v = minimo();
    v.arquivo(
        "projects/p/tasks/sem-callout.md",
        "# Só um título\n\nSem bloco nenhum.\n",
    );
    out.push(("sem-callout", v));

    // archived: true legado.
    let mut v = minimo();
    v.arquivo(
        "projects/p/tasks/legada.md",
        "---\narchived: true\n---\n\n> **Legada**\n> [[legada]]\n> 1 - Backlog\n> Branch: legada\n",
    );
    out.push(("archived-true-legado", v));

    // volta-em inválido e prioridade com typo.
    let mut v = minimo();
    v.arquivo("projects/p/tasks/adiada-errada.md", "---\nvolta-em: 2026-12-1\nprioridade: Alta\n---\n\n> [!tip] Adiada errada\n> [[projects/p/tasks/adiada-errada|adiada-errada]]\n");
    v.arquivo("projects/p/tasks/adiada-certa.md", "---\nvolta-em: 2099-12-01\n---\n\n> [!tip] Adiada certa\n> [[projects/p/tasks/adiada-certa|adiada-certa]]\n");
    out.push(("volta-em-invalido", v));

    // id duplicado entre projetos.
    let mut v = minimo();
    v.arquivo(
        "projects/q/tasks/t1.md",
        "> [!todo] Outra t1\n> [[projects/q/tasks/t1|t1]]\n",
    );
    out.push(("id-duplicado", v));

    // Projeto sem tasks/ e encerrado.
    let mut v = minimo();
    v.arquivo("projects/so-spec/so-spec.md", "# só spec\n");
    v.arquivo("projects/_encerrados/enc/tasks/archived/done/enc-feita.md", "> [!done] Feita\n> [[projects/_encerrados/enc/tasks/archived/done/enc-feita|enc-feita]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n");
    out.push(("projeto-sem-tasks-e-encerrado", v));

    // Vault sem nada (o caso do WSL: sem .freitask/ e sem projects/).
    let mut v = Vault::default();
    v.arquivo("README.md", "# vault do trabalho\n");
    out.push(("vault-sem-projects", v));
    out.push(("home-vazio", vazio()));

    // Status 0, callout com acento, e task sem projeto no link.
    let mut v = minimo();
    v.arquivo(
        "projects/p/tasks/typo.md",
        "> [!questão] Callout com typo\n> [[projects/p/tasks/typo|typo]]\n",
    );
    v.arquivo(
        "projects/p/tasks/archived/done/nao-done.md",
        "> [!todo] Em done sem done\n> [[projects/p/tasks/archived/done/nao-done|nao-done]]\n",
    );
    v.arquivo("projects/p/tasks/archived/dropped/sem-historico.md", "> [!warning] Largada sem log\n> [[projects/p/tasks/archived/dropped/sem-historico|sem-historico]]\n");
    v.arquivo("projects/p/tasks/id-errado.md", "---\nid: outro-id\n---\n\n> [!todo] Id errado\n> [[projects/p/tasks/id-errado|id-errado]]\n");
    out.push(("doctor-variado", v));

    // Link pendurado e link curto que não resolve (só o path-qualified conta).
    let mut v = minimo();
    v.arquivo(
        "notas/pendurado.md",
        "[[projects/p/tasks/sumiu|sumiu]] e [[nota-futura]]\n",
    );
    out.push(("link-pendurado", v));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mesma_semente_da_o_mesmo_vault() {
        let a = sintetico(42, &Opcoes::default());
        let b = sintetico(42, &Opcoes::default());
        assert_eq!(a, b);
        assert_ne!(a, sintetico(43, &Opcoes::default()));
    }

    #[test]
    fn sem_opcoes_nao_ha_prazo_nem_bloqueada_por() {
        for s in 0..50 {
            let v = sintetico(s, &Opcoes::default());
            for (rel, bytes) in &v.arquivos {
                let texto = String::from_utf8_lossy(bytes);
                assert!(!texto.contains("prazo:"), "{rel} tem prazo");
                assert!(!texto.contains("bloqueada-por:"), "{rel} tem bloqueada-por");
            }
        }
    }

    #[test]
    fn com_opcoes_aparecem() {
        let opts = Opcoes {
            dependencias: true,
            prazo: true,
            ..Opcoes::default()
        };
        let mut prazo = false;
        let mut deps = false;
        for s in 0..30 {
            let v = sintetico(s, &opts);
            for bytes in v.arquivos.values() {
                let texto = String::from_utf8_lossy(bytes);
                prazo |= texto.contains("prazo:");
                deps |= texto.contains("bloqueada-por:");
            }
        }
        assert!(prazo && deps);
    }

    #[test]
    fn grava_e_le_de_volta() {
        let dir = std::env::temp_dir().join(format!("gerador-{}", std::process::id()));
        let v = minimo();
        v.gravar(&dir).unwrap();
        assert!(dir.join("projects/p/tasks/t1.md").is_file());
        assert!(dir.join("projects/q/tasks").is_dir());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn patologicos_tem_nomes_unicos() {
        let nomes: Vec<&str> = patologicos().iter().map(|(n, _)| *n).collect();
        let set: BTreeSet<&str> = nomes.iter().copied().collect();
        assert_eq!(nomes.len(), set.len());
    }
}
