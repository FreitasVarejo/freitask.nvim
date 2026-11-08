//! Os deslocamentos de uma task: arquivar, desarquivar, renomear e mudar de
//! projeto. São o mesmo evento ("esse arquivo mudou de endereço"), com o
//! mesmo plano: mover o arquivo, regravá-lo no endereço novo (o link da
//! linha 2, a fase, o `## Histórico`, o `id:` no rename) e reescrever os
//! wikilinks das notas que o citam (`links`). A ordem é a de hoje: arquivo
//! primeiro, links depois; se algo falha no meio, o `doctor` acusa e o
//! `--fix` repara (spec §1.5).

use crate::date::Date;
use crate::links::{self, Endereco};
use crate::md;
use crate::model;
use crate::operacao::Visao;
use crate::path::{self, TaskRef};
use crate::plano::{Efeito, Plano};
use crate::status::Vocab;
use crate::task;
use crate::texto::Doc;
use crate::{config, trim};

/// O resultado de um deslocamento: onde a task foi parar, o plano, e quantos
/// links ele reescreve em quantas notas (o aviso de stderr).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deslocamento {
    pub novo_rel: String,
    pub plano: Plano,
    pub refs: usize,
    pub arquivos: usize,
}

/// O aviso de hoje quando algum link foi reescrito.
pub fn aviso(refs: usize, arquivos: usize) -> Option<String> {
    (refs > 0).then(|| format!("{refs} referência(s) atualizada(s) em {arquivos} arquivo(s)"))
}

/// As escritas que levam os wikilinks de `de` para `para` nas `notas` (rel,
/// bytes) do vault. O próprio arquivo, nos dois endereços, fica de fora: quem
/// o move já reescreveu a linha 2. Nota que não é UTF-8 não se reescreve.
/// Devolve as escritas, os links trocados e as notas tocadas.
pub fn referencias(
    notas: &[(String, Vec<u8>)],
    de: &str,
    para: &str,
) -> (Vec<Efeito>, usize, usize) {
    let old = Endereco::de_rel(de);
    let new = Endereco::de_rel(para);
    let mut out = Vec::new();
    let (mut refs, mut arquivos) = (0, 0);
    if old.full == new.full {
        return (out, refs, arquivos);
    }
    for (rel, bytes) in notas {
        if rel == de || rel == para {
            continue;
        }
        let Ok(texto) = std::str::from_utf8(bytes) else {
            continue;
        };
        if let Some((novo, n)) =
            links::rewrite_text(texto, |t, a| links::rewrite_link(t, a, &old, &new))
        {
            refs += n;
            arquivos += 1;
            out.push(Efeito::Escrever {
                rel: rel.clone(),
                antes: Some(bytes.clone()),
                depois: novo.into_bytes(),
            });
        }
    }
    (out, refs, arquivos)
}

/// Reserializa o bloco para o endereço novo: o link da linha 2 acompanha a
/// pasta. `id` é o do caminho, usado quando o bloco não tem link (ou sempre,
/// com `forca_id`). `ajusta` mexe no modelo antes (a fase, ao arquivar). Sem
/// bloco, as linhas ficam como estão.
fn relink_block(
    vocab: &Vocab,
    lines: &[String],
    t: &TaskRef,
    project: &str,
    archived: Option<&str>,
    forca_id: bool,
    ajusta: impl Fn(&mut model::Model),
) -> Vec<String> {
    let Some((s, e, bloco)) = md::first_block(lines) else {
        return lines.to_vec();
    };
    let mut m = model::parse_block(vocab, &bloco);
    if forca_id || m.id.is_empty() {
        m.id = t.id.clone();
    }
    m.project = Some(project.to_string());
    m.archived = archived.map(str::to_string);
    ajusta(&mut m);
    md::splice(lines, s, e, &model::serialize_block(vocab, &m))
}

