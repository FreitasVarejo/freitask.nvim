//! `freitask doctor [--fix] [--json] [--quiet]`. Sem `--fix` é leitura pura
//! (I6, D7): não toma o lock, não garante a raiz, não grava nada. Com `--fix`
//! toma o lock, garante a raiz, executa o plano do `core` com CAS e regenera
//! o painel, como hoje. A saída é a de hoje, byte a byte.

use std::io::Write;
use std::process::ExitCode;

use freitask_core::date::Date;
use freitask_core::doctor::{self, Achado, Diagnostico};
use freitask_vault::Vault;

use crate::cli::{falha, rebuild, say, travar};
use crate::escrita::{vault, vocab};

/// Um achado em texto: `%-6s %-22s %s\n         %s%s` do `cli.lua`.
pub fn texto_do_achado(a: &Achado) -> String {
    format!(
        "{:<6} {:<22} {}\n         {}{}\n",
        a.nivel.como_str(),
        a.kind,
        a.path,
        a.msg,
        if a.fixed { "  [reparado]" } else { "" }
    )
}

/// O texto inteiro: os achados, a contagem e a dica do `--fix` quando sobra
/// pendência e ele não foi pedido; ou `vault consistente`.
pub fn texto(d: &Diagnostico, fix: bool) -> String {
    if d.achados.is_empty() {
        return "freitask: vault consistente.\n".into();
    }
    let mut out: String = d.achados.iter().map(texto_do_achado).collect();
    let pendentes = d.pendentes();
    out.push_str(&format!(
        "\n{} achado(s), {pendentes} pendente(s).\n",
        d.achados.len()
    ));
    if pendentes > 0 && !fix {
        out.push_str("Repare o que for automático com: freitask doctor --fix\n");
    }
    out
}

/// O JSON de hoje: `{ok, findings:[{level, kind, path, msg, fixed}]}`. `ok` é
/// "não houve achado", mesmo reparado.
pub fn json(d: &Diagnostico) -> serde_json::Value {
    let findings: Vec<serde_json::Value> = d
        .achados
        .iter()
        .map(|a| {
            serde_json::json!({
                "level": a.nivel.como_str(),
                "kind": a.kind,
                "path": a.path,
                "msg": a.msg,
                "fixed": a.fixed,
            })
        })
        .collect();
    serde_json::json!({ "ok": d.achados.is_empty(), "findings": findings })
}

pub fn doctor(fix: bool, como_json: bool, quieto: bool, hoje: Date) -> ExitCode {
    let _trava = if fix {
        match travar() {
            Ok(t) => Some(t),
            Err(c) => return c,
        }
    } else {
        None
    };
    let vault = match vault() {
        Ok(v) => v,
        Err(c) => return c,
    };
    if fix && let Err(e) = vault.garantir_raiz() {
        return falha(&e);
    }
    let vocab = vocab(&vault);
    let d = doctor::diagnosticar(&vocab, &vault.fotografia_do_doctor(), fix, hoje);
    if fix && let Err(c) = reparar(&vault, &d, hoje) {
        return c;
    }
    if como_json {
        say(&json(&d).to_string());
    } else if !quieto {
        print!("{}", texto(&d, fix));
        let _ = std::io::stdout().flush();
    }
    if d.pendentes() > 0 {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Executa o plano do reparo e regenera o painel (sempre, como hoje).
fn reparar(vault: &Vault, d: &Diagnostico, hoje: Date) -> Result<(), ExitCode> {
    vault.aplicar(&d.plano).map_err(|e| falha(&e))?;
    rebuild(vault, hoje).map_err(|e| falha(&e))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use freitask_core::doctor::Nivel;

    fn achado(kind: &'static str, fixed: bool) -> Achado {
        Achado {
            nivel: Nivel::Warn,
            kind,
            path: "projects/p/tasks/x.md".into(),
            msg: "m".into(),
            fixed,
        }
    }

    #[test]
    fn texto_com_e_sem_achado_e_com_e_sem_dica() {
        let vazio = Diagnostico {
            achados: vec![],
            plano: vec![],
        };
        assert_eq!(texto(&vazio, false), "freitask: vault consistente.\n");
        assert_eq!(texto(&vazio, true), "freitask: vault consistente.\n");
        let d = Diagnostico {
            achados: vec![
                achado("status-0", false),
                achado("prioridade-desconhecida", true),
            ],
            plano: vec![],
        };
        let corpo = concat!(
            "warn   status-0               projects/p/tasks/x.md\n",
            "         m\n",
            "warn   prioridade-desconhecida projects/p/tasks/x.md\n",
            "         m  [reparado]\n",
            "\n",
            "2 achado(s), 1 pendente(s).\n",
        );
        assert_eq!(
            texto(&d, false),
            format!("{corpo}Repare o que for automático com: freitask doctor --fix\n")
        );
        assert_eq!(texto(&d, true), corpo);
        let tudo = Diagnostico {
            achados: vec![achado("status-0", true)],
            plano: vec![],
        };
        assert!(texto(&tudo, false).ends_with("1 achado(s), 0 pendente(s).\n"));
    }

    #[test]
    fn json_tem_ok_e_findings_mesmo_vazio() {
        let vazio = Diagnostico {
            achados: vec![],
            plano: vec![],
        };
        assert_eq!(
            json(&vazio),
            serde_json::json!({"ok": true, "findings": []})
        );
        let d = Diagnostico {
            achados: vec![achado("status-0", true)],
            plano: vec![],
        };
        assert_eq!(
            json(&d),
            serde_json::json!({"ok": false, "findings": [{
                "level": "warn", "kind": "status-0",
                "path": "projects/p/tasks/x.md", "msg": "m", "fixed": true
            }]})
        );
    }
}
