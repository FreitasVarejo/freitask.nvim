//! As teclas do modo normal, numa tabela só: a ajuda (`?`) lista as linhas
//! dela e o `update` despacha por ela. Uma tecla que a ajuda não lista não tem
//! handler, e uma que tem handler está na ajuda (teste
//! `toda_tecla_da_ajuda_tem_handler`).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Uma tecla como o `update` a distingue: o código e se o Ctrl está apertado.
/// Shift e Alt não contam (a crossterm entrega `A` com ou sem `SHIFT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tecla {
    pub codigo: KeyCode,
    pub ctrl: bool,
}

const fn char(c: char) -> Tecla {
    Tecla {
        codigo: KeyCode::Char(c),
        ctrl: false,
    }
}

const fn ctrl(c: char) -> Tecla {
    Tecla {
        codigo: KeyCode::Char(c),
        ctrl: true,
    }
}

const fn especial(codigo: KeyCode) -> Tecla {
    Tecla {
        codigo,
        ctrl: false,
    }
}

impl Tecla {
    pub fn casa(&self, tecla: &KeyEvent) -> bool {
        self.codigo == tecla.code && self.ctrl == tecla.modifiers.contains(KeyModifiers::CONTROL)
    }
}

/// O que uma tecla faz no modo normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesto {
    Descer,
    Subir,
    Topo,
    Fim,
    IrParaProjetos,
    IrParaTasks,
    AlternarPainel,
    Abrir,
    RolarBaixo,
    RolarCima,
    Filtrar,
    NovaTask,
    Fase,
    Titulo,
    Descricao,
    Prazo,
    Renomear,
    BloqueadaPor,
    Arquivar,
    MostrarArquivadas,
    MostrarEscondidas,
    Largar,
    NovoProjeto,
    AbrirCurrent,
    Reler,
    Ajuda,
    Limpar,
    Sair,
}

/// Uma linha da ajuda: o rótulo das teclas, o que elas fazem e quais teclas
/// (com o gesto de cada uma) ela cobre.
#[derive(Debug)]
pub struct Atalho {
    pub rotulo: &'static str,
    pub descricao: &'static str,
    pub teclas: &'static [(Tecla, Gesto)],
}

/// A tabela. A ordem é a da ajuda.
pub const ATALHOS: &[Atalho] = &[
    Atalho {
        rotulo: "j/k ↑/↓",
        descricao: "mover",
        teclas: &[
            (char('j'), Gesto::Descer),
            (especial(KeyCode::Down), Gesto::Descer),
            (char('k'), Gesto::Subir),
            (especial(KeyCode::Up), Gesto::Subir),
        ],
    },
    Atalho {
        rotulo: "h/l ←/→ Tab",
        descricao: "trocar de painel",
        teclas: &[
            (char('h'), Gesto::IrParaProjetos),
            (especial(KeyCode::Left), Gesto::IrParaProjetos),
            (char('l'), Gesto::IrParaTasks),
            (especial(KeyCode::Right), Gesto::IrParaTasks),
            (especial(KeyCode::Tab), Gesto::AlternarPainel),
            (especial(KeyCode::BackTab), Gesto::AlternarPainel),
        ],
    },
    Atalho {
        rotulo: "g/G Home/End",
        descricao: "topo / fim",
        teclas: &[
            (char('g'), Gesto::Topo),
            (especial(KeyCode::Home), Gesto::Topo),
            (char('G'), Gesto::Fim),
            (especial(KeyCode::End), Gesto::Fim),
        ],
    },
    Atalho {
        rotulo: "Enter / e",
        descricao: "abrir a task no editor",
        teclas: &[
            (especial(KeyCode::Enter), Gesto::Abrir),
            (char('e'), Gesto::Abrir),
        ],
    },
    Atalho {
        rotulo: "C-d / C-u",
        descricao: "rolar o preview",
        teclas: &[
            (ctrl('d'), Gesto::RolarBaixo),
            (ctrl('u'), Gesto::RolarCima),
        ],
    },
    Atalho {
        rotulo: "/",
        descricao: "filtrar tasks (id e título)",
        teclas: &[(char('/'), Gesto::Filtrar)],
    },
    Atalho {
        rotulo: "n",
        descricao: "nova task no projeto",
        teclas: &[(char('n'), Gesto::NovaTask)],
    },
    Atalho {
        rotulo: "s",
        descricao: "trocar a fase",
        teclas: &[(char('s'), Gesto::Fase)],
    },
    Atalho {
        rotulo: "t",
        descricao: "editar o título",
        teclas: &[(char('t'), Gesto::Titulo)],
    },
    Atalho {
        rotulo: "i",
        descricao: "editar a descrição",
        teclas: &[(char('i'), Gesto::Descricao)],
    },
    Atalho {
        rotulo: "p",
        descricao: "definir / limpar o prazo",
        teclas: &[(char('p'), Gesto::Prazo)],
    },
    Atalho {
        rotulo: "r",
        descricao: "renomear o id (arquivo e links)",
        teclas: &[(char('r'), Gesto::Renomear)],
    },
    Atalho {
        rotulo: "b",
        descricao: "editar a lista bloqueada-por",
        teclas: &[(char('b'), Gesto::BloqueadaPor)],
    },
    Atalho {
        rotulo: "a",
        descricao: "arquivar / desarquivar",
        teclas: &[(char('a'), Gesto::Arquivar)],
    },
    Atalho {
        rotulo: "A",
        descricao: "mostrar / esconder arquivadas",
        teclas: &[(char('A'), Gesto::MostrarArquivadas)],
    },
    Atalho {
        rotulo: "S",
        descricao: "mostrar / esconder Algum dia e adiadas",
        teclas: &[(char('S'), Gesto::MostrarEscondidas)],
    },
    Atalho {
        rotulo: "d",
        descricao: "largar a task (arquiva como dropped)",
        teclas: &[(char('d'), Gesto::Largar)],
    },
    Atalho {
        rotulo: "N",
        descricao: "novo projeto",
        teclas: &[(char('N'), Gesto::NovoProjeto)],
    },
    Atalho {
        rotulo: "c",
        descricao: "abrir o CURRENT.md",
        teclas: &[(char('c'), Gesto::AbrirCurrent)],
    },
    Atalho {
        rotulo: "R",
        descricao: "reler o vault",
        teclas: &[(char('R'), Gesto::Reler)],
    },
    Atalho {
        rotulo: "Esc",
        descricao: "limpar o filtro e o aviso",
        teclas: &[(especial(KeyCode::Esc), Gesto::Limpar)],
    },
    Atalho {
        rotulo: "?",
        descricao: "esta ajuda",
        teclas: &[(char('?'), Gesto::Ajuda)],
    },
    Atalho {
        rotulo: "q / C-c",
        descricao: "sair",
        teclas: &[(char('q'), Gesto::Sair), (ctrl('c'), Gesto::Sair)],
    },
];

