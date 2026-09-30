//! freitask-tui — navegar e mexer nas tasks do vault sem abrir o Neovim.
//!
//! É só tela: tudo que ela lê e escreve passa pelo executável `freitask` (ver
//! `engine.rs`). O shim `freitask` do dotfiles abre esta TUI quando é chamado
//! sem argumentos num terminal.
//!
//! Este arquivo é o laço: lê tecla, pede ao `App` o efeito, executa o efeito
//! (processo, editor, disco) e redesenha.

mod app;
mod engine;
mod ui;

use app::{After, App, Effect};
use engine::Engine;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::io;
use std::process::{Command, ExitCode};

const USAGE: &str = "freitask-tui — TUI do freitask

Uso: freitask-tui        (ou `freitask` sem argumentos, num terminal)

Fala com o motor só pela CLI `freitask` do PATH; FREITASK_BIN aponta para
outro executável. `?` dentro da TUI lista as teclas.";

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let engine = Engine::new();
    let data = match engine.load() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("freitask-tui: {e}");
            return ExitCode::from(1);
        }
    };
    let mut app = App::new(data);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &engine);
    ratatui::restore();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("freitask-tui: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, engine: &Engine) -> io::Result<()> {
    loop {
        if let Some(path) = app.preview_wanted() {
            let content = std::fs::read_to_string(&path).unwrap_or_else(|e| format!("(não consegui ler: {e})"));
            app.set_preview(path, content);
        }
        terminal.draw(|f| ui::draw(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match app.handle_key(key) {
            Effect::None => {}
            Effect::Quit => return Ok(()),
            Effect::Reload => {
                if reload(app, engine, &After::Keep, "") {
                    app.info("relido");
                }
            }
            Effect::Kebab(title) => {
                if let Ok(id) = engine.run(&["kebab", &title]) {
                    app.suggest_id(id);
                }
            }
            Effect::Run { args, after, ok } => match engine.run(&args) {
                Ok(out) => {
                    app.run_ok(ok);
                    reload(app, engine, &after, &out);
                }
                Err(e) => app.run_failed(e),
            },
            Effect::Open(path) => {
                if open(terminal, app, engine, &path)? {
                    return Ok(());
                }
            }
            Effect::OpenCurrent => match engine.run(&["rebuild"]) {
                Ok(path) => {
                    if open(terminal, app, engine, &path)? {
                        return Ok(());
                    }
                }
                Err(e) => app.error(e),
            },
        }
    }
}

/// Relê tudo do motor. Devolve falso (e deixa o erro na barra) se falhar.
fn reload(app: &mut App, engine: &Engine, after: &After, output: &str) -> bool {
    match engine.load() {
        Ok(d) => {
            app.set_data(d, after, output);
            true
        }
        Err(e) => {
            app.error(e);
            false
        }
    }
}

/// Abre um arquivo. Devolve verdadeiro quando a TUI deve sair.
///
/// Dentro de um terminal do Neovim (`$NVIM` é o socket do Neovim pai), abrir
/// outro Neovim aqui dentro empilharia um editor dentro do outro. Então o
/// arquivo é aberto NO PAI, e a TUI sai — é o que o lazygit faz no LazyVim.
/// Fora do Neovim, o `$EDITOR` roda em primeiro plano e a TUI volta depois.
fn open(terminal: &mut DefaultTerminal, app: &mut App, engine: &Engine, path: &str) -> io::Result<bool> {
    if let Some(sock) = std::env::var("NVIM").ok().filter(|s| !s.is_empty()) {
        let out = Command::new("nvim").args(["--server", &sock, "--remote-expr", &remote_open_expr(path)]).output();
        return Ok(match out {
            Ok(o) if o.status.success() => true,
            Ok(o) => {
                app.error(format!("nvim --remote-expr falhou: {}", String::from_utf8_lossy(&o.stderr).trim()));
                false
            }
            Err(e) => {
                app.error(format!("não consegui rodar nvim: {e}"));
                false
            }
        });
    }

    let editor = ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|e| !e.trim().is_empty())
        .unwrap_or_else(|| "nvim".to_string());
    let mut parts = editor.split_whitespace();
    let bin = parts.next().unwrap_or("nvim").to_string();

    ratatui::restore();
    let status = Command::new(&bin).args(parts).arg(path).status();
    *terminal = ratatui::init();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => app.error(format!("{bin} saiu com {s}")),
        Err(e) => app.error(format!("não consegui rodar {bin}: {e}")),
    }
    // O editor pode não ser um Neovim com o autocmd do freitask: o painel é
    // regenerado aqui para não ficar para trás da edição.
    let _ = engine.run(&["rebuild"]);
    reload(app, engine, &After::Keep, "");
    Ok(false)
}

/// A expressão Vim que o Neovim pai avalia para abrir `path`.
///
/// `--remote-expr` e não `--remote-send`: teclas mandadas a um buffer de
/// terminal iriam para o processo dentro dele, não para o Neovim. O código
/// roda agendado (vim.schedule) para não fechar janela no meio da chamada RPC:
/// se a janela atual é flutuante (o Snacks.terminal do atalho), ela é fechada
/// e o arquivo abre na de baixo; senão, abre na janela anterior.
fn remote_open_expr(path: &str) -> String {
    let lua = format!(
        "(function(p) vim.schedule(function() \
           local w = vim.api.nvim_get_current_win() \
           if vim.api.nvim_win_get_config(w).relative ~= \"\" then pcall(vim.api.nvim_win_close, w, true) \
           else vim.cmd(\"wincmd p\") end \
           vim.cmd.edit(vim.fn.fnameescape(p)) \
         end) end)({})",
        lua_string(path)
    );
    // String Vim entre aspas simples: a única coisa a escapar é a própria aspa.
    format!("luaeval('{}')", lua.replace('\'', "''"))
}

fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_string_escapa_aspas_e_barra() {
        assert_eq!(lua_string(r#"/a/"b"\c.md"#), r#""/a/\"b\"\\c.md""#);
    }

    #[test]
    fn aspa_simples_no_caminho_e_dobrada_para_o_vim() {
        let e = remote_open_expr("/v/it's.md");
        assert!(e.starts_with("luaeval('"));
        assert!(e.contains("it''s.md"));
    }
}
