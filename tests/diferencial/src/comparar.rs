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

/// O `doctor` em texto imprime cada achado em duas linhas; a ordem tem a mesma
/// instabilidade do JSON. Separa os pares (achado + mensagem) do resto (a
/// contagem, a dica, o "vault consistente").
fn separar_doctor_texto(s: &str) -> (Vec<String>, Vec<&str>) {
    let linhas: Vec<&str> = s.lines().collect();
    let mut pares: Vec<String> = Vec::new();
    let mut resto: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < linhas.len() {
        let l = linhas[i];
        let eh_achado = l.starts_with("error ") || l.starts_with("warn  ");
        if eh_achado && i + 1 < linhas.len() && linhas[i + 1].starts_with("         ") {
            // Só o recuo fixo de 9 espaços sai: recuo diferente é diferença.
            pares.push(format!("{l} / {}", &linhas[i + 1][9..]));
            i += 2;
        } else {
            resto.push(l);
            i += 1;
        }
    }
    (pares, resto)
}

/// O que está só em A e só em B, contando repetição (multiconjunto), em
/// ordem. Os achados do `doctor` se comparam assim: comparados por posição,
/// um achado a mais num lado deslocaria todos os seguintes e cada um viraria
/// uma diferença sem relação com a causa.
fn so_de_cada_lado(a: Vec<String>, b: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut resto_b = b;
    let mut so_a = Vec::new();
    for x in a {
        match resto_b.iter().position(|y| *y == x) {
            Some(i) => {
                resto_b.remove(i);
            }
            None => so_a.push(x),
        }
    }
    so_a.sort();
    resto_b.sort();
    (so_a, resto_b)
}

/// Um achado do `doctor --json` numa linha, no formato do texto (`level`
/// alinhado em 6, `kind path msg (fixed=…)`) para o mesmo `detalhe` casar
/// nos dois. Nada do objeto se perde: valor que não é string sai em JSON
/// (`null`, `1`), chave que falta sai `∅`, e chave fora das cinco do
/// contrato entra no fim como `+chave=valor`.
fn achado_em_linha(f: &Value) -> String {
    const CHAVES: [&str; 5] = ["level", "kind", "path", "msg", "fixed"];
    let repr = |v: Option<&Value>| match v {
        Some(Value::String(s)) => s.clone(),
        Some(outro) => outro.to_string(),
        None => "∅".to_string(),
    };
    let g = |k: &str| repr(f.get(k));
    let mut linha = format!(
        "{:<6} {} {} {} (fixed={})",
        g("level"),
        g("kind"),
        g("path"),
        g("msg"),
        g("fixed")
    );
    match f.as_object() {
        Some(o) => {
            let mut extras: Vec<_> = o
                .iter()
                .filter(|(k, _)| !CHAVES.contains(&k.as_str()))
                .collect();
            extras.sort_by(|x, y| x.0.cmp(y.0));
            for (k, v) in extras {
                linha.push_str(&format!(" +{k}={}", repr(Some(v))));
            }
        }
        None => linha = format!("não-objeto {f}"),
    }
    linha
}

/// Tira os `findings` dos dois lados e devolve o que está só em A e só em B.
/// Quem chama emite `achado só em A: …`, e não `só em A: …`: o prefixo
/// genérico é o de chave JSON que só um lado tem, e uma entrada larga para
/// ele (a D10) cobriria qualquer achado a mais.
/// `None` quando um dos lados não tem `findings` como array: aí a comparação
/// genérica cuida.
fn tirar_achados(va: &mut Value, vb: &mut Value) -> Option<(Vec<String>, Vec<String>)> {
    va.get("findings")?.as_array()?;
    vb.get("findings")?.as_array()?;
    let pega = |v: &mut Value| -> Vec<String> {
        let arr = v
            .get_mut("findings")
            .and_then(Value::as_array_mut)
            .map(std::mem::take)
            .unwrap_or_default();
        arr.iter().map(achado_em_linha).collect()
    };
    let (a, b) = (pega(va), pega(vb));
    Some(so_de_cada_lado(a, b))
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
            if args.first().map(String::as_str) == Some("doctor")
                && let Some((so_a, so_b)) = tirar_achados(&mut va, &mut vb)
            {
                for x in so_a {
                    out.push(mk("json:/findings".into(), format!("achado só em A: {x}")));
                }
                for x in so_b {
                    out.push(mk("json:/findings".into(), format!("achado só em B: {x}")));
                }
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
            if eh_doctor_texto {
                let (pa, ra) = separar_doctor_texto(&sa);
                let (pb, rb) = separar_doctor_texto(&sb);
                let (so_a, so_b) = so_de_cada_lado(pa, pb);
                for x in so_a {
                    out.push(mk("stdout".into(), format!("achado só em A: {x}")));
                }
                for x in so_b {
                    out.push(mk("stdout".into(), format!("achado só em B: {x}")));
                }
                let (ra, rb) = (ra.join("\n"), rb.join("\n"));
                if ra != rb {
                    out.push(mk("stdout".into(), resumo_diff(&ra, &rb)));
                }
            } else if sa != sb {
                out.push(mk("stdout".into(), resumo_diff(&sa, &sb)));
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    fn achado(level: &str) -> Value {
        json!({"level": level, "kind": "link-pendurado", "path": "p.md", "msg": "m", "fixed": false})
    }

    #[test]
    fn achado_em_linha_carrega_o_objeto_inteiro() {
        assert_eq!(
            achado_em_linha(&achado("error")),
            "error  link-pendurado p.md m (fixed=false)"
        );
        assert_ne!(
            achado_em_linha(&achado("error")),
            achado_em_linha(&achado("warn"))
        );
        let mut sem_path = achado("warn");
        sem_path.as_object_mut().unwrap().remove("path");
        let mut path_nulo = achado("warn");
        path_nulo["path"] = Value::Null;
        let mut path_vazio = achado("warn");
        path_vazio["path"] = json!("");
        let linhas: BTreeSet<String> = [&sem_path, &path_nulo, &path_vazio]
            .into_iter()
            .map(achado_em_linha)
            .collect();
        assert_eq!(linhas.len(), 3);
        let mut extra = achado("warn");
        extra["linha"] = json!(3);
        assert!(achado_em_linha(&extra).ends_with(" +linha=3"));
    }

    #[test]
    fn achados_se_comparam_como_multiconjunto() {
        let mut a =
            json!({"ok": false, "findings": [achado("error"), achado("error"), achado("warn")]});
        let mut b = json!({"ok": false, "findings": [achado("warn"), achado("error")]});
        let (so_a, so_b) = tirar_achados(&mut a, &mut b).unwrap();
        assert_eq!(so_a, vec!["error  link-pendurado p.md m (fixed=false)"]);
        assert!(so_b.is_empty());
        assert_eq!(a, json!({"ok": false, "findings": []}));
    }

    #[test]
    fn texto_do_doctor_guarda_o_recuo_alem_do_fixo() {
        let s = "warn   frontmatter-id         p.md\n         msg\nerror  sem-callout            q.md\n           msg2\n\n2 achado(s), 1 pendente(s).\n";
        let (pares, resto) = separar_doctor_texto(s);
        assert_eq!(
            pares,
            vec![
                "warn   frontmatter-id         p.md / msg",
                "error  sem-callout            q.md /   msg2"
            ]
        );
        assert_eq!(resto, vec!["", "2 achado(s), 1 pendente(s)."]);
    }
}