/// O gesto que esta tecla faz no modo normal, se faz algum.
pub fn gesto_de(tecla: &KeyEvent) -> Option<Gesto> {
    ATALHOS
        .iter()
        .flat_map(|a| a.teclas)
        .find(|(t, _)| t.casa(tecla))
        .map(|(_, g)| *g)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::tui::estado::testes::dados_sinteticos;
    use crate::tui::estado::{Evento, State, update};

    /// Como a ajuda escreve o nome de uma tecla.
    fn nome(t: &Tecla) -> String {
        match (t.codigo, t.ctrl) {
            (KeyCode::Char(c), false) => c.to_string(),
            (KeyCode::Char(c), true) => format!("C-{c}"),
            (KeyCode::Down, _) => "↓".into(),
            (KeyCode::Up, _) => "↑".into(),
            (KeyCode::Left, _) => "←".into(),
            (KeyCode::Right, _) => "→".into(),
            (KeyCode::Tab | KeyCode::BackTab, _) => "Tab".into(),
            (outro, _) => format!("{outro}"),
        }
    }

    fn evento(t: &Tecla) -> KeyEvent {
        let m = if t.ctrl {
            KeyModifiers::CONTROL
        } else {
            KeyModifiers::NONE
        };
        KeyEvent::new(t.codigo, m)
    }

    /// Toda tecla que se pode apertar: ASCII imprimível, as especiais e
    /// Ctrl-letra.
    fn universo() -> Vec<Tecla> {
        let mut u: Vec<Tecla> = (0x20u8..0x7f).map(|b| char(b as char)).collect();
        u.extend((b'a'..=b'z').map(|b| ctrl(b as char)));
        u.extend(
            [
                KeyCode::Enter,
                KeyCode::Tab,
                KeyCode::BackTab,
                KeyCode::Esc,
                KeyCode::Up,
                KeyCode::Down,
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Home,
                KeyCode::End,
                KeyCode::Backspace,
                KeyCode::Delete,
                KeyCode::PageUp,
                KeyCode::PageDown,
                KeyCode::F(1),
            ]
            .map(especial),
        );
        u
    }

    #[test]
    fn toda_tecla_da_ajuda_tem_handler() {
        // A ajuda e o despacho saem da mesma constante: toda tecla listada tem
        // o gesto que a tabela diz, e o rótulo da linha a cita…
        for atalho in ATALHOS {
            assert!(!atalho.teclas.is_empty(), "{} sem tecla", atalho.rotulo);
            for (tecla, gesto) in atalho.teclas {
                assert_eq!(
                    gesto_de(&evento(tecla)),
                    Some(*gesto),
                    "a tecla {tecla:?} da linha {:?} não tem o handler da tabela",
                    atalho.rotulo
                );
                assert!(
                    atalho.rotulo.contains(&nome(tecla)),
                    "o rótulo {:?} não cita a tecla {tecla:?}",
                    atalho.rotulo
                );
            }
        }
        // …e toda tecla que o modo normal trata está numa linha da ajuda: o
        // que a tabela não lista não muda o estado nem pede nada.
        let base = State::novo(dados_sinteticos());
        let caminho = base.task_atual().unwrap().path.clone();
        let (base, _) = update(
            base,
            Evento::PreviewLido {
                caminho,
                conteudo: String::new(),
            },
        );
        for tecla in universo() {
            let listada = ATALHOS
                .iter()
                .flat_map(|a| a.teclas)
                .any(|(t, _)| *t == tecla);
            if listada {
                continue;
            }
            let (depois, comandos) = update(base.clone(), Evento::Tecla(evento(&tecla)));
            assert!(
                depois == base && comandos.is_empty(),
                "a tecla {tecla:?} não está na ajuda, mas mexe no estado ou pede algo"
            );
        }
    }

    #[test]
    fn ctrl_com_outra_letra_e_alt_nao_confundem() {
        // Ctrl-j não é `j`; Alt-j é `j` (como hoje).
        let ctrl_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        assert_eq!(gesto_de(&ctrl_j), None);
        let alt_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT);
        assert_eq!(gesto_de(&alt_j), Some(Gesto::Descer));
        let shift_a = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(gesto_de(&shift_a), Some(Gesto::MostrarArquivadas));
    }
}
