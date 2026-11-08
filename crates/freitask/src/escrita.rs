//! Os comandos que escrevem: `new`, `new-project`, `set`, `delete`. Cada um
//! toma o lock, lê o vault, pede ao `core` o plano (ou a recusa), executa o
//! plano com CAS e regenera o painel, como o `cli.lua` de hoje. Nenhuma regra
//! mora aqui: a ordem das validações e as mensagens são do `core`.

use std::path::Path;
use std::process::ExitCode;

use freitask_core::date::Date;
use freitask_core::index::Indice;
use freitask_core::operacao::{self, PedidoSet};
use freitask_core::path;
use freitask_vault::Vault;

use crate::cli::{die, falha, rebuild, say, travar};

/// O vault do HOME.
pub(crate) fn vault() -> Result<Vault, ExitCode> {
    Vault::do_home().map_err(|e| die(&e, 2))
}

/// O vocabulário, com o aviso de `status.json` quebrado em stderr.
pub(crate) fn vocab(vault: &Vault) -> freitask_core::status::Vocab {
    let (vocab, aviso) = vault.vocab();
    if let Some(a) = aviso {
        eprintln!("{a}");
    }
    vocab
}

/// Os bytes da task, ou a saída 1 com o erro de I/O (permissão, por
/// exemplo), que não é o mesmo que a task não existir.
pub(crate) fn ler(vault: &Vault, rel: &str, abs: &str) -> Result<Vec<u8>, ExitCode> {
    std::fs::read(vault.caminho(rel)).map_err(|e| die(&format!("não consegui ler {abs}: {e}"), 1))
}

fn raiz(vault: &Vault) -> String {
    vault.raiz().to_string_lossy().into_owned()
}

/// O alvo de um comando sobre uma task: o caminho relativo ao vault, quando o
/// arquivo está dentro dele, e o absoluto que o stdout e as mensagens citam.
pub(crate) struct Alvo {
    pub rel: Option<String>,
    pub abs: String,
}

/// Resolve `<id|caminho>` como o `find_task` de hoje: um arquivo que existe
/// vale como está, mesmo que não seja task (quem recusa é a operação, com
/// "não é um arquivo de task"); senão o id, único entre ativas e arquivadas.
pub(crate) fn resolver(vault: &Vault, indice: &Indice, referencia: &str) -> Result<Alvo, String> {
    let p = Path::new(referencia);
    if p.is_file() {
        if let Some(rel) = vault.rel_de(referencia) {
            return Ok(Alvo {
                abs: vault.caminho_str(&rel),
                rel: Some(rel),
            });
        }
        let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
        return Ok(Alvo {
            rel: None,
            abs: abs.to_string_lossy().into_owned(),
        });
    }
    let rel = vault.find_task(indice, referencia)?;
    Ok(Alvo {
        abs: vault.caminho_str(&rel),
        rel: Some(rel),
    })
}

/// Regenera o painel depois de uma escrita e imprime o que o comando devolve.
pub(crate) fn fecha(vault: &Vault, hoje: Date, saida: &str) -> ExitCode {
    if let Err(e) = rebuild(vault, hoje) {
        return falha(&e);
    }
    say(saida);
    ExitCode::SUCCESS
}

pub fn new(projeto: &str, id: &str, titulo: &str, desc: &str, hoje: Date) -> ExitCode {
    let _trava = match travar() {
        Ok(t) => t,
        Err(c) => return c,
    };
    let vault = match vault() {
        Ok(v) => v,
        Err(c) => return c,
    };
    // Como hoje: a raiz nasce antes de validar o pedido, e o vocabulário é
    // lido depois dela.
    if let Err(e) = vault.garantir_raiz() {
        return falha(&e);
    }
    let vocab = vocab(&vault);
    let (rel, plano) =
        match operacao::nova_task(&vocab, &vault, &raiz(&vault), projeto, id, titulo, desc) {
            Ok(r) => r,
            Err(e) => return die(&e, 1),
        };
    if let Err(e) = vault.aplicar(&plano) {
        return falha(&e);
    }
    fecha(&vault, hoje, &vault.caminho_str(&rel))
}

pub fn new_project(nome: &str) -> ExitCode {
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
    let (id, plano) = match operacao::novo_projeto(&vault, nome) {
        Ok(r) => r,
        Err(e) => return die(&e, 1),
    };
    if let Err(e) = vault.aplicar(&plano) {
        return falha(&e);
    }
    // Sem `rebuild`, como hoje: o projeto vazio só entra no painel na
    // próxima regeneração (o diferencial compara o `CURRENT.md`).
    say(&id);
    ExitCode::SUCCESS
}

pub fn set(referencia: &str, pedido: &PedidoSet, hoje: Date) -> ExitCode {
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
    let t = alvo.rel.as_deref().and_then(path::split_task_path);
    let validado = match operacao::validar_set(
        &vocab,
        pedido,
        t.as_ref().and_then(|t| t.archived.as_deref()),
    ) {
        Ok(v) => v,
        Err(r) => return die(r.mensagem(), r.codigo()),
    };
    let (Some(t), Some(rel)) = (t, alvo.rel.as_deref()) else {
        return die(&format!("não é um arquivo de task: {}", alvo.abs), 1);
    };
    let antes = match ler(&vault, rel, &alvo.abs) {
        Ok(b) => b,
        Err(c) => return c,
    };
    let depois = match operacao::editar_set(&vocab, &t, &alvo.abs, &antes, &validado) {
        Ok(b) => b,
        Err(r) => return die(r.mensagem(), r.codigo()),
    };
    if let Err(e) = vault.gravar(rel, Some(&antes), &depois) {
        return falha(&e);
    }
    fecha(&vault, hoje, &alvo.abs)
}

pub fn delete(referencia: &str, hoje: Date) -> ExitCode {
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
    let Some(rel) = alvo.rel.as_deref() else {
        return die(&format!("não é um arquivo de task: {}", alvo.abs), 1);
    };
    let antes = match ler(&vault, rel, &alvo.abs) {
        Ok(b) => b,
        Err(c) => return c,
    };
    let plano = match operacao::apagar_task(rel, &alvo.abs, antes) {
        Ok(p) => p,
        Err(e) => return die(&e, 1),
    };
    if let Err(e) = vault.aplicar(&plano) {
        return falha(&e);
    }
    fecha(&vault, hoje, &alvo.abs)
}
