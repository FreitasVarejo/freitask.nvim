//! O terminal de verdade em volta da `Sessao`: lê tecla, entrega à sessão,
//! redesenha, e executa o que ela pede (sair, abrir no editor).

use std::io;
use std::path::Path;
use std::process::Command;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::tui::sessao::{Pedido, Sessao};

/// Roda a TUI até `q`/`Ctrl-C` (ou até abrir um arquivo no Neovim pai).
/// Devolve o terminal como estava, inclusive em erro.
pub fn rodar(sessao: &mut Sessao) -> io::Result<()> {
    // `try_init`, e não `init`: terminal que não abre é erro (saída 1,
    // decisão f7.3), não pânico.
    let resultado = ratatui::try_init().and_then(|mut terminal| laco(&mut terminal, sessao));
    ratatui::restore();
    resultado
}

fn laco(terminal: &mut DefaultTerminal, sessao: &mut Sessao) -> io::Result<()> {
    loop {
        terminal.draw(|f| sessao.desenhar(f))?;
        match event::read()? {
            Event::Key(tecla) if tecla.kind == KeyEventKind::Press => match sessao.tecla(tecla) {
                Pedido::Nada => {}
                Pedido::Sair => return Ok(()),
                Pedido::Abrir(caminho) => {
                    if abrir(terminal, sessao, &caminho)? {
                        return Ok(());
                    }
                }
            },
            Event::Resize(largura, altura) => sessao.redimensionou(largura, altura),
            _ => {}
        }
    }
}

/// Abre um arquivo. Devolve verdadeiro quando a TUI deve sair.
///
/// Dentro de um terminal do Neovim (`$NVIM` é o socket do Neovim pai), abrir
/// outro Neovim aqui dentro empilharia um editor dentro do outro. Então o
/// arquivo é aberto NO PAI, e a TUI sai — é o que o lazygit faz no LazyVim.
/// Fora do Neovim, o `$VISUAL`/`$EDITOR` roda em primeiro plano e a TUI volta
/// depois.
fn abrir(terminal: &mut DefaultTerminal, sessao: &mut Sessao, caminho: &Path) -> io::Result<bool> {
    let caminho = caminho.to_string_lossy();
    if let Some(socket) = std::env::var("NVIM").ok().filter(|s| !s.is_empty()) {
        let saida = Command::new("nvim")
            .args([
                "--server",
                &socket,
                "--remote-expr",
                &expressao_de_abrir(&caminho),
            ])
            .output();
        return Ok(match saida {
            Ok(o) if o.status.success() => true,
            Ok(o) => {
                sessao.avisar_erro(format!(
                    "nvim --remote-expr falhou: {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                ));
                false
            }
            Err(e) => {
                sessao.avisar_erro(format!("não consegui rodar nvim: {e}"));
                false
            }
        });
    }

    let editor = ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|e| !e.trim().is_empty())
        .unwrap_or_else(|| "nvim".to_string());
    let mut partes = editor.split_whitespace();
    let programa = partes.next().unwrap_or("nvim").to_string();

    ratatui::restore();
    let estado = Command::new(&programa)
        .args(partes)
        .arg(caminho.as_ref())
        .status();
    *terminal = ratatui::try_init()?;

    let erro = match estado {
        Ok(s) if s.success() => None,
        Ok(s) => Some(format!("{programa} saiu com {s}")),
        Err(e) => Some(format!("não consegui rodar {programa}: {e}")),
    };
    sessao.voltou_do_editor(erro);
    Ok(false)
}

/// A expressão Vim que o Neovim pai avalia para abrir `caminho`.
///
/// `--remote-expr` e não `--remote-send`: teclas mandadas a um buffer de
/// terminal iriam para o processo dentro dele, não para o Neovim. O código
/// roda agendado (vim.schedule) para não fechar janela no meio da chamada RPC:
/// se a janela atual é flutuante (o Snacks.terminal do atalho), ela é fechada
/// e o arquivo abre na de baixo; senão, abre na janela anterior.
fn expressao_de_abrir(caminho: &str) -> String {
    let lua = format!(
        "(function(p) vim.schedule(function() \
           local w = vim.api.nvim_get_current_win() \
           if vim.api.nvim_win_get_config(w).relative ~= \"\" then pcall(vim.api.nvim_win_close, w, true) \
           else vim.cmd(\"wincmd p\") end \
           vim.cmd.edit(vim.fn.fnameescape(p)) \
         end) end)({})",
        string_lua(caminho)
    );
    // String Vim entre aspas simples: a única coisa a escapar é a própria aspa.
    format!("luaeval('{}')", lua.replace('\'', "''"))
}

fn string_lua(s: &str) -> String {
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
    fn string_lua_escapa_aspas_e_barra() {
        assert_eq!(string_lua(r#"/a/"b"\c.md"#), r#""/a/\"b\"\\c.md""#);
    }

    #[test]
    fn string_lua_escapa_a_quebra_de_linha() {
        assert_eq!(string_lua("a\nb"), "\"a\\nb\"");
    }

    #[test]
    fn aspa_simples_no_caminho_e_dobrada_para_o_vim() {
        let e = expressao_de_abrir("/v/it's.md");
        assert!(e.starts_with("luaeval('"));
        assert!(e.contains("it''s.md"));
    }

    #[test]
    fn a_expressao_leva_o_caminho_como_string_lua() {
        let e = expressao_de_abrir("/v/a.md");
        assert!(e.contains("(\"/v/a.md\")"), "{e}");
        assert!(e.ends_with("')"));
    }
}
