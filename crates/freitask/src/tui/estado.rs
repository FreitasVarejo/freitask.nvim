//! O estado da tela e o que cada tecla faz.
//!
//! Puro de propósito (spec §1.3 e §1.7): `update(State, Evento)` devolve o
//! estado novo e os `Comando`s que o laço executa contra o vault; nada aqui
//! toca disco nem processo. É o que deixa as teclas testáveis sem terminal e
//! mantém num lugar só a fronteira com o motor, que é quem decide de verdade:
//! a TUI só monta o pedido (`Escrita`), como a de hoje montava os argumentos
//! da CLI.

use freitask_core::config::{ARCHIVED_TYPES, archived_label};
use freitask_core::operacao::PedidoSet;
use freitask_core::task::check_status_change;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::deslocar::Pedido;
use crate::linhas::Resumo;
use crate::tui::atalhos::{Gesto, gesto_de};
use crate::tui::dados::{Dados, Projeto};

#[cfg(test)]
pub(crate) mod testes;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Painel {
    #[default]
    Projetos,
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Campo {
    Titulo,
    Descricao,
    Prazo,
    Renomear,
    BloqueadaPor,
    NovoProjeto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escolha {
    Fase,
    Arquivar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmacao {
    Largar,
    Desarquivar,
}

/// O modal de task nova: título → id → descrição.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NovaTask {
    pub titulo: String,
    pub id: String,
    pub desc: String,
    /// 0 título, 1 id, 2 descrição.
    pub campo: usize,
    /// O id foi digitado à mão: a sugestão derivada do título não o sobrescreve.
    pub id_digitado: bool,
}

impl NovaTask {
    fn campo_mut(&mut self) -> &mut String {
        match self.campo {
            0 => &mut self.titulo,
            1 => &mut self.id,
            _ => &mut self.desc,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Modo {
    #[default]
    Normal,
    Filtro,
    NovaTask(NovaTask),
    Prompt {
        campo: Campo,
        valor: String,
    },
    Pick {
        escolha: Escolha,
        /// `(valor, rótulo)`.
        opcoes: Vec<(String, String)>,
        idx: usize,
    },
    Confirmar(Confirmacao),
    Ajuda,
}

/// O que selecionar depois que o comando rodou e os dados foram relidos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Depois {
    /// A mesma task de antes (pelo id), ou a mesma posição se ela sumiu.
    Manter,
    Task {
        projeto: String,
        id: String,
    },
    /// O stdout do comando é o id do projeto (é o que `new-project` devolve).
    ProjetoDaSaida,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mensagem {
    pub texto: String,
    pub erro: bool,
}

/// O que a TUI pede ao motor para gravar: o mesmo pedido que a CLI recebe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Escrita {
    NovaTask {
        projeto: String,
        id: String,
        titulo: String,
        desc: String,
    },
    NovoProjeto(String),
    /// `set <referencia>`.
    Set {
        referencia: String,
        pedido: PedidoSet,
    },
    /// `archive`, `unarchive`, `rename` e `move`.
    Deslocar {
        referencia: String,
        pedido: Pedido,
    },
}

/// O que o laço executa contra o vault (ou contra o terminal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comando {
    /// A TUI fecha.
    Sair,
    /// Grava; em sucesso, `Evento::Feito`; numa recusa, `Evento::Falhou`.
    Executar {
        escrita: Escrita,
        depois: Depois,
        ok: String,
    },
    /// Relê o vault e devolve `Evento::Dados`.
    Reler { depois: Depois, saida: String },
    /// Pede o id que o `kebab` deriva deste título.
    SugerirId(String),
    /// Lê o arquivo do preview.
    LerPreview(String),
    /// Abre a task (caminho absoluto) no editor.
    Abrir(String),
    /// Regenera o `CURRENT.md` e o abre no editor.
    AbrirCurrent,
}

/// O que acontece com a tela: uma tecla, ou o resultado de um `Comando`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evento {
    Tecla(KeyEvent),
    /// O terminal mudou de tamanho: só se redesenha, o estado não depende dele.
    Resize(u16, u16),
    /// O vault foi relido.
    Dados {
        dados: Dados,
        depois: Depois,
        saida: String,
    },
    PreviewLido {
        caminho: String,
        conteudo: String,
    },
    IdSugerido(String),
    /// O comando gravou.
    Feito {
        ok: String,
        depois: Depois,
        saida: String,
    },
    /// O comando foi recusado, ou algo falhou: a mensagem vai para a barra.
    Falhou(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub dados: Dados,
    pub painel: Painel,
    pub project_idx: usize,
    pub task_idx: usize,
    pub mostrar_arquivadas: bool,
    /// Mostra as tasks que o motor esconde por padrão (o Algum dia e as
    /// adiadas).
    pub mostrar_escondidas: bool,
    pub filtro: String,
    pub modo: Modo,
    pub mensagem: Option<Mensagem>,
    /// (caminho, conteúdo) do arquivo em preview.
    pub preview: Option<(String, String)>,
    /// O caminho cuja leitura já foi pedida, para pedir uma vez por seleção.
    preview_pedido: Option<String>,
    pub preview_scroll: u16,
    /// O modal que originou o `Executar` em voo: se o motor recusar, ele volta
    /// com o que foi digitado, em vez de obrigar a digitar tudo de novo.
    refazer: Option<Modo>,
}

/// Aplica um evento: o estado novo e o que o laço tem de executar.
pub fn update(mut estado: State, evento: Evento) -> (State, Vec<Comando>) {
    let mut comandos = estado.aplicar(evento);
    if let Some(caminho) = estado.preview_a_pedir() {
        estado.preview_pedido = Some(caminho.clone());
        comandos.push(Comando::LerPreview(caminho));
    }
    (estado, comandos)
}

impl State {
    pub fn novo(dados: Dados) -> State {
        State {
            dados,
            ..State::default()
        }
    }

    // --- leitura --------------------------------------------------------------

    pub fn projeto_atual(&self) -> Option<&Projeto> {
        self.dados.projetos.get(self.project_idx)
    }

    /// As tasks do projeto selecionado, na ordem do `list` (ativas por fase e
    /// id, arquivadas no fim), filtradas pelos toggles e por `/`.
    pub fn tasks_visiveis(&self) -> Vec<&Resumo> {
        let Some(p) = self.projeto_atual() else {
            return vec![];
        };
        let f = self.filtro.to_lowercase();
        self.dados
            .tasks
            .iter()
            .filter(|t| t.project == p.nome)
            .filter(|t| self.mostrar_arquivadas || t.archived.is_none())
            .filter(|t| self.mostrar_escondidas || !t.hidden)
            .filter(|t| {
                f.is_empty()
                    || t.id.to_lowercase().contains(&f)
                    || t.title.to_lowercase().contains(&f)
            })
            .collect()
    }

    pub fn task_atual(&self) -> Option<&Resumo> {
        self.tasks_visiveis().get(self.task_idx).copied()
    }

    /// O arquivo que o preview deveria mostrar e ainda não mostra nem pediu.
    fn preview_a_pedir(&self) -> Option<String> {
        let t = self.task_atual()?;
        let mostra = self.preview.as_ref().is_some_and(|(p, _)| *p == t.path);
        let pediu = self.preview_pedido.as_deref() == Some(t.path.as_str());
        (!mostra && !pediu).then(|| t.path.clone())
    }

    /// Esquece o preview: a próxima passada pede a leitura de novo.
    fn esquecer_preview(&mut self) {
        self.preview = None;
        self.preview_pedido = None;
    }

    // --- eventos ----------------------------------------------------------------

    fn aplicar(&mut self, evento: Evento) -> Vec<Comando> {
        match evento {
            Evento::Tecla(tecla) => self.tecla(tecla),
            Evento::Resize(..) => vec![],
            Evento::Dados {
                dados,
                depois,
                saida,
            } => {
                self.set_dados(dados, &depois, &saida);
                vec![]
            }
            Evento::PreviewLido { caminho, conteudo } => {
                self.preview = Some((caminho, conteudo));
                vec![]
            }
            Evento::IdSugerido(id) => {
                if let Modo::NovaTask(nt) = &mut self.modo
                    && !nt.id_digitado
                {
                    nt.id = id;
                }
                vec![]
            }
            Evento::Feito { ok, depois, saida } => {
                self.refazer = None;
                self.info(ok);
                vec![Comando::Reler { depois, saida }]
            }
            Evento::Falhou(mensagem) => {
                if let Some(m) = self.refazer.take() {
                    self.modo = m;
                }
                self.erro(mensagem);
                vec![]
            }
        }
    }

    fn set_dados(&mut self, dados: Dados, depois: &Depois, saida: &str) {
        let manter_projeto = self.projeto_atual().map(|p| p.nome.clone());
        let manter_task = self.task_atual().map(|t| t.id.clone());
        let task_idx_antigo = self.task_idx;
        self.dados = dados;
        self.esquecer_preview();

        let (quer_projeto, quer_task) = match depois {
            Depois::Manter => (manter_projeto, manter_task),
            Depois::Task { projeto, id } => (Some(projeto.clone()), Some(id.clone())),
            Depois::ProjetoDaSaida => (Some(saida.trim().to_string()), None),
        };
        if let Some(i) =
            quer_projeto.and_then(|w| self.dados.projetos.iter().position(|p| p.nome == w))
        {
            self.project_idx = i;
        }
        self.task_idx = quer_task
            .and_then(|id| self.tasks_visiveis().iter().position(|t| t.id == id))
            .unwrap_or(task_idx_antigo);
        if matches!(depois, Depois::ProjetoDaSaida) {
            self.task_idx = 0;
            self.painel = Painel::Projetos;
        }
        self.clamp();
    }

    pub fn info(&mut self, texto: impl Into<String>) {
        self.mensagem = Some(Mensagem {
            texto: texto.into(),
            erro: false,
        });
    }

    pub fn erro(&mut self, texto: impl Into<String>) {
        self.mensagem = Some(Mensagem {
            texto: texto.into(),
            erro: true,
        });
    }

    fn clamp(&mut self) {
        self.project_idx = self
            .project_idx
            .min(self.dados.projetos.len().saturating_sub(1));
        self.task_idx = self
            .task_idx
            .min(self.tasks_visiveis().len().saturating_sub(1));
    }

    // --- teclas ---------------------------------------------------------------

    fn tecla(&mut self, tecla: KeyEvent) -> Vec<Comando> {
        if tecla.modifiers.contains(KeyModifiers::CONTROL) && tecla.code == KeyCode::Char('c') {
            return vec![Comando::Sair];
        }
        match std::mem::take(&mut self.modo) {
            Modo::Normal => self.no_normal(tecla),
            Modo::Filtro => self.no_filtro(tecla),
            Modo::NovaTask(nt) => self.na_nova_task(nt, tecla),
            Modo::Prompt { campo, valor } => self.no_prompt(campo, valor, tecla),
            Modo::Pick {
                escolha,
                opcoes,
                idx,
            } => self.no_pick(escolha, opcoes, idx, tecla),
            Modo::Confirmar(c) => self.na_confirmacao(c, tecla),
            Modo::Ajuda => vec![],
        }
    }

    fn no_normal(&mut self, tecla: KeyEvent) -> Vec<Comando> {
        let Some(gesto) = gesto_de(&tecla) else {
            return vec![];
        };
        match gesto {
            Gesto::RolarBaixo => self.preview_scroll = self.preview_scroll.saturating_add(10),
            Gesto::RolarCima => self.preview_scroll = self.preview_scroll.saturating_sub(10),
            Gesto::Sair => return vec![Comando::Sair],
            Gesto::Limpar => {
                self.filtro.clear();
                self.mensagem = None;
                self.clamp();
            }
            Gesto::Ajuda => self.modo = Modo::Ajuda,
            Gesto::Descer => self.mover(1),
            Gesto::Subir => self.mover(-1),
            Gesto::Topo => self.mover(isize::MIN),
            Gesto::Fim => self.mover(isize::MAX),
            Gesto::IrParaProjetos => self.painel = Painel::Projetos,
            Gesto::IrParaTasks => self.painel = Painel::Tasks,
            Gesto::AlternarPainel => {
                self.painel = match self.painel {
                    Painel::Projetos => Painel::Tasks,
                    Painel::Tasks => Painel::Projetos,
                }
            }
            // Nos projetos, `Enter` só passa o foco; `e` abre a task de
            // qualquer painel.
            Gesto::Abrir => {
                if tecla.code == KeyCode::Enter && self.painel == Painel::Projetos {
                    self.painel = Painel::Tasks;
                } else if let Some(t) = self.task_atual() {
                    return vec![Comando::Abrir(t.path.clone())];
                }
            }
            Gesto::Filtrar => {
                self.painel = Painel::Tasks;
                self.modo = Modo::Filtro;
            }
            Gesto::NovaTask => {
                if self.projeto_atual().is_some() {
                    self.modo = Modo::NovaTask(NovaTask::default());
                } else {
                    self.erro("crie um projeto antes (N)");
                }
            }
            Gesto::NovoProjeto => {
                self.modo = Modo::Prompt {
                    campo: Campo::NovoProjeto,
                    valor: String::new(),
                }
            }
            Gesto::MostrarArquivadas => {
                self.mostrar_arquivadas = !self.mostrar_arquivadas;
                self.esquecer_preview();
                self.clamp();
            }
            Gesto::MostrarEscondidas => {
                self.mostrar_escondidas = !self.mostrar_escondidas;
                self.esquecer_preview();
                self.clamp();
            }
            Gesto::AbrirCurrent => return vec![Comando::AbrirCurrent],
            Gesto::Reler => {
                self.info("relido");
                return vec![Comando::Reler {
                    depois: Depois::Manter,
                    saida: String::new(),
                }];
            }
            Gesto::Fase
            | Gesto::Titulo
            | Gesto::Descricao
            | Gesto::Prazo
            | Gesto::Renomear
            | Gesto::BloqueadaPor
            | Gesto::Arquivar
            | Gesto::Largar => self.na_task(gesto),
        }
        vec![]
    }

    /// Teclas que agem sobre a task selecionada: abrem o modal certo.
    fn na_task(&mut self, gesto: Gesto) {
        let Some(t) = self.task_atual().cloned() else {
            self.erro("nenhuma task selecionada");
            return;
        };
        self.modo = match gesto {
            // Largar é arquivar como `dropped`, não apagar: o arquivo fica e o
            // `archive` reaponta os links. Apagar de vez é só pela CLI.
            Gesto::Largar if t.archived.is_some() => {
                self.erro(format!("{} já está arquivada (a desarquiva)", t.id));
                return;
            }
            Gesto::Largar => Modo::Confirmar(Confirmacao::Largar),
            Gesto::Arquivar if t.archived.is_some() => Modo::Confirmar(Confirmacao::Desarquivar),
            Gesto::Arquivar => Modo::Pick {
                escolha: Escolha::Arquivar,
                opcoes: ARCHIVED_TYPES
                    .iter()
                    .map(|tipo| {
                        (
                            tipo.to_string(),
                            format!("{tipo} — {}", archived_label(tipo)),
                        )
                    })
                    .collect(),
                idx: ARCHIVED_TYPES
                    .iter()
                    .position(|tipo| *tipo == t.suggested_archive)
                    .unwrap_or(1),
            },
            Gesto::Fase => {
                // Só se oferece o que o `set` aceita: o `core` recusa `done`
                // (que o `archive` grava) e o sentinela 0.
                let vocab = &self.dados.vocab;
                let opcoes: Vec<(String, String)> = vocab
                    .linhas()
                    .into_iter()
                    .filter(|(_, m, _)| check_status_change(vocab, &m.callout, None).is_ok())
                    .map(|(_, m, _)| (m.callout, format!("{} {}", m.icon, m.title)))
                    .collect();
                let idx = opcoes
                    .iter()
                    .position(|(c, _)| *c == t.callout)
                    .unwrap_or(0);
                Modo::Pick {
                    escolha: Escolha::Fase,
                    opcoes,
                    idx,
                }
            }
            Gesto::Renomear => Modo::Prompt {
                campo: Campo::Renomear,
                valor: t.id,
            },
            Gesto::BloqueadaPor => Modo::Prompt {
                campo: Campo::BloqueadaPor,
                valor: t.bloqueada_por.join(", "),
            },
            Gesto::Titulo => Modo::Prompt {
                campo: Campo::Titulo,
                valor: t.title,
            },
            Gesto::Descricao => Modo::Prompt {
                campo: Campo::Descricao,
                valor: t.desc,
            },
            Gesto::Prazo => Modo::Prompt {
                campo: Campo::Prazo,
                valor: t.prazo.unwrap_or_default(),
            },
            _ => Modo::Normal,
        };
    }

    fn mover(&mut self, delta: isize) {
        let (idx, len) = match self.painel {
            Painel::Projetos => (self.project_idx, self.dados.projetos.len()),
            Painel::Tasks => (self.task_idx, self.tasks_visiveis().len()),
        };
        if len == 0 {
            return;
        }
        let proximo = (idx as isize)
            .saturating_add(delta)
            .clamp(0, len as isize - 1) as usize;
        match self.painel {
            Painel::Projetos => {
                if proximo != self.project_idx {
                    self.project_idx = proximo;
                    self.task_idx = 0;
                }
            }
            Painel::Tasks => self.task_idx = proximo,
        }
        self.preview_scroll = 0;
    }

    fn no_filtro(&mut self, tecla: KeyEvent) -> Vec<Comando> {
        match tecla.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.task_idx = 0;
            }
            KeyCode::Enter => {}
            KeyCode::Down => {
                self.mover(1);
                self.modo = Modo::Filtro;
            }
            KeyCode::Up => {
                self.mover(-1);
                self.modo = Modo::Filtro;
            }
            KeyCode::Backspace => {
                self.filtro.pop();
                self.task_idx = 0;
                self.modo = Modo::Filtro;
            }
            KeyCode::Char(c) => {
                self.filtro.push(c);
                self.task_idx = 0;
                self.modo = Modo::Filtro;
            }
            _ => self.modo = Modo::Filtro,
        }
        vec![]
    }

    fn na_nova_task(&mut self, mut nt: NovaTask, tecla: KeyEvent) -> Vec<Comando> {
        let ctrl = tecla.modifiers.contains(KeyModifiers::CONTROL);
        let mut comandos = vec![];
        match tecla.code {
            KeyCode::Esc => return vec![],
            KeyCode::Enter if nt.campo == 2 => return self.enviar_nova_task(nt),
            KeyCode::Enter | KeyCode::Tab | KeyCode::Down => {
                let de = nt.campo;
                nt.campo = (nt.campo + 1) % 3;
                if de == 0 && !nt.id_digitado && !nt.titulo.trim().is_empty() {
                    comandos.push(Comando::SugerirId(nt.titulo.trim().to_string()));
                }
            }
            KeyCode::BackTab | KeyCode::Up => nt.campo = (nt.campo + 2) % 3,
            KeyCode::Char('u') if ctrl => {
                nt.campo_mut().clear();
                if nt.campo == 1 {
                    nt.id_digitado = false;
                }
            }
            KeyCode::Backspace => {
                nt.campo_mut().pop();
                if nt.campo == 1 {
                    nt.id_digitado = !nt.id.is_empty();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                nt.campo_mut().push(c);
                if nt.campo == 1 {
                    nt.id_digitado = true;
                }
            }
            _ => {}
        }
        self.modo = Modo::NovaTask(nt);
        comandos
    }

    fn enviar_nova_task(&mut self, mut nt: NovaTask) -> Vec<Comando> {
        let Some(projeto) = self.projeto_atual().map(|p| p.nome.clone()) else {
            return vec![];
        };
        let (titulo, id, desc) = (
            nt.titulo.trim().to_string(),
            nt.id.trim().to_string(),
            nt.desc.trim().to_string(),
        );
        if titulo.is_empty() || id.is_empty() {
            nt.campo = if titulo.is_empty() { 0 } else { 1 };
            self.erro(if titulo.is_empty() {
                "falta o título"
            } else {
                "falta o id"
            });
            self.modo = Modo::NovaTask(nt);
            return vec![];
        }
        self.refazer = Some(Modo::NovaTask(nt));
        vec![Comando::Executar {
            escrita: Escrita::NovaTask {
                projeto: projeto.clone(),
                id: id.clone(),
                titulo,
                desc,
            },
            depois: Depois::Task {
                projeto,
                id: id.clone(),
            },
            ok: format!("criada: {id}"),
        }]
    }

    fn no_prompt(&mut self, campo: Campo, mut valor: String, tecla: KeyEvent) -> Vec<Comando> {
        let ctrl = tecla.modifiers.contains(KeyModifiers::CONTROL);
        match tecla.code {
            KeyCode::Esc => return vec![],
            KeyCode::Enter => return self.enviar_prompt(campo, valor),
            KeyCode::Char('u') if ctrl => valor.clear(),
            KeyCode::Backspace => {
                valor.pop();
            }
            KeyCode::Char(c) if !ctrl => valor.push(c),
            _ => {}
        }
        self.modo = Modo::Prompt { campo, valor };
        vec![]
    }

    fn enviar_prompt(&mut self, campo: Campo, valor: String) -> Vec<Comando> {
        let v = valor.trim().to_string();
        if v.is_empty() && !matches!(campo, Campo::Descricao | Campo::BloqueadaPor | Campo::Prazo) {
            self.erro("não pode ficar vazio");
            self.modo = Modo::Prompt { campo, valor };
            return vec![];
        }
        if campo == Campo::NovoProjeto {
            self.refazer = Some(Modo::Prompt { campo, valor });
            return vec![Comando::Executar {
                escrita: Escrita::NovoProjeto(v),
                depois: Depois::ProjetoDaSaida,
                ok: "projeto criado".into(),
            }];
        }
        let Some(t) = self.task_atual().cloned() else {
            return vec![];
        };
        let set = |pedido: PedidoSet| Escrita::Set {
            referencia: t.path.clone(),
            pedido,
        };
        let (escrita, depois, ok) = match campo {
            Campo::Titulo if v == t.title => return vec![],
            Campo::Descricao if v == t.desc => return vec![],
            Campo::Renomear if v == t.id => return vec![],
            Campo::BloqueadaPor if v == t.bloqueada_por.join(", ") => return vec![],
            Campo::Prazo if v == t.prazo.as_deref().unwrap_or_default() => return vec![],
            Campo::Titulo => (
                set(PedidoSet {
                    title: Some(v),
                    ..PedidoSet::default()
                }),
                Depois::Manter,
                "título atualizado".to_string(),
            ),
            Campo::Descricao => (
                set(PedidoSet {
                    desc: Some(v),
                    ..PedidoSet::default()
                }),
                Depois::Manter,
                "descrição atualizada".to_string(),
            ),
            Campo::Prazo => {
                let ok = if v.is_empty() {
                    "prazo removido".to_string()
                } else {
                    format!("prazo: {v}")
                };
                (
                    set(PedidoSet {
                        prazo: Some(v),
                        ..PedidoSet::default()
                    }),
                    Depois::Manter,
                    ok,
                )
            }
            Campo::Renomear => (
                Escrita::Deslocar {
                    referencia: t.path.clone(),
                    pedido: Pedido::Rename(v.clone()),
                },
                Depois::Task {
                    projeto: t.project.clone(),
                    id: v.clone(),
                },
                format!("renomeada: {} → {v}", t.id),
            ),
            // A lista vai como foi digitada: separar, aparar e tirar repetidos
            // é do motor (decisão 61).
            Campo::BloqueadaPor => (
                set(PedidoSet {
                    bloqueada_por: Some(v),
                    ..PedidoSet::default()
                }),
                Depois::Manter,
                "bloqueada-por atualizada".to_string(),
            ),
            // Tratado acima: o nome do projeto não é de nenhuma task.
            Campo::NovoProjeto => return vec![],
        };
        self.refazer = Some(Modo::Prompt { campo, valor });
        vec![Comando::Executar {
            escrita,
            depois,
            ok,
        }]
    }

    fn no_pick(
        &mut self,
        escolha: Escolha,
        opcoes: Vec<(String, String)>,
        mut idx: usize,
        tecla: KeyEvent,
    ) -> Vec<Comando> {
        match tecla.code {
            KeyCode::Esc | KeyCode::Char('q') => return vec![],
            KeyCode::Char('j') | KeyCode::Down => {
                idx = (idx + 1).min(opcoes.len().saturating_sub(1))
            }
            KeyCode::Char('k') | KeyCode::Up => idx = idx.saturating_sub(1),
            KeyCode::Enter => {
                let (Some(t), Some((valor, rotulo))) =
                    (self.task_atual().cloned(), opcoes.get(idx))
                else {
                    return vec![];
                };
                return match escolha {
                    Escolha::Fase if *valor == t.callout => vec![],
                    Escolha::Fase => vec![Comando::Executar {
                        escrita: Escrita::Set {
                            referencia: t.path,
                            pedido: PedidoSet {
                                status: Some(valor.clone()),
                                ..PedidoSet::default()
                            },
                        },
                        depois: Depois::Manter,
                        ok: format!("fase: {rotulo}"),
                    }],
                    Escolha::Arquivar => vec![Comando::Executar {
                        escrita: Escrita::Deslocar {
                            referencia: t.path,
                            pedido: Pedido::Archive(Some(valor.clone())),
                        },
                        depois: Depois::Manter,
                        ok: format!("arquivada como {valor}: {}", t.id),
                    }],
                };
            }
            _ => {}
        }
        self.modo = Modo::Pick {
            escolha,
            opcoes,
            idx,
        };
        vec![]
    }

    fn na_confirmacao(&mut self, confirmacao: Confirmacao, tecla: KeyEvent) -> Vec<Comando> {
        if !matches!(tecla.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
            return vec![];
        }
        let Some(t) = self.task_atual().cloned() else {
            return vec![];
        };
        let comando = match confirmacao {
            Confirmacao::Largar => Comando::Executar {
                escrita: Escrita::Deslocar {
                    referencia: t.path,
                    pedido: Pedido::Archive(Some("dropped".into())),
                },
                depois: Depois::Manter,
                ok: format!("largada: {}", t.id),
            },
            Confirmacao::Desarquivar => Comando::Executar {
                escrita: Escrita::Deslocar {
                    referencia: t.path,
                    pedido: Pedido::Unarchive,
                },
                depois: Depois::Task {
                    projeto: t.project.clone(),
                    id: t.id.clone(),
                },
                ok: format!("desarquivada: {}", t.id),
            },
        };
        vec![comando]
    }
}
