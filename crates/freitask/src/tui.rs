//! A TUI do freitask em processo (spec §1.7 e §3.5).
//!
//! ESQUELETO da fatia 7: só o contrato que o aceite congela (decisão f7.1 em
//! `docs/decisoes-da-reescrita.md`). Os nomes e as assinaturas públicas abaixo
//! não mudam sem reavaliação; o que mora atrás deles é livre.
//!
//! `Sessao` é a TUI sem o terminal: o mesmo estado, as mesmas teclas e os
//! mesmos efeitos no vault que o laço de verdade usa. O que sai do processo
//! (sair, abrir um arquivo no editor) volta como `Pedido` para o laço.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use std::path::{Path, PathBuf};

/// O que a tecla pede ao laço, além de mudar a sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pedido {
    /// Nada sai do processo: o laço só redesenha.
    Nada,
    /// `q` ou `Ctrl-C`: a TUI fecha.
    Sair,
    /// Abrir este arquivo (caminho absoluto) no editor: a task selecionada
    /// (`Enter`/`e`) ou o `CURRENT.md` recém-regenerado (`c`).
    Abrir(PathBuf),
}

/// Uma TUI aberta sobre `<home>/ObsidianVault`.
pub struct Sessao {
    _privado: (),
}

impl Sessao {
    /// Abre como a TUI abre: regenera o `CURRENT.md` (`rebuild`) e então lê o
    /// vault. `hoje` é `AAAA-MM-DD` e faz o papel do `FREITASK_TODAY`. Se o
    /// `rebuild` falhar, abre mesmo assim, com o motivo na barra.
    pub fn abrir(home: &Path, hoje: &str) -> Result<Sessao, String> {
        let _ = (home, hoje);
        Err("a TUI ainda não foi portada (fatia 7)".into())
    }

    /// Uma tecla: atualiza o estado e executa no vault o que ela manda
    /// gravar; o que precisa do terminal volta como `Pedido`.
    pub fn tecla(&mut self, tecla: KeyEvent) -> Pedido {
        let _ = tecla;
        Pedido::Nada
    }

    /// A tela inteira num terminal de `largura` × `altura`.
    pub fn tela(&self, largura: u16, altura: u16) -> Buffer {
        Buffer::empty(Rect::new(0, 0, largura, altura))
    }
}
