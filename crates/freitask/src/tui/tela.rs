//! Desenho. Só lê o `State`; nenhuma decisão mora aqui.

use freitask_core::config::ARCHIVED_TYPES;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::linhas::Resumo;
use crate::tui::atalhos::ATALHOS;
use crate::tui::estado::{Campo, Confirmacao, Escolha, Modo, NovaTask, Painel, State};

/// A dica da barra quando não há nada a dizer.
const DICA: &str =
    "? ajuda · n nova · s fase · a arquivar · d largar · e abrir · / filtro · q sair";

/// Os grupos de highlight do status.json viram cor de terminal.
fn cor_do_grupo(hl: &str) -> Color {
    match hl {
        "DiagnosticInfo" => Color::Blue,
        "DiagnosticHint" => Color::Cyan,
        "DiagnosticWarn" => Color::Yellow,
        "DiagnosticOk" => Color::Green,
        "DiagnosticError" => Color::Red,
        "Comment" => Color::DarkGray,
        _ => Color::Reset,
    }
}

fn apagado() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn bloco_do_painel(titulo: String, com_foco: bool) -> Block<'static> {
    let (borda, estilo_do_titulo) = if com_foco {
        (
            Style::default().fg(Color::Green),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (apagado(), Style::default())
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(borda)
        .title(Span::styled(titulo, estilo_do_titulo))
}

fn destaque() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Desenha a tela inteira.
pub fn view(estado: &State, f: &mut Frame) {
    let [principal, barra] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(f.area());
    let [esquerda, meio, direita] = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(35),
        Constraint::Percentage(45),
    ])
    .areas(principal);
    projetos(f, estado, esquerda);
    tasks(f, estado, meio);
    preview(f, estado, direita);
    barra_de_baixo(f, estado, barra);

    match &estado.modo {
        Modo::NovaTask(nt) => modal_nova_task(f, estado, nt),
        Modo::Prompt { campo, valor } => modal_prompt(f, *campo, valor),
        Modo::Pick {
            escolha,
            opcoes,
            idx,
        } => modal_pick(f, estado, *escolha, opcoes, *idx),
        Modo::Confirmar(c) => modal_confirmar(f, estado, *c),
        Modo::Ajuda => modal_ajuda(f),
        Modo::Normal | Modo::Filtro => {}
    }
}

fn projetos(f: &mut Frame, estado: &State, area: Rect) {
    let itens: Vec<ListItem> = estado
        .dados
        .projetos
        .iter()
        .map(|p| {
            ListItem::new(Line::from(vec![
                Span::styled(" ", Style::default().fg(Color::Blue)),
                Span::raw(p.nome.clone()),
                Span::styled(format!(" {}", p.ativas), apagado()),
            ]))
        })
        .collect();
    let mut selecao = ListState::default();
    if !estado.dados.projetos.is_empty() {
        selecao.select(Some(estado.project_idx));
    }
    let lista = List::new(itens)
        .block(bloco_do_painel(
            " Projetos ".into(),
            estado.painel == Painel::Projetos,
        ))
        .highlight_style(destaque());
    f.render_stateful_widget(lista, area, &mut selecao);
}

