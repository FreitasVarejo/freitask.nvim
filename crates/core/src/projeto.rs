//! Encerrar e reabrir um projeto: a pasta inteira (MOC, spec, decisões,
//! anexos e as tasks arquivadas) muda de endereço num `rename` só, entre
//! `projects/<p>/` e `projects/_encerrados/<p>/`, e os wikilinks
//! path-qualificados que apontavam para dentro dela passam a apontar para o
//! endereço novo (`links::rewrite_prefix`). A reescrita roda DEPOIS do
//! `rename` e alcança as notas da própria pasta: é assim que a linha 2 de
//! cada task acompanha. Nenhum dos dois escreve no `## Histórico`: quem mudou
//! de lugar foi o projeto, não a task.

use crate::deslocar::Deslocamento;
use crate::doctor;
use crate::index::Indice;
use crate::links;
use crate::operacao::Visao;
use crate::path;
use crate::plano::{Efeito, Plano};
use crate::status::Vocab;
use crate::task;
use crate::trim;

/// As escritas que trocam o prefixo `de` por `para` nos alvos dos links das
/// `notas` (rel, bytes, lidas ANTES do `rename`). A nota que estava dentro da
/// pasta movida é gravada no endereço novo, com CAS contra os mesmos bytes
/// (o `rename` não os muda). Nota que não é UTF-8 não se reescreve. Devolve
/// as escritas, os links trocados e as notas tocadas.
pub fn referencias_da_pasta(
    notas: &[(String, Vec<u8>)],
    de: &str,
    para: &str,
) -> (Vec<Efeito>, usize, usize) {
    let (velho, novo) = (format!("{de}/"), format!("{para}/"));
    let mut out = Vec::new();
    let (mut refs, mut arquivos) = (0, 0);
    for (rel, bytes) in notas {
        let Ok(texto) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Some((depois, n)) = links::rewrite_text(texto, |t, _| {
            links::rewrite_prefix(t, &velho, &novo).map(|x| (x, None))
        }) else {
            continue;
        };
        refs += n;
        arquivos += 1;
        let destino = match rel.strip_prefix(&velho) {
            Some(resto) => format!("{novo}{resto}"),
            None => rel.clone(),
        };
        out.push(Efeito::Escrever {
            rel: destino,
            antes: Some(bytes.clone()),
            depois: depois.into_bytes(),
        });
    }
    (out, refs, arquivos)
}

/// O plano comum: a pasta-mãe do destino (se falta), o `rename` da pasta e
/// as referências.
fn plano_de(visao: &dyn Visao, de: &str, para: &str, notas: &[(String, Vec<u8>)]) -> Deslocamento {
    let mut plano = Plano::new();
    if let Some((mae, _)) = para.rsplit_once('/')
        && !visao.dir_existe(mae)
    {
        plano.push(Efeito::CriarDir {
            rel: mae.to_string(),
        });
    }
    plano.push(Efeito::MoverPasta {
        de: de.to_string(),
        para: para.to_string(),
    });
    let (escritas, refs, arquivos) = referencias_da_pasta(notas, de, para);
    plano.extend(escritas);
    Deslocamento {
        novo_rel: para.to_string(),
        plano,
        refs,
        arquivos,
    }
}

fn existe(visao: &dyn Visao, rel: &str) -> bool {
    visao.arquivo_existe(rel) || visao.dir_existe(rel)
}

/// O que `encerrar` precisa do vault.
pub struct Contexto<'a> {
    pub vocab: &'a Vocab,
    pub visao: &'a dyn Visao,
    /// A raiz do vault, para as mensagens com caminho absoluto.
    pub raiz: &'a str,
    /// As notas elegíveis a conter referências (rel, bytes).
    pub notas: &'a [(String, Vec<u8>)],
}

