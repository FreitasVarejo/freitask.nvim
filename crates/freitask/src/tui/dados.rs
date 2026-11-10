//! O que a tela mostra, lido do vault de uma vez: as fases, os projetos e as
//! tasks. Vem dos mesmos tipos do `core` que alimentam o `list --json` e o
//! `projects --json` (`linhas.rs`), então a TUI e a CLI não divergem (spec
//! §1.7).

use freitask_core::date::Date;
use freitask_core::status::Vocab;
use freitask_vault::Vault;

use crate::linhas::{self, Resumo};

/// Um projeto e quantas tasks o painel mostra dele.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projeto {
    pub nome: String,
    pub ativas: usize,
}

/// Tudo que a tela mostra.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dados {
    pub vocab: Vocab,
    pub projetos: Vec<Projeto>,
    /// Ativas e arquivadas, por projeto, na ordem do `list --json --archived`.
    pub tasks: Vec<Resumo>,
}

impl Dados {
    /// Lê o vault no dia `hoje`. Nada aqui falha: o que o vault não tem sai
    /// vazio, como no `list`.
    pub fn ler(vault: &Vault, hoje: Date) -> Dados {
        // O aviso de `status.json` quebrado é do `list`; a TUI só usa o
        // vocabulário que sobra (o default).
        let (vocab, _aviso) = vault.vocab();
        let indice = vault.indice(&vocab);
        let projetos = linhas::projetos(&vocab, &indice, hoje)
            .into_iter()
            .map(|(nome, ativas)| Projeto { nome, ativas })
            .collect();
        let tasks = linhas::todas(&indice, true)
            .into_iter()
            .map(|e| linhas::resumo(vault, &vocab, e, hoje))
            .collect();
        Dados {
            vocab,
            projetos,
            tasks,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn dia(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn gravar(raiz: &std::path::Path, rel: &str, conteudo: &str) {
        let p = raiz.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, conteudo).unwrap();
    }

    #[test]
    fn le_as_tasks_na_ordem_do_list_e_a_adiada_volta_no_dia_dela() {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path();
        gravar(raiz, "projects/a/tasks/um.md", "> [!todo] Um\n> [[um]]\n");
        gravar(
            raiz,
            "projects/a/tasks/dois.md",
            "---\nvolta-em: 2026-12-01\n---\n\n> [!tip] Dois\n> [[dois]]\n",
        );
        gravar(
            raiz,
            "projects/a/tasks/archived/done/zero.md",
            "> [!done] Zero\n> [[zero]]\n",
        );
        let vault = Vault::em(raiz);

        let d = Dados::ler(&vault, dia("2026-11-10"));
        let ids: Vec<&str> = d.tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["um", "dois", "zero"]);
        assert_eq!(
            d.projetos,
            [Projeto {
                nome: "a".into(),
                ativas: 1
            }]
        );
        assert!(d.tasks[1].hidden);
        assert_eq!(d.tasks[1].volta_em.as_deref(), Some("2026-12-01"));

        let d = Dados::ler(&vault, dia("2026-12-01"));
        assert!(!d.tasks[1].hidden);
        assert_eq!(d.tasks[1].volta_em, None);
        assert_eq!(d.projetos[0].ativas, 2);
    }

    #[test]
    fn vault_que_nao_existe_le_vazio() {
        let dir = tempfile::tempdir().unwrap();
        let d = Dados::ler(&Vault::em(dir.path().join("nada")), dia("2026-11-10"));
        assert!(d.projetos.is_empty() && d.tasks.is_empty());
    }
}