fn tasks(f: &mut Frame, estado: &State, area: Rect) {
    let visiveis = estado.tasks_visiveis();
    let itens: Vec<ListItem> = visiveis
        .iter()
        .map(|t| {
            let fase = estado.dados.vocab.meta(t.status_num);
            let cor = cor_do_grupo(&fase.hl_group);
            let mut spans = Vec::new();
            // Uma coluna fixa para a marca da prioridade alta, para a lista não
            // andar de lado conforme a task.
            if t.prioridade.is_some() {
                spans.push(Span::styled("! ", Style::default().fg(Color::Red)));
            } else {
                spans.push(Span::raw("  "));
            }
            if let Some(a) = &t.archived {
                spans.push(Span::styled(format!("[{a}] "), apagado()));
            }
            spans.push(Span::styled(
                format!("{} ", fase.icon),
                Style::default().fg(cor),
            ));
            spans.extend(marca_bloqueada(t));
            spans.push(Span::styled(
                t.id.clone(),
                if t.archived.is_some() {
                    apagado()
                } else {
                    Style::default()
                },
            ));
            // A adiada só aparece com `S`, e a data de volta fica colada no id: a
            // coluna do título é a que o painel estreito corta.
            if let Some(volta) = &t.volta_em {
                spans.push(Span::styled(
                    format!("  volta {volta}"),
                    Style::default().fg(Color::Yellow),
                ));
            }
            spans.push(Span::styled(format!("  {}", t.title), apagado()));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let projeto = estado.projeto_atual().map_or("—", |p| p.nome.as_str());
    let mut titulo = format!(" Tasks: {projeto} ");
    if estado.mostrar_arquivadas {
        titulo.push_str("+arquivadas ");
    }
    if estado.mostrar_escondidas {
        titulo.push_str("+algum-dia/adiadas ");
    }
    if !estado.filtro.is_empty() {
        titulo.push_str(&format!("/{} ", estado.filtro));
    }
    let mut selecao = ListState::default();
    if !visiveis.is_empty() {
        selecao.select(Some(estado.task_idx));
    }
    let lista = List::new(itens)
        .block(bloco_do_painel(titulo, estado.painel == Painel::Tasks))
        .highlight_style(destaque());
    f.render_stateful_widget(lista, area, &mut selecao);
}

/// A marca da task que espera outra (f9b.2): sem coluna fixa, só aparece
/// quando o `bloqueada` do JSON é verdadeiro.
fn marca_bloqueada(t: &Resumo) -> Option<Span<'static>> {
    t.bloqueada
        .then(|| Span::styled("⛔ ", Style::default().fg(Color::Red)))
}

fn preview(f: &mut Frame, estado: &State, area: Rect) {
    let bloco = bloco_do_painel(" Preview ".into(), false);
    let Some(t) = estado.task_atual() else {
        f.render_widget(
            Paragraph::new(Span::styled("sem task selecionada", apagado())).block(bloco),
            area,
        );
        return;
    };
    let fase = estado.dados.vocab.meta(t.status_num);
    let cor = cor_do_grupo(&fase.hl_group);

    let mut linhas = vec![
        Line::from(Span::styled(
            t.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("{} {}", fase.icon, t.status),
                Style::default().fg(cor),
            ),
            Span::styled(format!("  ·  {}", t.id), apagado()),
            Span::styled(
                t.archived
                    .as_ref()
                    .map(|a| format!("  ·  archived/{a}"))
                    .unwrap_or_default(),
                apagado(),
            ),
            Span::styled(
                t.volta_em
                    .as_ref()
                    .map(|v| format!("  ·  volta em {v}"))
                    .unwrap_or_default(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];
    if !t.desc.is_empty() {
        linhas.push(Line::from(Span::styled(
            t.desc.clone(),
            Style::default().add_modifier(Modifier::ITALIC),
        )));
    }
    linhas.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(2) as usize),
        apagado(),
    )));

    if let Some((_, conteudo)) = estado
        .preview
        .as_ref()
        .filter(|(caminho, _)| *caminho == t.path)
    {
        for l in conteudo.lines() {
            let estilo = if l.starts_with('#') {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else if l.starts_with('>') {
                Style::default().fg(cor)
            } else if l == "---" {
                apagado()
            } else {
                Style::default()
            };
            linhas.push(Line::from(Span::styled(l.to_string(), estilo)));
        }
    }

    let p = Paragraph::new(linhas)
        .block(bloco)
        .wrap(Wrap { trim: false })
        .scroll((estado.preview_scroll, 0));
    f.render_widget(p, area);
}

fn barra_de_baixo(f: &mut Frame, estado: &State, area: Rect) {
    let linha = if estado.modo == Modo::Filtro {
        let texto = format!("/{}", estado.filtro);
        let largura = Span::raw(texto.as_str()).width() as u16;
        f.set_cursor_position((area.x.saturating_add(largura), area.y));
        Line::from(Span::styled(texto, Style::default().fg(Color::Yellow)))
    } else if let Some(m) = &estado.mensagem {
        let cor = if m.erro { Color::Red } else { Color::Green };
        Line::from(Span::styled(m.texto.clone(), Style::default().fg(cor)))
    } else {
        Line::from(Span::styled(DICA, apagado()))
    };
    f.render_widget(Paragraph::new(linha), area);
}

fn centralizado(area: Rect, largura: u16, altura: u16) -> Rect {
    let largura = largura.min(area.width);
    let altura = altura.min(area.height);
    Rect {
        x: area.x + (area.width - largura) / 2,
        y: area.y + (area.height - altura) / 2,
        width: largura,
        height: altura,
    }
}

fn modal(titulo: String) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            titulo,
            Style::default().add_modifier(Modifier::BOLD),
        ))
}

/// Põe o cursor no fim do texto, sem sair do retângulo de dentro do modal.
fn cursor_no_fim(f: &mut Frame, dentro: Rect, linha: u16, largura_do_texto: usize) {
    let x = dentro
        .x
        .saturating_add(u16::try_from(largura_do_texto).unwrap_or(u16::MAX));
    f.set_cursor_position((
        x.min(dentro.right().saturating_sub(1)),
        dentro
            .y
            .saturating_add(linha)
            .min(dentro.bottom().saturating_sub(1)),
    ));
}

