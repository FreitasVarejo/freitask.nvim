//! O resultado de um comando que escreve, sem terminal e sem `HOME`: a CLI o
//! imprime e a TUI o mostra na barra (decisão f7.2 em
//! `docs/decisoes-da-reescrita.md`).
//!
//! Um comando recebe o vault e o caminho do lock num `Contexto` e devolve o
//! que sairia em stdout, ou as linhas de stderr (sem o `freitask: `) e o
//! código de saída. Quem imprime acrescenta o prefixo; a TUI junta as linhas
//! com ` · `, como o `engine.rs` da TUI de hoje fazia com o stderr da CLI.

use std::path::{Path, PathBuf};

use freitask_core::date::Date;
use freitask_core::status::Vocab;
use freitask_vault::trava::{self, ErroTrava, Trava};
use freitask_vault::{ErroEscrita, Vault};

/// O prefixo de toda linha de stderr da CLI.
pub const PREFIXO: &str = "freitask: ";

/// O que um comando que escreve precisa: o vault, onde mora o lock e o dia.
#[derive(Debug, Clone, Copy)]
pub struct Contexto<'a> {
    pub vault: &'a Vault,
    pub trava: &'a Path,
    pub hoje: Date,
}

/// Um comando que deu certo: a linha de stdout (a CLI a imprime com `\n`) e os
/// avisos que foram a stderr no caminho.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saida {
    pub stdout: String,
    pub stderr: Vec<String>,
}

/// Um comando que não deu certo: tudo que foi a stderr, em ordem, e a saída.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Falha {
    pub stderr: Vec<String>,
    pub codigo: u8,
}

pub type Resultado = Result<Saida, Falha>;

