//! A TUI sem o terminal: o `State` e o executor dos `Comando`s contra o vault.
//!
//! É o que o laço de verdade (`laco.rs`) usa, e o que o aceite da fatia 7
//! chama em processo. O que precisa do terminal (sair, abrir um arquivo no
//! editor) volta como `Pedido`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use freitask_core::date::Date;
use freitask_vault::Vault;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use crate::deslocar;
use crate::escrita;
use crate::saida::{self, Contexto, Falha, Resultado, Saida};
use crate::tui::dados::Dados;
use crate::tui::estado::{Comando, Depois, Escrita, Evento, State, update};
use crate::tui::tela::view;

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

/// De onde a `Sessao` tira o dia: perguntado a cada comando e a cada leitura,
/// porque uma TUI aberta às 23:50 ainda está aberta depois da meia-noite, e o
/// painel e as adiadas não podem ficar no dia de ontem. O erro é a mensagem de
/// quem não soube dizer o dia.
pub type Relogio = Box<dyn Fn() -> Result<Date, String>>;

/// Uma TUI aberta sobre `<home>/ObsidianVault`.
pub struct Sessao {
    estado: State,
    vault: Vault,
    /// Onde mora o lock de escrita (`$XDG_RUNTIME_DIR` ou o `home`).
    trava: PathBuf,
    relogio: Relogio,
}

impl Sessao {
    /// Abre como a TUI abre: regenera o `CURRENT.md` (`rebuild`) e então lê o
    /// vault. `hoje` é `AAAA-MM-DD` e faz o papel do `FREITASK_TODAY`: o dia
    /// fica fixo na sessão inteira (é o que os testes de aceite precisam). Se
    /// o `rebuild` falhar, abre mesmo assim, com o motivo na barra. Nunca lê
    /// `HOME`: o vault é `<home>/ObsidianVault`.
    pub fn abrir(home: &Path, hoje: &str) -> Result<Sessao, String> {
        let hoje = saida::dia(hoje)?;
        Sessao::abrir_com_relogio(home, Box::new(move || Ok(hoje)))
    }

    /// O mesmo, para a TUI de verdade: o dia vem de `relogio`, perguntado a
    /// cada comando e a cada leitura do vault (a `CLI` fazia o mesmo ao ser
    /// chamada a cada ação). Se o relógio falhar na abertura, a sessão não
    /// abre; se falhar depois, o comando que precisava do dia não roda e o
    /// motivo vai para a barra, como o de um `rebuild` que falha.
    pub fn abrir_com_relogio(home: &Path, relogio: Relogio) -> Result<Sessao, String> {
        relogio()?;
        let trava =
            saida::caminho_da_trava(Some(&home.to_string_lossy())).map_err(|f| f.para_a_barra())?;
        let mut sessao = Sessao {
            estado: State::default(),
            vault: Vault::em(home.join("ObsidianVault")),
            trava,
            relogio,
        };
        // O `rebuild` vem antes da leitura para que as duas concordem: a task
        // adiada que voltou hoje entra no painel sem ninguém ter escrito nela.
        let rebuild = sessao.regenerar();
        let mut eventos = vec![sessao.dados_novos(Depois::Manter, String::new())];
        if let Err(motivo) = rebuild {
            eventos.push(Evento::Falhou(format!("rebuild falhou: {motivo}")));
        }
        sessao.processar(eventos);
        Ok(sessao)
    }

    /// Uma tecla: atualiza o estado e executa no vault o que ela manda
    /// gravar; o que precisa do terminal volta como `Pedido`.
    pub fn tecla(&mut self, tecla: KeyEvent) -> Pedido {
        self.processar([Evento::Tecla(tecla)])
    }

    /// A tela inteira num terminal de `largura` × `altura`.
    pub fn tela(&self, largura: u16, altura: u16) -> Buffer {
        let vazio = || Buffer::empty(Rect::new(0, 0, largura, altura));
        let Ok(mut terminal) = Terminal::new(TestBackend::new(largura, altura)) else {
            return vazio();
        };
        if terminal.draw(|f| self.desenhar(f)).is_err() {
            return vazio();
        }
        terminal.backend().buffer().clone()
    }