/// `encerrar`. Recusa, na ordem de hoje: nome inválido ou reservado, projeto
/// já encerrado, projeto que não existe (pasta sem `tasks/` não é projeto),
/// task ativa em qualquer fase (inclusive Algum dia, adiada, status 0 e
/// arquivo sem callout, listadas na ordem do painel) e, por último, D3:
/// `*.sync-conflict-*` em qualquer lugar da pasta ou `.md` fora do padrão em
/// `tasks/`, listados por caminho. `arquivos` são os caminhos (relativos ao
/// vault) de todos os arquivos sob a pasta do projeto.
pub fn encerrar(
    cx: &Contexto,
    indice: &Indice,
    projeto: &str,
    arquivos: &[String],
) -> Result<Deslocamento, String> {
    let projeto = trim(projeto);
    task::validate_project(projeto)?;
    let (de, _) = path::project_dir(projeto, false);
    let (para, _) = path::project_dir(projeto, true);
    if existe(cx.visao, &para) {
        return Err(format!(
            "o projeto \"{projeto}\" já está encerrado: {}/{para} existe",
            cx.raiz
        ));
    }
    if !indice.projetos.iter().any(|p| p == projeto) {
        return Err(format!(
            "o projeto \"{projeto}\" não existe: não há {}/{de}/tasks/",
            cx.raiz
        ));
    }
    let ativas = indice.entries_for(projeto);
    if !ativas.is_empty() {
        let linhas: Vec<String> = ativas
            .iter()
            .map(|e| format!("  {} ({})", e.task_id, cx.vocab.meta(e.status_num).title))
            .collect();
        return Err(format!(
            "o projeto \"{projeto}\" tem task ativa; arquive-a antes de encerrar (`freitask archive <id> <tipo>`):\n{}",
            linhas.join("\n")
        ));
    }
    let mut fora: Vec<String> = arquivos
        .iter()
        .filter(|rel| rel.starts_with(&format!("{de}/")))
        .filter_map(|rel| {
            if doctor::eh_conflito(rel) {
                Some(format!("  {rel} (sync-conflict)"))
            } else if doctor::fora_do_padrao(rel) {
                Some(format!("  {rel} (fora-do-padrao)"))
            } else {
                None
            }
        })
        .collect();
    if !fora.is_empty() {
        fora.sort();
        return Err(format!(
            "o projeto \"{projeto}\" tem arquivo fora do padrão; resolva antes de encerrar (`freitask doctor`):\n{}",
            fora.join("\n")
        ));
    }
    Ok(plano_de(cx.visao, &de, &para, cx.notas))
}

