//! A CLI: clap com `derive`, um `enum` de subcomandos, saída texto e `--json`,
//! códigos de saída (spec §2). Nenhuma regra mora aqui: o `core` decide, o
//! `vault` executa, e esta camada traduz argumentos em chamadas e valores em
//! texto, byte a byte como o `cli.lua` de hoje.
//!
//! Códigos: 0 ok · 1 recusa do motor · 2 uso inválido · 3 conflito de escrita.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use freitask_core::date::Date;
use freitask_core::operacao::PedidoSet;
use freitask_vault::{ErroEscrita, Vault};

use crate::escrita;
use crate::linhas;
use crate::usage;

#[derive(Parser, Debug)]
#[command(
    name = "freitask",
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
#[command(disable_version_flag = true)]
enum Cmd {
    /// Abre a TUI.
    Tui,
    /// Abre o CURRENT.md no Neovim.
    Current,
    Doctor {
        #[arg(long)]
        fix: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        quiet: bool,
    },
    Archive {
        #[arg(name = "ref")]
        referencia: Option<String>,
        tipo: Option<String>,
    },
    Unarchive {
        #[arg(name = "ref")]
        referencia: Option<String>,
    },
    Rename {
        #[arg(name = "ref")]
        referencia: Option<String>,
        #[arg(name = "novo-id")]
        novo_id: Option<String>,
    },
    Move {
        #[arg(name = "ref")]
        referencia: Option<String>,
        #[arg(name = "projeto")]
        projeto: Option<String>,
    },
    Encerrar {
        #[arg(name = "projeto")]
        projeto: Option<String>,
    },
    Reabrir {
        #[arg(name = "projeto")]
        projeto: Option<String>,
    },
    New {
        #[arg(name = "projeto")]
        projeto: Option<String>,
        id: Option<String>,
        #[arg(name = "título")]
        titulo: Option<String>,
        #[arg(long, num_args = 0..=1)]
        desc: Option<Option<String>>,
        #[arg(long)]
        json: bool,
    },
    Rebuild,
    Set {
        #[arg(name = "ref")]
        referencia: Option<String>,
        #[arg(long, num_args = 0..=1)]
        status: Option<Option<String>>,
        #[arg(long, num_args = 0..=1)]
        title: Option<Option<String>>,
        #[arg(long, num_args = 0..=1)]
        desc: Option<Option<String>>,
        #[arg(long = "volta-em", num_args = 0..=1)]
        volta_em: Option<Option<String>>,
        #[arg(long, num_args = 0..=1)]
        prazo: Option<Option<String>>,
        #[arg(long = "bloqueada-por", num_args = 0..=1)]
        bloqueada_por: Option<Option<String>>,
    },
    Show {
        #[arg(name = "ref")]
        referencia: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Delete {
        #[arg(name = "ref")]
        referencia: Option<String>,
    },
    #[command(name = "new-project")]
    NewProject {
        nome: Option<String>,
    },
    Kebab {
        /// Texto que começa com `-` também é texto (`--foo--bar--`): não há
        /// flag em `kebab` além de `--help`.
        #[arg(allow_hyphen_values = true)]
        texto: Option<String>,
    },
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        archived: bool,
    },
    Projects {
        #[arg(long)]
        json: bool,
    },
    Statuses {
        #[arg(long)]
        json: bool,
    },
    Version {
        #[arg(long)]
        json: bool,
    },
    Help {
        #[arg(long)]
        json: bool,
    },
    #[command(external_subcommand)]
    Outro(Vec<OsString>),
}

/// Subcomandos que escrevem no vault (para `help --json` e para o lock).
const ESCREVEM: &[&str] = &[
    "new",
    "set",
    "archive",
    "unarchive",
    "rename",
    "move",
    "encerrar",
    "reabrir",
    "delete",
    "new-project",
    "rebuild",
];

