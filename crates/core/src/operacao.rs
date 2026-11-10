//! As operações de escrita da CLI como funções puras: recebem o que foi lido
//! (bytes, e uma `Visao` do que existe no disco) e devolvem um plano ou uma
//! recusa. Nenhuma toca o disco; a ordem das validações e o texto de cada
//! recusa são os do `cli.lua`/`task.lua` de hoje, menos as divergências
//! aprovadas (D2).
//!
//! As mensagens citam caminhos absolutos, como hoje: quem chama passa a raiz
//! do vault (`raiz`) ou o caminho já absoluto (`abs`).

use crate::md;
use crate::meta;
use crate::model::{self, Model};
use crate::path::{self, TaskRef};
use crate::plano::{Efeito, Plano};
use crate::status::Vocab;
use crate::task;
use crate::texto::Doc;
use crate::{config, trim};

/// O que uma operação precisa saber do disco além dos bytes que lê.
pub trait Visao {
    /// Um arquivo (não pasta) existe em `rel`.
    fn arquivo_existe(&self, rel: &str) -> bool;
    /// Uma pasta existe em `rel`.
    fn dir_existe(&self, rel: &str) -> bool;
}

/// Por que uma operação não acontece. `Uso` é saída 2 (o pedido está errado);
/// `Motor` é saída 1 (o pedido é válido e o vault o recusa).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recusa {
    Uso(String),
    Motor(String),
}

impl Recusa {
    pub fn codigo(&self) -> u8 {
        match self {
            Recusa::Uso(_) => 2,
            Recusa::Motor(_) => 1,
        }
    }

    pub fn mensagem(&self) -> &str {
        match self {
            Recusa::Uso(m) | Recusa::Motor(m) => m,
        }
    }
}

/// Recusa o nome de um projeto encerrado: ele volta por `reabrir`, com os
/// links junto, não por `new`, `new-project` nem `move`.
fn checa_nao_encerrado(visao: &dyn Visao, project: &str) -> Result<(), String> {
    let p = trim(project);
    if visao.dir_existe(&path::project_dir(p, true).0) {
        return Err(format!(
            "o projeto \"{p}\" está encerrado; reabra-o com `freitask reabrir {p}`"
        ));
    }
    Ok(())
}

/// `freitask new`: valida, recusa id ocupado (inclusive arquivado) e devolve
/// o caminho da task nova e o plano (a pasta do projeto, se falta, e o
/// arquivo). O projeto é usado como veio para o caminho, como hoje; só a
/// validação o apara.
pub fn nova_task(
    vocab: &Vocab,
    visao: &dyn Visao,
    raiz: &str,
    project: &str,
    id: &str,
    title: &str,
    desc: &str,
) -> Result<(String, Plano), String> {
    let id = trim(id);
    task::validate_new(project, id)?;
    checa_nao_encerrado(visao, project)?;
    let ativa = path::task_rel(project, id, None);
    let ocupados = std::iter::once(ativa.clone()).chain(
        config::ARCHIVED_TYPES
            .iter()
            .map(|t| path::task_rel(project, id, Some(t))),
    );
    for rel in ocupados {
        if visao.arquivo_existe(&rel) {
            return Err(format!("já existe uma task com esse id: {raiz}/{rel}"));
        }
    }
    let mut plano = Plano::new();
    let dir = path::vault_dir(project, None);
    if !visao.dir_existe(&dir) {
        plano.push(Efeito::CriarDir { rel: dir });
    }
    // Toda task nasce em `todo` (1): a fase é escolha de quem trabalha.
    let m = Model {
        status_num: 1,
        title: title.to_string(),
        id: id.to_string(),
        desc: desc.to_string(),
        project: Some(project.to_string()),
        ..Model::default()
    };
    plano.push(Efeito::Escrever {
        rel: ativa.clone(),
        antes: None,
        depois: Doc::novo(task::template(vocab, &m)).to_bytes(),
    });
    Ok((ativa, plano))
}

