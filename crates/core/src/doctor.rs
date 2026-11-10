//! O `doctor`: verificação e reparo do vault, como função de uma fotografia
//! dele em memória. Puro: recebe os bytes que a crate `vault` leu e devolve os
//! achados e, com `--fix`, o plano que repara o que é derivável do caminho.
//!
//! Existe porque o freitask não é o único escritor do vault (o Obsidian, o
//! celular pelo Syncthing, agentes com shell): a estratégia não é impedir o
//! desvio, é torná-lo barulhento e, quando der, reparável (`docs/freitask.md`,
//! "`freitask doctor`"). A ordem dos checks e as mensagens são as do
//! `doctor.lua` de hoje; o que muda está no log da reescrita (decisões 38 a
//! 43) e no `divergencias.toml`.

use std::collections::{BTreeMap, BTreeSet};

use crate::date::Date;
use crate::deps;
use crate::index::{self, Indice};
use crate::links;
use crate::md;
use crate::meta;
use crate::model;
use crate::path::{self, TaskRef};
use crate::plano::{Efeito, Plano};
use crate::status::Vocab;
use crate::texto::Doc;
use crate::{config, lua_q};

/// A gravidade de um achado. `Error` é o que não se repara sem decisão humana.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nivel {
    Error,
    Warn,
}

impl Nivel {
    pub fn como_str(self) -> &'static str {
        match self {
            Nivel::Error => "error",
            Nivel::Warn => "warn",
        }
    }
}

/// Um achado: o `{level, kind, path, msg, fixed}` do JSON de hoje.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achado {
    pub nivel: Nivel,
    pub kind: &'static str,
    /// Relativo ao vault.
    pub path: String,
    pub msg: String,
    /// Reparado nesta passagem (só com `--fix`).
    pub fixed: bool,
}

/// O que o `doctor` precisa ler do vault. A crate `vault` monta; nada aqui
/// abre arquivo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fotografia {
    /// Todo `.md` do vault fora de pasta ou nome oculto, `(rel, bytes)`,
    /// inclusive `daily/`, o `CURRENT.md` e as cópias de conflito: é o
    /// `glob("**/*.md")` de hoje.
    pub notas: Vec<(String, Vec<u8>)>,
    /// Todo arquivo sob `projects/` com `.sync-conflict-` no nome, de qualquer
    /// extensão (I7).
    pub conflitos: Vec<String>,
    /// Os `.freitask.<nome>.<pid>.tmp` das pastas em que o freitask grava,
    /// `(rel, bytes)`.
    pub temporarios: Vec<(String, Vec<u8>)>,
}

/// O resultado: os achados, na ordem do texto, e o plano do `--fix` (vazio
/// sem `--fix`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostico {
    pub achados: Vec<Achado>,
    pub plano: Plano,
}

impl Diagnostico {
    /// Quantos achados sobraram sem reparo: decidem a saída 1.
    pub fn pendentes(&self) -> usize {
        self.achados.iter().filter(|a| !a.fixed).count()
    }
}

pub const MSG_CONFLITO: &str = "conflito do Syncthing — compare com o original e resolva à mão";
pub const MSG_FORA: &str =
    "não casa projects/<projeto>/tasks/[archived/<tipo>/]<id>.md — invisível ao freitask";
pub const MSG_TEMPORARIO: &str =
    "temporário de uma gravação interrompida — o arquivo de destino ficou como estava";

/// Verdadeiro se o nome é de uma cópia de conflito do Syncthing.
pub fn eh_conflito(rel: &str) -> bool {
    basename(rel).contains(".sync-conflict-")
}

fn basename(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

/// Verdadeiro se `rel` está sob `projects/<algo>/tasks/`.
fn sob_tasks(rel: &str) -> bool {
    rel.strip_prefix("projects/")
        .and_then(|r| r.split_once('/'))
        .is_some_and(|(p, resto)| !p.is_empty() && resto.starts_with("tasks/"))
}

/// Verdadeiro se é um `.md` sob `projects/<p>/tasks/` que o freitask não
/// enxerga como task (subpasta, tipo de arquivamento que não existe, projeto
/// reservado), fora as cópias de conflito, que têm achado próprio.
pub fn fora_do_padrao(rel: &str) -> bool {
    rel.ends_with(".md")
        && sob_tasks(rel)
        && !eh_conflito(rel)
        && path::split_task_path(rel).is_none()
}

/// Verdadeiro se o nome é o de um temporário de gravação:
/// `.freitask.<nome>.<pid>.tmp`, com o pid em dígitos (§1.5).
pub fn eh_temporario(nome: &str) -> bool {
    nome.strip_prefix(".freitask.")
        .and_then(|r| r.strip_suffix(".tmp"))
        .and_then(|r| r.rsplit_once('.'))
        .is_some_and(|(alvo, pid)| {
            !alvo.is_empty() && !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit())
        })
}

/// O alvo de link que o `doctor` cobra: `projects/<p>/tasks/…`. Um link curto
/// que não resolve é o placeholder normal do Obsidian; um link com caminho
/// para uma pasta de tasks foi gerado pelo freitask e tem de resolver.
fn cobrado(alvo: &str) -> bool {
    alvo.strip_prefix("projects/")
        .and_then(|r| r.split_once('/'))
        .is_some_and(|(p, resto)| !p.is_empty() && resto.starts_with("tasks/"))
}

struct Coletor {
    achados: Vec<Achado>,
}

impl Coletor {
    fn add(&mut self, nivel: Nivel, kind: &'static str, path: &str, msg: String, fixed: bool) {
        self.achados.push(Achado {
            nivel,
            kind,
            path: path.to_string(),
            msg,
            fixed,
        });
    }
}

