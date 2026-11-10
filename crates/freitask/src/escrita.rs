//! Os comandos que escrevem: `new`, `new-project`, `set`, `delete`, `rebuild`.
//! Cada um toma o lock, lê o vault, pede ao `core` o plano (ou a recusa),
//! executa o plano com CAS e regenera o painel, como o `cli.lua` de hoje.
//! Nenhuma regra mora aqui: a ordem das validações e as mensagens são do
//! `core`.
//!
//! Cada função recebe o vault e o lock num `Contexto` e devolve um
//! `Resultado` (`saida.rs`): a CLI o imprime e a TUI o mostra na barra, pelo
//! mesmo caminho (decisão f7.2).

use std::path::Path;

use freitask_core::deps;
use freitask_core::index::Indice;
use freitask_core::operacao::{self, PedidoSet};
use freitask_core::path;
use freitask_vault::Vault;

use crate::saida::{Contexto, Corrida, Falha, Resultado};

/// Os bytes da task, ou a saída 1 com o erro de I/O (permissão, por
/// exemplo), que não é o mesmo que a task não existir.
pub(crate) fn ler(c: &mut Corrida, vault: &Vault, rel: &str, abs: &str) -> Result<Vec<u8>, Falha> {
    std::fs::read(vault.caminho(rel))
        .map_err(|e| c.falha(format!("não consegui ler {abs}: {e}"), 1))
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

/// `kebab`: o id que o texto rende. É também a sugestão de id da TUI.
pub fn kebab(texto: &str) -> Resultado {
    let mut c = Corrida::default();
    let id = path::kebab(texto);
    if id.is_empty() {
        return Err(c.falha(format!("o texto não rende um id: {texto}"), 1));
    }
    c.ok(id)
}

/// `rebuild`: regenera o `CURRENT.md` e devolve o caminho dele.
pub fn rebuild(cx: &Contexto) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    c.regenerar(cx.vault, cx.hoje)
        .map_err(|e| c.erro_escrita(&e))?;
    c.ok(cx.vault.caminho_str("CURRENT.md"))
}

/// `new`: cria a task, regenera o painel e devolve o caminho dela; com `json`,
/// devolve no lugar do caminho `{"id":…,"path":…,"project":…}` (spec §2.2).
pub fn new(
    cx: &Contexto,
    projeto: &str,
    id: &str,
    titulo: &str,
    desc: &str,
    json: bool,
) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    let vault = cx.vault;
    // Como hoje: a raiz nasce antes de validar o pedido, e o vocabulário é
    // lido depois dela.
    vault.garantir_raiz().map_err(|e| c.erro_escrita(&e))?;
    let vocab = c.vocab(vault);
    let (rel, plano) = operacao::nova_task(&vocab, vault, &raiz(vault), projeto, id, titulo, desc)
        .map_err(|e| c.falha(e, 1))?;
    // A saída se monta antes de gravar: se o caminho não se deixasse separar,
    // nada teria sido escrito.
    let abs = vault.caminho_str(&rel);
    let saida = if json {
        linha_json(&rel, &abs)
            .ok_or_else(|| c.falha(format!("caminho de task inesperado: {rel}"), 1))?
    } else {
        abs
    };
    vault.aplicar(&plano).map_err(|e| c.erro_escrita(&e))?;
    c.fecha(cx, &saida)
}

/// A linha de `new --json`. `project` e `id` saem do caminho que o plano
/// grava, não dos argumentos: o `nova_task` apara o id, e é o que foi gravado
/// que o chamador precisa. `path` é o absoluto que o `new` sem `--json` imprime.
fn linha_json(rel: &str, abs: &str) -> Option<String> {
    let t = path::split_task_path(rel)?;
    Some(serde_json::json!({ "id": t.id, "path": abs, "project": t.project }).to_string())
}

/// `new-project`: devolve o id do projeto.
pub fn new_project(cx: &Contexto, nome: &str) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    let vault = cx.vault;
    vault.garantir_raiz().map_err(|e| c.erro_escrita(&e))?;
    let (id, plano) = operacao::novo_projeto(vault, nome).map_err(|e| c.falha(e, 1))?;
    vault.aplicar(&plano).map_err(|e| c.erro_escrita(&e))?;
    // Sem `rebuild`, como hoje: o projeto vazio só entra no painel na
    // próxima regeneração (o diferencial compara o `CURRENT.md`).
    c.ok(id)
}