/// `freitask new-project`: o nome passa pelo `kebab`; devolve o id do projeto
/// e o plano que cria `projects/<id>/tasks/`. Já existir é erro (quase sempre
/// um typo); encerrado também.
pub fn novo_projeto(visao: &dyn Visao, nome: &str) -> Result<(String, Plano), String> {
    let aparado = trim(nome);
    // D2: `.` e `..` são recusados como reservados, como no `new`, antes de o
    // `kebab` os esvaziar.
    if aparado == "." || aparado == ".." {
        return Err(format!(
            "\"{aparado}\" é um diretório reservado, não um projeto"
        ));
    }
    let id = path::kebab(aparado);
    if id.is_empty() {
        return Err(format!("nome de projeto inválido: {nome}"));
    }
    if config::is_reserved(&id) {
        return Err(format!("\"{id}\" é um diretório reservado, não um projeto"));
    }
    checa_nao_encerrado(visao, &id)?;
    let dir = path::vault_dir(&id, None);
    if visao.dir_existe(&dir) {
        return Err(format!("projeto já existe: {id}"));
    }
    Ok((id, vec![Efeito::CriarDir { rel: dir }]))
}

/// O pedido de `freitask set`, como chegou da linha de comando.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PedidoSet {
    pub status: Option<String>,
    pub title: Option<String>,
    pub desc: Option<String>,
    pub volta_em: Option<String>,
    /// A lista como veio (`"a,b"`; `""` limpa). Só o `escrita::set` a valida,
    /// porque depende do índice do vault.
    pub bloqueada_por: Option<String>,
    pub prazo: Option<String>,
}

impl PedidoSet {
    pub fn vazio(&self) -> bool {
        self.status.is_none()
            && self.title.is_none()
            && self.desc.is_none()
            && self.volta_em.is_none()
            && self.bloqueada_por.is_none()
            && self.prazo.is_none()
    }

    fn mexe_no_bloco(&self) -> bool {
        self.status.is_some() || self.title.is_some() || self.desc.is_some()
    }
}

/// O pedido de `set` já validado: o que gravar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetValidado {
    /// `(número, callout)` da fase nova.
    status: Option<(u32, String)>,
    title: Option<String>,
    desc: Option<String>,
    /// `Some(None)` limpa a `volta-em`; `Some(Some(d))` grava `d`.
    volta_em: Option<Option<String>>,
    /// A lista já validada (vazia remove a chave). `validar_set` a deixa
    /// `None`: validar a lista pede o índice, e quem o tem a preenche.
    bloqueada_por: Option<Vec<String>>,
    /// Idem para o `prazo`.
    prazo: Option<Option<String>>,
    bloco: bool,
}

impl SetValidado {
    /// Junta a lista de `--bloqueada-por` já validada por
    /// `deps::validar_bloqueada_por`.
    pub fn com_bloqueada_por(mut self, ids: Vec<String>) -> SetValidado {
        self.bloqueada_por = Some(ids);
        self
    }
}

/// Valida o pedido de `set` inteiro antes de qualquer escrita, na ordem de
/// hoje: fase, título, `volta-em`, `prazo`. `archived` é o tipo de arquivamento da task
/// (do caminho).
pub fn validar_set(
    vocab: &Vocab,
    pedido: &PedidoSet,
    archived: Option<&str>,
) -> Result<SetValidado, Recusa> {
    let status = match &pedido.status {
        Some(s) => match task::check_status_change(vocab, s, archived) {
            Ok(num) => Some((num, s.clone())),
            // Fase que não existe é uso (2); fase que existe mas foi recusada
            // é o motor negando (1).
            Err((msg, None)) => return Err(Recusa::Uso(msg)),
            Err((msg, Some(_))) => return Err(Recusa::Motor(msg)),
        },
        None => None,
    };
    let title = match &pedido.title {
        Some(t) => {
            let t = trim(t);
            if t.is_empty() {
                return Err(Recusa::Uso("o título não pode ficar vazio".into()));
            }
            Some(t.to_string())
        }
        None => None,
    };
    let desc = pedido.desc.as_deref().map(|d| trim(d).to_string());
    let volta_em = match &pedido.volta_em {
        Some(v) => Some(task::check_volta_em(v).map_err(Recusa::Uso)?),
        None => None,
    };
    let prazo = match &pedido.prazo {
        Some(p) => Some(task::check_prazo(p).map_err(Recusa::Uso)?),
        None => None,
    };
    Ok(SetValidado {
        status,
        title,
        desc,
        volta_em,
        bloqueada_por: None,
        prazo,
        bloco: pedido.mexe_no_bloco(),
    })
}