fn modal_nova_task(f: &mut Frame, estado: &State, nt: &NovaTask) {
    let projeto = estado.projeto_atual().map_or("", |p| p.nome.as_str());
    let area = centralizado(f.area(), 64, 5);
    f.render_widget(Clear, area);
    let bloco = modal(format!(" Nova task em {projeto} ")).title_bottom(
        Line::from(" Tab próximo · Enter no último cria · Esc cancela ").style(apagado()),
    );
    let dentro = bloco.inner(area);
    f.render_widget(bloco, area);

    const ROTULOS: [&str; 3] = ["Título ", "Id     ", "Estado "];
    let valores = [&nt.titulo, &nt.id, &nt.desc];
    let linhas: Vec<Line> = ROTULOS
        .iter()
        .zip(valores)
        .enumerate()
        .map(|(i, (rotulo, valor))| {
            let estilo = if i == nt.campo {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                apagado()
            };
            Line::from(vec![
                Span::styled(*rotulo, estilo),
                Span::raw(valor.clone()),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(linhas), dentro);
    cursor_no_fim(
        f,
        dentro,
        nt.campo as u16,
        Span::raw(ROTULOS[nt.campo]).width() + Span::raw(valores[nt.campo].as_str()).width(),
    );
}

fn modal_prompt(f: &mut Frame, campo: Campo, valor: &str) {
    let titulo = match campo {
        Campo::Titulo => " Título ",
        Campo::Descricao => " Descrição do estado (vazio apaga) ",
        Campo::Renomear => " Novo id (renomeia arquivo e links do vault) ",
        Campo::BloqueadaPor => " Bloqueada por (ids separados por vírgula; vazio limpa) ",
        Campo::NovoProjeto => " Novo projeto ",
    };
    let area = centralizado(f.area(), 64, 3);
    f.render_widget(Clear, area);
    let bloco = modal(titulo.into())
        .title_bottom(Line::from(" Enter confirma · Esc cancela ").style(apagado()));
    let dentro = bloco.inner(area);
    f.render_widget(bloco, area);
    f.render_widget(Paragraph::new(valor.to_string()), dentro);
    cursor_no_fim(f, dentro, 0, Span::raw(valor).width());
}

fn modal_pick(
    f: &mut Frame,
    estado: &State,
    escolha: Escolha,
    opcoes: &[(String, String)],
    idx: usize,
) {
    let id = estado.task_atual().map_or("", |t| t.id.as_str());
    let titulo = match escolha {
        Escolha::Fase => format!(" Fase de {id} "),
        Escolha::Arquivar => format!(" Arquivar {id} como "),
    };
    let area = centralizado(f.area(), 44, opcoes.len() as u16 + 2);
    f.render_widget(Clear, area);
    let itens: Vec<ListItem> = opcoes
        .iter()
        .map(|(valor, rotulo)| {
            let cor = match escolha {
                Escolha::Fase => estado
                    .dados
                    .vocab
                    .by_callout(valor)
                    .map_or(Color::Reset, |n| {
                        cor_do_grupo(&estado.dados.vocab.meta(n).hl_group)
                    }),
                // O primeiro tipo é o que as fases prontas sugerem.
                Escolha::Arquivar => {
                    if ARCHIVED_TYPES[0] == valor {
                        Color::Green
                    } else {
                        Color::Reset
                    }
                }
            };
            ListItem::new(Span::styled(rotulo.clone(), Style::default().fg(cor)))
        })
        .collect();
    let mut selecao = ListState::default();
    selecao.select(Some(idx));
    let lista = List::new(itens)
        .block(modal(titulo))
        .highlight_style(destaque())
        .highlight_symbol("› ");
    f.render_stateful_widget(lista, area, &mut selecao);
}

fn modal_confirmar(f: &mut Frame, estado: &State, confirmacao: Confirmacao) {
    let id = estado.task_atual().map_or("", |t| t.id.as_str());
    let texto = match confirmacao {
        Confirmacao::Largar => format!("Largar {id}? Ela vai para archived/dropped."),
        Confirmacao::Desarquivar => format!("Desarquivar {id}?"),
    };
    let largura = u16::try_from(texto.chars().count() + 4).unwrap_or(u16::MAX);
    let area = centralizado(f.area(), largura.max(30), 3);
    f.render_widget(Clear, area);
    let bloco = modal(" Confirmar ".into())
        .title_bottom(Line::from(" y sim · qualquer outra não ").style(apagado()));
    f.render_widget(Paragraph::new(texto).block(bloco), area);
}

fn modal_ajuda(f: &mut Frame) {
    // Larga o bastante para a descrição mais comprida caber inteira (o `S`).
    let area = centralizado(f.area(), 58, ATALHOS.len() as u16 + 2);
    f.render_widget(Clear, area);
    let linhas: Vec<Line> = ATALHOS
        .iter()
        .map(|a| {
            Line::from(vec![
                Span::styled(
                    format!("{:>14}  ", a.rotulo),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(a.descricao),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(linhas).block(modal(" Teclas ".into())), area);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    //! Camada 3 da §3.5: a tela num `TestBackend`, uma por tela, com
    //! asserções semânticas sobre a grade (o que está escrito onde, o que
    //! está em destaque). Cada tela tem a sua função `tela_*`, que monta o
    //! `Buffer` e tira o snapshot do `Debug` dele em 100×30 e 60×15 (`foto`,
    //! decisão f7.9).

    use freitask_core::status::Vocab;
    use ratatui::backend::{Backend, TestBackend};
    use ratatui::buffer::Buffer;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::style::Color;
    use ratatui::text::Line;
    use ratatui::{Terminal, layout::Rect};

    use super::*;
    use crate::linhas::Resumo;
    use crate::tui::dados::{Dados, Projeto};
    use crate::tui::estado::{Evento, update};

    /// Uma task com a fase tirada do vocabulário default, como o `list` a dá.
    fn tarefa(projeto: &str, id: &str, titulo: &str, fase: u32) -> Resumo {
        let vocab = Vocab::default();
        let meta = vocab.meta(fase);
        Resumo {
            project: projeto.into(),
            id: id.into(),
            status: meta.title.clone(),
            status_num: fase,
            callout: meta.callout.clone(),
            title: titulo.into(),
            desc: String::new(),
            archived: None,
            suggested_archive: if meta.callout == "check" {
                "done"
            } else {
                "dropped"
            },
            hidden: false,
            prioridade: None,
            volta_em: None,
            path: format!("/v/projects/{projeto}/tasks/{id}.md"),
            bloqueada_por: vec![],
            bloqueada: false,
            bloqueia: vec![],
        }
    }

    /// `alfa`: escrever-spec (Não iniciada, com descrição), medir-tempo (Em
    /// implementação), fechar-ciclo (Pronta); escondidas: sonho (Algum dia) e
    /// adiada (volta em 2026-12-01); arquivada: velha-feita. `beta`:
    /// beta-urgente (prioridade alta) e beta-um.
    fn dados() -> Dados {
        let mut escrever = tarefa("alfa", "escrever-spec", "Escrever a spec", 1);
        escrever.desc = "em curso".into();
        let sonho = Resumo {
            hidden: true,
            ..tarefa("alfa", "sonho", "Sonho distante", 8)
        };
        let adiada = Resumo {
            hidden: true,
            volta_em: Some("2026-12-01".into()),
            ..tarefa("alfa", "adiada", "Adiada até dezembro", 3)
        };
        let velha = Resumo {
            archived: Some("done".into()),
            path: "/v/projects/alfa/tasks/archived/done/velha-feita.md".into(),
            ..tarefa("alfa", "velha-feita", "Velha feita", 9)
        };
        let urgente = Resumo {
            prioridade: Some("alta"),
            ..tarefa("beta", "beta-urgente", "Bê urgente", 4)
        };
        Dados {
            vocab: Vocab::default(),
            projetos: vec![
                Projeto {
                    nome: "alfa".into(),
                    ativas: 3,
                },
                Projeto {
                    nome: "beta".into(),
                    ativas: 2,
                },
            ],
            tasks: vec![
                escrever,
                adiada,
                tarefa("alfa", "medir-tempo", "Medir o tempo", 4),
                tarefa("alfa", "fechar-ciclo", "Fechar o ciclo", 7),
                sonho,
                velha,
                urgente,
                tarefa("beta", "beta-um", "Bê um", 1),
            ],
        }
    }

    fn tecla(c: char) -> KeyEvent {
        let m = if c.is_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        KeyEvent::new(KeyCode::Char(c), m)
    }

    /// O estado depois de apertar estas teclas num vault com o preview já lido
    /// (o laço lê o arquivo a cada seleção; aqui o conteúdo é fixo).
    fn estado_apos(teclas: &[KeyEvent]) -> State {
        estado_com(dados(), teclas)
    }

    fn estado_com(d: Dados, teclas: &[KeyEvent]) -> State {
        let mut s = com_preview(State::novo(d));
        for t in teclas {
            s = com_preview(update(s, Evento::Tecla(*t)).0);
        }
        s
    }

    fn com_preview(s: State) -> State {
        let Some(t) = s.task_atual() else {
            return s;
        };
        let caminho = t.path.clone();
        let conteudo = format!(
            "> [!todo] {}\n> [[{}]]\n\n## Histórico\n\n- 2026-11-10 — criada\n",
            t.title, t.id
        );
        update(s, Evento::PreviewLido { caminho, conteudo }).0
    }

    fn teclas(texto: &str) -> Vec<KeyEvent> {
        texto.chars().map(tecla).collect()
    }

    /// Desenha e devolve a grade e onde o cursor ficou.
    fn desenha(s: &State, largura: u16, altura: u16) -> (Buffer, (u16, u16)) {
        let mut terminal = Terminal::new(TestBackend::new(largura, altura)).unwrap();
        terminal.draw(|f| view(s, f)).unwrap();
        let cursor = terminal.backend_mut().get_cursor_position().unwrap();
        (terminal.backend().buffer().clone(), (cursor.x, cursor.y))
    }

    fn linhas(b: &Buffer) -> Vec<String> {
        (0..b.area.height)
            .map(|y| {
                (0..b.area.width)
                    .map(|x| b[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    fn texto(b: &Buffer) -> String {
        linhas(b).join("\n")
    }

    /// Nenhuma linha passa da largura do terminal e a grade tem o tamanho
    /// pedido. A célula depois do `⛔` é a metade escondida dele: não conta.
    fn cabe(b: &Buffer, largura: u16, altura: u16) {
        assert_eq!(b.area, Rect::new(0, 0, largura, altura));
        for l in linhas(b) {
            let l = l.replace("⛔ ", "⛔");
            assert!(Line::from(l.as_str()).width() <= largura as usize, "{l:?}");
        }
    }

    /// A área de dentro do painel (ou do modal) cujo título contém `titulo`.
    fn painel(b: &Buffer, titulo: &str) -> Rect {
        let ls = linhas(b);
        let (y, l) = ls
            .iter()
            .enumerate()
            .find(|(_, l)| l.contains(titulo))
            .unwrap_or_else(|| panic!("sem {titulo:?} na tela:\n{}", texto(b)));
        let x_titulo = l[..l.find(titulo).unwrap()].chars().count() as u16;
        let esq = (0..=x_titulo)
            .rev()
            .find(|&x| b[(x, y as u16)].symbol() == "╭")
            .unwrap_or_else(|| panic!("sem canto esquerdo de {titulo:?}:\n{}", texto(b)));
        let dir = (x_titulo..b.area.width)
            .find(|&x| b[(x, y as u16)].symbol() == "╮")
            .unwrap_or_else(|| panic!("sem canto direito de {titulo:?}:\n{}", texto(b)));
        let baixo = (y as u16 + 1..b.area.height)
            .find(|&yy| b[(esq, yy)].symbol() == "╰")
            .unwrap_or(b.area.height);
        Rect::new(esq + 1, y as u16 + 1, dir - esq - 1, baixo - y as u16 - 1)
    }

    fn linhas_do(b: &Buffer, p: Rect) -> Vec<String> {
        (p.y..p.bottom())
            .map(|y| {
                (p.x..p.right())
                    .map(|x| b[(x, y)].symbol())
                    .collect::<String>()
                    .trim()
                    .to_string()
            })
            .collect()
    }

    fn itens(b: &Buffer, p: Rect) -> Vec<String> {
        linhas_do(b, p)
            .into_iter()
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// As linhas do painel desenhadas em destaque (fundo cinza).
    fn destacadas(b: &Buffer, p: Rect) -> Vec<String> {
        (p.y..p.bottom())
            .filter(|&y| (p.x..p.right()).any(|x| b[(x, y)].bg == Color::DarkGray))
            .map(|y| {
                (p.x..p.right())
                    .map(|x| b[(x, y)].symbol())
                    .collect::<String>()
                    .trim()
                    .to_string()
            })
            .collect()
    }

    fn barra(b: &Buffer) -> String {
        linhas(b).pop().unwrap().trim().to_string()
    }

    fn cor_da_barra(b: &Buffer) -> Color {
        let y = b.area.height - 1;
        let x = (0..b.area.width)
            .find(|&x| !b[(x, y)].symbol().trim().is_empty())
            .unwrap();
        b[(x, y)].fg
    }

    // --- uma função por tela, com o snapshot dela ------------------------------

    /// Tira o snapshot do `Debug` da grade (com os estilos) nos dois tamanhos
    /// da §3.5, 100×30 e 60×15, e devolve a de 100×30 com o cursor. Os `.snap`
    /// nascem uma vez por `scripts/snapshots-novos.sh` e não se regravam.
    fn foto(nome: &str, s: &State) -> (Buffer, (u16, u16)) {
        let pequena = desenha(s, 60, 15).0;
        insta::assert_debug_snapshot!(format!("{nome}_60x15"), pequena);
        let grande = desenha(s, 100, 30);
        insta::assert_debug_snapshot!(format!("{nome}_100x30"), grande.0);
        grande
    }

    fn tela_da_lista() -> Buffer {
        foto("lista", &estado_apos(&[])).0
    }

    fn tela_da_task_selecionada() -> Buffer {
        foto("task_selecionada", &estado_apos(&teclas("lj"))).0
    }

    fn tela_do_filtro() -> Buffer {
        foto("filtro", &estado_apos(&teclas("/spec"))).0
    }

    fn tela_da_task_nova() -> (Buffer, (u16, u16)) {
        let mut t = teclas("ln");
        t.extend(teclas("Nova"));
        foto("task_nova", &estado_apos(&t))
    }

    fn tela_da_fase() -> Buffer {
        foto("fase", &estado_apos(&teclas("ls"))).0
    }

    fn tela_do_arquivar() -> Buffer {
        foto("arquivar", &estado_apos(&teclas("la"))).0
    }

    fn tela_da_confirmacao() -> Buffer {
        foto("confirmacao", &estado_apos(&teclas("ld"))).0
    }

    fn tela_da_ajuda() -> Buffer {
        foto("ajuda", &estado_apos(&teclas("?"))).0
    }

    fn tela_das_arquivadas() -> Buffer {
        foto("arquivadas", &estado_apos(&teclas("lA"))).0
    }

    fn tela_das_escondidas() -> Buffer {
        foto("escondidas", &estado_apos(&teclas("lS"))).0
    }

    fn tela_do_vault_vazio() -> Buffer {
        foto("vault_vazio", &State::novo(Dados::default())).0
    }

    /// `dados()` com `medir-tempo` esperando `escrever-spec` e `beta-urgente`
    /// (prioridade alta) esperando `beta-um`, numa cópia: o `dados()` das
    /// outras telas não muda.
    fn dados_bloqueada() -> Dados {
        let mut d = dados();
        for t in &mut d.tasks {
            match t.id.as_str() {
                "medir-tempo" => {
                    t.bloqueada_por = vec!["escrever-spec".into()];
                    t.bloqueada = true;
                }
                "escrever-spec" => t.bloqueia = vec!["medir-tempo".into()],
                "beta-urgente" => {
                    t.bloqueada_por = vec!["beta-um".into()];
                    t.bloqueada = true;
                }
                "beta-um" => t.bloqueia = vec!["beta-urgente".into()],
                _ => {}
            }
        }
        d
    }

    fn tela_da_task_bloqueada() -> Buffer {
        foto(
            "task_bloqueada",
            &estado_com(dados_bloqueada(), &teclas("lj")),
        )
        .0
    }

    fn tela_do_prompt_bloqueada_por() -> (Buffer, (u16, u16)) {
        foto(
            "prompt_bloqueada_por",
            &estado_com(dados_bloqueada(), &teclas("ljb")),
        )
    }

    fn tela_estreita() -> Buffer {
        let s = estado_apos(&teclas("lG"));
        foto("estreita", &s);
        desenha(&s, 60, 15).0
    }

    // --- as telas -----------------------------------------------------------------

    #[test]
    fn a_lista_tem_os_tres_paineis_o_projeto_em_destaque_e_a_dica() {
        let b = tela_da_lista();
        cabe(&b, 100, 30);
        let l = linhas(&b);
        let (p, t, v) = (
            l[0].find("Projetos").unwrap(),
            l[0].find("Tasks: alfa").unwrap(),
            l[0].find("Preview").unwrap(),
        );
        assert!(p < t && t < v, "projetos | tasks | preview:\n{}", texto(&b));

        let projetos = painel(&b, "Projetos");
        let nomes = itens(&b, projetos);
        assert!(nomes[0].contains("alfa 3"), "{nomes:?}");
        assert!(nomes[1].contains("beta 2"), "{nomes:?}");
        // O foco abre nos projetos: o projeto selecionado é o em destaque.
        let d = destacadas(&b, projetos);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].contains("alfa"), "{d:?}");
        // As tasks, na ordem, sem as escondidas nem as arquivadas.
        let tasks = itens(&b, painel(&b, "Tasks"));
        let ids: Vec<&str> = ["escrever-spec", "medir-tempo", "fechar-ciclo"].to_vec();
        assert_eq!(tasks.len(), ids.len(), "{tasks:?}");
        for (t, id) in tasks.iter().zip(ids) {
            assert!(t.contains(id), "{t:?} devia ter {id}");
        }
        assert_eq!(barra(&b), DICA);
    }

    #[test]
    fn a_task_selecionada_esta_em_destaque_e_o_preview_a_descreve() {
        let b = tela_da_task_selecionada();
        cabe(&b, 100, 30);
        let tasks = painel(&b, "Tasks");
        let d = destacadas(&b, tasks);
        assert_eq!(d.len(), 1, "só a selecionada: {d:?}");
        assert!(d[0].contains("medir-tempo"), "{d:?}");
        let p = linhas_do(&b, painel(&b, "Preview")).join("\n");
        for esperado in [
            "Medir o tempo",
            "Em implementação",
            "medir-tempo",
            "Histórico",
        ] {
            assert!(
                p.contains(esperado),
                "o preview não traz {esperado:?}:\n{p}"
            );
        }
    }

    #[test]
    fn o_preview_mostra_a_descricao_da_task() {
        let b = desenha(&estado_apos(&[]), 100, 30).0;
        let p = linhas_do(&b, painel(&b, "Preview")).join("\n");
        assert!(
            p.contains("Escrever a spec") && p.contains("em curso"),
            "{p}"
        );
    }

    #[test]
    fn o_filtro_mostra_o_que_se_digita_na_barra_e_so_as_tasks_que_casam() {
        let b = tela_do_filtro();
        cabe(&b, 100, 30);
        assert_eq!(barra(&b), "/spec");
        assert_eq!(cor_da_barra(&b), Color::Yellow);
        let tasks = itens(&b, painel(&b, "Tasks"));
        assert_eq!(tasks.len(), 1, "{tasks:?}");
        assert!(tasks[0].contains("escrever-spec"));
        assert!(linhas(&b)[0].contains("/spec"), "o título diz o filtro");
    }

    #[test]
    fn o_modal_de_task_nova_tem_os_tres_campos_e_o_cursor_no_campo_certo() {
        let (b, cursor) = tela_da_task_nova();
        cabe(&b, 100, 30);
        let m = painel(&b, "Nova task em alfa");
        let c = linhas_do(&b, m);
        assert!(c[0].starts_with("Título Nova"), "{c:?}");
        assert!(c[1].starts_with("Id"), "{c:?}");
        assert!(c[2].starts_with("Estado"), "{c:?}");
        assert!(texto(&b).contains("Tab próximo · Enter no último cria · Esc cancela"));
        assert_eq!(
            cursor,
            (m.x + "Título Nova".chars().count() as u16, m.y),
            "o cursor fica no fim do título, na linha do título"
        );
    }

    #[test]
    fn a_escolha_de_fase_lista_as_fases_com_a_atual_em_destaque() {
        let b = tela_da_fase();
        cabe(&b, 100, 30);
        let m = painel(&b, "Fase de escrever-spec");
        let fases = itens(&b, m);
        assert_eq!(fases.len(), 8, "{fases:?}");
        assert!(fases[0].contains("Não iniciada"));
        assert!(!texto(&b).contains("Arquivada"), "done não se oferece");
        let d = destacadas(&b, m);
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("Não iniciada"), "{d:?}");
    }

    #[test]
    fn a_escolha_de_arquivar_sugere_o_tipo_em_destaque() {
        let b = tela_do_arquivar();
        cabe(&b, 100, 30);
        let m = painel(&b, "Arquivar escrever-spec como");
        let tipos = itens(&b, m);
        assert_eq!(tipos.len(), 3, "{tipos:?}");
        assert!(tipos[0].contains("done — feito"));
        let d = destacadas(&b, m);
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("dropped — abandonado"), "{d:?}");
    }

    #[test]
    fn a_confirmacao_pergunta_e_diz_como_responder() {
        let b = tela_da_confirmacao();
        cabe(&b, 100, 30);
        painel(&b, "Confirmar");
        let t = texto(&b);
        assert!(t.contains("Largar escrever-spec? Ela vai para archived/dropped."));
        assert!(t.contains("y sim · qualquer outra não"));
    }

    #[test]
    fn a_ajuda_cabe_inteira_e_lista_toda_linha_da_tabela() {
        let b = tela_da_ajuda();
        cabe(&b, 100, 30);
        let m = painel(&b, "Teclas");
        assert_eq!(m.height as usize, ATALHOS.len(), "a moldura mostra todas");
        let cont = linhas_do(&b, m);
        for a in ATALHOS {
            assert!(
                cont.iter()
                    .any(|l| l.starts_with(a.rotulo) && l.ends_with(a.descricao)),
                "a linha {:?} → {:?} não está inteira:\n{}",
                a.rotulo,
                a.descricao,
                texto(&b)
            );
        }
    }

    #[test]
    fn com_a_maiuscula_o_titulo_diz_mais_arquivadas_e_a_arquivada_vem_com_o_tipo() {
        let b = tela_das_arquivadas();
        cabe(&b, 100, 30);
        assert!(linhas(&b)[0].contains("+arquivadas"), "{}", linhas(&b)[0]);
        let tasks = itens(&b, painel(&b, "Tasks"));
        assert_eq!(tasks.len(), 4, "{tasks:?}");
        let velha = tasks.last().unwrap();
        assert!(
            velha.find("[done]") < velha.find("velha-feita") && velha.contains("[done]"),
            "{velha:?}"
        );
    }

    #[test]
    fn com_s_maiusculo_a_adiada_mostra_a_data_de_volta_na_linha_dela() {
        let b = tela_das_escondidas();
        cabe(&b, 100, 30);
        assert!(
            linhas(&b)[0].contains("+algum-dia/adiadas"),
            "{}",
            linhas(&b)[0]
        );
        let tasks = itens(&b, painel(&b, "Tasks"));
        assert_eq!(tasks.len(), 5, "{tasks:?}");
        for t in &tasks {
            if t.contains(" adiada") {
                assert!(t.contains("volta 2026-12-01"), "{t:?}");
            } else {
                assert!(!t.contains("volta"), "{t:?}");
            }
        }
    }

    #[test]
    fn o_preview_da_adiada_diz_quando_ela_volta() {
        let mut t = teclas("lS");
        t.push(tecla('j'));
        // Em 160 colunas a linha do estado cabe numa linha só.
        let b = desenha(&estado_apos(&t), 160, 30).0;
        let d = destacadas(&b, painel(&b, "Tasks"));
        assert!(d[0].contains("adiada"), "{d:?}");
        let p = linhas_do(&b, painel(&b, "Preview")).join("\n");
        assert!(p.contains("volta em 2026-12-01"), "{p}");
    }

    #[test]
    fn a_prioridade_alta_leva_a_marca() {
        let b = desenha(&estado_apos(&teclas("j")), 100, 30).0;
        let tasks = itens(&b, painel(&b, "Tasks"));
        assert!(tasks[0].starts_with('!') && tasks[0].contains("beta-urgente"));
        assert!(!tasks[1].contains('!'));
    }

    /// O texto de uma linha da grade como se lê: a célula depois do `⛔`, que
    /// ocupa duas colunas, é a metade escondida dele e não entra.
    fn lida(b: &Buffer, y: u16, p: Rect) -> String {
        let mut out = String::new();
        let mut x = p.x;
        while x < p.right() {
            let sim = b[(x, y)].symbol();
            out.push_str(sim);
            x += if sim == "⛔" { 2 } else { 1 };
        }
        out.trim().to_string()
    }

    #[test]
    fn a_task_bloqueada_leva_a_marca_colada_antes_do_id_e_so_ela() {
        let b = tela_da_task_bloqueada();
        cabe(&b, 100, 30);
        let tasks = painel(&b, "Tasks");
        let linhas: Vec<String> = (tasks.y..tasks.bottom())
            .map(|y| lida(&b, y, tasks))
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(linhas.len(), 3, "{linhas:?}");
        assert!(
            linhas[0].contains("escrever-spec") && !linhas[0].contains('⛔'),
            "{linhas:?}"
        );
        assert!(
            linhas[1].contains("⛔ medir-tempo"),
            "a marca, um espaço e o id: {linhas:?}"
        );
        assert!(
            linhas[1].find(' ') < linhas[1].find('⛔'),
            "o ícone da fase vem antes da marca: {linhas:?}"
        );
        assert!(!linhas[2].contains('⛔'), "{linhas:?}");
        // É a linha em destaque (selecionada com `lj`).
        let d = destacadas(&b, tasks);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].contains("medir-tempo"), "{d:?}");
        // O preview e os projetos não ganham a marca.
        let fora = linhas_do(&b, painel(&b, "Preview")).join("\n")
            + &linhas_do(&b, painel(&b, "Projetos")).join("\n");
        assert!(!fora.contains('⛔'), "{fora}");
    }

    #[test]
    fn a_marca_cabe_no_painel_estreito_e_convive_com_a_prioridade() {
        let s = estado_com(dados_bloqueada(), &teclas("lj"));
        let b = desenha(&s, 60, 15).0;
        cabe(&b, 60, 15);
        let tasks = painel(&b, "Tasks");
        let linha = (tasks.y..tasks.bottom())
            .map(|y| lida(&b, y, tasks))
            .find(|l| l.contains("medir-tempo"))
            .unwrap();
        assert!(linha.contains("⛔ medir-tempo"), "{linha:?}");

        let b = desenha(&estado_com(dados_bloqueada(), &teclas("j")), 100, 30).0;
        let tasks = painel(&b, "Tasks");
        let linha = (tasks.y..tasks.bottom())
            .map(|y| lida(&b, y, tasks))
            .find(|l| l.contains("beta-urgente"))
            .unwrap();
        assert!(
            linha.starts_with('!') && linha.contains("⛔ beta-urgente"),
            "{linha:?}"
        );
    }

    #[test]
    fn o_prompt_do_b_tem_a_moldura_o_texto_preenchido_e_o_cursor_no_fim() {
        let (b, cursor) = tela_do_prompt_bloqueada_por();
        cabe(&b, 100, 30);
        let m = painel(
            &b,
            " Bloqueada por (ids separados por vírgula; vazio limpa) ",
        );
        assert_eq!(linhas_do(&b, m)[0], "escrever-spec");
        assert!(texto(&b).contains("Enter confirma · Esc cancela"));
        assert_eq!(
            cursor,
            (m.x + "escrever-spec".chars().count() as u16, m.y),
            "o cursor fica no fim do texto"
        );
        // Em 60×15 a moldura e o texto continuam inteiros.
        let (p, _) = desenha(&estado_com(dados_bloqueada(), &teclas("ljb")), 60, 15);
        cabe(&p, 60, 15);
        let m = painel(&p, "Bloqueada por (ids separados por vírgula; vazio limpa)");
        assert_eq!(linhas_do(&p, m)[0], "escrever-spec");
    }

    #[test]
    fn o_vault_vazio_mostra_os_paineis_sem_task_e_o_preview_vazio() {
        let b = tela_do_vault_vazio();
        cabe(&b, 100, 30);
        assert!(itens(&b, painel(&b, "Projetos")).is_empty());
        assert!(itens(&b, painel(&b, "Tasks: —")).is_empty());
        let p = linhas_do(&b, painel(&b, "Preview")).join("\n");
        assert!(p.contains("sem task selecionada"), "{p}");
        assert_eq!(barra(&b), DICA);
    }

    #[test]
    fn a_barra_mostra_a_mensagem_em_verde_ou_vermelho() {
        let mut s = State::novo(dados());
        s.info("ok");
        let b = desenha(&s, 100, 30).0;
        assert_eq!((barra(&b).as_str(), cor_da_barra(&b)), ("ok", Color::Green));
        s.erro("deu ruim");
        let b = desenha(&s, 100, 30).0;
        assert_eq!(
            (barra(&b).as_str(), cor_da_barra(&b)),
            ("deu ruim", Color::Red)
        );
    }

    #[test]
    fn o_terminal_estreito_mantem_os_paineis_a_selecao_e_o_comeco_da_dica() {
        let b = tela_estreita();
        cabe(&b, 60, 15);
        let l = linhas(&b);
        assert!(
            l[0].find("Projetos") < l[0].find("Tasks") && l[0].find("Tasks") < l[0].find("Preview"),
            "{}",
            texto(&b)
        );
        let d = destacadas(&b, painel(&b, "Tasks"));
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("fechar-ciclo"), "{d:?}");
        assert!(barra(&b).starts_with("? ajuda"), "{}", barra(&b));
    }

    #[test]
    fn em_terminal_estreito_os_modais_ficam_dentro_da_tela_e_o_cursor_tambem() {
        for modo in ["ln", "lt", "lr", "lb", "N", "ls", "la", "ld", "?"] {
            for (w, h) in [(20, 5), (24, 7), (40, 10), (60, 15)] {
                let s = estado_apos(&teclas(modo));
                let (b, (cx, cy)) = desenha(&s, w, h);
                cabe(&b, w, h);
                assert!(cx < w && cy < h, "{modo} {w}x{h}: cursor em ({cx}, {cy})");
            }
        }
    }

    #[test]
    fn a_barra_do_filtro_poe_o_cursor_depois_do_texto() {
        let s = estado_apos(&teclas("/ab"));
        let (_, cursor) = desenha(&s, 100, 30);
        assert_eq!(cursor, (3, 29));
    }

    #[test]
    fn a_nao_ser_pelo_estilo_a_tela_nao_muda_com_o_foco() {
        // Só a moldura e o título do painel com foco mudam de cor: o texto é o
        // mesmo.
        let a = desenha(&estado_apos(&[]), 100, 30).0;
        let b = desenha(&estado_apos(&teclas("l")), 100, 30).0;
        assert_eq!(texto(&a), texto(&b));
        assert_ne!(a, b);
    }
}
