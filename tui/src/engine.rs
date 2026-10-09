//! A única porta para o motor: o executável `freitask`.
//!
//! A TUI não conhece regra nenhuma de task. Tudo que ela sabe do vault vem do
//! JSON que a CLI imprime, e tudo que ela muda passa por um subcomando — o
//! mesmo contrato que agentes e scripts usam. Uma segunda cópia das regras
//! aqui divergiria do Lua em semanas (ver o cabeçalho de `lua/freitask/cli.lua`).

use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::process::Command;

/// Uma linha de `freitask list --json --archived`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Task {
    pub project: String,
    pub id: String,
    /// Título da fase ("Não iniciada"), não da task.
    pub status: String,
    pub status_num: i64,
    pub callout: String,
    pub title: String,
    pub desc: String,
    pub archived: Option<String>,
    pub suggested_archive: String,
    pub path: String,
}

/// Uma linha de `freitask projects --json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Project {
    pub project: String,
    pub active: usize,
}

/// Uma linha de `freitask statuses --json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Status {
    pub num: i64,
    pub callout: String,
    pub title: String,
    pub icon: String,
    pub hl_group: String,
}

/// Tudo que a tela mostra, lido de uma vez.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Data {
    pub statuses: Vec<Status>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
}

pub struct Engine {
    bin: String,
}

impl Engine {
    /// `FREITASK_BIN` existe para apontar a TUI para outro executável (um
    /// clone em teste, um vault descartável); o normal é o `freitask` do PATH.
    pub fn new() -> Self {
        let bin = std::env::var("FREITASK_BIN")
            .ok()
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "freitask".to_string());
        Engine { bin }
    }

    /// Roda um subcomando e devolve o stdout. Em falha, devolve o stderr —
    /// é lá que o motor explica o porquê, e é isso que vai para a barra.
    pub fn run<S: AsRef<str>>(&self, args: &[S]) -> Result<String, String> {
        let args: Vec<&str> = args.iter().map(|a| a.as_ref()).collect();
        let out = Command::new(&self.bin)
            .args(&args)
            .output()
            .map_err(|e| format!("não consegui rodar `{}`: {e}", self.bin))?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string());
        }
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| l.strip_prefix("freitask: ").unwrap_or(l))
            .collect::<Vec<_>>()
            .join(" · ");
        if err.is_empty() {
            Err(format!(
                "`freitask {}` saiu com {}",
                args.first().unwrap_or(&""),
                out.status.code().map_or("sinal".to_string(), |c| c.to_string())
            ))
        } else {
            Err(err)
        }
    }

    fn json<T: DeserializeOwned>(&self, args: &[&str]) -> Result<T, String> {
        let out = self.run(args)?;
        serde_json::from_str(&out).map_err(|e| format!("`freitask {}` devolveu JSON inesperado: {e}", args.join(" ")))
    }

    pub fn load(&self) -> Result<Data, String> {
        Ok(Data {
            statuses: self.json(&["statuses", "--json"])?,
            projects: self.json(&["projects", "--json"])?,
            tasks: self.json(&["list", "--json", "--archived"])?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_o_contrato_do_list() {
        let raw = r#"[{"project":"p","id":"t1","status":"Em implementação","status_num":3,
            "callout":"example","title":"Um","desc":"","archived":null,
            "suggested_archive":"dropped","path":"/v/projects/p/tasks/t1.md"},
            {"project":"p","id":"t0","status":"Arquivada","status_num":7,"callout":"done",
            "title":"Zero","desc":"x","archived":"done","suggested_archive":"done",
            "path":"/v/projects/p/tasks/archived/done/t0.md"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(raw).unwrap();
        assert_eq!(tasks[0].archived, None);
        assert_eq!(tasks[1].archived.as_deref(), Some("done"));
        assert_eq!(tasks[0].status_num, 3);
    }

    #[test]
    fn campo_a_mais_no_json_nao_quebra() {
        // O contrato pode crescer; a TUI só lê o que conhece.
        let raw = r#"[{"project":"p","active":2,"novo":true}]"#;
        let p: Vec<Project> = serde_json::from_str(raw).unwrap();
        assert_eq!(p[0].active, 2);
    }

    #[test]
    fn falha_devolve_o_stderr_sem_o_prefixo() {
        let e = Engine { bin: "sh".into() };
        let err = e
            .run(&["-c", "echo 'freitask: task não encontrada: x' >&2; exit 1"])
            .unwrap_err();
        assert_eq!(err, "task não encontrada: x");
    }

    #[test]
    fn sucesso_devolve_o_stdout_aparado() {
        let e = Engine { bin: "sh".into() };
        assert_eq!(e.run(&["-c", "echo aviso >&2; echo /a/b.md"]).unwrap(), "/a/b.md");
    }
}