/// Aplica um `set` validado aos bytes da task e devolve os bytes novos (que
/// podem ser os mesmos: aí nada se grava). O id é o do CAMINHO: `set` nunca
/// renomeia, e um link de linha 2 dessincronizado sai consertado. O bloco só é
/// reserializado quando fase, título ou descrição foram pedidos; `--volta-em`,
/// `--prazo` e `--bloqueada-por` mexem só no frontmatter.
pub fn editar_set(
    vocab: &Vocab,
    t: &TaskRef,
    abs: &str,
    bytes: &[u8],
    v: &SetValidado,
) -> Result<Vec<u8>, Recusa> {
    let doc = Doc::parse(bytes)
        .map_err(|_| Recusa::Motor(format!("não é um arquivo de texto UTF-8: {abs}")))?;
    let mut lines = doc.lines.clone();
    if v.bloco {
        let Some((s, e)) = md::first_block_range(&lines) else {
            return Err(Recusa::Motor(format!(
                "task sem callout (rode `freitask doctor`): {abs}"
            )));
        };
        let mut nm = model::parse_block(vocab, &lines[s..=e]);
        nm.id = t.id.clone();
        if let Some((num, callout)) = &v.status {
            nm.status_num = *num;
            nm.raw_callout = callout.clone();
        }
        if let Some(title) = &v.title {
            nm.title = title.clone();
        }
        if let Some(desc) = &v.desc {
            nm.desc = desc.clone();
        }
        nm.project = Some(t.project.clone());
        nm.archived = t.archived.clone();
        let bloco = model::serialize_block(vocab, &nm);
        lines = md::splice(&lines, s, e, &bloco);
    }
    if let Some(volta) = &v.volta_em {
        meta::write_volta_em(&mut lines, volta.as_deref());
    }
    if let Some(ids) = &v.bloqueada_por {
        meta::write_bloqueada_por(&mut lines, ids);
    }
    if let Some(prazo) = &v.prazo {
        meta::write_prazo(&mut lines, prazo.as_deref());
    }
    Ok(doc.com_linhas(lines).to_bytes())
}