    /// Desenha a tela no `Frame` do laço.
    pub fn desenhar(&self, f: &mut Frame) {
        view(&self.estado, f);
    }

    /// O terminal mudou de tamanho.
    pub fn redimensionou(&mut self, largura: u16, altura: u16) {
        self.processar([Evento::Resize(largura, altura)]);
    }

    /// Põe um erro na barra (o laço não conseguiu abrir o editor, por
    /// exemplo).
    pub fn avisar_erro(&mut self, mensagem: String) {
        self.processar([Evento::Falhou(mensagem)]);
    }

    /// O editor fechou: o painel é regenerado e o vault relido. O editor pode
    /// não ser um Neovim com o autocmd do freitask, e o painel não pode ficar
    /// para trás da edição. Se o `rebuild` falhar, a leitura segue.
    pub fn voltou_do_editor(&mut self, erro: Option<String>) {
        let mut eventos = Vec::new();
        if let Some(e) = erro {
            eventos.push(Evento::Falhou(e));
        }
        let _ = self.regenerar();
        eventos.push(self.dados_novos(Depois::Manter, String::new()));
        self.processar(eventos);
    }

    /// O estado, para quem desenha.
    pub fn estado(&self) -> &State {
        &self.estado
    }

    /// O dia de agora, perguntado ao relógio; o erro já vem como a barra o
    /// mostra.
    fn dia(&self) -> Result<Date, String> {
        (self.relogio)().map_err(|e| format!("relógio falhou: {e}"))
    }

