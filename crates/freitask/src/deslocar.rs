//! Os comandos que mudam uma task de endereço (`archive`, `unarchive`,
//! `rename`, `move`) e os que mudam um projeto inteiro (`encerrar`,
//! `reabrir`). Mesmo fluxo dos outros que escrevem (lock, leitura, plano do
//! `core`, execução com CAS, painel), mais o aviso de quantas referências o
//! plano reescreveu, em stderr, como hoje.

use std::process::ExitCode;

use freitask_core::date::Date;
use freitask_core::deslocar::{self, Contexto, Deslocamento};
use freitask_core::path;
use freitask_core::projeto;
use freitask_vault::Vault;

use crate::cli::{die, falha, travar};
use crate::escrita::{fecha, ler, resolver, vault, vocab};

/// O que se pede ao deslocamento.
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
    fn recusa(&self, msg: &str) -> ExitCode {
        match self {
            Pedido::Archive(_) | Pedido::Unarchive => {
                eprintln!("freitask: {msg}");
                die("operação não concluída (veja as mensagens acima)", 1)
            }
            Pedido::Rename(_) | Pedido::Move(_) => die(msg, 1),
        }
    }
}

pub fn deslocar(referencia: &str, pedido: Pedido, hoje: Date) -> ExitCode {
    let _trava = match travar() {
        Ok(t) => t,
        Err(c) => return c,
    };
    let vault = match vault() {
        Ok(v) => v,
        Err(c) => return c,
    };
    let vocab = vocab(&vault);
    let indice = vault.indice(&vocab);
    let alvo = match resolver(&vault, &indice, referencia) {
        Ok(a) => a,
        Err(e) => return die(&e, 1),
    };
    if let Pedido::Archive(None) = pedido {
        return die("falta o tipo: done|dropped|failed", 2);
    }
    if let Pedido::Rename(novo) = &pedido
        && let Some(t) = alvo.rel.as_deref().and_then(path::split_task_path)
        && &t.id == novo
    {
        return die(&format!("a task já se chama {novo}"), 1);
    }
    let Some(rel) = alvo.rel.as_deref() else {
        return pedido.recusa(&format!("não é um arquivo de task: {}", alvo.abs));
    };
    let bytes = match ler(&vault, rel, &alvo.abs) {
        Ok(b) => b,
        Err(c) => return c,
    };
    let notas = vault.notas();
    let raiz = vault.raiz().to_string_lossy().into_owned();
    let cx = Contexto {
        vocab: &vocab,
        visao: &vault,
        raiz: &raiz,
        notas: &notas,
        hoje,
    };
    let r = match &pedido {
        Pedido::Archive(tipo) => deslocar::arquivar(&cx, rel, &alvo.abs, &bytes, tipo.as_deref()),
        Pedido::Unarchive => deslocar::arquivar(&cx, rel, &alvo.abs, &bytes, None),
        Pedido::Rename(novo) => deslocar::renomear(&cx, rel, &alvo.abs, &bytes, novo).map(Some),
        Pedido::Move(projeto) => {
            deslocar::mover(&cx, rel, &alvo.abs, &bytes, projeto, &vault.projetos()).map(Some)
        }
    };
    match r {
        Err(e) => pedido.recusa(&e),
        // Já está onde deveria: nada a gravar, e o caminho de sempre.
        Ok(None) => fecha(&vault, hoje, &alvo.abs),
        Ok(Some(d)) => executa(&vault, hoje, &d),
    }
}

fn executa(vault: &Vault, hoje: Date, d: &Deslocamento) -> ExitCode {
    if let Err(e) = vault.aplicar(&d.plano) {
        return falha(&e);
    }
    if let Some(a) = deslocar::aviso(d.refs, d.arquivos) {
        eprintln!("freitask: {a}");
    }
    fecha(vault, hoje, &vault.caminho_str(&d.novo_rel))
}

/// `encerrar` (`fechar = true`) e `reabrir`. Como hoje, a raiz é garantida
/// antes de validar; as recusas saem 1.
pub fn projeto(nome: &str, fechar: bool, hoje: Date) -> ExitCode {
    let _trava = match travar() {
        Ok(t) => t,
        Err(c) => return c,
    };
    let vault = match vault() {
        Ok(v) => v,
        Err(c) => return c,
    };
    if let Err(e) = vault.garantir_raiz() {
        return falha(&e);
    }
    let vocab = vocab(&vault);
    let notas = vault.notas();
    let raiz = vault.raiz().to_string_lossy().into_owned();
    let cx = projeto::Contexto {
        vocab: &vocab,
        visao: &vault,
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
        projeto::encerrar(&cx, &indice, nome, &arquivos)
    } else {
        projeto::reabrir(&cx, nome)
    };
    match r {
        Err(e) => die(&e, 1),
        Ok(d) => executa(&vault, hoje, &d),
    }
}
