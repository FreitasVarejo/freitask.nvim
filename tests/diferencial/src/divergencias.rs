//! A lista aprovada de divergências (`tests/aceite/divergencias.toml`).
//!
//! Formato:
//!
//! ```toml
//! [[divergencia]]
//! id = "D1"
//! motivo = "por que o novo diverge do oráculo"
//! teste = "nome_do_teste_em_tests_aceite_dores_rs"
//! cenario = "regex sobre o nome do cenário"
//! onde = "regex sobre `codigo`, `stdout`, `stderr`, `json:<ponteiro>` ou `arquivo:<rel>`"
//! detalhe = "regex opcional sobre o detalhe"
//!
//! [[divergencia.tambem]]   # opcional, quantas vezes for preciso
//! cenario = "…"
//! onde = "…"
//! detalhe = "…"
//! ```
//!
//! Uma diferença está coberta quando alguma entrada casa o cenário, o `onde`
//! e (se houver) o detalhe, na cláusula principal ou numa `tambem`. A
//! `tambem` existe porque as três regex são independentes: uma causa que
//! aparece com formas diferentes em cenários diferentes, numa cláusula só,
//! levaria cada forma para todos os cenários. Uma entrada foi exercitada quando cobriu ao menos
//! uma diferença. `observavel = false` marca a que o harness não enxerga por
//! construção (ordem de chaves, mtime) e não precisa ser exercitada.

use std::path::Path;

use regex::Regex;
use serde::Deserialize;

use crate::comparar::Diferenca;

#[derive(Debug, Deserialize)]
struct Arquivo {
    #[serde(default)]
    divergencia: Vec<Entrada>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Entrada {
    pub id: String,
    pub motivo: String,
    #[serde(default)]
    pub teste: String,
    pub cenario: String,
    pub onde: String,
    #[serde(default)]
    pub detalhe: Option<String>,
    /// `false` para divergência que o harness não enxerga por construção
    /// (ordem de chaves do JSON, mtime): fica na lista como registro, não
    /// precisa ser exercitada e o relatório a marca como "não observável".
    #[serde(default = "verdadeiro")]
    pub observavel: bool,
    /// Cláusulas a mais da mesma entrada, cada uma com o próprio cenário,
    /// `onde` e detalhe.
    #[serde(default)]
    pub tambem: Vec<Clausula>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Clausula {
    pub cenario: String,
    pub onde: String,
    #[serde(default)]
    pub detalhe: Option<String>,
}

/// As três regex de uma cláusula, compiladas.
pub struct Regras {
    cenario: Regex,
    onde: Regex,
    detalhe: Option<Regex>,
}

impl Regras {
    fn compilar(
        id: &str,
        cenario: &str,
        onde: &str,
        detalhe: Option<&str>,
    ) -> Result<Regras, String> {
        let r = |nome: &str, x: &str| Regex::new(x).map_err(|e| format!("{id}: {nome}: {e}"));
        Ok(Regras {
            cenario: r("cenario", cenario)?,
            onde: r("onde", onde)?,
            detalhe: detalhe.map(|d| r("detalhe", d)).transpose()?,
        })
    }

    fn casa(&self, d: &Diferenca) -> bool {
        self.cenario.is_match(&d.cenario)
            && self.onde.is_match(&d.onde)
            && self.detalhe.as_ref().is_none_or(|r| r.is_match(&d.detalhe))
    }
}

fn verdadeiro() -> bool {
    true
}

pub struct Lista {
    /// Cada entrada com as regras da cláusula principal e das `tambem`.
    pub entradas: Vec<(Entrada, Vec<Regras>)>,
}

pub fn carregar(caminho: &Path) -> Result<Lista, String> {
    let texto = match std::fs::read_to_string(caminho) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("ler {}: {e}", caminho.display())),
    };
    let arq: Arquivo = toml::from_str(&texto).map_err(|e| format!("{}: {e}", caminho.display()))?;
    let mut entradas = Vec::new();
    for e in arq.divergencia {
        let mut regras = vec![Regras::compilar(
            &e.id,
            &e.cenario,
            &e.onde,
            e.detalhe.as_deref(),
        )?];
        for (n, c) in e.tambem.iter().enumerate() {
            let id = format!("{} (tambem {})", e.id, n + 1);
            regras.push(Regras::compilar(
                &id,
                &c.cenario,
                &c.onde,
                c.detalhe.as_deref(),
            )?);
        }
        entradas.push((e, regras));
    }
    Ok(Lista { entradas })
}

pub struct Cobertura<'a> {
    /// (diferença, id da entrada que a cobre)
    pub cobertas: Vec<(&'a Diferenca, String)>,
    pub descobertas: Vec<&'a Diferenca>,
    pub exercitadas: Vec<&'a str>,
    pub nao_exercitadas: Vec<&'a str>,
}

pub fn cobrir<'a>(lista: &'a Lista, diferencas: &'a [Diferenca]) -> Cobertura<'a> {
    let mut cobertas = Vec::new();
    let mut descobertas = Vec::new();
    let mut usadas = vec![false; lista.entradas.len()];
    for d in diferencas {
        let mut quem = None;
        for (i, (e, regras)) in lista.entradas.iter().enumerate() {
            if regras.iter().any(|r| r.casa(d)) {
                quem = Some((i, e.id.clone()));
                break;
            }
        }
        match quem {
            Some((i, id)) => {
                usadas[i] = true;
                cobertas.push((d, id));
            }
            None => descobertas.push(d),
        }
    }
    let exercitadas = lista
        .entradas
        .iter()
        .zip(&usadas)
        .filter(|(_, u)| **u)
        .map(|((e, ..), _)| e.id.as_str())
        .collect();
    let nao_exercitadas = lista
        .entradas
        .iter()
        .zip(&usadas)
        .filter(|((e, ..), u)| !**u && e.observavel)
        .map(|((e, ..), _)| e.id.as_str())
        .collect();
    Cobertura {
        cobertas,
        descobertas,
        exercitadas,
        nao_exercitadas,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn dif(cenario: &str, onde: &str, detalhe: &str) -> Diferenca {
        Diferenca {
            cenario: cenario.into(),
            passo: 1,
            comando: String::new(),
            onde: onde.into(),
            detalhe: detalhe.into(),
        }
    }

    #[test]
    fn tambem_cobre_so_a_propria_combinacao() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("d.toml");
        std::fs::write(
            &p,
            r#"
[[divergencia]]
id = "D1"
motivo = "m"
cenario = '::tabela$'
onde = '^arquivo:'
detalhe = '^bytes'

[[divergencia.tambem]]
cenario = '::doctor$'
onde = '^codigo$'
detalhe = '^A=1 B=0$'
"#,
        )
        .unwrap();
        let lista = carregar(&p).unwrap();
        let difs = [
            dif("x::tabela", "arquivo:a.md", "bytes diferentes"),
            dif("x::doctor", "codigo", "A=1 B=0"),
            dif("x::doctor", "arquivo:a.md", "bytes diferentes"),
            dif("x::tabela", "codigo", "A=1 B=0"),
        ];
        let c = cobrir(&lista, &difs);
        assert_eq!(c.cobertas.len(), 2);
        assert_eq!(c.descobertas.len(), 2);
        assert_eq!(c.exercitadas, vec!["D1"]);
    }
}
