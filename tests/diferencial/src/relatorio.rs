//! O relatório em Markdown que o `make portoes` embute e o dono lê.

use std::path::Path;

use crate::comparar::Diferenca;
use crate::divergencias::{Cobertura, Lista};

pub struct PassoRodado {
    pub comando: String,
    pub codigo_a: i32,
    pub codigo_b: i32,
}

pub struct ResultadoCenario {
    pub nome: String,
    pub corpus: String,
    pub passos: Vec<PassoRodado>,
    pub pulado: Option<String>,
    pub diferencas: usize,
}

// O relatório junta tudo que a rodada produziu; cada parâmetro é uma seção.
#[allow(clippy::too_many_arguments)]
pub fn montar(
    modo: &str,
    resultados: &[ResultadoCenario],
    diferencas: &[Diferenca],
    cobertura: &Cobertura,
    pulados: usize,
    completo: bool,
    nao_exercitadas: &[&str],
    repo: &Path,
    lista: &Lista,
) -> String {
    let mut s = String::new();
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    s.push_str(&format!("# Diferencial — {modo}\n\n"));
    s.push_str(&format!(
        "Commit `{commit}`, {} em {}. {} cenários rodados, {} pulados. {} diferenças: {} cobertas pela lista, **{} fora da lista**.{}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M"),
        std::env::consts::OS,
        resultados.iter().filter(|r| r.pulado.is_none()).count(),
        pulados,
        diferencas.len(),
        cobertura.cobertas.len(),
        cobertura.descobertas.len(),
        if completo { " Rodada completa: entrada não exercitada conta como falha." } else { " Rodada parcial." }
    ));

    s.push_str("## Entradas de divergencias.toml\n\n| id | exercitada | diferenças cobertas | teste | motivo |\n| --- | --- | --- | --- | --- |\n");
    for (e, ..) in &lista.entradas {
        let n = cobertura
            .cobertas
            .iter()
            .filter(|(_, i)| *i == e.id)
            .count();
        let estado = if cobertura.exercitadas.contains(&e.id.as_str()) {
            "sim".to_string()
        } else if !e.observavel {
            "não observável (ordem/mtime)".to_string()
        } else if nao_exercitadas.contains(&e.id.as_str()) {
            "não **(falha)**".to_string()
        } else {
            "não".to_string()
        };
        s.push_str(&format!(
            "| {} | {estado} | {n} | `{}` | {} |\n",
            e.id,
            e.teste,
            e.motivo.replace('|', "\\|")
        ));
    }

    if !cobertura.descobertas.is_empty() {
        s.push_str("\n## Diferenças FORA da lista\n\n| cenário | passo | comando | onde | detalhe |\n| --- | --- | --- | --- | --- |\n");
        for d in &cobertura.descobertas {
            s.push_str(&linha(d, None));
        }
    }

    s.push_str("\n## Todas as diferenças\n\n");
    if diferencas.is_empty() {
        s.push_str("Nenhuma.\n");
    } else {
        s.push_str("| cenário | passo | comando | onde | detalhe | entrada |\n| --- | --- | --- | --- | --- | --- |\n");
        for d in diferencas {
            let id = cobertura
                .cobertas
                .iter()
                .find(|(x, _)| *x == d)
                .map(|(_, i)| i.as_str());
            s.push_str(&linha(d, Some(id.unwrap_or("—"))));
        }
    }

    s.push_str("\n## Cenários\n\n");
    for r in resultados {
        match &r.pulado {
            Some(m) => s.push_str(&format!("- `{}` ({}): pulado — {m}\n", r.nome, r.corpus)),
            None => {
                s.push_str(&format!(
                    "- `{}` ({}): {} passos, {} diferenças\n",
                    r.nome,
                    r.corpus,
                    r.passos.len(),
                    r.diferencas
                ));
                for (i, p) in r.passos.iter().enumerate() {
                    let marca = if p.codigo_a == p.codigo_b {
                        ""
                    } else {
                        " **≠**"
                    };
                    s.push_str(&format!(
                        "  {i}. `{}` → A {} / B {}{marca}\n",
                        p.comando, p.codigo_a, p.codigo_b
                    ));
                }
            }
        }
    }
    s
}

fn linha(d: &Diferenca, entrada: Option<&str>) -> String {
    let esc = |x: &str| x.replace('|', "\\|").replace('\n', " ");
    let mut l = format!(
        "| {} | {} | `{}` | `{}` | {} |",
        esc(&d.cenario),
        d.passo,
        esc(&d.comando),
        esc(&d.onde),
        esc(&d.detalhe)
    );
    if let Some(e) = entrada {
        l.push_str(&format!(" {e} |"));
    }
    l.push('\n');
    l
}
