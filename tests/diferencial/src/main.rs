//! Diferencial (spec §3.4): roda a mesma sequência de comandos no oráculo Lua
//! (`nvim -l lua/freitask/cli.lua`, HOME=A) e no binário novo (HOME=B), sobre
//! cópias do mesmo corpus, e compara stdout, stderr, código de saída e o
//! manifesto da árvore. Toda diferença precisa casar com uma entrada de
//! `tests/aceite/divergencias.toml`; entrada que nenhum cenário exercitou
//! também falha na rodada completa (não acumula entrada morta).
//!
//! `--oraculo-vs-oraculo` roda o Lua dos dois lados: é o portão da fatia 0, que
//! prova que o harness está certo antes de haver Rust.
//!
//! Uso: cargo run -p diferencial -- [--novo <bin>] [--oraculo-vs-oraculo]
//!        [--corpus real|sintetico|patologico|todos] [--cenarios <regex>]
//!        [--parcial] [--relatorio <arquivo>] [--divergencias <toml>] [--manter]

mod cenarios;
mod comparar;
mod corpus;
mod divergencias;
mod executar;
mod relatorio;

use std::path::PathBuf;
use std::process::ExitCode;

use regex::Regex;

use crate::cenarios::Cenario;
use crate::comparar::Diferenca;
use crate::executar::{Lado, Runner};

pub struct Args {
    pub novo: Option<PathBuf>,
    pub oraculo_vs_oraculo: bool,
    pub corpus: String,
    pub cenarios: Option<Regex>,
    /// Rodada parcial: não falha por entrada de divergência não exercitada.
    pub parcial: bool,
    pub relatorio: PathBuf,
    pub divergencias: PathBuf,
    pub manter: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        novo: None,
        oraculo_vs_oraculo: false,
        corpus: "todos".into(),
        cenarios: None,
        parcial: false,
        relatorio: PathBuf::from("target/diferencial/relatorio.md"),
        divergencias: PathBuf::from("tests/aceite/divergencias.toml"),
        manter: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut valor = |nome: &str| it.next().ok_or_else(|| format!("falta o valor de {nome}"));
        match a.as_str() {
            "--novo" => args.novo = Some(PathBuf::from(valor("--novo")?)),
            "--oraculo-vs-oraculo" => args.oraculo_vs_oraculo = true,
            "--corpus" => args.corpus = valor("--corpus")?,
            "--cenarios" => {
                let re = valor("--cenarios")?;
                args.cenarios = Some(Regex::new(&re).map_err(|e| format!("--cenarios: {e}"))?);
            }
            "--parcial" => args.parcial = true,
            "--relatorio" => args.relatorio = PathBuf::from(valor("--relatorio")?),
            "--divergencias" => args.divergencias = PathBuf::from(valor("--divergencias")?),
            "--manter" => args.manter = true,
            outro => return Err(format!("argumento desconhecido: {outro}")),
        }
    }
    if !args.oraculo_vs_oraculo && args.novo.is_none() {
        return Err("passe --novo <binário> ou --oraculo-vs-oraculo".into());
    }
    Ok(args)
}