pub fn say(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(s.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

pub fn die(msg: &str, code: u8) -> ExitCode {
    let _ = std::io::stdout().flush();
    eprintln!("freitask: {msg}");
    ExitCode::from(code)
}

/// O dia de hoje: `FREITASK_TODAY=AAAA-MM-DD` (só os testes) ou o relógio local.
pub fn hoje() -> Result<Date, String> {
    if let Some(v) = std::env::var_os("FREITASK_TODAY") {
        let s = v.to_string_lossy();
        return Date::parse(&s)
            .ok_or_else(|| format!("FREITASK_TODAY inválida: {s} (use AAAA-MM-DD)"));
    }
    let agora = chrono::Local::now().date_naive();
    use chrono::Datelike;
    Date::new(agora.year(), agora.month(), agora.day())
        .ok_or_else(|| "relógio local devolveu uma data inválida".into())
}

/// O comando clap com o `--help` de cada subcomando trocado pelo trecho do
/// usage de hoje (D8): o clap resolve `-h`/`--help` antes de qualquer vault.
fn comando() -> clap::Command {
    let mut cmd = Cli::command();
    let nomes: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for nome in nomes {
        let trecho = usage::trecho(&nome);
        cmd = cmd.mut_subcommand(&nome, move |s| {
            s.override_help(trecho).disable_version_flag(true)
        });
    }
    cmd
}

pub fn run(args: Vec<OsString>) -> ExitCode {
    // `freitask -h` / `freitask --help` como primeiro argumento: o usage, saída 0
    // (como `cli.lua:611`). O `--help` de cada subcomando fica com o clap.
    if matches!(args.first().and_then(|a| a.to_str()), Some("-h" | "--help")) {
        println!("{}", usage::USAGE);
        return ExitCode::SUCCESS;
    }
    let matches = match comando()
        .try_get_matches_from(std::iter::once(OsString::from("freitask")).chain(args))
    {
        Ok(m) => m,
        Err(e) => {
            return match e.kind() {
                clap::error::ErrorKind::DisplayHelp => {
                    let texto = e.to_string();
                    print!("{texto}");
                    let _ = std::io::stdout().flush();
                    ExitCode::SUCCESS
                }
                _ => {
                    // Flag desconhecida, valor faltando: a mensagem do clap, saída 2.
                    let _ = e.print();
                    ExitCode::from(2)
                }
            };
        }
    };
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(2);
        }
    };
    // `FREITASK_TODAY` é lido uma vez, aqui, e vale para todo o comando: um
    // valor inválido é erro de uso antes de qualquer leitura do vault.
    let hoje = match hoje() {
        Ok(d) => d,
        Err(e) => return die(&e, 2),
    };
    let Some(cmd) = cli.cmd else {
        return sem_argumentos();
    };
    despachar(cmd, hoje)
}

/// `freitask` sem argumentos: a TUI num terminal; senão o usage com saída 2.
fn sem_argumentos() -> ExitCode {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        return tui();
    }
    println!("{}", usage::USAGE);
    ExitCode::from(2)
}

fn tui() -> ExitCode {
    die("a TUI ainda não foi portada (fatia 7)", 2)
}