/// `freitask delete`: só arquivo de task se apaga. As referências a ela ficam
/// penduradas de propósito; o `doctor` as aponta.
pub fn apagar_task(rel: &str, abs: &str, bytes: Vec<u8>) -> Result<Plano, String> {
    if path::split_task_path(rel).is_none() {
        return Err(format!("não é um arquivo de task: {abs}"));
    }
    Ok(vec![Efeito::Apagar {
        rel: rel.to_string(),
        antes: bytes,
    }])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Um disco de mentira: os arquivos e as pastas que "existem".
    #[derive(Default)]
    struct Disco {
        arquivos: BTreeSet<String>,
        dirs: BTreeSet<String>,
    }

    impl Disco {
        fn com(arquivos: &[&str], dirs: &[&str]) -> Disco {
            Disco {
                arquivos: arquivos.iter().map(|s| s.to_string()).collect(),
                dirs: dirs.iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl Visao for Disco {
        fn arquivo_existe(&self, rel: &str) -> bool {
            self.arquivos.contains(rel)
        }
        fn dir_existe(&self, rel: &str) -> bool {
            self.dirs.contains(rel)
        }
    }

    const RAIZ: &str = "/h/ObsidianVault";

    fn tref(rel: &str) -> TaskRef {
        path::split_task_path(rel).unwrap()
    }

    fn set(bytes: &str, rel: &str, p: PedidoSet) -> Result<String, Recusa> {
        let vo = Vocab::default();
        let t = tref(rel);
        let v = validar_set(&vo, &p, t.archived.as_deref())?;
        editar_set(&vo, &t, "/abs", bytes.as_bytes(), &v).map(|b| String::from_utf8(b).unwrap())
    }

    #[test]
    fn nova_task_cria_a_pasta_e_o_arquivo() {
        let (rel, plano) = nova_task(
            &Vocab::default(),
            &Disco::default(),
            RAIZ,
            "p",
            "x",
            "Xis",
            "estado",
        )
        .unwrap();
        assert_eq!(rel, "projects/p/tasks/x.md");
        assert_eq!(
            plano,
            vec![
                Efeito::CriarDir {
                    rel: "projects/p/tasks".into()
                },
                Efeito::Escrever {
                    rel: "projects/p/tasks/x.md".into(),
                    antes: None,
                    depois: b"> [!todo] Xis\n> [[projects/p/tasks/x|x]]\n> _estado_\n\n".to_vec(),
                }
            ]
        );
    }

    #[test]
    fn nova_task_em_projeto_existente_nao_cria_pasta_e_apara_o_id() {
        let d = Disco::com(&[], &["projects/p/tasks"]);
        let (rel, plano) = nova_task(&Vocab::default(), &d, RAIZ, "p", " x ", "Xis", "").unwrap();
        assert_eq!(rel, "projects/p/tasks/x.md");
        assert_eq!(plano.len(), 1);
        assert!(matches!(&plano[0], Efeito::Escrever { depois, .. }
            if depois == b"> [!todo] Xis\n> [[projects/p/tasks/x|x]]\n\n"));
    }

    #[test]
    fn nova_task_recusa_id_ocupado_ativo_ou_arquivado() {
        for ocupado in [
            "projects/p/tasks/x.md",
            "projects/p/tasks/archived/done/x.md",
            "projects/p/tasks/archived/dropped/x.md",
            "projects/p/tasks/archived/failed/x.md",
        ] {
            let d = Disco::com(&[ocupado], &["projects/p/tasks"]);
            assert_eq!(
                nova_task(&Vocab::default(), &d, RAIZ, "p", "x", "X", "").unwrap_err(),
                format!("já existe uma task com esse id: {RAIZ}/{ocupado}")
            );
        }
        let d = Disco::com(&["projects/q/tasks/x.md"], &["projects/p/tasks"]);
        assert!(nova_task(&Vocab::default(), &d, RAIZ, "p", "x", "X", "").is_ok());
    }

    #[test]
    fn nova_task_recusa_encerrado_reservado_e_id_ruim() {
        let d = Disco::com(&[], &["projects/_encerrados/p"]);
        assert_eq!(
            nova_task(&Vocab::default(), &d, RAIZ, "p", "x", "X", "").unwrap_err(),
            "o projeto \"p\" está encerrado; reabra-o com `freitask reabrir p`"
        );
        let vazio = Disco::default();
        assert!(
            nova_task(&Vocab::default(), &vazio, RAIZ, "..", "x", "X", "")
                .unwrap_err()
                .contains("reservado")
        );
        assert!(
            nova_task(&Vocab::default(), &vazio, RAIZ, "p", "Id Ruim", "X", "")
                .unwrap_err()
                .contains("kebab")
        );
    }

    #[test]
    fn novo_projeto_passa_pelo_kebab_e_cria_tasks() {
        assert_eq!(
            novo_projeto(&Disco::default(), "Meu Projeto").unwrap(),
            (
                "meu-projeto".to_string(),
                vec![Efeito::CriarDir {
                    rel: "projects/meu-projeto/tasks".into()
                }]
            )
        );
        // `projects/x` sem `tasks/` não é projeto ainda: cria o `tasks/`.
        assert!(novo_projeto(&Disco::com(&[], &["projects/x"]), "x").is_ok());
    }

    #[test]
    fn novo_projeto_recusas() {
        let d = Disco::com(&[], &["projects/a/tasks", "projects/_encerrados/e"]);
        assert_eq!(novo_projeto(&d, "A").unwrap_err(), "projeto já existe: a");
        assert_eq!(
            novo_projeto(&d, "e").unwrap_err(),
            "o projeto \"e\" está encerrado; reabra-o com `freitask reabrir e`"
        );
        assert_eq!(
            novo_projeto(&d, "Daily").unwrap_err(),
            "\"daily\" é um diretório reservado, não um projeto"
        );
        assert_eq!(
            novo_projeto(&d, "!!!").unwrap_err(),
            "nome de projeto inválido: !!!"
        );
        assert_eq!(
            novo_projeto(&d, " .. ").unwrap_err(),
            "\"..\" é um diretório reservado, não um projeto"
        );
        assert_eq!(
            novo_projeto(&d, ".").unwrap_err(),
            "\".\" é um diretório reservado, não um projeto"
        );
    }

    #[test]
    fn validar_set_na_ordem_de_hoje() {
        let vo = Vocab::default();
        let p = |s: Option<&str>, t: Option<&str>, v: Option<&str>| PedidoSet {
            status: s.map(Into::into),
            title: t.map(Into::into),
            volta_em: v.map(Into::into),
            ..Default::default()
        };
        assert_eq!(
            validar_set(&vo, &p(Some("bogus"), Some(""), None), None).unwrap_err(),
            Recusa::Uso("fase desconhecida: bogus".into())
        );
        assert_eq!(
            validar_set(&vo, &p(Some("done"), None, None), None)
                .unwrap_err()
                .codigo(),
            1
        );
        assert_eq!(
            validar_set(&vo, &p(Some("check"), None, None), Some("done"))
                .unwrap_err()
                .codigo(),
            1
        );
        assert_eq!(
            validar_set(&vo, &p(Some("todo"), Some("  "), None), None).unwrap_err(),
            Recusa::Uso("o título não pode ficar vazio".into())
        );
        let e = validar_set(&vo, &p(Some("todo"), None, Some("amanha")), None).unwrap_err();
        assert_eq!(e.codigo(), 2);
        assert!(e.mensagem().contains("amanha"));
        assert!(validar_set(&vo, &p(Some("todo"), None, Some("")), None).is_ok());
    }

    #[test]
    fn validar_set_valida_o_prazo_depois_da_volta_em() {
        let vo = Vocab::default();
        let p = |v: Option<&str>, pr: Option<&str>| PedidoSet {
            volta_em: v.map(Into::into),
            prazo: pr.map(Into::into),
            ..Default::default()
        };
        let e = validar_set(&vo, &p(None, Some("amanha")), None).unwrap_err();
        assert_eq!(
            e,
            Recusa::Uso(
                "data inválida: \"amanha\" (use AAAA-MM-DD, de um dia que existe; \"\" limpa)"
                    .into()
            )
        );
        // As duas inválidas: a `volta-em` é acusada primeiro.
        let e = validar_set(&vo, &p(Some("a"), Some("b")), None).unwrap_err();
        assert!(e.mensagem().contains("\"a\""), "{}", e.mensagem());
        // Prazo inválido barra o resto do pedido, mesmo com a fase válida.
        let pedido = PedidoSet {
            status: Some("todo".into()),
            prazo: Some("2026-02-30".into()),
            ..Default::default()
        };
        assert_eq!(validar_set(&vo, &pedido, None).unwrap_err().codigo(), 2);
        assert!(validar_set(&vo, &p(None, Some("2028-02-29")), None).is_ok());
        assert!(validar_set(&vo, &p(None, Some("")), None).is_ok());
    }

    #[test]
    fn pedido_vazio_e_so_sem_nenhuma_flag() {
        assert!(PedidoSet::default().vazio());
        for p in [
            PedidoSet {
                status: Some("x".into()),
                ..Default::default()
            },
            PedidoSet {
                title: Some("x".into()),
                ..Default::default()
            },
            PedidoSet {
                desc: Some(String::new()),
                ..Default::default()
            },
            PedidoSet {
                volta_em: Some(String::new()),
                ..Default::default()
            },
            PedidoSet {
                bloqueada_por: Some(String::new()),
                ..Default::default()
            },
            PedidoSet {
                prazo: Some(String::new()),
                ..Default::default()
            },
        ] {
            assert!(!p.vazio(), "{p:?}");
        }
    }

    #[test]
    fn editar_set_troca_fase_titulo_e_desc_e_conserta_o_link() {
        let antes = "---\nfoo: bar\n---\n\n> [!todo] Velho\n> [[outro]]\n> _d_\n> nota\n\n## Histórico\n\n- 2026-01-01 — x\n";
        let r = set(
            antes,
            "projects/p/tasks/x.md",
            PedidoSet {
                status: Some("example".into()),
                title: Some("  Novo ".into()),
                desc: Some("  e  ".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            r,
            "---\nfoo: bar\n---\n\n> [!example] Novo\n> [[projects/p/tasks/x|x]]\n> _e_\n> nota\n\n## Histórico\n\n- 2026-01-01 — x\n"
        );
    }

    #[test]
    fn editar_set_em_arquivada_mantem_o_link_de_archived() {
        let r = set(
            "> [!todo] X\n> [[projects/p/tasks/archived/dropped/x|x]]\n",
            "projects/p/tasks/archived/dropped/x.md",
            PedidoSet {
                title: Some("Y".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            r,
            "> [!todo] Y\n> [[projects/p/tasks/archived/dropped/x|x]]\n"
        );
    }

    #[test]
    fn editar_set_volta_em_so_mexe_no_frontmatter() {
        // Bloco em formato fora do canônico: `--volta-em` não o reserializa.
        let antes = "> [!todo] **X**\n> [[x]]\n";
        let r = set(
            antes,
            "projects/p/tasks/x.md",
            PedidoSet {
                volta_em: Some("2026-12-01".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r, format!("---\nvolta-em: 2026-12-01\n---\n\n{antes}"));
        let limpo = set(
            &r,
            "projects/p/tasks/x.md",
            PedidoSet {
                volta_em: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(limpo, antes);
    }

    #[test]
    fn editar_set_bloqueada_por_so_mexe_no_frontmatter_e_vazio_limpa() {
        let vo = Vocab::default();
        let t = tref("projects/p/tasks/x.md");
        // Bloco fora do canônico: `--bloqueada-por` não o reserializa.
        let antes = "> [!todo] **X**\n> [[x]]\n";
        let pedido = PedidoSet {
            bloqueada_por: Some("a,b".into()),
            ..Default::default()
        };
        let v = validar_set(&vo, &pedido, None).unwrap();
        assert_eq!(v.bloqueada_por, None, "o validar_set não valida a lista");
        let v = v.com_bloqueada_por(vec!["a".into(), "b".into()]);
        let r = editar_set(&vo, &t, "/abs", antes.as_bytes(), &v).unwrap();
        let r = String::from_utf8(r).unwrap();
        assert_eq!(r, format!("---\nbloqueada-por: [a, b]\n---\n\n{antes}"));
        let limpa = validar_set(&vo, &pedido, None)
            .unwrap()
            .com_bloqueada_por(Vec::new());
        let limpo = editar_set(&vo, &t, "/abs", r.as_bytes(), &limpa).unwrap();
        assert_eq!(String::from_utf8(limpo).unwrap(), antes);
    }

    #[test]
    fn editar_set_prazo_so_mexe_no_frontmatter_e_nao_toca_a_volta_em() {
        // Bloco fora do canônico, sem `## Histórico`: nada além do `prazo:`.
        let antes = "---\nvolta-em: 2026-12-01\n---\n\n> [!todo] **X**\n> [[x]]\n";
        let r = set(
            antes,
            "projects/p/tasks/x.md",
            PedidoSet {
                prazo: Some("2026-12-24".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            r,
            "---\nvolta-em: 2026-12-01\nprazo: 2026-12-24\n---\n\n> [!todo] **X**\n> [[x]]\n"
        );
        let troca = set(
            &r,
            "projects/p/tasks/x.md",
            PedidoSet {
                prazo: Some("2026-12-25".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(troca.contains("prazo: 2026-12-25\n") && !troca.contains("12-24"));
        let limpo = set(
            &r,
            "projects/p/tasks/x.md",
            PedidoSet {
                prazo: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            limpo,
            "---\nvolta-em: 2026-12-01\n---\n\n> [!todo] **X**\n> [[x]]\n"
        );
        // Sem o campo, limpar não muda um byte.
        let igual = set(
            "> [!todo] X\r\n> [[x]]\r\n",
            "projects/p/tasks/x.md",
            PedidoSet {
                prazo: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(igual, "> [!todo] X\r\n> [[x]]\r\n");
    }

    #[test]
    fn editar_set_sem_callout_recusa_so_quando_mexe_no_bloco() {
        let antes = "texto\n";
        let e = set(
            antes,
            "projects/p/tasks/x.md",
            PedidoSet {
                status: Some("todo".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            e,
            Recusa::Motor("task sem callout (rode `freitask doctor`): /abs".into())
        );
        assert!(
            set(
                antes,
                "projects/p/tasks/x.md",
                PedidoSet {
                    volta_em: Some("2026-12-01".into()),
                    ..Default::default()
                },
            )
            .is_ok()
        );
    }

    #[test]
    fn editar_set_sem_mudanca_devolve_os_mesmos_bytes() {
        let antes = "> [!todo] X\r\n> [[projects/p/tasks/x|x]]\r\n";
        let r = set(
            antes,
            "projects/p/tasks/x.md",
            PedidoSet {
                status: Some("todo".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r, antes);
    }

    #[test]
    fn editar_set_recusa_o_que_nao_e_utf8() {
        let vo = Vocab::default();
        let t = tref("projects/p/tasks/x.md");
        let v = validar_set(
            &vo,
            &PedidoSet {
                title: Some("a".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        assert_eq!(
            editar_set(&vo, &t, "/abs", &[0xFF, 0xFE], &v).unwrap_err(),
            Recusa::Motor("não é um arquivo de texto UTF-8: /abs".into())
        );
    }

    #[test]
    fn apagar_so_arquivo_de_task() {
        assert_eq!(
            apagar_task("projects/p/tasks/x.md", "/a", b"b".to_vec()).unwrap(),
            vec![Efeito::Apagar {
                rel: "projects/p/tasks/x.md".into(),
                antes: b"b".to_vec()
            }]
        );
        assert_eq!(
            apagar_task("notas/x.md", "/a", vec![]).unwrap_err(),
            "não é um arquivo de task: /a"
        );
        assert!(
            apagar_task(
                "projects/p/tasks/x.sync-conflict-20260101-000000-A.md",
                "/a",
                vec![]
            )
            .is_err()
        );
    }

    #[test]
    fn recusa_diz_o_codigo_e_a_mensagem() {
        assert_eq!(Recusa::Uso("u".into()).codigo(), 2);
        assert_eq!(Recusa::Motor("m".into()).codigo(), 1);
        assert_eq!(Recusa::Uso("u".into()).mensagem(), "u");
        assert_eq!(Recusa::Motor("m".into()).mensagem(), "m");
    }
}