impl Falha {
    /// A recusa como a barra da TUI a mostra: as linhas, aparadas e sem as
    /// vazias, juntas com ` · `.
    pub fn para_a_barra(&self) -> String {
        self.stderr
            .iter()
            .flat_map(|mensagem| mensagem.lines())
            .map(str::trim)
            .filter(|linha| !linha.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// O que um comando acumula em stderr enquanto roda. Cada `Resultado` leva
/// consigo o que foi acumulado, para a CLI imprimir na mesma ordem de sempre.
#[derive(Debug, Default)]
pub struct Corrida {
    stderr: Vec<String>,
}

impl Corrida {
    /// Uma linha de stderr que não encerra o comando.
    pub fn aviso(&mut self, mensagem: impl Into<String>) {
        self.stderr.push(mensagem.into());
    }

    /// Tira o que foi acumulado (para quem não devolve um `Resultado`).
    pub fn descarrega(&mut self) -> Vec<String> {
        std::mem::take(&mut self.stderr)
    }

    /// Encerra o comando com esta recusa.
    pub fn falha(&mut self, mensagem: impl Into<String>, codigo: u8) -> Falha {
        self.stderr.push(mensagem.into());
        Falha {
            stderr: self.descarrega(),
            codigo,
        }
    }

    pub fn erro_escrita(&mut self, e: &ErroEscrita) -> Falha {
        self.falha(e.to_string(), e.codigo())
    }

    pub fn ok(&mut self, stdout: impl Into<String>) -> Resultado {
        Ok(Saida {
            stdout: stdout.into(),
            stderr: self.descarrega(),
        })
    }

    /// Toma o lock de escrita (spec §1.5), esperando até `trava::ESPERA`.
    pub fn travar(&mut self, cx: &Contexto) -> Result<Trava, Falha> {
        trava::tomar(cx.trava, trava::ESPERA).map_err(|e| self.falha(e.to_string(), e.codigo()))
    }

    /// O vocabulário; o aviso de `status.json` quebrado vai a stderr.
    pub fn vocab(&mut self, vault: &Vault) -> Vocab {
        let (vocab, aviso) = vault.vocab();
        if let Some(a) = aviso {
            self.aviso(a.strip_prefix(PREFIXO).unwrap_or(&a));
        }
        vocab
    }

    /// Regenera o `CURRENT.md`: garante a raiz (como o `ensure_root` de hoje),
    /// relê o vocabulário e o índice, arquiva o painel de outro dia em
    /// `daily/` e grava o novo só se mudou (§1.5), com CAS contra o painel que
    /// leu.
    pub fn regenerar(&mut self, vault: &Vault, hoje: Date) -> Result<(), ErroEscrita> {
        vault.garantir_raiz()?;
        let vocab = self.vocab(vault);
        let indice = vault.indice(&vocab);
        let anterior = vault.ler("CURRENT.md");
        let painel = freitask_core::board::render(&vocab, &indice, hoje, anterior.as_deref());
        if let Some((rel, bytes)) = &painel.daily
            && !vault.existe(rel)
        {
            vault.gravar(rel, None, bytes)?;
        }
        vault.gravar("CURRENT.md", anterior.as_deref(), &painel.bytes)?;
        Ok(())
    }

    /// Regenera o painel depois de uma escrita e devolve o que o comando
    /// imprime.
    pub fn fecha(&mut self, cx: &Contexto, saida: &str) -> Resultado {
        self.regenerar(cx.vault, cx.hoje)
            .map_err(|e| self.erro_escrita(&e))?;
        self.ok(saida)
    }
}

/// Onde o lock mora para este `home`: o `trava::caminho` com o
/// `XDG_RUNTIME_DIR` do ambiente. Sem nenhum dos dois, a recusa que a CLI
/// sempre deu (saída 1).
pub fn caminho_da_trava(home: Option<&str>) -> Result<PathBuf, Falha> {
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok();
    trava_em(xdg.as_deref(), home)
}

fn trava_em(xdg: Option<&str>, home: Option<&str>) -> Result<PathBuf, Falha> {
    trava::caminho(xdg, home).ok_or_else(|| {
        let e = ErroTrava::Io(
            PathBuf::from("freitask.lock"),
            std::io::Error::other("nem XDG_RUNTIME_DIR nem HOME estão definidos"),
        );
        Falha {
            stderr: vec![e.to_string()],
            codigo: e.codigo(),
        }
    })
}

/// O dia de hoje a partir do texto `AAAA-MM-DD` (o `FREITASK_TODAY` e o `hoje`
/// que a TUI recebe).
pub fn dia(texto: &str) -> Result<Date, String> {
    Date::parse(texto).ok_or_else(|| format!("FREITASK_TODAY inválida: {texto} (use AAAA-MM-DD)"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_barra_junta_as_linhas_sem_as_vazias() {
        let f = Falha {
            stderr: vec!["a".into(), "  b\n\nc  ".into(), "".into()],
            codigo: 1,
        };
        assert_eq!(f.para_a_barra(), "a · b · c");
    }

    #[test]
    fn falha_leva_os_avisos_que_vieram_antes_e_esvazia_a_corrida() {
        let mut c = Corrida::default();
        c.aviso("antes");
        let f = c.falha("depois", 3);
        assert_eq!(f.stderr, ["antes", "depois"]);
        assert_eq!(f.codigo, 3);
        assert_eq!(c.ok("x").unwrap().stderr, Vec::<String>::new());
    }

    #[test]
    fn ok_leva_os_avisos() {
        let mut c = Corrida::default();
        c.aviso("aviso");
        let s = c.ok("linha").unwrap();
        assert_eq!(s.stdout, "linha");
        assert_eq!(s.stderr, ["aviso"]);
    }

    #[test]
    fn o_vocabulario_quebrado_avisa_sem_o_prefixo_e_cai_no_default() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".freitask")).unwrap();
        std::fs::write(dir.path().join(".freitask/status.json"), "{ quebrado").unwrap();
        let mut c = Corrida::default();
        let vocab = c.vocab(&Vault::em(dir.path()));
        assert_eq!(vocab, Vocab::default());
        assert_eq!(
            c.descarrega(),
            ["não consegui parsear status.json, usando defaults"]
        );
    }

    #[test]
    fn sem_xdg_nem_home_a_trava_recusa_com_saida_1() {
        let f = trava_em(None, None).unwrap_err();
        assert_eq!(f.codigo, 1);
        assert_eq!(
            f.stderr,
            [
                "não consegui abrir o lock freitask.lock: nem XDG_RUNTIME_DIR nem HOME estão definidos"
            ]
        );
        assert_eq!(
            trava_em(None, Some("/h")).unwrap(),
            PathBuf::from("/h/.local/state/freitask/lock")
        );
        assert_eq!(
            trava_em(Some("/run/u"), None).unwrap(),
            PathBuf::from("/run/u/freitask.lock")
        );
    }

    #[test]
    fn dia_aceita_a_data_e_recusa_o_resto() {
        assert_eq!(dia("2026-11-10").unwrap().to_string(), "2026-11-10");
        assert_eq!(
            dia("ontem").unwrap_err(),
            "FREITASK_TODAY inválida: ontem (use AAAA-MM-DD)"
        );
    }
}