fn despachar(cmd: Cmd, hoje: Date) -> ExitCode {
    match cmd {
        Cmd::Help { json: false } => {
            println!("{}", usage::USAGE);
            ExitCode::SUCCESS
        }
        Cmd::Help { json: true } => {
            say(&gramatica().to_string());
            ExitCode::SUCCESS
        }
        Cmd::Version { json } => {
            if json {
                say(&serde_json::json!({ "freitask": freitask_core::config::VERSAO, "json": freitask_core::config::JSON_VERSAO }).to_string());
            } else {
                say(&format!("freitask {}", freitask_core::config::VERSAO));
            }
            ExitCode::SUCCESS
        }
        Cmd::Kebab { texto } => {
            let Some(texto) = texto else {
                return die("falta o texto", 2);
            };
            // Com `allow_hyphen_values`, `--help` chegaria aqui como texto; a
            // ajuda continua sendo ajuda (D8).
            if texto == "--help" || texto == "-h" {
                print!("{}", usage::trecho("kebab"));
                let _ = std::io::stdout().flush();
                return ExitCode::SUCCESS;
            }
            let id = freitask_core::path::kebab(&texto);
            if id.is_empty() {
                return die(&format!("o texto não rende um id: {texto}"), 1);
            }
            say(&id);
            ExitCode::SUCCESS
        }
        Cmd::Statuses { json } => {
            let vault = match Vault::do_home() {
                Ok(v) => v,
                Err(e) => return die(&e, 2),
            };
            let (vocab, aviso) = vault.vocab();
            if let Some(a) = aviso {
                eprintln!("{a}");
            }
            let linhas = vocab.linhas();
            if json {
                let rows: Vec<serde_json::Value> = linhas
                    .iter()
                    .map(|(n, m, hidden)| {
                        serde_json::json!({
                            "num": n, "callout": m.callout, "title": m.title,
                            "icon": m.icon, "hl_group": m.hl_group, "hidden": hidden,
                        })
                    })
                    .collect();
                say(&serde_json::Value::Array(rows).to_string());
            } else {
                let mut out = String::new();
                for (n, m, _) in &linhas {
                    out.push_str(&format!(
                        "{n}  {} {}\n",
                        freitask_core::pad_bytes(&m.callout, 10),
                        m.title
                    ));
                }
                print!("{out}");
                let _ = std::io::stdout().flush();
            }
            ExitCode::SUCCESS
        }
        Cmd::List { json, archived } => {
            let vault = match Vault::do_home() {
                Ok(v) => v,
                Err(e) => return die(&e, 2),
            };
            let (vocab, aviso) = vault.vocab();
            if let Some(a) = aviso {
                eprintln!("{a}");
            }
            let indice = vault.indice(&vocab);
            let linhas = linhas::todas(&indice, archived);
            if json {
                let rows: Vec<serde_json::Value> = linhas
                    .iter()
                    .map(|e| linhas::linha(&vault, &vocab, &indice, e, hoje))
                    .collect();
                say(&serde_json::Value::Array(rows).to_string());
            } else {
                let mut out = String::new();
                for e in &linhas {
                    out.push_str(&linhas::texto(&vocab, e));
                    out.push('\n');
                }
                print!("{out}");
                let _ = std::io::stdout().flush();
            }
            ExitCode::SUCCESS
        }
        Cmd::Show { referencia, json } => {
            let Some(referencia) = referencia else {
                return die("falta o id ou caminho da task", 2);
            };
            let vault = match Vault::do_home() {
                Ok(v) => v,
                Err(e) => return die(&e, 2),
            };
            let (vocab, aviso) = vault.vocab();
            if let Some(a) = aviso {
                eprintln!("{a}");
            }
            let indice = vault.indice(&vocab);
            let rel = match vault.find_task(&indice, &referencia) {
                Ok(r) => r,
                Err(e) => return die(&e, 1),
            };
            let Some(e) = indice.por_rel(&rel) else {
                return die(
                    &format!("não é um arquivo de task: {}", vault.caminho_str(&rel)),
                    1,
                );
            };
            let ciclo = freitask_core::deps::ciclo(&indice, &e.task_id);
            if json {
                let mut v = linhas::linha(&vault, &vocab, &indice, e, hoje);
                v["ciclo"] = serde_json::json!(ciclo);
                say(&v.to_string());
            } else {
                let mut out = String::new();
                for l in &e.block {
                    out.push_str(l);
                    out.push('\n');
                }
                let deps = freitask_core::deps::dependencias(&indice, e);
                if !e.meta.bloqueada_por.is_empty() {
                    out.push_str(&format!(
                        "bloqueada por: {}\n",
                        e.meta.bloqueada_por.join(", ")
                    ));
                }
                if !deps.bloqueia.is_empty() {
                    out.push_str(&format!(
                        "bloqueia: {} ({})\n",
                        deps.bloqueia.len(),
                        deps.bloqueia.join(", ")
                    ));
                }
                if let Some(p) = &e.meta.prazo {
                    match freitask_core::meta::dias_para_o_prazo(&e.meta, hoje) {
                        Some(d) if d < 0 => {
                            out.push_str(&format!("prazo: {p} (vencido há {} dias)\n", -d))
                        }
                        Some(d) => out.push_str(&format!("prazo: {p} (em {d} dias)\n")),
                        None => out.push_str(&format!("prazo: {p} (inválido)\n")),
                    }
                }
                print!("{out}");
                let _ = std::io::stdout().flush();
            }
            if let Some(c) = ciclo {
                eprintln!("CICLO: {}", c.join(" → "));
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Cmd::Projects { json } => {
            let vault = match Vault::do_home() {
                Ok(v) => v,
                Err(e) => return die(&e, 2),
            };
            let (vocab, aviso) = vault.vocab();
            if let Some(a) = aviso {
                eprintln!("{a}");
            }
            let indice = vault.indice(&vocab);
            let rows: Vec<(String, usize)> = indice
                .projetos
                .iter()
                .map(|p| (p.clone(), indice.mostradas(&vocab, p, hoje)))
                .collect();
            if json {
                let v: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|(p, n)| serde_json::json!({ "project": p, "active": n }))
                    .collect();
                say(&serde_json::Value::Array(v).to_string());
            } else {
                let mut out = String::new();
                for (p, n) in &rows {
                    out.push_str(&format!("{} {n}\n", freitask_core::pad_bytes(p, 14)));
                }
                print!("{out}");
                let _ = std::io::stdout().flush();
            }
            ExitCode::SUCCESS
        }
        Cmd::Rebuild => {
            let _trava = match travar() {
                Ok(t) => t,
                Err(c) => return c,
            };
            let vault = match Vault::do_home() {
                Ok(v) => v,
                Err(e) => return die(&e, 2),
            };
            match rebuild(&vault, hoje) {
                Ok(()) => {
                    say(&vault.caminho_str("CURRENT.md"));
                    ExitCode::SUCCESS
                }
                Err(e) => falha(&e),
            }
        }
        Cmd::New {
            projeto,
            id,
            titulo,
            desc,
            json,
        } => {
            let desc = match valor("desc", desc) {
                Ok(d) => d,
                Err(c) => return c,
            };
            let Some(projeto) = projeto else {
                return die("falta o projeto", 2);
            };
            let Some(id) = id else {
                return die("falta o id", 2);
            };
            let Some(titulo) = titulo else {
                return die("falta o título", 2);
            };
            if json {
                return die("`new --json` ainda não foi portado nesta fatia", 2);
            }
            escrita::new(&projeto, &id, &titulo, desc.as_deref().unwrap_or(""), hoje)
        }
        Cmd::NewProject { nome } => {
            let Some(nome) = nome else {
                return die("falta o nome do projeto", 2);
            };
            escrita::new_project(&nome)
        }
        Cmd::Set {
            referencia,
            status,
            title,
            desc,
            volta_em,
            prazo,
            bloqueada_por,
        } => {
            let mut pedido = PedidoSet::default();
            for (flag, alvo, valor_cru) in [
                ("status", &mut pedido.status, status),
                ("title", &mut pedido.title, title),
                ("desc", &mut pedido.desc, desc),
                ("volta-em", &mut pedido.volta_em, volta_em),
            ] {
                match valor(flag, valor_cru) {
                    Ok(v) => *alvo = v,
                    Err(c) => return c,
                }
            }
            if prazo.is_some() || bloqueada_por.is_some() {
                return die(
                    "`--prazo` e `--bloqueada-por` ainda não foram portados nesta fatia",
                    2,
                );
            }
            let Some(referencia) = referencia else {
                return die("falta o id ou caminho da task", 2);
            };
            if pedido.vazio() {
                return die(
                    "nada a mudar: passe --status, --title, --desc e/ou --volta-em",
                    2,
                );
            }
            escrita::set(&referencia, &pedido, hoje)
        }
        Cmd::Delete { referencia } => {
            let Some(referencia) = referencia else {
                return die("falta o id ou caminho da task", 2);
            };
            escrita::delete(&referencia, hoje)
        }
        Cmd::Tui => tui(),
        Cmd::Current => die("`current` ainda não foi portado (fatia 6)", 2),
        Cmd::Outro(args) => {
            let nome = args
                .first()
                .map(|a| a.to_string_lossy().to_string())
                .unwrap_or_default();
            die(
                &format!("comando desconhecido: {nome}\n\n{}", usage::USAGE),
                2,
            )
        }
        outro => die(
            &format!("`{}` ainda não foi portado nesta fatia", nome_do(&outro)),
            2,
        ),
    }
}

/// Regenera o `CURRENT.md`: garante a raiz (como o `ensure_root` de hoje),
/// relê o vocabulário e o índice, arquiva o painel de outro dia em `daily/`
/// e grava o novo só se mudou (§1.5), com CAS contra o painel que leu.
pub fn rebuild(vault: &Vault, hoje: Date) -> Result<(), ErroEscrita> {
    vault.garantir_raiz()?;
    let (vocab, aviso) = vault.vocab();
    if let Some(a) = aviso {
        eprintln!("{a}");
    }
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

/// A falha de uma gravação: a mensagem e o código (3 para conflito).
pub fn falha(e: &ErroEscrita) -> ExitCode {
    die(&e.to_string(), e.codigo())
}

/// Toma o lock de escrita, ou devolve a saída (3 se ficou ocupado).
pub fn travar() -> Result<freitask_vault::trava::Trava, ExitCode> {
    Vault::travar().map_err(|e| die(&e.to_string(), e.codigo()))
}

/// O valor de uma flag que leva valor: `--x` sozinho é `falta o valor de --x`
/// (saída 2), como o `parse_flags` de hoje.
fn valor(flag: &str, v: Option<Option<String>>) -> Result<Option<String>, ExitCode> {
    match v {
        Some(None) => Err(die(&format!("falta o valor de --{flag}"), 2)),
        Some(Some(s)) => Ok(Some(s)),
        None => Ok(None),
    }
}

fn nome_do(cmd: &Cmd) -> &'static str {
    match cmd {
        Cmd::Tui => "tui",
        Cmd::Current => "current",
        Cmd::Doctor { .. } => "doctor",
        Cmd::Archive { .. } => "archive",
        Cmd::Unarchive { .. } => "unarchive",
        Cmd::Rename { .. } => "rename",
        Cmd::Move { .. } => "move",
        Cmd::Encerrar { .. } => "encerrar",
        Cmd::Reabrir { .. } => "reabrir",
        Cmd::New { .. } => "new",
        Cmd::Rebuild => "rebuild",
        Cmd::Set { .. } => "set",
        Cmd::Show { .. } => "show",
        Cmd::Delete { .. } => "delete",
        Cmd::NewProject { .. } => "new-project",
        Cmd::Kebab { .. } => "kebab",
        Cmd::List { .. } => "list",
        Cmd::Projects { .. } => "projects",
        Cmd::Statuses { .. } => "statuses",
        Cmd::Version { .. } => "version",
        Cmd::Help { .. } => "help",
        Cmd::Outro(_) => "?",
    }
}

/// A gramática dos subcomandos, gerada das definições do clap (`help --json`,
/// spec §2.3): quais escrevem, os posicionais, as flags com e sem valor.
pub fn gramatica() -> serde_json::Value {
    let cmd = Cli::command();
    let mut subs = serde_json::Map::new();
    for s in cmd.get_subcommands() {
        let nome = s.get_name();
        if nome.is_empty() {
            continue;
        }
        let mut positionals = Vec::new();
        let mut value_flags = Vec::new();
        let mut bool_flags = Vec::new();
        for a in s.get_arguments() {
            if a.is_positional() {
                positionals.push(a.get_id().to_string());
            } else if let Some(l) = a.get_long() {
                if l == "help" {
                    continue;
                }
                let leva_valor = a.get_num_args().is_some_and(|n| n.max_values() > 0);
                if leva_valor {
                    value_flags.push(l.to_string());
                } else {
                    bool_flags.push(l.to_string());
                }
            }
        }
        subs.insert(
            nome.to_string(),
            serde_json::json!({
                "writes": ESCREVEM.contains(&nome),
                "positionals": positionals,
                "value_flags": value_flags,
                "bool_flags": bool_flags,
            }),
        );
    }
    serde_json::json!({ "json": freitask_core::config::JSON_VERSAO, "subcommands": subs })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_definicao_do_clap_e_consistente() {
        Cli::command().debug_assert();
    }

    #[test]
    fn gramatica_diz_quem_escreve_e_que_flag_leva_valor() {
        let g = gramatica();
        let set = &g["subcommands"]["set"];
        assert_eq!(set["writes"], true);
        assert_eq!(set["positionals"], serde_json::json!(["ref"]));
        let vf: Vec<&str> = set["value_flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            vf,
            [
                "status",
                "title",
                "desc",
                "volta-em",
                "prazo",
                "bloqueada-por"
            ]
        );
        let list = &g["subcommands"]["list"];
        assert_eq!(list["writes"], false);
        assert_eq!(list["bool_flags"], serde_json::json!(["json", "archived"]));
        assert_eq!(g["json"], 1);
        assert!(g["subcommands"].get("help").is_some());
    }

    #[test]
    fn hoje_le_a_variavel_ou_recusa_o_que_nao_e_data() {
        // SAFETY: `set_var` é inseguro porque outra thread pode estar lendo o
        // ambiente; aqui é o único teste da crate que toca `FREITASK_TODAY`, e
        // nenhum outro lê o ambiente, então não há leitor concorrente.
        unsafe { std::env::set_var("FREITASK_TODAY", "2026-11-08") };
        assert_eq!(hoje().unwrap().to_string(), "2026-11-08");
        unsafe { std::env::set_var("FREITASK_TODAY", "ontem") };
        assert!(hoje().is_err());
        unsafe { std::env::remove_var("FREITASK_TODAY") };
        assert!(hoje().is_ok());
    }
}