/// Diagnostica o vault. Somente leitura sem `fix`; com `fix`, o plano repara
/// o que é derivável do caminho (bloco, fase de `archived/done/`, `id:` do
/// frontmatter, histórico ausente) e apaga os temporários órfãos. O link
/// pendurado é procurado depois do reparo, como hoje: o link da linha 2 de uma
/// task reparada deixa de ser achado.
pub fn diagnosticar(vocab: &Vocab, foto: &Fotografia, fix: bool, hoje: Date) -> Diagnostico {
    let mut c = Coletor {
        achados: Vec::new(),
    };
    let mut plano = Plano::new();

    // 1. Conflitos do Syncthing, em qualquer lugar sob `projects/` (I7).
    // Nunca reparados: escolher a cópia que vale é decisão editorial.
    let mut conflitos: Vec<&String> = foto.conflitos.iter().collect();
    conflitos.sort();
    for rel in conflitos {
        c.add(
            Nivel::Error,
            "sync-conflict",
            rel,
            MSG_CONFLITO.to_string(),
            false,
        );
    }

    let mut notas: Vec<&(String, Vec<u8>)> = foto.notas.iter().collect();
    notas.sort_by(|a, b| a.0.cmp(&b.0));

    // 2. `.md` sob `tasks/` que a ferramenta não enxerga.
    for (rel, _) in &notas {
        if fora_do_padrao(rel) {
            c.add(
                Nivel::Warn,
                "fora-do-padrao",
                rel,
                MSG_FORA.to_string(),
                false,
            );
        }
    }

    // As tasks: ativas por caminho, depois arquivadas por caminho.
    let mut tasks: Vec<(TaskRef, &String, &Vec<u8>)> = notas
        .iter()
        .filter_map(|(rel, b)| path::split_task_path(rel).map(|t| (t, rel, b)))
        .collect();
    tasks.sort_by(|a, b| {
        a.0.archived
            .is_some()
            .cmp(&b.0.archived.is_some())
            .then(a.1.cmp(b.1))
    });

    // 3. Id repetido: `[[id]]` fica ambíguo. Em ordem de id; o caminho é o da
    // primeira cópia.
    let mut por_id: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for (t, rel, _) in &tasks {
        por_id.entry(t.id.as_str()).or_default().push(rel);
    }
    for (id, rels) in &por_id {
        if rels.len() > 1 {
            c.add(
                Nivel::Warn,
                "id-duplicado",
                rels[0],
                format!(
                    "id {} existe em {} arquivos — `[[{id}]]` fica ambíguo",
                    lua_q(id),
                    rels.len()
                ),
                false,
            );
        }
    }

    // 4–6. Os checks de cada task. O reparo devolve os bytes novos, que a
    // busca de link pendurado lê no lugar dos antigos.
    let mut reparados: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
    for (t, rel, bytes) in &tasks {
        if let Some(depois) = checar_task(vocab, &mut c, t, rel, bytes, fix, hoje) {
            plano.push(Efeito::Escrever {
                rel: rel.to_string(),
                antes: Some(bytes.to_vec()),
                depois: depois.clone(),
            });
            reparados.insert(rel.as_str(), depois);
        }
    }

    // 7. Wikilinks path-qualificados pendurados: a impressão digital de um
    // rename feito sem a CLI. Não se reparam (nada registra o nome novo).
    let resolviveis: BTreeSet<String> = notas
        .iter()
        .flat_map(|(rel, _)| {
            let sem = rel.strip_suffix(".md").unwrap_or(rel);
            [sem.to_string(), path::basename_sem_extensao(sem)]
        })
        .collect();
    for (rel, bytes) in &notas {
        if rel.starts_with(&format!("{}/", config::DAILY_DIR)) {
            continue;
        }
        let bytes = reparados.get(rel.as_str()).unwrap_or(bytes);
        let texto = String::from_utf8_lossy(bytes);
        for l in links::scan(&texto) {
            let alvo = links::chave(&texto[l.target.clone()]);
            if cobrado(alvo) && !resolviveis.contains(alvo) {
                c.add(
                    Nivel::Error,
                    "link-pendurado",
                    rel,
                    format!("linha {}: `[[{alvo}]]` não resolve", l.linha + 1),
                    false,
                );
            }
        }
    }

    // 8. Dependências (spec §10.1): o grafo se lê do que está no disco, e o
    // celular ou um merge do Syncthing escrevem `bloqueada-por` sem passar
    // pelo `set`. Sem reparo: não se adivinha o id, e escolher que aresta
    // cortar é decisão editorial.
    let entradas = tasks
        .iter()
        .filter_map(|(_, rel, bytes)| index::scan_task(vocab, rel, bytes))
        .collect();
    let indice = Indice::new(Vec::new(), entradas);
    for e in indice.entradas.iter().filter(|e| e.archived.is_none()) {
        let mut vistos: Vec<&String> = Vec::new();
        for id in &e.meta.bloqueada_por {
            if vistos.contains(&id) {
                continue;
            }
            vistos.push(id);
            if indice.por_id(id).is_empty() {
                c.add(
                    Nivel::Warn,
                    "dependencia-pendurada",
                    &e.rel,
                    format!(
                        "bloqueada-por cita {}, que não existe — não bloqueia",
                        lua_q(id)
                    ),
                    false,
                );
            }
        }
    }
    for ciclo in deps::ciclos(&indice) {
        // `ciclos` só devolve ids de tasks ativas, e o primeiro é o da task
        // que o achou.
        let Some(primeira) = indice
            .entradas
            .iter()
            .find(|e| e.archived.is_none() && ciclo.first() == Some(&e.task_id))
        else {
            continue;
        };
        c.add(
            Nivel::Error,
            "dependencia-ciclica",
            &primeira.rel,
            format!("ciclo de dependência: {}", ciclo.join(" → ")),
            false,
        );
    }

    // Temporário de uma gravação que não chegou ao `rename`: o destino está
    // no estado anterior (I8), então apagá-lo não perde nada gravado.
    let mut temporarios: Vec<&(String, Vec<u8>)> = foto.temporarios.iter().collect();
    temporarios.sort_by(|a, b| a.0.cmp(&b.0));
    for (rel, bytes) in temporarios {
        c.add(
            Nivel::Warn,
            "temporario-orfao",
            rel,
            MSG_TEMPORARIO.to_string(),
            fix,
        );
        if fix {
            plano.push(Efeito::Apagar {
                rel: rel.clone(),
                antes: bytes.clone(),
            });
        }
    }

    Diagnostico {
        achados: c.achados,
        plano,
    }
}

