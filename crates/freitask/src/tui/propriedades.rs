//! Propriedades do `update` e do `view` (camada 2 da §3.5): sequências
//! aleatórias de teclas, redimensionamentos e resultados de comando sobre
//! dados sintéticos. Sem pânico em nenhum tamanho de 20×5 a 200×60; os índices
//! ficam dentro da lista; o foco está num painel que existe; o `Esc` volta ao
//! modo normal em no máximo dois passos.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::linhas::Resumo;
use crate::tui::dados::{Dados, Projeto};
use crate::tui::estado::{Depois, Evento, Modo, Painel, State, update};
use crate::tui::tela::view;

/// As teclas de comando (as da ajuda, e `y` das confirmações).
const COMANDOS: [char; 24] = [
    'j', 'k', 'h', 'l', 'g', 'G', 'n', 's', 't', 'i', 'p', 'r', 'a', 'A', 'S', 'd', 'N', 'c', '/',
    '?', 'e', 'R', 'y', 'q',
];

/// Letras para os campos de texto (com acento, espaço e hífen).
const TEXTO: [char; 9] = ['a', 'b', 'x', ' ', 'Á', 'ç', '-', '1', 'Z'];

const ESPECIAIS: [KeyCode; 12] = [
    KeyCode::Esc,
    KeyCode::Enter,
    KeyCode::Tab,
    KeyCode::BackTab,
    KeyCode::Backspace,
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::Delete,
];

fn caractere(c: char) -> KeyEvent {
    let m = if c.is_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(c), m)
}

fn tecla() -> impl Strategy<Value = KeyEvent> {
    prop_oneof![
        55 => select(COMANDOS.to_vec()).prop_map(caractere),
        20 => select(TEXTO.to_vec()).prop_map(caractere),
        20 => select(ESPECIAIS.to_vec()).prop_map(|k| KeyEvent::new(k, KeyModifiers::NONE)),
        5 => select(vec!['d', 'u', 'c', 'x']).prop_map(|c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)),
    ]
}

/// O prazo de uma task sintética: o texto escrito e os dias que o motor
/// calcularia (`None` no prazo inválido).
type Prazo = Option<(String, Option<i64>)>;

fn prazo() -> impl Strategy<Value = Prazo> {
    prop_oneof![
        3 => Just(None),
        2 => (1u32..=28, -30i64..30).prop_map(|(d, n)| Some((format!("2026-11-{d:02}"), Some(n)))),
        1 => Just(Some(("amanhã".to_string(), None))),
    ]
}

/// Uma task sintética:
/// `(projeto, id, arquivada, escondida, adiada, alta, fase, bloqueada)`.
type Semente = (usize, String, bool, bool, bool, bool, u32, bool);

/// Os projetos, as tasks e o prazo de cada uma (por posição).
type Entrada = (usize, Vec<Semente>, Vec<Prazo>);

fn dados() -> impl Strategy<Value = Dados> {
    (
        0usize..4,
        vec(
            (
                0usize..4,
                "[a-d]{1,2}",
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
                1u32..10,
                any::<bool>(),
            ),
            0..12,
        ),
        vec(prazo(), 12),
    )
        .prop_map(|(n_projetos, sementes, prazos): Entrada| {
            let mut tasks: Vec<Resumo> = sementes
                .into_iter()
                .map(
                    |(p, id, arquivada, hidden, adiada, alta, fase, bloqueada)| Resumo {
                        project: format!("p{p}"),
                        path: format!("/v/p{p}/{id}.md"),
                        id,
                        status: "fase".into(),
                        status_num: fase,
                        callout: "todo".into(),
                        title: "Título".into(),
                        desc: String::new(),
                        archived: arquivada.then(|| "done".to_string()),
                        suggested_archive: "dropped",
                        hidden: hidden && !arquivada,
                        prioridade: (alta && !arquivada).then_some("alta"),
                        volta_em: (adiada && !arquivada).then(|| "2026-12-01".to_string()),
                        bloqueada_por: if bloqueada { vec!["x".into()] } else { vec![] },
                        bloqueada,
                        bloqueia: vec![],
                        prazo: None,
                        dias_para_o_prazo: None,
                    },
                )
                .collect();
            // O prazo vem à parte, para o literal acima não mudar de forma.
            for (t, prazo) in tasks.iter_mut().zip(prazos) {
                if let Some((texto, dias)) = prazo {
                    t.prazo = Some(texto);
                    t.dias_para_o_prazo = dias;
                }
            }
            let projetos = (0..n_projetos)
                .map(|p| Projeto {
                    nome: format!("p{p}"),
                    ativas: tasks
                        .iter()
                        .filter(|t| t.project == format!("p{p}") && t.archived.is_none())
                        .count(),
                })
                .collect();
            Dados {
                projetos,
                tasks,
                ..Dados::default()
            }
        })
}