/// O plano comum: a pasta de destino, o `rename`, a gravação do conteúdo novo
/// no endereço novo e as referências.
fn plano_de(
    visao: &dyn Visao,
    rel: &str,
    novo_rel: &str,
    bytes: &[u8],
    doc: &Doc,
    linhas: Vec<String>,
    notas: &[(String, Vec<u8>)],
) -> Deslocamento {
    let mut plano = Plano::new();
    if let Some((dir, _)) = novo_rel.rsplit_once('/')
        && !visao.dir_existe(dir)
    {
        plano.push(Efeito::CriarDir {
            rel: dir.to_string(),
        });
    }
    plano.push(Efeito::Mover {
        de: rel.to_string(),
        para: novo_rel.to_string(),
        antes: bytes.to_vec(),
    });
    plano.push(Efeito::Escrever {
        rel: novo_rel.to_string(),
        antes: Some(bytes.to_vec()),
        depois: doc.com_linhas(linhas).to_bytes(),
    });
    let (escritas, refs, arquivos) = referencias(notas, rel, novo_rel);
    plano.extend(escritas);
    Deslocamento {
        novo_rel: novo_rel.to_string(),
        plano,
        refs,
        arquivos,
    }
}

fn doc_de(bytes: &[u8], abs: &str) -> Result<Doc, String> {
    Doc::parse(bytes).map_err(|_| format!("não é um arquivo de texto UTF-8: {abs}"))
}

/// O endereço de uma task, ou a recusa "não é um arquivo de task".
pub fn task_de(rel: &str, abs: &str) -> Result<TaskRef, String> {
    path::split_task_path(rel).ok_or_else(|| format!("não é um arquivo de task: {abs}"))
}

/// O que `archive`/`unarchive` precisam do pedido e do vault.
pub struct Contexto<'a> {
    pub vocab: &'a Vocab,
    pub visao: &'a dyn Visao,
    /// A raiz do vault, para as mensagens com caminho absoluto.
    pub raiz: &'a str,
    /// As notas elegíveis a conter referências (rel, bytes).
    pub notas: &'a [(String, Vec<u8>)],
    pub hoje: Date,
}

/// `archive` (`dest = Some(tipo)`) e `unarchive` (`dest = None`). `Ok(None)`
/// quando a task já está onde deveria: nada a fazer, e a CLI imprime o
/// caminho de sempre. A fase acompanha a pasta (`status_after_move`) e o
/// `## Histórico` ganha a linha datada.
pub fn arquivar(
    cx: &Contexto,
    rel: &str,
    abs: &str,
    bytes: &[u8],
    dest: Option<&str>,
) -> Result<Option<Deslocamento>, String> {
    let t = task_de(rel, abs)?;
    if let Some(d) = dest
        && !config::is_archived_type(d)
    {
        return Err(format!("tipo de arquivamento inválido: {d}"));
    }
    if dest == t.archived.as_deref() {
        return Ok(None);
    }
    let novo_rel = path::task_rel(&t.project, &t.id, dest);
    if cx.visao.arquivo_existe(&novo_rel) {
        return Err(format!("já existe um arquivo em {}/{novo_rel}", cx.raiz));
    }
    let doc = doc_de(bytes, abs)?;
    let mut linhas = relink_block(cx.vocab, &doc.lines, &t, &t.project, dest, false, |m| {
        m.status_num = task::status_after_move(cx.vocab, m.status_num, dest);
    });
    md::append_history(&mut linhas, dest, cx.hoje);
    Ok(Some(plano_de(
        cx.visao, rel, &novo_rel, bytes, &doc, linhas, cx.notas,
    )))
}

/// `rename`: o id novo passa pela validação de uma task nova (kebab, livre
/// inclusive arquivado no projeto), o bloco sai com o link novo, o `id:` do
/// frontmatter acompanha se existe, e o `## Histórico` guarda o id antigo.
/// Links curtos `[[id]]` também se reescrevem (o basename mudou).
pub fn renomear(
    cx: &Contexto,
    rel: &str,
    abs: &str,
    bytes: &[u8],
    novo_id: &str,
) -> Result<Deslocamento, String> {
    let t = task_de(rel, abs)?;
    task::validate_new(&t.project, novo_id)?;
    for tipo in std::iter::once(None).chain(config::ARCHIVED_TYPES.iter().map(|t| Some(*t))) {
        let ocupado = path::task_rel(&t.project, novo_id, tipo);
        if cx.visao.arquivo_existe(&ocupado) {
            return Err(format!(
                "já existe uma task com esse id: {}/{ocupado}",
                cx.raiz
            ));
        }
    }
    let doc = doc_de(bytes, abs)?;
    let Some((s, e)) = md::first_block_range(&doc.lines) else {
        return Err(format!("task sem callout (rode `freitask doctor`): {abs}"));
    };
    let mut m = model::parse_block(cx.vocab, &doc.lines[s..=e]);
    m.id = novo_id.to_string();
    m.project = Some(t.project.clone());
    m.archived = t.archived.clone();
    let mut linhas = md::splice(&doc.lines, s, e, &model::serialize_block(cx.vocab, &m));
    md::update_frontmatter_key(&mut linhas, "id", novo_id);
    md::append_history_renamed(&mut linhas, &t.id, novo_id, cx.hoje);
    let novo_rel = path::task_rel(&t.project, novo_id, t.archived.as_deref());
    Ok(plano_de(
        cx.visao, rel, &novo_rel, bytes, &doc, linhas, cx.notas,
    ))
}