/// `reabrir`: o caminho inverso. Recusa nome inválido ou reservado, projeto
/// que não está encerrado e nome que já existe em `projects/` (reabrir por
/// cima juntaria duas pastas).
pub fn reabrir(cx: &Contexto, projeto: &str) -> Result<Deslocamento, String> {
    let projeto = trim(projeto);
    task::validate_project(projeto)?;
    let (de, _) = path::project_dir(projeto, true);
    let (para, _) = path::project_dir(projeto, false);
    if !cx.visao.dir_existe(&de) {
        return Err(format!(
            "o projeto \"{projeto}\" não está encerrado: não há {}/{de}",
            cx.raiz
        ));
    }
    if existe(cx.visao, &para) {
        return Err(format!(
            "já existe {}/{para}: reabrir juntaria duas pastas num projeto só; decida antes o que fica",
            cx.raiz
        ));
    }
    Ok(plano_de(cx.visao, &de, &para, cx.notas))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::index;
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

    fn notas(n: &[(&str, &str)]) -> Vec<(String, Vec<u8>)> {
        n.iter()
            .map(|(r, b)| (r.to_string(), b.as_bytes().to_vec()))
            .collect()
    }

    fn indice(vocab: &Vocab, projetos: &[&str], tasks: &[(&str, &str)]) -> Indice {
        Indice::new(
            projetos.iter().map(|p| p.to_string()).collect(),
            tasks
                .iter()
                .filter_map(|(r, b)| index::scan_task(vocab, r, b.as_bytes()))
                .collect(),
        )
    }

    fn rels(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn referencias_trocam_o_prefixo_e_levam_a_nota_da_pasta_ao_endereco_novo() {
        let ns = notas(&[
            (
                "projects/a/a.md",
                "[[projects/a/spec]] e [[projects/a-b/x]] e [[projects/a]]\n",
            ),
            (
                "n.md",
                "| [[projects/a/tasks/x\\|x]] | `[[projects/a/y]]`\n",
            ),
            ("m.md", "nada [[curto]]\n"),
            ("bin.md", "\u{0}"),
        ]);
        let mut ns = ns;
        ns.push(("x.md".into(), vec![0xFF, b'[', b'[']));
        let (e, refs, arq) = referencias_da_pasta(&ns, "projects/a", "projects/_encerrados/a");
        assert_eq!((refs, arq), (2, 2));
        assert_eq!(
            e,
            vec![
                Efeito::Escrever {
                    rel: "projects/_encerrados/a/a.md".into(),
                    antes: Some(ns[0].1.clone()),
                    depois:
                        b"[[projects/_encerrados/a/spec]] e [[projects/a-b/x]] e [[projects/a]]\n"
                            .to_vec(),
                },
                Efeito::Escrever {
                    rel: "n.md".into(),
                    antes: Some(ns[1].1.clone()),
                    depois: b"| [[projects/_encerrados/a/tasks/x\\|x]] | `[[projects/a/y]]`\n"
                        .to_vec(),
                },
            ]
        );
    }

    #[test]
    fn encerrar_move_a_pasta_cria_a_mae_e_reescreve() {
        let vo = Vocab::default();
        let d = disco(&[], &["projects/a", "projects/a/tasks"]);
        let ns = notas(&[(
            "projects/a/tasks/archived/done/v.md",
            "> [!done] V\n> [[projects/a/tasks/archived/done/v|v]]\n",
        )]);
        let idx = indice(
            &vo,
            &["a"],
            &[("projects/a/tasks/archived/done/v.md", "> [!done] V\n")],
        );
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &ns,
        };
        let r = encerrar(
            &cx,
            &idx,
            " a ",
            &rels(&[
                "projects/a/tasks/archived/done/v.md",
                "projects/a/anexos/f.pdf",
            ]),
        )
        .unwrap();
        assert_eq!(r.novo_rel, "projects/_encerrados/a");
        assert_eq!((r.refs, r.arquivos), (1, 1));
        assert_eq!(
            r.plano[..2],
            [
                Efeito::CriarDir {
                    rel: "projects/_encerrados".into()
                },
                Efeito::MoverPasta {
                    de: "projects/a".into(),
                    para: "projects/_encerrados/a".into()
                }
            ]
        );
        assert_eq!(
            r.plano[2],
            Efeito::Escrever {
                rel: "projects/_encerrados/a/tasks/archived/done/v.md".into(),
                antes: Some(ns[0].1.clone()),
                depois: b"> [!done] V\n> [[projects/_encerrados/a/tasks/archived/done/v|v]]\n"
                    .to_vec(),
            }
        );
        assert_eq!(r.plano.len(), 3);
        let d = disco(&[], &["projects/_encerrados"]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &[],
        };
        let r = encerrar(&cx, &idx, "a", &[]).unwrap();
        assert_eq!(r.plano.len(), 1, "a mãe já existe: só o rename");
        assert_eq!((r.refs, r.arquivos), (0, 0));
    }

    #[test]
    fn encerrar_recusas_na_ordem() {
        let vo = Vocab::default();
        let d = disco(&["projects/_encerrados/f"], &["projects/_encerrados/e"]);
        let idx = indice(
            &vo,
            &["e", "f", "g", "h"],
            &[
                (
                    "projects/g/tasks/z.md",
                    "---\nprioridade: alta\n---\n> [!warning] Z\n",
                ),
                ("projects/g/tasks/b.md", "> [!bogus] B\n"),
                ("projects/g/tasks/a.md", "> [!todo] A\n"),
                ("projects/g/tasks/archived/done/x.md", "> [!done] X\n"),
            ],
        );
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &[],
        };
        let enc = |p: &str, arqs: &[&str]| encerrar(&cx, &idx, p, &rels(arqs)).unwrap_err();
        assert_eq!(
            enc("daily", &[]),
            "\"daily\" é um diretório reservado, não um projeto"
        );
        assert_eq!(
            enc("e", &[]),
            "o projeto \"e\" já está encerrado: /v/projects/_encerrados/e existe"
        );
        assert_eq!(
            enc("f", &[]),
            "o projeto \"f\" já está encerrado: /v/projects/_encerrados/f existe"
        );
        assert_eq!(
            enc("nada", &[]),
            "o projeto \"nada\" não existe: não há /v/projects/nada/tasks/"
        );
        assert_eq!(
            enc("g", &["projects/g/x.sync-conflict-1.md"]),
            "o projeto \"g\" tem task ativa; arquive-a antes de encerrar (`freitask archive <id> <tipo>`):\n  z (Bloqueada)\n  b (Sem status)\n  a (Não iniciada)"
        );
        assert_eq!(
            enc(
                "h",
                &[
                    "projects/h/tasks/s/x.md",
                    "projects/h/h.sync-conflict-1.md",
                    "projects/h/anexos/f.sync-conflict-1.pdf",
                    "projects/h/tasks/s/ok.txt",
                    "projects/h/tasks/archived/done/ok.md",
                    "projects/hh/tasks/s/x.md",
                ]
            ),
            "o projeto \"h\" tem arquivo fora do padrão; resolva antes de encerrar (`freitask doctor`):\n  projects/h/anexos/f.sync-conflict-1.pdf (sync-conflict)\n  projects/h/h.sync-conflict-1.md (sync-conflict)\n  projects/h/tasks/s/x.md (fora-do-padrao)"
        );
    }

    #[test]
    fn reabrir_move_de_volta_e_recusa() {
        let vo = Vocab::default();
        let d = disco(
            &["projects/f"],
            &[
                "projects",
                "projects/_encerrados/a",
                "projects/_encerrados/e",
                "projects/e",
                "projects/_encerrados/f",
            ],
        );
        let ns = notas(&[("n.md", "[[projects/_encerrados/a/a|a]]\n")]);
        let cx = Contexto {
            vocab: &vo,
            visao: &d,
            raiz: "/v",
            notas: &ns,
        };
        let r = reabrir(&cx, " a ").unwrap();
        assert_eq!(r.novo_rel, "projects/a");
        assert_eq!(
            r.plano,
            vec![
                Efeito::MoverPasta {
                    de: "projects/_encerrados/a".into(),
                    para: "projects/a".into()
                },
                Efeito::Escrever {
                    rel: "n.md".into(),
                    antes: Some(ns[0].1.clone()),
                    depois: b"[[projects/a/a|a]]\n".to_vec(),
                }
            ]
        );
        assert_eq!((r.refs, r.arquivos), (1, 1));
        assert_eq!(
            reabrir(&cx, "x/y").unwrap_err(),
            "projeto não pode conter '/': x/y"
        );
        assert_eq!(
            reabrir(&cx, "b").unwrap_err(),
            "o projeto \"b\" não está encerrado: não há /v/projects/_encerrados/b"
        );
        let ja = |p: &str| {
            format!(
                "já existe /v/projects/{p}: reabrir juntaria duas pastas num projeto só; decida antes o que fica"
            )
        };
        assert_eq!(reabrir(&cx, "e").unwrap_err(), ja("e"));
        assert_eq!(reabrir(&cx, "f").unwrap_err(), ja("f"));
    }
}
