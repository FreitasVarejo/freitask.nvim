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
//! ```
//!
//! Uma diferença está coberta quando alguma entrada casa o cenário, o `onde`
//! e (se houver) o detalhe. Uma entrada foi exercitada quando cobriu ao menos
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
}

fn verdadeiro() -> bool {
    true
}

pub struct Lista {
    pub entradas: Vec<(Entrada, Regex, Regex, Option<Regex>)>,
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
        let rc = Regex::new(&e.cenario).map_err(|x| format!("{}: cenario: {x}", e.id))?;
        let ro = Regex::new(&e.onde).map_err(|x| format!("{}: onde: {x}", e.id))?;
        let rd = match &e.detalhe {
            Some(d) => Some(Regex::new(d).map_err(|x| format!("{}: detalhe: {x}", e.id))?),
            None => None,
        };
        entradas.push((e, rc, ro, rd));
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
        for (i, (e, rc, ro, rd)) in lista.entradas.iter().enumerate() {
            if rc.is_match(&d.cenario)
                && ro.is_match(&d.onde)
                && rd.as_ref().is_none_or(|r| r.is_match(&d.detalhe))
            {
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