/// `move`: leva a task para outro projeto, no mesmo `archived/<tipo>/` se
/// estava arquivada; fase e id não mudam, e por isso os links curtos seguem
/// valendo. `projetos` são os projetos vivos (pastas com `tasks/`).
pub fn mover(
    cx: &Contexto,
    rel: &str,
    abs: &str,
    bytes: &[u8],
    projeto: &str,
    projetos: &[String],
) -> Result<Deslocamento, String> {
    let t = task_de(rel, abs)?;
    let projeto = trim(projeto);
    task::validate_project(projeto)?;
    // Antes do "não existe": um encerrado também não existe em projects/.
    if cx.visao.dir_existe(&path::project_dir(projeto, true).0) {
        return Err(format!(
            "o projeto \"{projeto}\" está encerrado; reabra-o com `freitask reabrir {projeto}`"
        ));
    }
    if !projetos.iter().any(|p| p == projeto) {
        return Err(format!(
            "o projeto \"{projeto}\" não existe; crie-o antes com `freitask new-project {projeto}`"
        ));
    }
    if projeto == t.project {
        return Err(format!("a task já está no projeto {projeto}"));
    }
    for tipo in std::iter::once(None).chain(config::ARCHIVED_TYPES.iter().map(|t| Some(*t))) {
        let ocupado = path::task_rel(projeto, &t.id, tipo);
        if cx.visao.arquivo_existe(&ocupado) {
            return Err(format!(
                "já existe uma task com esse id em {projeto}: {}/{ocupado}",
                cx.raiz
            ));
        }
    }
    let doc = doc_de(bytes, abs)?;
    let archived = t.archived.as_deref();
    // O id do ARQUIVO: com a linha 2 dessincronizada, o do bloco gravaria no
    // projeto novo um link para uma task que não existe.
    let mut linhas = relink_block(cx.vocab, &doc.lines, &t, projeto, archived, true, |_| {});
    md::append_history_moved(&mut linhas, &t.project, projeto, cx.hoje);
    let novo_rel = path::task_rel(projeto, &t.id, archived);
    Ok(plano_de(
        cx.visao, rel, &novo_rel, bytes, &doc, linhas, cx.notas,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(Default)]
    struct Disco {
        arquivos: BTreeSet<String>,
        dirs: BTreeSet<String>,
    }

    impl Visao for Disco {
        fn arquivo_existe(&self, rel: &str) -> bool {
            self.arquivos.contains(rel)
        }
        fn dir_existe(&self, rel: &str) -> bool {
            self.dirs.contains(rel)
        }
    }

    fn disco(arquivos: &[&str], dirs: &[&str]) -> Disco {
        Disco {
            arquivos: arquivos.iter().map(|s| s.to_string()).collect(),
            dirs: dirs.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn hoje() -> Date {
        Date::parse("2026-11-08").unwrap()
    }

    const X: &str = "projects/p/tasks/x.md";
    const X_MD: &str = "> [!check] Xis\n> [[projects/p/tasks/x|x]]\n";

    fn notas(n: &[(&str, &str)]) -> Vec<(String, Vec<u8>)> {
        n.iter()
            .map(|(r, b)| (r.to_string(), b.as_bytes().to_vec()))
            .collect()
    }

    fn escrita<'a>(d: &'a Deslocamento, rel: &str) -> &'a str {
        d.plano
            .iter()
            .find_map(|e| match e {
                Efeito::Escrever { rel: r, depois, .. } if r == rel => {
                    Some(std::str::from_utf8(depois).unwrap())
                }
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn arquivar_done_move_regrava_e_reescreve_as_referencias() {
        let vo = Vocab::default();
        let d = disco(&[X], &["projects/p/tasks"]);
        let ns = notas(&[
            (X, X_MD),
            ("notas/a.md", "ver [[projects/p/tasks/x|x]] e [[x]]\n"),
            ("notas/b.md", "nada\n"),
        ]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &ns,
            hoje: hoje(),
        };
        let r = arquivar(&cx, X, "/v/x", X_MD.as_bytes(), Some("done"))
            .unwrap()
            .unwrap();
        let novo = "projects/p/tasks/archived/done/x.md";
        assert_eq!(r.novo_rel, novo);
        assert_eq!((r.refs, r.arquivos), (1, 1), "o curto não muda no archive");
        assert_eq!(
            r.plano[0],
            Efeito::CriarDir {
                rel: "projects/p/tasks/archived/done".into()
            }
        );
        assert_eq!(
            r.plano[1],
            Efeito::Mover {
                de: X.into(),
                para: novo.into(),
                antes: X_MD.as_bytes().to_vec()
            }
        );
        assert_eq!(
            escrita(&r, novo),
            "> [!done] Xis\n> [[projects/p/tasks/archived/done/x|x]]\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/done` (feito)\n"
        );
        assert_eq!(
            escrita(&r, "notas/a.md"),
            "ver [[projects/p/tasks/archived/done/x|x]] e [[x]]\n"
        );
        assert_eq!(r.plano.len(), 4);
        assert_eq!(
            aviso(r.refs, r.arquivos).unwrap(),
            "1 referência(s) atualizada(s) em 1 arquivo(s)"
        );
        assert_eq!(aviso(0, 0), None);
    }

    #[test]
    fn arquivar_no_mesmo_lugar_e_nada_e_as_recusas() {
        let vo = Vocab::default();
        let d = disco(&[X, "projects/p/tasks/archived/dropped/x.md"], &[]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &[],
            hoje: hoje(),
        };
        assert_eq!(arquivar(&cx, X, "/a", b"", None).unwrap(), None);
        assert_eq!(
            arquivar(&cx, X, "/a", b"", Some("misc")).unwrap_err(),
            "tipo de arquivamento inválido: misc"
        );
        assert_eq!(
            arquivar(&cx, X, "/a", b"", Some("dropped")).unwrap_err(),
            "já existe um arquivo em /v/projects/p/tasks/archived/dropped/x.md"
        );
        assert_eq!(
            arquivar(&cx, "notas/n.md", "/a", b"", Some("done")).unwrap_err(),
            "não é um arquivo de task: /a"
        );
    }

    #[test]
    fn desarquivar_done_volta_como_check_e_sem_bloco_so_ganha_historico() {
        let vo = Vocab::default();
        let d = disco(&[], &["projects/p/tasks"]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &[],
            hoje: hoje(),
        };
        let velha = "projects/p/tasks/archived/done/x.md";
        let r = arquivar(
            &cx,
            velha,
            "/a",
            b"> [!done] X\n> [[projects/p/tasks/archived/done/x|x]]\n",
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r.plano.len(), 2, "pasta existe: só mover e gravar");
        assert_eq!(
            escrita(&r, X),
            "> [!check] X\n> [[projects/p/tasks/x|x]]\n\n## Histórico\n\n- 2026-11-08 — desarquivada, de volta ao board\n"
        );
        let r = arquivar(&cx, velha, "/a", b"texto\n", None)
            .unwrap()
            .unwrap();
        assert_eq!(
            escrita(&r, X),
            "texto\n\n## Histórico\n\n- 2026-11-08 — desarquivada, de volta ao board\n"
        );
    }

    #[test]
    fn renomear_troca_link_id_e_historico_e_os_curtos() {
        let vo = Vocab::default();
        let d = disco(&[X], &["projects/p/tasks"]);
        let ns = notas(&[(
            "n.md",
            "[[x]] [[x|x]] [[x|Outro]] [[projects/p/tasks/x#S|x]] [[x-2]]\n",
        )]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &ns,
            hoje: hoje(),
        };
        let antes = "---\nid: x\nfoo: 1\n---\n> [!todo] X\n> [[x]]\n";
        let r = renomear(&cx, X, "/a", antes.as_bytes(), "y").unwrap();
        assert_eq!(r.novo_rel, "projects/p/tasks/y.md");
        assert_eq!(
            escrita(&r, "projects/p/tasks/y.md"),
            "---\nid: y\nfoo: 1\n---\n> [!todo] X\n> [[projects/p/tasks/y|y]]\n\n## Histórico\n\n- 2026-11-08 — renomeada de `x` para `y`\n"
        );
        assert_eq!(
            escrita(&r, "n.md"),
            "[[y]] [[y|y]] [[y|Outro]] [[projects/p/tasks/y#S|y]] [[x-2]]\n"
        );
        assert_eq!((r.refs, r.arquivos), (4, 1));
    }

    #[test]
    fn renomear_recusas() {
        let vo = Vocab::default();
        let d = disco(&[X, "projects/p/tasks/archived/failed/y.md"], &[]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &[],
            hoje: hoje(),
        };
        assert!(
            renomear(&cx, X, "/a", b"", "Y Y")
                .unwrap_err()
                .contains("kebab")
        );
        assert_eq!(
            renomear(&cx, X, "/a", b"", "y").unwrap_err(),
            "já existe uma task com esse id: /v/projects/p/tasks/archived/failed/y.md"
        );
        assert_eq!(
            renomear(&cx, X, "/a", b"sem bloco\n", "z").unwrap_err(),
            "task sem callout (rode `freitask doctor`): /a"
        );
        assert_eq!(
            renomear(&cx, X, "/a", &[0xFF], "z").unwrap_err(),
            "não é um arquivo de texto UTF-8: /a"
        );
    }

    #[test]
    fn mover_leva_ao_mesmo_archived_e_forca_o_id_do_caminho() {
        let vo = Vocab::default();
        let d = disco(&[], &["projects/q/tasks"]);
        let projetos = vec!["p".to_string(), "q".to_string()];
        let ns = notas(&[("n.md", "[[projects/p/tasks/archived/dropped/x|x]] [[x]]\n")]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &ns,
            hoje: hoje(),
        };
        let rel = "projects/p/tasks/archived/dropped/x.md";
        let r = mover(
            &cx,
            rel,
            "/a",
            b"> [!todo] X\n> [[outro]]\n",
            " q ",
            &projetos,
        )
        .unwrap();
        assert_eq!(r.novo_rel, "projects/q/tasks/archived/dropped/x.md");
        assert_eq!(
            r.plano[0],
            Efeito::CriarDir {
                rel: "projects/q/tasks/archived/dropped".into()
            }
        );
        assert_eq!(
            escrita(&r, "projects/q/tasks/archived/dropped/x.md"),
            "> [!todo] X\n> [[projects/q/tasks/archived/dropped/x|x]]\n\n## Histórico\n\n- 2026-11-08 — movida de `p` para `q`\n"
        );
        assert_eq!(
            escrita(&r, "n.md"),
            "[[projects/q/tasks/archived/dropped/x|x]] [[x]]\n"
        );
    }

    #[test]
    fn mover_recusas_na_ordem_de_hoje() {
        let vo = Vocab::default();
        let d = disco(
            &["projects/q/tasks/archived/done/x.md"],
            &["projects/_encerrados/e"],
        );
        let projetos = vec!["p".to_string(), "q".to_string()];
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &[],
            hoje: hoje(),
        };
        let m = |p: &str| mover(&cx, X, "/a", b"", p, &projetos).unwrap_err();
        assert!(m("..").contains("reservado"));
        assert_eq!(
            m("e"),
            "o projeto \"e\" está encerrado; reabra-o com `freitask reabrir e`"
        );
        assert_eq!(
            m("z"),
            "o projeto \"z\" não existe; crie-o antes com `freitask new-project z`"
        );
        assert_eq!(m("p"), "a task já está no projeto p");
        assert_eq!(
            m("q"),
            "já existe uma task com esse id em q: /v/projects/q/tasks/archived/done/x.md"
        );
        assert_eq!(
            mover(&cx, "n.md", "/a", b"", "q", &projetos).unwrap_err(),
            "não é um arquivo de task: /a"
        );
    }

    #[test]
    fn referencias_pulam_o_proprio_arquivo_o_que_nao_e_utf8_e_o_mesmo_endereco() {
        let ns = vec![
            (X.to_string(), b"[[projects/p/tasks/x]]".to_vec()),
            ("b.md".to_string(), vec![0xFF, b'[']),
            ("c.md".to_string(), b"[[projects/p/tasks/x]]".to_vec()),
        ];
        let (e, refs, arq) = referencias(&ns, X, "projects/q/tasks/x.md");
        assert_eq!((refs, arq), (1, 1));
        assert_eq!(e.len(), 1);
        let (e, refs, arq) = referencias(&ns, X, X);
        assert!(e.is_empty());
        assert_eq!((refs, arq), (0, 0));
    }
}
