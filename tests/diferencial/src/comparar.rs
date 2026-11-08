//! Comparar as saídas de um passo e os manifestos. O prefixo do HOME vira
//! `<HOME>`; JSON é decodificado e comparado como valor; o resto, byte a byte.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::executar::Saida;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diferenca {
    pub cenario: String,
    pub passo: usize,
    pub comando: String,
    /// `codigo`, `stdout`, `stderr`, `json:<ponteiro>`, `arquivo:<rel>`.
    pub onde: String,
    pub detalhe: String,
}

fn normalizar(bytes: &[u8], home_a: &Path, home_b: &Path) -> String {
    let s = String::from_utf8_lossy(bytes).to_string();
    s.replace(&home_a.display().to_string(), "<HOME>")
        .replace(&home_b.display().to_string(), "<HOME>")
}

fn normalizar_json(v: &mut Value, home_a: &str, home_b: &str) {
    match v {
        Value::String(s) => {
            if s.contains(home_a) || s.contains(home_b) {
                *s = s.replace(home_a, "<HOME>").replace(home_b, "<HOME>");
            }
        }
        Value::Array(a) => a
            .iter_mut()
            .for_each(|x| normalizar_json(x, home_a, home_b)),
        Value::Object(o) => o
            .values_mut()
            .for_each(|x| normalizar_json(x, home_a, home_b)),
        _ => {}
    }
}

/// Os achados do `doctor --json` saem na ordem de um `pairs()` do Lua, que não
/// é estável entre processos: ordenam-se por (kind, path, msg) antes de
/// comparar. Nenhum consumidor depende da ordem (check.sh filtra por level).
fn ordenar_achados(v: &mut Value) {
    if let Some(findings) = v.get_mut("findings").and_then(Value::as_array_mut) {
        findings.sort_by_key(|f| {
            let g = |k: &str| f.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            (g("kind"), g("path"), g("msg"))
        });
    }
}

/// O `doctor` em texto imprime cada achado em duas linhas; a ordem tem a mesma
/// instabilidade do JSON. Os pares são ordenados; o resumo fica onde está.
fn canonizar_doctor_texto(s: &str) -> String {
    let linhas: Vec<&str> = s.lines().collect();
    let mut pares: Vec<String> = Vec::new();
    let mut resto: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < linhas.len() {
        let l = linhas[i];
        let eh_achado = l.starts_with("error ") || l.starts_with("warn  ");
        if eh_achado && i + 1 < linhas.len() && linhas[i + 1].starts_with("         ") {
            pares.push(format!("{l}\n{}", linhas[i + 1]));
            i += 2;
        } else {
            resto.push(l);
            i += 1;
        }
    }
    pares.sort();
    let mut out = pares.join("\n");
    if !resto.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&resto.join("\n"));
    }
    out
}

// Oito parâmetros porque a comparação de um passo precisa dos dois lados
// (saída e HOME de cada um) e da identidade do passo; agrupar num struct
// só mudaria o lugar da lista.
#[allow(clippy::too_many_arguments)]
pub fn passo(
    cenario: &str,
    n: usize,
    comando: &str,
    args: &[String],
    a: &Saida,
    b: &Saida,
    home_a: &Path,
    home_b: &Path,
) -> Vec<Diferenca> {
    let mut out = Vec::new();
    let mk = |onde: String, detalhe: String| Diferenca {
        cenario: cenario.to_string(),
        passo: n,
        comando: comando.to_string(),
        onde,
        detalhe,
    };

    if a.codigo != b.codigo {
        out.push(mk(
            "codigo".into(),
            format!("A={} B={}", a.codigo, b.codigo),
        ));
    }

    let quer_json = args.iter().any(|x| x == "--json");
    let sa = normalizar(&a.stdout, home_a, home_b);
    let sb = normalizar(&b.stdout, home_a, home_b);
    let ha = home_a.display().to_string();
    let hb = home_b.display().to_string();
    let ja = serde_json::from_slice::<Value>(&a.stdout).ok();
    let jb = serde_json::from_slice::<Value>(&b.stdout).ok();
    match (quer_json, ja, jb) {
        (_, Some(mut va), Some(mut vb)) => {
            normalizar_json(&mut va, &ha, &hb);
            normalizar_json(&mut vb, &ha, &hb);
            if args.first().map(String::as_str) == Some("doctor") {
                ordenar_achados(&mut va);
                ordenar_achados(&mut vb);
            }
            let mut ptr = String::new();
            diff_json(&va, &vb, &mut ptr, &mut out, &mk);
        }
        (true, None, Some(_)) if !a.stdout.is_empty() => {
            out.push(mk("stdout".into(), "só B é JSON".into()));
        }
        (true, Some(_), None) if !b.stdout.is_empty() => {
            out.push(mk("stdout".into(), "só A é JSON".into()));
        }
        _ => {
            let eh_doctor_texto = args.first().map(String::as_str) == Some("doctor") && !quer_json;
            let (ca, cb) = if eh_doctor_texto {
                (canonizar_doctor_texto(&sa), canonizar_doctor_texto(&sb))
            } else {
                (sa.clone(), sb.clone())
            };
            if ca != cb {
                out.push(mk("stdout".into(), resumo_diff(&ca, &cb)));
            }
        }
    }

    let ea = normalizar(&a.stderr, home_a, home_b);
    let eb = normalizar(&b.stderr, home_a, home_b);
    if ea != eb {
        out.push(mk("stderr".into(), resumo_diff(&ea, &eb)));
    }
    out
}