pub fn set(cx: &Contexto, referencia: &str, pedido: &PedidoSet) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    let vault = cx.vault;
    let vocab = c.vocab(vault);
    let indice = vault.indice(&vocab);
    let alvo = resolver(vault, &indice, referencia).map_err(|e| c.falha(e, 1))?;
    let t = alvo.rel.as_deref().and_then(path::split_task_path);
    let mut validado = operacao::validar_set(
        &vocab,
        pedido,
        t.as_ref().and_then(|t| t.archived.as_deref()),
    )
    .map_err(|r| c.falha(r.mensagem(), r.codigo()))?;
    let (Some(t), Some(rel)) = (t, alvo.rel.as_deref()) else {
        return Err(c.falha(format!("não é um arquivo de task: {}", alvo.abs), 1));
    };
    // A lista só se valida aqui: pede o índice (ids que existem, ciclo).
    if let Some(texto) = &pedido.bloqueada_por {
        let Some(entrada) = indice.por_rel(rel) else {
            return Err(c.falha(format!("não é um arquivo de texto UTF-8: {}", alvo.abs), 1));
        };
        let ids =
            deps::validar_bloqueada_por(&indice, entrada, texto).map_err(|e| c.falha(e, 1))?;
        validado = validado.com_bloqueada_por(ids);
    }
    let antes = ler(&mut c, vault, rel, &alvo.abs)?;
    let depois = operacao::editar_set(&vocab, &t, &alvo.abs, &antes, &validado)
        .map_err(|r| c.falha(r.mensagem(), r.codigo()))?;
    vault
        .gravar(rel, Some(&antes), &depois)
        .map_err(|e| c.erro_escrita(&e))?;
    c.fecha(cx, &alvo.abs)
}

pub fn delete(cx: &Contexto, referencia: &str) -> Resultado {
    let mut c = Corrida::default();
    let _trava = c.travar(cx)?;
    let vault = cx.vault;
    let vocab = c.vocab(vault);
    let indice = vault.indice(&vocab);
    let alvo = resolver(vault, &indice, referencia).map_err(|e| c.falha(e, 1))?;
    let Some(rel) = alvo.rel.as_deref() else {
        return Err(c.falha(format!("não é um arquivo de task: {}", alvo.abs), 1));
    };
    let antes = ler(&mut c, vault, rel, &alvo.abs)?;
    let plano = operacao::apagar_task(rel, &alvo.abs, antes).map_err(|e| c.falha(e, 1))?;
    vault.aplicar(&plano).map_err(|e| c.erro_escrita(&e))?;
    c.fecha(cx, &alvo.abs)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn linha_json_tem_as_tres_chaves_em_ordem_alfabetica() {
        assert_eq!(
            linha_json(
                "projects/beta/tasks/nova.md",
                "/home/u/ObsidianVault/projects/beta/tasks/nova.md"
            )
            .unwrap(),
            r#"{"id":"nova","path":"/home/u/ObsidianVault/projects/beta/tasks/nova.md","project":"beta"}"#
        );
    }

    #[test]
    fn linha_json_escapa_o_que_o_json_escapa() {
        // `project` não é kebab: só não pode ter `/`. Aspas e contrabarra
        // têm de sair escapadas, não quebrar a linha.
        let linha = linha_json(
            "projects/a\"b\\c/tasks/x.md",
            "/v/projects/a\"b\\c/tasks/x.md",
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&linha).unwrap();
        assert_eq!(v["project"], "a\"b\\c");
        assert_eq!(v["path"], "/v/projects/a\"b\\c/tasks/x.md");
        assert_eq!(v["id"], "x");
        assert!(!linha.contains('\n'));
    }

    #[test]
    fn linha_json_recusa_caminho_que_nao_e_de_task() {
        assert_eq!(linha_json("CURRENT.md", "/v/CURRENT.md"), None);
        assert_eq!(linha_json("projects/daily/tasks/x.md", "/v/x"), None);
    }

    #[test]
    fn kebab_devolve_o_id_ou_recusa_o_texto_que_nao_rende() {
        assert_eq!(kebab("Ávida nova").unwrap().stdout, "avida-nova");
        let f = kebab("---").unwrap_err();
        assert_eq!(f.stderr, ["o texto não rende um id: ---"]);
        assert_eq!(f.codigo, 1);
    }
}