/// Os checks de uma task (4 a 6 do `doctor.lua`). Devolve os bytes reparados
/// quando o `fix` mudou algo. Arquivo que não é UTF-8 não é nota do vault e
/// fica fora dos checks, como fica fora do índice.
fn checar_task(
    vocab: &Vocab,
    c: &mut Coletor,
    t: &TaskRef,
    rel: &str,
    bytes: &[u8],
    fix: bool,
    hoje: Date,
) -> Option<Vec<u8>> {
    let doc = Doc::parse(bytes).ok()?;
    let mut lines = doc.lines.clone();
    let Some((s, e, blk)) = md::first_block(&lines) else {
        c.add(
            Nivel::Error,
            "sem-callout",
            rel,
            "arquivo de task sem bloco de callout".into(),
            false,
        );
        return None;
    };
    let mut m = model::parse_block(vocab, &blk);
    let mut dirty = false;

    if m.status_num == 0 {
        c.add(
            Nivel::Warn,
            "status-0",
            rel,
            format!(
                "callout {} não existe em status.json",
                lua_q(&m.raw_callout)
            ),
            false,
        );
    }

    // O NOME DO ARQUIVO vence sempre: quem renomeou com `mv` deixou o id
    // antigo no bloco, e respeitá-lo deixaria a task para sempre
    // dessincronizada do próprio caminho.
    m.id = t.id.clone();
    m.project = Some(t.project.clone());
    m.archived = t.archived.clone();
    let mut want = model::serialize_block(vocab, &m);
    let mut reserializar = false;
    if want != blk {
        reserializar = fix;
        c.add(
            Nivel::Warn,
            "bloco-dessincronizado",
            rel,
            "bloco não corresponde ao caminho (link/id)".into(),
            fix,
        );
    }

    // Callout que contradiz `archived/done/`. Em `dropped` e `failed` a fase
    // de quando foi largada é informação e fica.
    if let Some(done) = vocab.by_callout("done")
        && t.archived.as_deref() == Some("done")
        && m.status_num != done
    {
        if fix {
            m.status_num = done;
            want = model::serialize_block(vocab, &m);
            reserializar = true;
        }
        let raw = if m.raw_callout.is_empty() {
            "?"
        } else {
            &m.raw_callout
        };
        c.add(
            Nivel::Warn,
            "callout-vs-pasta",
            rel,
            format!("está em archived/done/ com callout {}", lua_q(raw)),
            fix,
        );
    }

    if reserializar {
        lines = md::splice(&lines, s, e, &want);
        dirty = true;
    }

    let mut probe = lines.clone();
    if md::update_frontmatter_key(&mut probe, "id", &t.id) {
        if fix {
            lines = probe;
            dirty = true;
        }
        c.add(
            Nivel::Warn,
            "frontmatter-id",
            rel,
            "`id:` do frontmatter ≠ nome do arquivo".into(),
            fix,
        );
    }

    // 5. Arquivada sem registro de arquivamento: o reparo escreve uma linha
    // marcada como reconstruída.
    if let Some(tipo) = t.archived.as_deref()
        && !md::tem_entrada_de_arquivamento(&lines)
    {
        if fix {
            md::append_history_reconstruida(&mut lines, tipo, hoje);
            dirty = true;
        }
        c.add(
            Nivel::Warn,
            "historico-ausente",
            rel,
            "está em archived/ sem entrada de histórico".into(),
            fix,
        );
    }

    // 6. O frontmatter que o motor lê, só nas ativas. Sem reparo: não se
    // adivinha o que o dono quis dizer.
    if t.archived.is_none() {
        let fm = meta::read(&lines);
        if let Some(p) = &fm.prioridade
            && p != meta::HIGH
        {
            c.add(
                Nivel::Warn,
                "prioridade-desconhecida",
                rel,
                format!(
                    "prioridade {} não é `{}` — a task conta como normal",
                    lua_q(p),
                    meta::HIGH
                ),
                false,
            );
        }
        if let Some(v) = &fm.volta_em
            && Date::parse(v).is_none()
        {
            c.add(
                Nivel::Warn,
                "volta-em-invalido",
                rel,
                format!(
                    "volta-em {} não é uma data AAAA-MM-DD — a task aparece, sem adiar",
                    lua_q(v)
                ),
                false,
            );
        }
        if let Some(p) = &fm.prazo
            && Date::parse(p).is_none()
        {
            c.add(
                Nivel::Warn,
                "prazo-invalido",
                rel,
                format!(
                    "prazo {} não é uma data AAAA-MM-DD — não conta dias",
                    lua_q(p)
                ),
                false,
            );
        }
        // Só com a task ainda adiada: passada a `volta-em`, ela já voltou à
        // tela e não há mais o que acusar (decisão 55 do log).
        if let (Some(p), Some(v)) = (&fm.prazo, &fm.volta_em)
            && let (Some(prazo), Some(volta)) = (Date::parse(p), Date::parse(v))
            && prazo < volta
            && meta::is_postponed(&fm, hoje)
        {
            c.add(
                Nivel::Warn,
                "prazo-antes-da-volta",
                rel,
                format!("prazo {p} vem antes da volta-em {v} — a task volta à tela já vencida"),
                false,
            );
        }
    }

    dirty.then(|| doc.com_linhas(lines).to_bytes())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn hoje() -> Date {
        Date::parse("2026-11-08").unwrap()
    }

    fn foto(notas: &[(&str, &str)]) -> Fotografia {
        Fotografia {
            notas: notas
                .iter()
                .map(|(r, b)| (r.to_string(), b.as_bytes().to_vec()))
                .collect(),
            ..Fotografia::default()
        }
    }

    fn roda(f: &Fotografia, fix: bool) -> Diagnostico {
        diagnosticar(&Vocab::default(), f, fix, hoje())
    }

    /// `(kind, path, msg, fixed)` de cada achado, na ordem.
    fn resumo(d: &Diagnostico) -> Vec<(&str, &str, &str, bool)> {
        d.achados
            .iter()
            .map(|a| (a.kind, a.path.as_str(), a.msg.as_str(), a.fixed))
            .collect()
    }

    fn escrita<'a>(d: &'a Diagnostico, rel: &str) -> &'a str {
        d.plano
            .iter()
            .find_map(|e| match e {
                Efeito::Escrever { rel: r, depois, .. } if r == rel => {
                    Some(std::str::from_utf8(depois).unwrap())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("sem escrita em {rel}"))
    }

    const OK: &str = "projects/p/tasks/ok.md";
    const OK_MD: &str = "> [!todo] Ok\n> [[projects/p/tasks/ok|ok]]\n";

    #[test]
    fn vault_consistente_nao_tem_achado_nem_plano() {
        let f = foto(&[
            (OK, OK_MD),
            (
                "projects/p/tasks/archived/done/f.md",
                "> [!done] F\n> [[projects/p/tasks/archived/done/f|f]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n",
            ),
            (
                "n.md",
                "[[projects/p/tasks/ok]] [[projects/p/tasks/archived/done/f|f]] [[curto]]\n",
            ),
        ]);
        for fix in [false, true] {
            let d = roda(&f, fix);
            assert!(d.achados.is_empty(), "{:?}", d.achados);
            assert!(d.plano.is_empty());
            assert_eq!(d.pendentes(), 0);
        }
    }

    #[test]
    fn nivel_como_str() {
        assert_eq!(Nivel::Error.como_str(), "error");
        assert_eq!(Nivel::Warn.como_str(), "warn");
    }

    #[test]
    fn conflito_e_fora_do_padrao_em_ordem_de_caminho_e_nunca_reparados() {
        let mut f = foto(&[
            (OK, OK_MD),
            ("projects/p/tasks/z/solta.md", "x\n"),
            ("projects/p/tasks/archived/misc/m.md", "x\n"),
            ("projects/p/tasks/archived/a.md", "x\n"),
            ("projects/daily/tasks/r.md", "x\n"),
            ("projects/p/notas/livre.md", "x\n"),
            ("projects/p/tasks/ok.sync-conflict-1-2-A.md", OK_MD),
        ]);
        f.conflitos = vec![
            "projects/p/tasks/ok.sync-conflict-1-2-A.md".into(),
            "projects/p/anexos/f.sync-conflict-1-2-A.pdf".into(),
        ];
        let d = roda(&f, true);
        assert_eq!(
            resumo(&d),
            vec![
                (
                    "sync-conflict",
                    "projects/p/anexos/f.sync-conflict-1-2-A.pdf",
                    MSG_CONFLITO,
                    false
                ),
                (
                    "sync-conflict",
                    "projects/p/tasks/ok.sync-conflict-1-2-A.md",
                    MSG_CONFLITO,
                    false
                ),
                (
                    "fora-do-padrao",
                    "projects/daily/tasks/r.md",
                    MSG_FORA,
                    false
                ),
                (
                    "fora-do-padrao",
                    "projects/p/tasks/archived/a.md",
                    MSG_FORA,
                    false
                ),
                (
                    "fora-do-padrao",
                    "projects/p/tasks/archived/misc/m.md",
                    MSG_FORA,
                    false
                ),
                (
                    "fora-do-padrao",
                    "projects/p/tasks/z/solta.md",
                    MSG_FORA,
                    false
                ),
            ]
        );
        assert_eq!(d.achados[0].nivel, Nivel::Error);
        assert_eq!(d.achados[2].nivel, Nivel::Warn);
        assert!(d.plano.is_empty());
        assert_eq!(d.pendentes(), 6);
    }

    #[test]
    fn os_predicados_de_nome() {
        assert!(eh_conflito("a/b.sync-conflict-1.md"));
        assert!(!eh_conflito("a.sync-conflict-/b.md"), "só o nome conta");
        assert!(!eh_conflito("encerrar-recusa-sync-conflict.md"));
        assert!(fora_do_padrao("projects/p/tasks/s/x.md"));
        assert!(!fora_do_padrao("projects/p/tasks/s/x.txt"));
        assert!(!fora_do_padrao("projects/p/tasks/x.md"));
        assert!(!fora_do_padrao("projects/p/outra/s/x.md"));
        assert!(!fora_do_padrao("projects//tasks/s/x.md"));
        assert!(!fora_do_padrao("notas/p/tasks/s/x.md"));
        assert!(!fora_do_padrao("projects/p/tasks/s/x.sync-conflict-1.md"));
        assert!(eh_temporario(".freitask.ok.md.123.tmp"));
        assert!(eh_temporario(".freitask.a.b.c.9.tmp"));
        assert!(!eh_temporario(".freitask.ok.md.12a.tmp"));
        assert!(!eh_temporario(".freitask.ok.md..tmp"));
        assert!(!eh_temporario(".freitask..123.tmp"));
        assert!(!eh_temporario(".freitask.123.tmp"), "falta o nome");
        assert!(!eh_temporario("freitask.ok.md.1.tmp"));
        assert!(!eh_temporario(".freitask.ok.md.1.tm"));
        assert!(cobrado("projects/p/tasks/x"));
        assert!(!cobrado("projects/p/x"));
        assert!(!cobrado("projects//tasks/x"));
        assert!(!cobrado("x/projects/p/tasks/x"));
    }

    #[test]
    fn id_duplicado_em_ordem_de_id_com_o_caminho_da_primeira_copia() {
        let f = foto(&[
            (
                "projects/b/tasks/bb.md",
                "> [!todo] B\n> [[projects/b/tasks/bb|bb]]\n",
            ),
            (
                "projects/a/tasks/bb.md",
                "> [!todo] B\n> [[projects/a/tasks/bb|bb]]\n",
            ),
            (
                "projects/a/tasks/archived/done/aa.md",
                "> [!done] A\n> [[projects/a/tasks/archived/done/aa|aa]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n",
            ),
            (
                "projects/z/tasks/aa.md",
                "> [!todo] A\n> [[projects/z/tasks/aa|aa]]\n",
            ),
            (
                "projects/z/tasks/so.md",
                "> [!todo] S\n> [[projects/z/tasks/so|so]]\n",
            ),
        ]);
        assert_eq!(
            resumo(&roda(&f, false)),
            vec![
                (
                    "id-duplicado",
                    "projects/z/tasks/aa.md",
                    "id \"aa\" existe em 2 arquivos — `[[aa]]` fica ambíguo",
                    false
                ),
                (
                    "id-duplicado",
                    "projects/a/tasks/bb.md",
                    "id \"bb\" existe em 2 arquivos — `[[bb]]` fica ambíguo",
                    false
                ),
            ]
        );
    }

    #[test]
    fn sem_callout_e_status_0() {
        let f = foto(&[
            ("projects/p/tasks/sc.md", "# nada\n"),
            (
                "projects/p/tasks/s0.md",
                "> [!bogus] Zero\n> [[projects/p/tasks/s0|s0]]\n",
            ),
        ]);
        let d = roda(&f, true);
        assert_eq!(
            resumo(&d),
            vec![
                (
                    "status-0",
                    "projects/p/tasks/s0.md",
                    "callout \"bogus\" não existe em status.json",
                    false
                ),
                (
                    "sem-callout",
                    "projects/p/tasks/sc.md",
                    "arquivo de task sem bloco de callout",
                    false
                ),
            ]
        );
        assert_eq!(d.achados[1].nivel, Nivel::Error);
        assert!(d.plano.is_empty(), "status 0 não se repara");
    }

    #[test]
    fn task_que_nao_e_utf8_fica_fora_dos_checks() {
        let mut f = foto(&[]);
        f.notas
            .push(("projects/p/tasks/bin.md".into(), vec![0xFF, b'>']));
        assert!(roda(&f, true).achados.is_empty());
    }

    const DES: &str = "projects/p/tasks/des.md";
    const DES_MD: &str = "> [!todo] Des\n> [[projects/p/tasks/des|outro]]\n> _x_\n";

    #[test]
    fn bloco_dessincronizado_so_se_repara_com_fix() {
        let f = foto(&[(DES, DES_MD)]);
        let d = roda(&f, false);
        assert_eq!(
            resumo(&d),
            vec![(
                "bloco-dessincronizado",
                DES,
                "bloco não corresponde ao caminho (link/id)",
                false
            )]
        );
        assert!(d.plano.is_empty());
        let d = roda(&f, true);
        assert!(d.achados[0].fixed);
        assert_eq!(d.pendentes(), 0);
        assert_eq!(
            d.plano,
            vec![Efeito::Escrever {
                rel: DES.into(),
                antes: Some(DES_MD.as_bytes().to_vec()),
                depois: b"> [!todo] Des\n> [[projects/p/tasks/des|des]]\n> _x_\n".to_vec(),
            }]
        );
    }

    #[test]
    fn callout_vs_pasta_so_em_done_e_com_o_callout_cru() {
        let hist = "\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/x` (y)\n";
        let f = foto(&[
            (
                "projects/p/tasks/archived/done/a.md",
                &format!("> [!todo] A\n> [[projects/p/tasks/archived/done/a|a]]\n{hist}"),
            ),
            (
                "projects/p/tasks/archived/done/b.md",
                &format!("> [!bogus] B\n> [[projects/p/tasks/archived/done/b|b]]\n{hist}"),
            ),
            (
                "projects/p/tasks/archived/done/c.md",
                &format!("> [!] C\n> [[projects/p/tasks/archived/done/c|c]]\n{hist}"),
            ),
            (
                "projects/p/tasks/archived/dropped/d.md",
                &format!("> [!todo] D\n> [[projects/p/tasks/archived/dropped/d|d]]\n{hist}"),
            ),
            (
                "projects/p/tasks/archived/done/e.md",
                &format!("> [!done] E\n> [[projects/p/tasks/archived/done/e|e]]\n{hist}"),
            ),
        ]);
        let d = roda(&f, false);
        let cvp: Vec<_> = resumo(&d)
            .into_iter()
            .filter(|a| a.0 == "callout-vs-pasta")
            .map(|a| (a.1, a.2))
            .collect();
        assert_eq!(
            cvp,
            vec![
                (
                    "projects/p/tasks/archived/done/a.md",
                    "está em archived/done/ com callout \"todo\""
                ),
                (
                    "projects/p/tasks/archived/done/b.md",
                    "está em archived/done/ com callout \"bogus\""
                ),
                (
                    "projects/p/tasks/archived/done/c.md",
                    "está em archived/done/ com callout \"?\""
                ),
            ]
        );
        let d = roda(&f, true);
        assert!(escrita(&d, "projects/p/tasks/archived/done/a.md").starts_with("> [!done] A\n"));
        assert!(escrita(&d, "projects/p/tasks/archived/done/b.md").starts_with("> [!done] B\n"));
        assert_eq!(d.plano.len(), 3, "dropped e a done certa ficam");
    }

    #[test]
    fn sem_done_no_vocabulario_nao_ha_callout_vs_pasta() {
        let vocab = Vocab::from_json(r#"{"1":{"title":"A","callout":"todo"}}"#).unwrap();
        let f = foto(&[(
            "projects/p/tasks/archived/done/a.md",
            "> [!todo] A\n> [[projects/p/tasks/archived/done/a|a]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em x\n",
        )]);
        assert!(diagnosticar(&vocab, &f, true, hoje()).achados.is_empty());
    }

    #[test]
    fn frontmatter_id_alinha_so_com_fix_e_nao_cria_a_chave() {
        let fm = "projects/p/tasks/fm.md";
        let antes = "---\na: 1\nid: errado\n---\n> [!todo] F\n> [[projects/p/tasks/fm|fm]]\n";
        let f = foto(&[
            (fm, antes),
            (
                "projects/p/tasks/sem.md",
                "---\na: 1\n---\n> [!todo] S\n> [[projects/p/tasks/sem|sem]]\n",
            ),
        ]);
        let d = roda(&f, false);
        assert_eq!(
            resumo(&d),
            vec![(
                "frontmatter-id",
                fm,
                "`id:` do frontmatter ≠ nome do arquivo",
                false
            )]
        );
        assert!(d.plano.is_empty());
        let d = roda(&f, true);
        assert_eq!(
            escrita(&d, fm),
            "---\na: 1\nid: fm\n---\n> [!todo] F\n> [[projects/p/tasks/fm|fm]]\n"
        );
        assert_eq!(d.plano.len(), 1);
    }

    #[test]
    fn historico_ausente_so_em_arquivada_e_reconstroi_com_a_marca() {
        let h = "projects/p/tasks/archived/failed/h.md";
        let f = foto(&[
            (
                h,
                "> [!todo] H\n> [[projects/p/tasks/archived/failed/h|h]]\n",
            ),
            (
                "projects/p/tasks/ativa.md",
                "> [!todo] A\n> [[projects/p/tasks/ativa|ativa]]\n",
            ),
        ]);
        let d = roda(&f, false);
        assert_eq!(
            resumo(&d),
            vec![(
                "historico-ausente",
                h,
                "está em archived/ sem entrada de histórico",
                false
            )]
        );
        let d = roda(&f, true);
        assert_eq!(
            escrita(&d, h),
            "> [!todo] H\n> [[projects/p/tasks/archived/failed/h|h]]\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/failed` (falhou) [reconstruído pelo doctor; data real desconhecida]\n"
        );
    }

    #[test]
    fn prioridade_e_volta_em_so_nas_ativas_e_sem_reparo() {
        let a = "projects/p/tasks/a.md";
        let f = foto(&[
            (
                a,
                "---\nprioridade: Alta\nvolta-em: 2026-02-30\n---\n> [!todo] A\n> [[projects/p/tasks/a|a]]\n",
            ),
            (
                "projects/p/tasks/b.md",
                "---\nprioridade: alta\nvolta-em: 2026-02-28\n---\n> [!todo] B\n> [[projects/p/tasks/b|b]]\n",
            ),
            (
                "projects/p/tasks/archived/done/c.md",
                "---\nprioridade: x\nvolta-em: y\n---\n> [!done] C\n> [[projects/p/tasks/archived/done/c|c]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n",
            ),
        ]);
        let d = roda(&f, true);
        assert_eq!(
            resumo(&d),
            vec![
                (
                    "prioridade-desconhecida",
                    a,
                    "prioridade \"Alta\" não é `alta` — a task conta como normal",
                    false
                ),
                (
                    "volta-em-invalido",
                    a,
                    "volta-em \"2026-02-30\" não é uma data AAAA-MM-DD — a task aparece, sem adiar",
                    false
                ),
            ]
        );
        assert!(d.plano.is_empty());
    }

    /// Os achados de prazo (e só eles) de uma task ativa `t` com o frontmatter
    /// dado, no dia 2026-11-08.
    fn achados_de_prazo(fm: &str) -> Vec<(&'static str, String)> {
        let f = foto(&[(
            "projects/p/tasks/t.md",
            &format!("---\n{fm}---\n> [!todo] T\n> [[projects/p/tasks/t|t]]\n"),
        )]);
        let d = roda(&f, true);
        assert!(d.plano.is_empty(), "nunca se repara: {:?}", d.plano);
        d.achados
            .iter()
            .filter(|a| a.kind.starts_with("prazo-"))
            .map(|a| {
                assert!(!a.fixed && a.nivel == Nivel::Warn);
                (a.kind, a.msg.clone())
            })
            .collect()
    }

    #[test]
    fn prazo_invalido_acusa_so_o_que_nao_e_data_e_nao_repara() {
        assert_eq!(
            achados_de_prazo("prazo: 2026-02-30\n"),
            vec![(
                "prazo-invalido",
                "prazo \"2026-02-30\" não é uma data AAAA-MM-DD — não conta dias".to_string()
            )]
        );
        assert_eq!(achados_de_prazo("prazo: 2026-12-1\n").len(), 1);
        assert!(achados_de_prazo("prazo:\n").is_empty());
        assert!(achados_de_prazo("prazo: 2026-01-01\n").is_empty());
        assert!(achados_de_prazo("prazo: 2028-02-29\n").is_empty());
    }

    #[test]
    fn prazo_de_task_arquivada_nao_e_olhado() {
        let f = foto(&[(
            "projects/p/tasks/archived/done/c.md",
            "---\nprazo: x\nvolta-em: 2026-12-01\n---\n> [!done] C\n> [[projects/p/tasks/archived/done/c|c]]\n\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n",
        )]);
        assert!(roda(&f, true).achados.is_empty());
    }

    #[test]
    fn prazo_antes_da_volta_e_estrito_e_so_com_a_task_ainda_adiada() {
        assert_eq!(
            achados_de_prazo("volta-em: 2026-12-01\nprazo: 2026-11-20\n"),
            vec![(
                "prazo-antes-da-volta",
                "prazo 2026-11-20 vem antes da volta-em 2026-12-01 — a task volta à tela já vencida"
                    .to_string()
            )]
        );
        // Mesmo dia e depois: sem achado.
        assert!(achados_de_prazo("volta-em: 2026-12-01\nprazo: 2026-12-01\n").is_empty());
        assert!(achados_de_prazo("volta-em: 2026-12-01\nprazo: 2026-12-02\n").is_empty());
        // `volta-em` de hoje ou passada: a task já voltou, sem achado.
        assert!(achados_de_prazo("volta-em: 2026-11-08\nprazo: 2026-11-01\n").is_empty());
        assert!(achados_de_prazo("volta-em: 2026-11-01\nprazo: 2026-10-01\n").is_empty());
        // O dia anterior à volta ainda a deixa adiada.
        assert_eq!(
            achados_de_prazo("volta-em: 2026-11-09\nprazo: 2026-11-08\n").len(),
            1
        );
    }

    #[test]
    fn prazo_e_volta_em_com_uma_data_invalida_nao_se_comparam() {
        // Como texto, `2026-12-1` < `2026-12-05`; como data não é nada.
        let kinds = |fm: &str| -> Vec<&'static str> {
            achados_de_prazo(fm).into_iter().map(|(k, _)| k).collect()
        };
        assert_eq!(
            kinds("volta-em: 2026-12-05\nprazo: 2026-12-1\n"),
            vec!["prazo-invalido"]
        );
        assert!(kinds("volta-em: 2026-12-1\nprazo: 2026-11-20\n").is_empty());
        assert_eq!(
            kinds("volta-em: 2026-13-01\nprazo: 2026-02-30\n"),
            vec!["prazo-invalido"]
        );
    }

    #[test]
    fn a_ordem_dentro_de_uma_task_e_a_dos_checks() {
        let m = "projects/p/tasks/archived/done/m.md";
        let f = foto(&[(
            m,
            "---\nid: x\n---\n> [!example] M\n> [[projects/p/tasks/m|m]]\n",
        )]);
        let d = roda(&f, true);
        let kinds: Vec<&str> = d.achados.iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            vec![
                "bloco-dessincronizado",
                "callout-vs-pasta",
                "frontmatter-id",
                "historico-ausente"
            ],
            "e o link da linha 2 já resolve depois do reparo"
        );
        assert_eq!(
            escrita(&d, m),
            "---\nid: m\n---\n> [!done] M\n> [[projects/p/tasks/archived/done/m|m]]\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/done` (feito) [reconstruído pelo doctor; data real desconhecida]\n"
        );
        let d = roda(&f, false);
        assert_eq!(d.achados.last().unwrap().kind, "link-pendurado");
        assert_eq!(
            d.achados.last().unwrap().msg,
            "linha 5: `[[projects/p/tasks/m]]` não resolve"
        );
    }

    #[test]
    fn o_reparo_preserva_crlf_bom_e_a_falta_de_quebra_final() {
        let r = "projects/p/tasks/archived/done/x.md";
        let lf = "---\nid: e\n---\n> [!todo] X\n> [[projects/p/tasks/x|x]]\n\ncorpo";
        let crlf = format!("\u{feff}{}", lf.replace('\n', "\r\n"));
        let f = foto(&[(r, &crlf)]);
        let d = roda(&f, true);
        assert_eq!(
            escrita(&d, r),
            format!(
                "\u{feff}{}",
                "---\nid: x\n---\n> [!done] X\n> [[projects/p/tasks/archived/done/x|x]]\n\ncorpo\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/done` (feito) [reconstruído pelo doctor; data real desconhecida]"
                    .replace('\n', "\r\n")
            )
        );
    }

    #[test]
    fn link_pendurado_segue_o_scanner_e_pula_daily() {
        let f = foto(&[
            (OK, OK_MD),
            ("daily/d.md", "[[projects/p/tasks/sumiu]]\n"),
            (
                "n.md",
                "a [[projects/p/tasks/sumiu|s]] [[projects/p/tasks/ok]] [[projects/p/tasks/ok.md|o]]\n`[[projects/p/tasks/span]]` [[projects/p/x]] [[curto]]\n| [[projects/p/tasks/ok\\|ok]] | [[projects/p/tasks/t\\|t]] |\n[[projects/p/tasks/d]]\n",
            ),
        ]);
        assert_eq!(
            resumo(&roda(&f, true)),
            vec![
                (
                    "link-pendurado",
                    "n.md",
                    "linha 1: `[[projects/p/tasks/sumiu]]` não resolve",
                    false
                ),
                (
                    "link-pendurado",
                    "n.md",
                    "linha 3: `[[projects/p/tasks/t]]` não resolve",
                    false
                ),
                (
                    "link-pendurado",
                    "n.md",
                    "linha 4: `[[projects/p/tasks/d]]` não resolve",
                    false
                ),
            ]
        );
    }

    #[test]
    fn link_resolve_por_qualquer_nota_inclusive_daily_e_por_basename() {
        let f = foto(&[
            ("daily/projects/p/tasks/x.md", "nada\n"),
            ("projects/p/tasks/notas/y.md", "nada\n"),
            (
                "n.md",
                "[[projects/p/tasks/notas/y]] [[projects/q/tasks/z]]\n",
            ),
            (
                "projects/q/tasks/z.md",
                "> [!todo] Z\n> [[projects/q/tasks/z|z]]\n",
            ),
        ]);
        let d = roda(&f, false);
        assert_eq!(
            resumo(&d),
            vec![(
                "fora-do-padrao",
                "projects/p/tasks/notas/y.md",
                MSG_FORA,
                false
            )]
        );
    }

    /// Uma task ativa consistente com o caminho, com a lista `bloqueada-por`
    /// dada (`None` sem a chave).
    fn task_md(rel: &str, deps: Option<&str>) -> (String, String) {
        let t = path::split_task_path(rel).unwrap();
        let fm = deps
            .map(|d| format!("---\nbloqueada-por: [{d}]\n---\n\n"))
            .unwrap_or_default();
        let link = rel.strip_suffix(".md").unwrap();
        let (callout, historico) = match t.archived {
            Some(_) => (
                "done",
                "\n## Histórico\n\n- 2026-01-01 — arquivada em `archived/done` (feito)\n",
            ),
            None => ("todo", ""),
        };
        (
            rel.to_string(),
            format!("{fm}> [!{callout}] T\n> [[{link}|{}]]\n{historico}", t.id),
        )
    }

    fn foto_de_tasks(tasks: &[(&str, Option<&str>)]) -> Fotografia {
        Fotografia {
            notas: tasks
                .iter()
                .map(|(rel, deps)| {
                    let (r, b) = task_md(rel, *deps);
                    (r, b.into_bytes())
                })
                .collect(),
            ..Fotografia::default()
        }
    }

    #[test]
    fn dependencia_pendurada_so_de_ativa_um_achado_por_id_e_sem_reparo() {
        let f = foto_de_tasks(&[
            ("projects/p/tasks/b.md", Some("a, zz, zz, ww")),
            ("projects/p/tasks/a.md", None),
            // Arquivada que cita quem não existe: não espera por nada.
            ("projects/p/tasks/archived/done/f.md", Some("sumiu")),
            // A arquivada existe: está resolvida, não é pendurada.
            ("projects/q/tasks/c.md", Some("f, b")),
        ]);
        for fix in [false, true] {
            let d = roda(&f, fix);
            assert_eq!(
                resumo(&d),
                vec![
                    (
                        "dependencia-pendurada",
                        "projects/p/tasks/b.md",
                        "bloqueada-por cita \"zz\", que não existe — não bloqueia",
                        false
                    ),
                    (
                        "dependencia-pendurada",
                        "projects/p/tasks/b.md",
                        "bloqueada-por cita \"ww\", que não existe — não bloqueia",
                        false
                    ),
                ]
            );
            assert!(d.achados.iter().all(|a| a.nivel == Nivel::Warn));
            assert!(d.plano.is_empty());
            assert_eq!(d.pendentes(), 2);
        }
    }

    #[test]
    fn dependencia_ciclica_um_achado_por_ciclo_com_o_caminho_e_sem_reparo() {
        let f = foto_de_tasks(&[
            ("projects/p/tasks/a.md", Some("b")),
            ("projects/p/tasks/b.md", Some("c")),
            ("projects/p/tasks/c.md", Some("a")),
            // Espera o ciclo, mas não está nele.
            ("projects/p/tasks/d.md", Some("a")),
            ("projects/q/tasks/x.md", Some("y")),
            ("projects/q/tasks/y.md", Some("x")),
            // Passa por arquivada: está resolvida, não fecha ciclo.
            ("projects/q/tasks/m.md", Some("n")),
            ("projects/q/tasks/archived/done/n.md", Some("m")),
        ]);
        for fix in [false, true] {
            let d = roda(&f, fix);
            assert_eq!(
                resumo(&d),
                vec![
                    (
                        "dependencia-ciclica",
                        "projects/p/tasks/a.md",
                        "ciclo de dependência: a → b → c → a",
                        false
                    ),
                    (
                        "dependencia-ciclica",
                        "projects/q/tasks/x.md",
                        "ciclo de dependência: x → y → x",
                        false
                    ),
                ]
            );
            assert!(d.achados.iter().all(|a| a.nivel == Nivel::Error));
            assert!(d.plano.is_empty());
        }
    }

    #[test]
    fn dependencias_vem_depois_do_link_pendurado_e_antes_dos_temporarios() {
        let mut f = foto_de_tasks(&[("projects/p/tasks/a.md", Some("zz"))]);
        f.notas
            .push(("n.md".into(), b"[[projects/p/tasks/x]]\n".to_vec()));
        f.temporarios = vec![(".freitask.a.1.tmp".into(), b"x".to_vec())];
        let d = roda(&f, false);
        let kinds: Vec<_> = d.achados.iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            [
                "link-pendurado",
                "dependencia-pendurada",
                "temporario-orfao"
            ]
        );
    }

    #[test]
    fn dependencias_ignoram_task_que_nao_e_utf8() {
        let mut f = foto_de_tasks(&[("projects/p/tasks/a.md", Some("b"))]);
        f.notas
            .push(("projects/p/tasks/b.md".to_string(), vec![0xFF, 0xFE]));
        let d = roda(&f, false);
        let kinds: Vec<_> = d.achados.iter().map(|a| a.kind).collect();
        assert_eq!(kinds, ["dependencia-pendurada"], "b não é nota, não existe");
    }

    #[test]
    fn temporario_orfao_vem_por_ultimo_e_o_fix_o_apaga() {
        let mut f = foto(&[(OK, OK_MD), ("n.md", "[[projects/p/tasks/x]]\n")]);
        f.temporarios = vec![
            ("z/.freitask.a.1.tmp".into(), b"z".to_vec()),
            (".freitask.CURRENT.md.2.tmp".into(), b"r".to_vec()),
        ];
        let d = roda(&f, false);
        assert_eq!(
            resumo(&d),
            vec![
                (
                    "link-pendurado",
                    "n.md",
                    "linha 1: `[[projects/p/tasks/x]]` não resolve",
                    false
                ),
                (
                    "temporario-orfao",
                    ".freitask.CURRENT.md.2.tmp",
                    MSG_TEMPORARIO,
                    false
                ),
                (
                    "temporario-orfao",
                    "z/.freitask.a.1.tmp",
                    MSG_TEMPORARIO,
                    false
                ),
            ]
        );
        assert_eq!(d.achados[1].nivel, Nivel::Warn);
        assert!(d.plano.is_empty());
        let d = roda(&f, true);
        assert_eq!(d.pendentes(), 1);
        assert_eq!(
            d.plano,
            vec![
                Efeito::Apagar {
                    rel: ".freitask.CURRENT.md.2.tmp".into(),
                    antes: b"r".to_vec()
                },
                Efeito::Apagar {
                    rel: "z/.freitask.a.1.tmp".into(),
                    antes: b"z".to_vec()
                },
            ]
        );
    }

    #[test]
    fn fix_e_idempotente_sobre_a_fotografia_reparada() {
        let mut f = foto(&[
            (DES, DES_MD),
            (
                "projects/p/tasks/archived/done/m.md",
                "---\nid: x\n---\n> [!example] M\n> [[projects/p/tasks/m|m]]\n",
            ),
            ("projects/p/tasks/sc.md", "sem\n"),
        ]);
        f.temporarios = vec![("a/.freitask.b.1.tmp".into(), b"x".to_vec())];
        let d = roda(&f, true);
        for e in &d.plano {
            match e {
                Efeito::Escrever { rel, depois, .. } => {
                    let n = f.notas.iter_mut().find(|n| &n.0 == rel).unwrap();
                    n.1 = depois.clone();
                }
                Efeito::Apagar { rel, .. } => f.temporarios.retain(|t| &t.0 != rel),
                outro => panic!("efeito inesperado: {outro:?}"),
            }
        }
        let d2 = roda(&f, true);
        assert!(d2.plano.is_empty());
        assert_eq!(
            resumo(&d2),
            vec![(
                "sem-callout",
                "projects/p/tasks/sc.md",
                "arquivo de task sem bloco de callout",
                false
            )]
        );
    }
}