fn diff_json(
    a: &Value,
    b: &Value,
    ptr: &mut String,
    out: &mut Vec<Diferenca>,
    mk: &dyn Fn(String, String) -> Diferenca,
) {
    match (a, b) {
        (Value::Object(oa), Value::Object(ob)) => {
            for (k, va) in oa {
                let len = ptr.len();
                ptr.push('/');
                ptr.push_str(k);
                match ob.get(k) {
                    Some(vb) => diff_json(va, vb, ptr, out, mk),
                    None => out.push(mk(format!("json:{ptr}"), format!("só em A: {}", curto(va)))),
                }
                ptr.truncate(len);
            }
            for (k, vb) in ob {
                if !oa.contains_key(k) {
                    out.push(mk(
                        format!("json:{ptr}/{k}"),
                        format!("só em B: {}", curto(vb)),
                    ));
                }
            }
        }
        (Value::Array(aa), Value::Array(ab)) => {
            if aa.len() != ab.len() {
                out.push(mk(
                    format!("json:{ptr}"),
                    format!("tamanho A={} B={}", aa.len(), ab.len()),
                ));
            }
            for (i, (va, vb)) in aa.iter().zip(ab.iter()).enumerate() {
                let len = ptr.len();
                ptr.push_str(&format!("/{i}"));
                diff_json(va, vb, ptr, out, mk);
                ptr.truncate(len);
            }
        }
        _ => {
            if a != b {
                out.push(mk(
                    format!("json:{ptr}"),
                    format!("A={} B={}", curto(a), curto(b)),
                ));
            }
        }
    }
}

fn curto(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 120 {
        let t: String = s.chars().take(117).collect();
        format!("{t}...")
    } else {
        s
    }
}

/// Um resumo legível da primeira linha que difere.
pub fn resumo_diff(a: &str, b: &str) -> String {
    let la: Vec<&str> = a.lines().collect();
    let lb: Vec<&str> = b.lines().collect();
    for (i, (x, y)) in la.iter().zip(lb.iter()).enumerate() {
        if x != y {
            return format!("linha {}: A={:?} B={:?}", i + 1, trunc(x), trunc(y));
        }
    }
    if la.len() != lb.len() {
        let (lado, extra) = if la.len() > lb.len() {
            ("A", la[lb.len()])
        } else {
            ("B", lb[la.len()])
        };
        return format!(
            "{} linhas em A, {} em B; a primeira a mais em {lado}: {:?}",
            la.len(),
            lb.len(),
            trunc(extra)
        );
    }
    format!("A={:?} B={:?}", trunc(a), trunc(b))
}

fn trunc(s: &str) -> String {
    if s.chars().count() > 160 {
        let t: String = s.chars().take(157).collect();
        format!("{t}...")
    } else {
        s.to_string()
    }
}

pub fn manifestos(
    cenario: &str,
    passo: usize,
    a: &BTreeMap<String, String>,
    b: &BTreeMap<String, String>,
) -> Vec<Diferenca> {
    let mut out = Vec::new();
    let mk = |onde: String, detalhe: String| Diferenca {
        cenario: cenario.to_string(),
        passo,
        comando: "<manifesto final>".into(),
        onde,
        detalhe,
    };
    for (k, ha) in a {
        match b.get(k) {
            None => out.push(mk(format!("arquivo:{k}"), "só em A".into())),
            Some(hb) if hb != ha => out.push(mk(format!("arquivo:{k}"), "bytes diferentes".into())),
            _ => {}
        }
    }
    for k in b.keys() {
        if !a.contains_key(k) {
            out.push(mk(format!("arquivo:{k}"), "só em B".into()));
        }
    }
    out
}
