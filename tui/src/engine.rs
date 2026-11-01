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
    /// "alta" ou nada: o motor só devolve o valor que conta.
    #[serde(default)]
    pub prioridade: Option<String>,
    /// O que o motor manda esconder por padrão: a fase "Algum dia" ou a task
    /// adiada. A regra é dele; a TUI só obedece. Nunca vale para arquivada, que
    /// tem o próprio toggle.
    #[serde(default)]
    pub hidden: bool,
    /// O dia (AAAA-MM-DD) em que a task adiada volta, ou nada: o motor só o
    /// devolve para quem está adiada agora. A TUI mostra o texto ao lado da
    /// task; não lê nem compara data nenhuma.
    #[serde(default)]
    pub volta_em: Option<String>,
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

    /// O que a TUI faz ao abrir: regenera o painel e só então lê tudo. A task
    /// adiada volta sozinha, sem nada gravar nela, então num dia sem escrita o
    /// `CURRENT.md` ficaria com a tela de ontem; abrir a TUI é a ocasião de pôr
    /// o painel em dia. O `rebuild` vem antes do `load` para que as duas
    /// leituras concordem.
    ///
    /// Se o `rebuild` falhar a TUI abre mesmo assim — o que ela mostra vem do
    /// `load`, não do painel — e o motivo volta como aviso para a barra.
    pub fn open(&self) -> Result<(Data, Option<String>), String> {
        let warning = self.run(&["rebuild"]).err().map(|e| format!("rebuild falhou: {e}"));
        Ok((self.load()?, warning))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_o_contrato_do_list() {
        let raw = r#"[{"project":"p","id":"t1","status":"Em implementação","status_num":4,
            "callout":"example","title":"Um","desc":"","archived":null,
            "suggested_archive":"dropped","path":"/v/projects/p/tasks/t1.md",
            "hidden":false,"prioridade":"alta"},
            {"project":"p","id":"t0","status":"Arquivada","status_num":9,"callout":"done",
            "title":"Zero","desc":"x","archived":"done","suggested_archive":"done",
            "path":"/v/projects/p/tasks/archived/done/t0.md"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(raw).unwrap();
        assert_eq!(tasks[0].archived, None);
        assert_eq!(tasks[1].archived.as_deref(), Some("done"));
        assert_eq!(tasks[0].status_num, 4);
        assert_eq!(tasks[0].prioridade.as_deref(), Some("alta"));
        assert!(!tasks[0].hidden);
        // Contrato antigo, sem os campos novos: a TUI segue lendo.
        assert_eq!(tasks[1].prioridade, None);
        assert!(!tasks[1].hidden);
    }

    #[test]
    fn prioridade_nula_e_hidden_verdadeiro() {
        let raw = r#"[{"project":"p","id":"t","status":"Algum dia","status_num":8,
            "callout":"quote","title":"T","desc":"","archived":null,
            "suggested_archive":"dropped","path":"/v/projects/p/tasks/t.md",
            "hidden":true,"prioridade":null}]"#;
        let tasks: Vec<Task> = serde_json::from_str(raw).unwrap();
        assert_eq!(tasks[0].prioridade, None);
        assert!(tasks[0].hidden);
    }

    #[test]
    fn volta_em_chega_so_para_a_adiada() {
        let raw = r#"[{"project":"p","id":"adiada","status":"Refinada","status_num":3,
            "callout":"tip","title":"A","desc":"","archived":null,
            "suggested_archive":"dropped","path":"/v/projects/p/tasks/adiada.md",
            "hidden":true,"prioridade":null,"volta_em":"2026-12-01"},
            {"project":"p","id":"normal","status":"Refinada","status_num":3,
            "callout":"tip","title":"N","desc":"","archived":null,
            "suggested_archive":"dropped","path":"/v/projects/p/tasks/normal.md",
            "hidden":false,"prioridade":null,"volta_em":null},
            {"project":"p","id":"antigo","status":"Refinada","status_num":3,
            "callout":"tip","title":"O","desc":"","archived":null,
            "suggested_archive":"dropped","path":"/v/projects/p/tasks/antigo.md"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(raw).unwrap();
        assert_eq!(tasks[0].volta_em.as_deref(), Some("2026-12-01"));
        assert!(tasks[0].hidden);
        assert_eq!(tasks[1].volta_em, None);
        // Contrato antigo, sem o campo: a TUI segue lendo.
        assert_eq!(tasks[2].volta_em, None);
    }

    #[test]
    fn abrir_regenera_o_painel_antes_de_ler_e_nao_aborta_se_ele_falhar() {
        // Um `freitask` de mentira (tests/fake-freitask.sh) que anota cada chamada
        // e responde o mínimo. Fica num arquivo versionado, e não gerado aqui, para
        // o teste não correr o risco de executar um arquivo recém-escrito.
        let dir = std::env::temp_dir().join(format!("freitask-tui-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("chamadas");
        std::env::set_var("FAKE_FREITASK_LOG", &log);
        let e = Engine {
            bin: format!("{}/tests/fake-freitask.sh", env!("CARGO_MANIFEST_DIR")),
        };
        let chamadas = || -> Vec<String> {
            std::fs::read_to_string(&log)
                .unwrap()
                .lines()
                .map(String::from)
                .collect()
        };

        std::env::remove_var("FAKE_FREITASK_REBUILD");
        let (data, aviso) = e.open().unwrap();
        assert_eq!(aviso, None);
        assert!(data.tasks.is_empty());
        assert_eq!(
            chamadas(),
            [
                "rebuild",
                "statuses --json",
                "projects --json",
                "list --json --archived"
            ],
            "o rebuild vem primeiro, e depois as três leituras"
        );

        // Rebuild que falha: a TUI abre, lê tudo, e o motivo vira aviso.
        std::fs::remove_file(&log).unwrap();
        std::env::set_var("FAKE_FREITASK_REBUILD", "falha");
        let (_, aviso) = e.open().unwrap();
        let aviso = aviso.expect("o rebuild falhou: tem de haver aviso");
        assert!(aviso.starts_with("rebuild falhou: "), "{aviso}");
        assert!(aviso.contains("CURRENT.md tem alterações"), "{aviso}");
        assert_eq!(chamadas().len(), 4, "as leituras rodaram mesmo assim");

        std::env::remove_var("FAKE_FREITASK_REBUILD");
        std::fs::remove_dir_all(&dir).unwrap();
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