fn depois() -> impl Strategy<Value = Depois> {
    prop_oneof![
        Just(Depois::Manter),
        Just(Depois::ProjetoDaSaida),
        (0usize..5, "[a-d]{1,2}").prop_map(|(p, id)| Depois::Task {
            projeto: format!("p{p}"),
            id
        }),
    ]
}

fn evento() -> impl Strategy<Value = Evento> {
    prop_oneof![
        85 => tecla().prop_map(Evento::Tecla),
        5 => (20u16..=200, 5u16..=60).prop_map(|(w, h)| Evento::Resize(w, h)),
        3 => "[a-z ]{0,20}".prop_map(Evento::Falhou),
        3 => "[a-d-]{0,6}".prop_map(Evento::IdSugerido),
        4 => (dados(), depois(), "p[0-4]").prop_map(|(dados, depois, saida)| Evento::Dados { dados, depois, saida }),
    ]
}

fn desenha(estado: &State, largura: u16, altura: u16) {
    let mut terminal = Terminal::new(TestBackend::new(largura, altura)).unwrap();
    terminal.draw(|f| view(estado, f)).unwrap();
}

/// Os invariantes que valem depois de qualquer evento.
fn confere(s: &State) {
    assert!(s.project_idx < s.dados.projetos.len().max(1), "project_idx");
    assert!(s.task_idx < s.tasks_visiveis().len().max(1), "task_idx");
    assert!(matches!(s.painel, Painel::Projetos | Painel::Tasks));
    match &s.modo {
        Modo::NovaTask(nt) => assert!(nt.campo < 3, "campo do modal de task nova"),
        Modo::Pick { opcoes, idx, .. } => assert!(*idx < opcoes.len(), "idx do pick"),
        _ => {}
    }
    // `Esc` leva ao modo normal em no máximo dois passos.
    let mut t = s.clone();
    for _ in 0..2 {
        t = update(
            t,
            Evento::Tecla(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        )
        .0;
    }
    assert_eq!(t.modo, Modo::Normal, "dois Esc deviam voltar ao normal");
}

proptest! {
    #[test]
    fn sequencias_de_eventos_nao_quebram_os_invariantes_nem_o_desenho(
        inicial in dados(),
        eventos in vec(evento(), 0..60),
    ) {
        let mut s = State::novo(inicial);
        let mut tamanho = (100u16, 30u16);
        confere(&s);
        for e in eventos {
            if let Evento::Resize(w, h) = &e {
                tamanho = (*w, *h);
            }
            s = update(s, e).0;
            confere(&s);
            desenha(&s, tamanho.0, tamanho.1);
        }
    }

    #[test]
    fn o_desenho_cabe_em_qualquer_tamanho_em_qualquer_modo(
        inicial in dados(),
        teclas in vec(tecla(), 0..12),
        (largura, altura) in (20u16..=200, 5u16..=60),
    ) {
        let mut s = State::novo(inicial);
        for t in teclas {
            s = update(s, Evento::Tecla(t)).0;
        }
        desenha(&s, largura, altura);
    }
}