fn repo_raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("raiz do repo")
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("diferencial: {e}");
            return ExitCode::from(2);
        }
    };
    let repo = repo_raiz();
    let hoje = chrono::Local::now().date_naive();

    if which("nvim").is_none() {
        eprintln!("diferencial: nvim não está no PATH; o oráculo precisa dele");
        return ExitCode::from(2);
    }

    let lista = match divergencias::carregar(&args.divergencias) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("diferencial: {e}");
            return ExitCode::from(2);
        }
    };

    let runner_a = Runner::Oraculo { repo: repo.clone() };
    let runner_b = match (&args.oraculo_vs_oraculo, &args.novo) {
        (true, _) => Runner::Oraculo { repo: repo.clone() },
        (false, Some(bin)) => Runner::Binario(bin.canonicalize().unwrap_or_else(|_| bin.clone())),
        (false, None) => unreachable!(),
    };
    let modo = if args.oraculo_vs_oraculo {
        "oráculo vs oráculo"
    } else {
        "oráculo vs binário"
    };

    let todos = cenarios::todos(&args.corpus);
    let selecionados: Vec<&Cenario> = todos
        .iter()
        .filter(|c| args.cenarios.as_ref().is_none_or(|re| re.is_match(&c.nome)))
        .collect();
    if selecionados.is_empty() {
        eprintln!("diferencial: nenhum cenário casa a seleção");
        return ExitCode::from(2);
    }

    let raiz_tmp = tempfile::Builder::new()
        .prefix("freitask-diferencial-")
        .tempdir()
        .expect("tempdir");
    let mut resultados: Vec<relatorio::ResultadoCenario> = Vec::new();
    let mut diferencas: Vec<Diferenca> = Vec::new();
    let mut pulados = 0usize;
    let mut cache_corpus: std::collections::BTreeMap<String, Option<cenarios::Descobertos>> =
        Default::default();

    for (i, c) in selecionados.iter().enumerate() {
        let dir = raiz_tmp.path().join(format!("{i:03}"));
        let home_a = dir.join("A");
        let home_b = dir.join("B");
        std::fs::create_dir_all(&home_a).expect("A");
        std::fs::create_dir_all(&home_b).expect("B");
        corpus::preparar(&c.corpus, &home_a);
        corpus::preparar(&c.corpus, &home_b);

        // Placeholders ({task1}, {proj2}…) são resolvidos pelo oráculo sobre
        // uma cópia à parte do mesmo corpus, uma vez por corpus.
        let chave = format!("{:?}", c.corpus);
        let descobertos = cache_corpus
            .entry(chave)
            .or_insert_with(|| {
                let scratch = dir.join("descoberta");
                std::fs::create_dir_all(&scratch).expect("scratch");
                corpus::preparar(&c.corpus, &scratch);
                let lado = Lado {
                    nome: "descoberta",
                    home: scratch,
                    runner: runner_a.clone(),
                    hoje,
                };
                cenarios::descobrir(&lado)
            })
            .clone();
        let passos = match cenarios::resolver(&c.passos, descobertos.as_ref()) {
            Some(p) => p,
            None => {
                pulados += 1;
                resultados.push(relatorio::ResultadoCenario {
                    nome: c.nome.clone(),
                    corpus: format!("{:?}", c.corpus),
                    passos: vec![],
                    pulado: Some("placeholders sem valor neste corpus".into()),
                    diferencas: 0,
                });
                continue;
            }
        };

        let a = Lado {
            nome: "A",
            home: home_a,
            runner: runner_a.clone(),
            hoje,
        };
        let b = Lado {
            nome: "B",
            home: home_b,
            runner: runner_b.clone(),
            hoje,
        };
        let mut saidas_passos = Vec::new();
        let antes = diferencas.len();
        for (n, passo) in passos.iter().enumerate() {
            let sa = a.rodar(passo);
            let sb = b.rodar(passo);
            let descricao = format!("freitask {}", shell_join(passo));
            diferencas.extend(comparar::passo(
                &c.nome, n, &descricao, passo, &sa, &sb, &a.home, &b.home,
            ));
            saidas_passos.push(relatorio::PassoRodado {
                comando: descricao,
                codigo_a: sa.codigo,
                codigo_b: sb.codigo,
            });
        }
        let ma = executar::manifesto(&a.home.join("ObsidianVault"));
        let mb = executar::manifesto(&b.home.join("ObsidianVault"));
        diferencas.extend(comparar::manifestos(&c.nome, passos.len(), &ma, &mb));
        resultados.push(relatorio::ResultadoCenario {
            nome: c.nome.clone(),
            corpus: format!("{:?}", c.corpus),
            passos: saidas_passos,
            pulado: None,
            diferencas: diferencas.len() - antes,
        });
        if !args.manter {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    if chrono::Local::now().date_naive() != hoje {
        eprintln!(
            "diferencial: o dia mudou durante a rodada; os painéis não são comparáveis. Rode de novo."
        );
        return ExitCode::from(2);
    }

    let cobertura = divergencias::cobrir(&lista, &diferencas);
    let completo = !args.parcial
        && args.cenarios.is_none()
        && args.corpus == "todos"
        && !args.oraculo_vs_oraculo;
    let nao_exercitadas: Vec<&str> = if completo {
        cobertura.nao_exercitadas.clone()
    } else {
        vec![]
    };

    let rel = relatorio::montar(
        modo,
        &resultados,
        &diferencas,
        &cobertura,
        pulados,
        completo,
        &nao_exercitadas,
        &repo,
        &lista,
    );
    if let Some(p) = args.relatorio.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(&args.relatorio, &rel).expect("escrever o relatório");

    let rodados = resultados.iter().filter(|r| r.pulado.is_none()).count();
    println!(
        "diferencial: {modo}; relatório em {}",
        args.relatorio.display()
    );
    println!("cenários: {rodados} rodados, {pulados} pulados");
    println!(
        "divergências: {} no total, {} cobertas, {} fora da lista",
        diferencas.len(),
        cobertura.cobertas.len(),
        cobertura.descobertas.len()
    );
    println!(
        "entradas: exercitadas [{}], não exercitadas [{}]",
        cobertura.exercitadas.join(", "),
        cobertura.nao_exercitadas.join(", ")
    );
    for d in &cobertura.descobertas {
        println!(
            "  FORA DA LISTA: {} passo {} ({}) em {}: {}",
            d.cenario, d.passo, d.comando, d.onde, d.detalhe
        );
    }
    if args.manter {
        println!("cópias mantidas em {}", raiz_tmp.keep().display());
    }

    if !cobertura.descobertas.is_empty() || !nao_exercitadas.is_empty() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join(bin))
            .find(|c| c.is_file())
    })
}

pub fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|a| {
            if a.is_empty() || a.chars().any(|c| c.is_whitespace() || "\"'$`|".contains(c)) {
                format!("'{}'", a.replace('\'', "'\\''"))
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