    fn contexto(&self) -> Result<Contexto<'_>, String> {
        Ok(Contexto {
            vault: &self.vault,
            trava: &self.trava,
            hoje: self.dia()?,
        })
    }

    /// O `rebuild`, com o motivo da falha já como a barra o mostra.
    fn regenerar(&self) -> Result<Saida, String> {
        escrita::rebuild(&self.contexto()?).map_err(|f| f.para_a_barra())
    }

    /// A leitura do vault no dia de agora; se o relógio falha, a tela fica
    /// como está e a barra diz por quê.
    fn dados_novos(&self, depois: Depois, saida: String) -> Evento {
        match self.dia() {
            Ok(hoje) => Evento::Dados {
                dados: Dados::ler(&self.vault, hoje),
                depois,
                saida,
            },
            Err(motivo) => Evento::Falhou(motivo),
        }
    }

    /// Alimenta o `update` com os eventos e com o que os comandos devolvem,
    /// até a fila esvaziar. O que pede o terminal sai como `Pedido`.
    fn processar(&mut self, eventos: impl IntoIterator<Item = Evento>) -> Pedido {
        let mut fila: VecDeque<Evento> = eventos.into_iter().collect();
        let mut pedido = Pedido::Nada;
        while let Some(evento) = fila.pop_front() {
            let (novo, comandos) = update(std::mem::take(&mut self.estado), evento);
            self.estado = novo;
            for comando in comandos {
                if let Some(p) = self.executar(comando, &mut fila) {
                    pedido = p;
                }
            }
        }
        pedido
    }

    fn executar(&self, comando: Comando, fila: &mut VecDeque<Evento>) -> Option<Pedido> {
        match comando {
            Comando::Sair => return Some(Pedido::Sair),
            Comando::Abrir(caminho) => return Some(Pedido::Abrir(PathBuf::from(caminho))),
            Comando::AbrirCurrent => match self.regenerar() {
                Ok(saida) => return Some(Pedido::Abrir(PathBuf::from(saida.stdout))),
                Err(motivo) => fila.push_back(Evento::Falhou(motivo)),
            },
            Comando::Reler { depois, saida } => fila.push_back(self.dados_novos(depois, saida)),
            Comando::SugerirId(titulo) => {
                // Título que não rende id: sem sugestão, o campo fica como está.
                if let Ok(saida) = escrita::kebab(&titulo) {
                    fila.push_back(Evento::IdSugerido(saida.stdout));
                }
            }
            Comando::LerPreview(caminho) => {
                let conteudo = std::fs::read_to_string(&caminho)
                    .unwrap_or_else(|e| format!("(não consegui ler: {e})"));
                fila.push_back(Evento::PreviewLido { caminho, conteudo });
            }
            Comando::Executar {
                escrita: acao,
                depois,
                ok,
            } => fila.push_back(match self.escrever(acao) {
                Ok(saida) => Evento::Feito {
                    ok,
                    depois,
                    saida: saida.stdout,
                },
                Err(f) => Evento::Falhou(f.para_a_barra()),
            }),
        }
        None
    }

    /// Grava pelo mesmo caminho da CLI: lock, leitura, plano do `core` ou
    /// recusa, escrita com CAS e `rebuild`.
    fn escrever(&self, acao: Escrita) -> Resultado {
        let cx = match self.contexto() {
            Ok(cx) => cx,
            Err(motivo) => {
                return Err(Falha {
                    stderr: vec![motivo],
                    codigo: 1,
                });
            }
        };
        match acao {
            Escrita::NovaTask {
                projeto,
                id,
                titulo,
                desc,
            } => escrita::new(&cx, &projeto, &id, &titulo, &desc, false),
            Escrita::NovoProjeto(nome) => escrita::new_project(&cx, &nome),
            Escrita::Set { referencia, pedido } => escrita::set(&cx, &referencia, &pedido),
            Escrita::Deslocar { referencia, pedido } => {
                deslocar::deslocar(&cx, &referencia, pedido)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn home_com_uma_task() -> tempfile::TempDir {
        let home = tempfile::tempdir().unwrap();
        let task = home.path().join("ObsidianVault/projects/a/tasks/um.md");
        std::fs::create_dir_all(task.parent().unwrap()).unwrap();
        std::fs::write(task, "> [!todo] Um\n> [[um]]\n").unwrap();
        home
    }

    fn ids(s: &Sessao) -> Vec<String> {
        s.estado()
            .tasks_visiveis()
            .iter()
            .map(|t| t.id.clone())
            .collect()
    }

    #[test]
    fn hoje_invalido_nao_abre() {
        let home = home_com_uma_task();
        let e = Sessao::abrir(home.path(), "ontem").err().unwrap();
        assert_eq!(e, "FREITASK_TODAY inválida: ontem (use AAAA-MM-DD)");
    }

    #[test]
    fn abrir_le_o_vault_de_dentro_do_home_e_regenera_o_painel() {
        let home = home_com_uma_task();
        let s = Sessao::abrir(home.path(), "2026-11-10").unwrap();
        assert_eq!(ids(&s), ["um"]);
        assert!(s.estado().mensagem.is_none());
        let painel = std::fs::read_to_string(home.path().join("ObsidianVault/CURRENT.md")).unwrap();
        assert!(painel.contains("date: 2026-11-10"), "{painel}");
        assert!(painel.contains("> [!todo] Um\n"), "{painel}");
    }

    #[test]
    fn o_preview_da_primeira_task_ja_esta_lido_ao_abrir() {
        let home = home_com_uma_task();
        let s = Sessao::abrir(home.path(), "2026-11-10").unwrap();
        let (caminho, conteudo) = s.estado().preview.clone().unwrap();
        assert!(caminho.ends_with("projects/a/tasks/um.md"), "{caminho}");
        assert_eq!(conteudo, "> [!todo] Um\n> [[um]]\n");
    }

    #[test]
    fn voltou_do_editor_rele_o_vault_e_mostra_o_erro_do_editor() {
        let home = home_com_uma_task();
        let mut s = Sessao::abrir(home.path(), "2026-11-10").unwrap();
        let nova = home.path().join("ObsidianVault/projects/a/tasks/dois.md");
        std::fs::write(nova, "> [!todo] Dois\n> [[dois]]\n").unwrap();
        s.voltou_do_editor(Some("vi saiu com 1".into()));
        assert_eq!(ids(&s), ["dois", "um"]);
        let m = s.estado().mensagem.clone().unwrap();
        assert_eq!((m.texto.as_str(), m.erro), ("vi saiu com 1", true));
        let painel = std::fs::read_to_string(home.path().join("ObsidianVault/CURRENT.md")).unwrap();
        assert!(
            painel.contains("> [!todo] Dois\n"),
            "o painel acompanha a edição"
        );
    }

    #[test]
    fn avisar_erro_poe_a_mensagem_na_barra_como_erro() {
        let home = home_com_uma_task();
        let mut s = Sessao::abrir(home.path(), "2026-11-10").unwrap();
        s.avisar_erro("nvim --remote-expr falhou: x".into());
        let m = s.estado().mensagem.clone().unwrap();
        assert_eq!(
            (m.texto.as_str(), m.erro),
            ("nvim --remote-expr falhou: x", true)
        );
    }

    /// Um relógio que o teste move: o dia atual e se ele está quebrado.
    struct Mostrador {
        dia: std::cell::Cell<Date>,
        quebrado: std::cell::Cell<bool>,
    }

    fn mostrador(dia: &str) -> std::rc::Rc<Mostrador> {
        std::rc::Rc::new(Mostrador {
            dia: std::cell::Cell::new(saida::dia(dia).unwrap()),
            quebrado: std::cell::Cell::new(false),
        })
    }

    fn relogio(m: &std::rc::Rc<Mostrador>) -> Relogio {
        let m = std::rc::Rc::clone(m);
        Box::new(move || {
            if m.quebrado.get() {
                Err("sem relógio".to_string())
            } else {
                Ok(m.dia.get())
            }
        })
    }

    fn painel(home: &tempfile::TempDir) -> String {
        std::fs::read_to_string(home.path().join("ObsidianVault/CURRENT.md")).unwrap()
    }

    fn tecla(c: char) -> KeyEvent {
        KeyEvent::from(ratatui::crossterm::event::KeyCode::Char(c))
    }

    fn digita(s: &mut Sessao, texto: &str) {
        for c in texto.chars() {
            s.tecla(tecla(c));
        }
    }

    fn enter() -> KeyEvent {
        KeyEvent::from(ratatui::crossterm::event::KeyCode::Enter)
    }

    #[test]
    fn o_dia_e_perguntado_a_cada_comando_e_a_cada_leitura() {
        let home = home_com_uma_task();
        let adiada = home.path().join("ObsidianVault/projects/a/tasks/dois.md");
        std::fs::write(
            adiada,
            "---\nvolta-em: 2026-11-11\n---\n\n> [!todo] Dois\n> [[dois]]\n",
        )
        .unwrap();
        let relogio_manual = mostrador("2026-11-10");
        let mut s = Sessao::abrir_com_relogio(home.path(), relogio(&relogio_manual)).unwrap();
        assert_eq!(ids(&s), ["um"], "a adiada ainda não voltou");
        assert!(painel(&home).contains("date: 2026-11-10"));

        // Passou a meia-noite com a TUI aberta.
        relogio_manual.dia.set(saida::dia("2026-11-11").unwrap());
        s.tecla(tecla('R'));
        assert_eq!(ids(&s), ["dois", "um"], "a leitura usa o dia novo");
        assert!(
            painel(&home).contains("date: 2026-11-10"),
            "reler não grava o painel"
        );

        let pedido = s.tecla(tecla('c'));
        assert!(matches!(pedido, Pedido::Abrir(_)));
        let novo = painel(&home);
        assert!(novo.contains("date: 2026-11-11"), "{novo}");
        assert!(novo.contains("> [!todo] Dois\n"), "{novo}");
    }

    #[test]
    fn a_gravacao_regenera_o_painel_no_dia_de_agora() {
        let home = home_com_uma_task();
        let relogio_manual = mostrador("2026-11-10");
        let mut s = Sessao::abrir_com_relogio(home.path(), relogio(&relogio_manual)).unwrap();
        relogio_manual.dia.set(saida::dia("2026-11-11").unwrap());
        s.tecla(tecla('i'));
        digita(&mut s, "estado novo");
        s.tecla(enter());
        let task =
            std::fs::read_to_string(home.path().join("ObsidianVault/projects/a/tasks/um.md"))
                .unwrap();
        assert!(task.contains("estado novo"), "{task}");
        let novo = painel(&home);
        assert!(novo.contains("date: 2026-11-11"), "{novo}");
        assert!(
            home.path()
                .join("ObsidianVault/daily/2026-11-10.md")
                .is_file(),
            "o painel de ontem virou o snapshot do dia dele"
        );
    }

    #[test]
    fn relogio_que_falha_na_abertura_nao_abre() {
        let home = home_com_uma_task();
        let m = mostrador("2026-11-10");
        m.quebrado.set(true);
        let e = Sessao::abrir_com_relogio(home.path(), relogio(&m))
            .err()
            .unwrap();
        assert_eq!(e, "sem relógio");
        assert!(!home.path().join("ObsidianVault/CURRENT.md").exists());
    }

    #[test]
    fn relogio_que_falha_no_meio_da_sessao_vai_para_a_barra_e_nao_derruba() {
        let home = home_com_uma_task();
        let m = mostrador("2026-11-10");
        let mut s = Sessao::abrir_com_relogio(home.path(), relogio(&m)).unwrap();
        let antes = painel(&home);
        m.quebrado.set(true);
        let barra = |s: &Sessao| {
            let m = s.estado().mensagem.clone().unwrap();
            assert!(m.erro);
            m.texto
        };

        s.tecla(tecla('R'));
        assert_eq!(barra(&s), "relógio falhou: sem relógio");
        assert_eq!(ids(&s), ["um"], "a tela fica como estava");

        assert_eq!(s.tecla(tecla('c')), Pedido::Nada);
        assert_eq!(barra(&s), "relógio falhou: sem relógio");

        s.tecla(tecla('i'));
        digita(&mut s, "estado novo");
        s.tecla(enter());
        assert_eq!(barra(&s), "relógio falhou: sem relógio");
        let task =
            std::fs::read_to_string(home.path().join("ObsidianVault/projects/a/tasks/um.md"))
                .unwrap();
        assert_eq!(task, "> [!todo] Um\n> [[um]]\n", "sem dia, nada é gravado");
        // O campo volta aberto, com o que foi digitado; Esc o fecha.
        s.tecla(KeyEvent::from(ratatui::crossterm::event::KeyCode::Esc));

        s.voltou_do_editor(None);
        assert_eq!(barra(&s), "relógio falhou: sem relógio");
        assert_eq!(painel(&home), antes, "sem dia, o painel não é regravado");

        // O relógio volta e a sessão segue de onde estava.
        m.quebrado.set(false);
        s.tecla(tecla('R'));
        assert_eq!(ids(&s), ["um"]);
        assert!(matches!(s.tecla(tecla('c')), Pedido::Abrir(_)));
    }

    #[test]
    fn relogio_que_falha_logo_depois_de_abrir_abre_com_o_motivo_na_barra() {
        let home = home_com_uma_task();
        let chamadas = std::rc::Rc::new(std::cell::Cell::new(0));
        let c = std::rc::Rc::clone(&chamadas);
        let hoje = saida::dia("2026-11-10").unwrap();
        let s = Sessao::abrir_com_relogio(
            home.path(),
            Box::new(move || {
                c.set(c.get() + 1);
                if c.get() == 1 {
                    Ok(hoje)
                } else {
                    Err("caiu".to_string())
                }
            }),
        )
        .unwrap();
        let m = s.estado().mensagem.clone().unwrap();
        assert!(m.erro);
        assert_eq!(m.texto, "rebuild falhou: relógio falhou: caiu");
        assert!(!home.path().join("ObsidianVault/CURRENT.md").exists());
    }

    #[test]
    fn a_tela_tem_o_tamanho_pedido_mesmo_sem_nada_para_mostrar() {
        let home = tempfile::tempdir().unwrap();
        let s = Sessao::abrir(home.path(), "2026-11-10").unwrap();
        let b = s.tela(30, 8);
        assert_eq!((b.area.width, b.area.height), (30, 8));
    }
}
