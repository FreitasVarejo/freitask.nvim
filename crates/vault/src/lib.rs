//! freitask-vault — a fronteira com o disco.
//!
//! Varre e lê o vault, grava com CAS (temporário, fsync, rename), toma o lock
//! entre dois `freitask` na mesma máquina e executa os planos do `core`. O
//! vault é `$HOME/ObsidianVault`, como hoje; nenhuma variável o troca (decisão
//! de `09c4fba`): testes trocam `HOME`.

use std::path::{Path, PathBuf};

use freitask_core::status::Vocab;

/// O vault em disco. Só guarda a raiz; cada operação lê o que precisa na hora.
#[derive(Debug, Clone)]
pub struct Vault {
    raiz: PathBuf,
}

impl Vault {
    /// `$HOME/ObsidianVault`. Erro sem `HOME`.
    pub fn do_home() -> Result<Vault, String> {
        let home = std::env::var_os("HOME").ok_or("HOME não está definido")?;
        Ok(Vault {
            raiz: PathBuf::from(home).join("ObsidianVault"),
        })
    }

    pub fn em(raiz: impl Into<PathBuf>) -> Vault {
        Vault { raiz: raiz.into() }
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }

    pub fn caminho(&self, rel: &str) -> PathBuf {
        self.raiz.join(rel)
    }

    /// O `.freitask/status.json`, se existe. Leitura pura: não cria nada.
    pub fn status_json(&self) -> Option<String> {
        std::fs::read_to_string(self.caminho(".freitask/status.json")).ok()
    }

    /// O vocabulário de fases, relido a cada execução: o arquivo do vault, ou
    /// o default embutido quando ele falta. Quando o arquivo existe e não
    /// parseia, cai no default e devolve o aviso que o Lua dava por
    /// `vim.notify` (`status.lua:62`).
    pub fn vocab(&self) -> (Vocab, Option<String>) {
        match self.status_json() {
            Some(texto) => match Vocab::from_json(&texto) {
                Ok(v) => (v, None),
                Err(_) => (
                    Vocab::default(),
                    Some("freitask: não consegui parsear status.json, usando defaults".into()),
                ),
            },
            None => (Vocab::default(), None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn vocab_cai_no_default_sem_arquivo_e_avisa_com_arquivo_quebrado() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::em(dir.path());
        let (vocab, aviso) = v.vocab();
        assert_eq!(vocab, Vocab::default());
        assert!(aviso.is_none());
        std::fs::create_dir_all(dir.path().join(".freitask")).unwrap();
        std::fs::write(dir.path().join(".freitask/status.json"), "{ quebrado").unwrap();
        let (vocab, aviso) = v.vocab();
        assert_eq!(vocab, Vocab::default());
        assert!(aviso.unwrap().contains("usando defaults"));
        std::fs::write(
            dir.path().join(".freitask/status.json"),
            r#"{"1":{"title":"A","callout":"todo"}}"#,
        )
        .unwrap();
        assert_eq!(v.vocab().0.meta(1).title, "A");
    }
}
