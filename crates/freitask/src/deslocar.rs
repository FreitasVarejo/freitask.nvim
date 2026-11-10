//! Os comandos que mudam uma task de endereço (`archive`, `unarchive`,
//! `rename`, `move`) e os que mudam um projeto inteiro (`encerrar`,
//! `reabrir`). Mesmo fluxo dos outros que escrevem (lock, leitura, plano do
//! `core`, execução com CAS, painel), mais o aviso de quantas referências o
//! plano reescreveu, em stderr, como hoje. Devolvem um `Resultado`
//! (`saida.rs`), como `escrita.rs`.

use freitask_core::deslocar::{self, Contexto as ContextoCore, Deslocamento};
use freitask_core::path;
use freitask_core::projeto;

use crate::escrita::{ler, resolver};
use crate::saida::{Contexto, Corrida, Falha, Resultado};

/// O que se pede ao deslocamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pedido {
    /// `archive`: o tipo pode faltar (saída 2, depois de achar a task).
    Archive(Option<String>),
    Unarchive,
    Rename(String),
    Move(String),
}

impl Pedido {
    /// `archive`/`unarchive` recusam em duas linhas, como hoje: o motivo (que
    /// o motor dava por `vim.notify`) e o fecho.
    fn recusa(&self, c: &mut Corrida, msg: &str) -> Falha {
        match self {
            Pedido::Archive(_) | Pedido::Unarchive => {
                c.aviso(msg);
                c.falha("operação não concluída (veja as mensagens acima)", 1)
            }
            Pedido::Rename(_) | Pedido::Move(_) => c.falha(msg, 1),
        }
    }
}

pub fn deslocar(cx: &Contexto, referencia: &str, pedido: Pedido) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    let vault = cx.vault;
    let hoje = cx.hoje;
    let vocab = c.vocab(vault);
    let indice = vault.indice(&vocab);
    let alvo = resolver(vault, &indice, referencia).map_err(|e| c.falha(e, 1))?;
    if let Pedido::Archive(None) = pedido {
        return Err(c.falha("falta o tipo: done|dropped|failed", 2));
    }
    if let Pedido::Rename(novo) = &pedido
        && let Some(t) = alvo.rel.as_deref().and_then(path::split_task_path)
        && &t.id == novo
    {
        return Err(c.falha(format!("a task já se chama {novo}"), 1));
    }
    let Some(rel) = alvo.rel.as_deref() else {
        let msg = format!("não é um arquivo de task: {}", alvo.abs);
        return Err(pedido.recusa(&mut c, &msg));
    };
    let bytes = ler(&mut c, vault, rel, &alvo.abs)?;
    let notas = vault.notas();
    let raiz = vault.raiz().to_string_lossy().into_owned();
    let core = ContextoCore {
        vocab: &vocab,
        visao: vault,
        raiz: &raiz,
        notas: &notas,
        hoje,
    };
    let r = match &pedido {
        Pedido::Archive(tipo) => deslocar::arquivar(&core, rel, &alvo.abs, &bytes, tipo.as_deref()),
        Pedido::Unarchive => deslocar::arquivar(&core, rel, &alvo.abs, &bytes, None),
        Pedido::Rename(novo) => deslocar::renomear(&core, rel, &alvo.abs, &bytes, novo).map(Some),
        Pedido::Move(projeto) => {
            deslocar::mover(&core, rel, &alvo.abs, &bytes, projeto, &vault.projetos()).map(Some)
        }
    };
    match r {
        Err(e) => Err(pedido.recusa(&mut c, &e)),
        // Já está onde deveria: nada a gravar, e o caminho de sempre.
        Ok(None) => c.fecha(cx, &alvo.abs),
        Ok(Some(d)) => executa(&mut c, cx, &d),
    }
}

fn executa(c: &mut Corrida, cx: &Contexto, d: &Deslocamento) -> Resultado {
    cx.vault.aplicar(&d.plano).map_err(|e| c.erro_escrita(&e))?;
    if let Some(a) = deslocar::aviso(d.refs, d.arquivos) {
        c.aviso(a);
    }
    c.fecha(cx, &cx.vault.caminho_str(&d.novo_rel))
}

/// `encerrar` (`fechar = true`) e `reabrir`. Como hoje, a raiz é garantida
/// antes de validar; as recusas saem 1.
pub fn projeto(cx: &Contexto, nome: &str, fechar: bool) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    let vault = cx.vault;
    vault.garantir_raiz().map_err(|e| c.erro_escrita(&e))?;
    let vocab = c.vocab(vault);
    let notas = vault.notas();
    let raiz = vault.raiz().to_string_lossy().into_owned();
    let core = projeto::Contexto {
        vocab: &vocab,
        visao: vault,
        raiz: &raiz,
        notas: &notas,
    };
    let r = if fechar {
        let indice = vault.indice(&vocab);
        // Nome inválido (`..`, `a/b`) não é listado: o `encerrar` o recusa
        // antes de olhar os arquivos, e listar seria sair do vault.
        let nome_t = freitask_core::trim(nome);
        let arquivos = match freitask_core::task::validate_project(nome_t) {
            Ok(()) => vault.arquivos_sob(&path::project_dir(nome_t, false).0),
            Err(_) => Vec::new(),
        };
        projeto::encerrar(&core, &indice, nome, &arquivos)
    } else {
        projeto::reabrir(&core, nome)
    };
    match r {
        Err(e) => Err(c.falha(e, 1)),
        Ok(d) => executa(&mut c, cx, &d),
    }
}
