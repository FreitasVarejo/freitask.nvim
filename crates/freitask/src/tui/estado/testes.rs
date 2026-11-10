//! Testes do `update`: as teclas e os eventos sobre dados sintéticos, afirmando
//! o `State` e os `Comando`s (camada 1 da §3.5). Os 21 testes do `app.rs` da
//! TUI de hoje, portados, mais os do que a fatia 7 acrescenta (o `Comando`
//! como fronteira, o preview, os resultados que voltam como evento).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use freitask_core::operacao::PedidoSet;
use freitask_core::status::Vocab;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::tui::dados::{Dados, Projeto};

pub(crate) fn tarefa(
    projeto: &str,
    id: &str,
    callout: &str,
    num: u32,
    arquivada: Option<&str>,
) -> Resumo {
    let path = match arquivada {
        Some(a) => format!("/v/projects/{projeto}/tasks/archived/{a}/{id}.md"),
        None => format!("/v/projects/{projeto}/tasks/{id}.md"),
    };
    Resumo {
        project: projeto.into(),
        id: id.into(),
        status: String::new(),
        status_num: num,
        callout: callout.into(),
        title: format!("Título {id}"),
        desc: String::new(),
        archived: arquivada.map(Into::into),
        suggested_archive: if callout == "check" {
            "done"
        } else {
            "dropped"
        },
        hidden: false,
        prioridade: None,
        volta_em: None,
        prazo: None,
        dias_para_o_prazo: None,
        path,
    }
}

/// Um vocabulário curto: `todo`, `example`, `check` e `done`.
fn vocab() -> Vocab {
    Vocab::from_json(
        r#"{
          "1": {"title": "todo", "callout": "todo", "icon": "*", "hl_group": ""},
          "3": {"title": "example", "callout": "example", "icon": "*", "hl_group": ""},
          "6": {"title": "check", "callout": "check", "icon": "*", "hl_group": ""},
          "7": {"title": "done", "callout": "done", "icon": "*", "hl_group": ""}
        }"#,
    )
    .unwrap()
}

/// Dois projetos: `alfa` (a1, a2 e a0 arquivada) e `beta` (b1).
pub(crate) fn dados_sinteticos() -> Dados {
    Dados {
        vocab: vocab(),
        projetos: vec![
            Projeto {
                nome: "alfa".into(),
                ativas: 2,
            },
            Projeto {
                nome: "beta".into(),
                ativas: 1,
            },
        ],
        tasks: vec![
            tarefa("alfa", "a1", "todo", 1, None),
            tarefa("alfa", "a2", "check", 6, None),
            tarefa("alfa", "a0", "done", 7, Some("done")),
            tarefa("beta", "b1", "example", 3, None),
        ],
    }
}

fn estado() -> State {
    State::novo(dados_sinteticos())
}

fn tecla(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn codigo(k: KeyCode) -> KeyEvent {
    KeyEvent::new(k, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// Aplica o evento e devolve os comandos, sem o `LerPreview` (que o `update`
/// pede sozinho a cada seleção nova e tem teste próprio).
fn aplica(s: &mut State, evento: Evento) -> Vec<Comando> {
    let (novo, comandos) = update(std::mem::take(s), evento);
    *s = novo;
    comandos
        .into_iter()
        .filter(|c| !matches!(c, Comando::LerPreview(_)))
        .collect()
}

fn aperta(s: &mut State, k: KeyEvent) -> Vec<Comando> {
    aplica(s, Evento::Tecla(k))
}

fn digita(s: &mut State, texto: &str) {
    for c in texto.chars() {
        aperta(s, tecla(c));
    }
}

fn executar(comandos: Vec<Comando>) -> (Escrita, Depois, String) {
    match <[Comando; 1]>::try_from(comandos) {
        Ok(
            [
                Comando::Executar {
                    escrita,
                    depois,
                    ok,
                },
            ],
        ) => (escrita, depois, ok),
        outro => panic!("esperava um Executar, veio {outro:?}"),
    }
}

fn ids(s: &State) -> Vec<String> {
    s.tasks_visiveis().iter().map(|t| t.id.clone()).collect()
}

fn set_de(referencia: &str, pedido: PedidoSet) -> Escrita {
    Escrita::Set {
        referencia: referencia.into(),
        pedido,
    }
}

// --- os 21 testes do `app.rs`, portados ---------------------------------------

#[test]
fn arquivadas_ficam_escondidas_ate_o_toggle() {
    let mut s = estado();
    assert_eq!(s.tasks_visiveis().len(), 2);
    aperta(&mut s, tecla('A'));
    assert_eq!(s.tasks_visiveis().len(), 3);
}

#[test]
fn algum_dia_fica_escondido_ate_o_toggle() {
    let mut d = dados_sinteticos();
    d.tasks.push(Resumo {
        hidden: true,
        ..tarefa("alfa", "a9", "quote", 8, None)
    });
    let mut s = State::novo(d);
    assert_eq!(ids(&s), ["a1", "a2"]);
    aperta(&mut s, tecla('S'));
    assert_eq!(ids(&s), ["a1", "a2", "a9"]);
    aperta(&mut s, tecla('S'));
    assert_eq!(ids(&s), ["a1", "a2"]);
}

#[test]
fn task_adiada_fica_escondida_ate_o_toggle_qualquer_que_seja_a_fase() {
    // A TUI obedece ao `hidden` do motor: a adiada está em `example`, uma fase
    // que o painel mostra, e some do mesmo jeito que o Algum dia.
    let mut d = dados_sinteticos();
    d.tasks.push(Resumo {
        hidden: true,
        volta_em: Some("2026-12-01".into()),
        ..tarefa("alfa", "a8", "example", 3, None)
    });
    let mut s = State::novo(d);
    assert_eq!(ids(&s), ["a1", "a2"]);
    aperta(&mut s, tecla('S'));
    assert_eq!(ids(&s), ["a1", "a2", "a8"]);
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('G'));
    assert_eq!(
        s.task_atual().unwrap().volta_em.as_deref(),
        Some("2026-12-01"),
        "a data de volta vem com a task, para a tela mostrar"
    );
}

#[test]
fn o_toggle_de_algum_dia_nao_mexe_nas_arquivadas() {
    let mut d = dados_sinteticos();
    d.tasks.push(Resumo {
        hidden: true,
        ..tarefa("alfa", "a9", "quote", 8, None)
    });
    let mut s = State::novo(d);
    aperta(&mut s, tecla('A'));
    assert_eq!(
        s.tasks_visiveis().len(),
        3,
        "arquivadas à mostra, Algum dia ainda não"
    );
    aperta(&mut s, tecla('S'));
    assert_eq!(s.tasks_visiveis().len(), 4);
}

#[test]
fn toggle_recoloca_a_selecao_dentro_da_lista() {
    let mut d = dados_sinteticos();
    d.tasks.push(Resumo {
        hidden: true,
        ..tarefa("alfa", "a9", "quote", 8, None)
    });
    let mut s = State::novo(d);
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('S'));
    aperta(&mut s, tecla('G'));
    assert_eq!(s.task_atual().unwrap().id, "a9");
    aperta(&mut s, tecla('S'));
    assert_eq!(s.task_atual().unwrap().id, "a2");
}

#[test]
fn trocar_de_projeto_volta_a_task_para_o_topo() {
    let mut s = estado();
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('j'));
    assert_eq!(s.task_atual().unwrap().id, "a2");
    aperta(&mut s, tecla('h'));
    aperta(&mut s, tecla('j'));
    assert_eq!(s.projeto_atual().unwrap().nome, "beta");
    assert_eq!(s.task_atual().unwrap().id, "b1");
}

#[test]
fn filtro_casa_id_e_titulo_e_esc_limpa() {
    let mut s = estado();
    aperta(&mut s, tecla('/'));
    digita(&mut s, "a2");
    assert_eq!(s.tasks_visiveis().len(), 1);
    aperta(&mut s, codigo(KeyCode::Enter));
    assert_eq!(s.modo, Modo::Normal);
    assert_eq!(s.filtro, "a2");
    aperta(&mut s, codigo(KeyCode::Esc));
    assert_eq!(s.tasks_visiveis().len(), 2);
}

#[test]
fn nova_task_pede_o_id_ao_motor_e_monta_o_new() {
    let mut s = estado();
    aperta(&mut s, tecla('n'));
    digita(&mut s, "Ávida nova");
    assert_eq!(
        aperta(&mut s, codigo(KeyCode::Tab)),
        [Comando::SugerirId("Ávida nova".into())]
    );
    aplica(&mut s, Evento::IdSugerido("avida-nova".into()));
    aperta(&mut s, codigo(KeyCode::Tab));
    digita(&mut s, "estado");
    let comandos = aperta(&mut s, codigo(KeyCode::Enter));
    assert_eq!(
        comandos,
        [Comando::Executar {
            escrita: Escrita::NovaTask {
                projeto: "alfa".into(),
                id: "avida-nova".into(),
                titulo: "Ávida nova".into(),
                desc: "estado".into(),
            },
            depois: Depois::Task {
                projeto: "alfa".into(),
                id: "avida-nova".into()
            },
            ok: "criada: avida-nova".into(),
        }]
    );
}

#[test]
fn id_digitado_a_mao_nao_e_sobrescrito_pela_sugestao() {
    let mut s = estado();
    aperta(&mut s, tecla('n'));
    // Título vazio: nada a sugerir.
    assert_eq!(aperta(&mut s, codigo(KeyCode::Tab)), []);
    digita(&mut s, "meu-id");
    aplica(&mut s, Evento::IdSugerido("outro".into()));
    let Modo::NovaTask(nt) = &s.modo else {
        panic!("o modal fechou")
    };
    assert_eq!(nt.id, "meu-id");
}

#[test]
fn recusa_do_motor_devolve_o_modal_preenchido() {
    let mut s = estado();
    aperta(&mut s, tecla('n'));
    digita(&mut s, "X");
    aperta(&mut s, codigo(KeyCode::Tab));
    digita(&mut s, "Id Ruim");
    aperta(&mut s, codigo(KeyCode::Tab));
    executar(aperta(&mut s, codigo(KeyCode::Enter)));
    aplica(&mut s, Evento::Falhou("id precisa ser kebab-case".into()));
    let Modo::NovaTask(nt) = &s.modo else {
        panic!("modal não voltou")
    };
    assert_eq!(nt.id, "Id Ruim");
    assert!(s.mensagem.as_ref().unwrap().erro);
}

#[test]
fn fase_oferece_so_o_que_o_set_aceita() {
    let mut s = estado();
    aperta(&mut s, tecla('s'));
    let Modo::Pick { opcoes, idx, .. } = &s.modo else {
        panic!("sem o pick")
    };
    let callouts: Vec<&str> = opcoes.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(callouts, ["todo", "example", "check"]);
    assert_eq!(*idx, 0); // a1 está em todo
    aperta(&mut s, tecla('j'));
    let (escrita, depois, ok) = executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert_eq!(
        escrita,
        set_de(
            "/v/projects/alfa/tasks/a1.md",
            PedidoSet {
                status: Some("example".into()),
                ..PedidoSet::default()
            }
        )
    );
    assert_eq!((depois, ok.as_str()), (Depois::Manter, "fase: * example"));
}

#[test]
fn arquivar_parte_da_sugestao_do_motor() {
    let mut s = estado();
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('j')); // a2, em check → sugere done
    aperta(&mut s, tecla('a'));
    let Modo::Pick { idx, .. } = &s.modo else {
        panic!("sem o pick")
    };
    assert_eq!(*idx, 0);
    let (escrita, _, ok) = executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert_eq!(
        escrita,
        Escrita::Deslocar {
            referencia: "/v/projects/alfa/tasks/a2.md".into(),
            pedido: Pedido::Archive(Some("done".into())),
        }
    );
    assert_eq!(ok, "arquivada como done: a2");
}

#[test]
fn a_numa_arquivada_confirma_e_desarquiva() {
    let mut s = estado();
    aperta(&mut s, tecla('A'));
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('G'));
    assert_eq!(s.task_atual().unwrap().id, "a0");
    aperta(&mut s, tecla('a'));
    assert_eq!(s.modo, Modo::Confirmar(Confirmacao::Desarquivar));
    let (escrita, depois, ok) = executar(aperta(&mut s, tecla('y')));
    assert_eq!(
        escrita,
        Escrita::Deslocar {
            referencia: "/v/projects/alfa/tasks/archived/done/a0.md".into(),
            pedido: Pedido::Unarchive,
        }
    );
    assert_eq!(
        depois,
        Depois::Task {
            projeto: "alfa".into(),
            id: "a0".into()
        }
    );
    assert_eq!(ok, "desarquivada: a0");
}

#[test]
fn largar_so_com_y_e_arquiva_como_dropped() {
    let mut s = estado();
    aperta(&mut s, tecla('d'));
    assert_eq!(aperta(&mut s, tecla('n')), []);
    assert_eq!(s.modo, Modo::Normal);
    aperta(&mut s, tecla('d'));
    let (escrita, _, ok) = executar(aperta(&mut s, tecla('y')));
    assert_eq!(
        escrita,
        Escrita::Deslocar {
            referencia: "/v/projects/alfa/tasks/a1.md".into(),
            pedido: Pedido::Archive(Some("dropped".into())),
        }
    );
    assert_eq!(ok, "largada: a1");
}

#[test]
fn d_numa_arquivada_nao_reclassifica() {
    let mut s = estado();
    aperta(&mut s, tecla('A'));
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('G'));
    assert_eq!(s.task_atual().unwrap().id, "a0");
    assert_eq!(aperta(&mut s, tecla('d')), []);
    assert_eq!(s.modo, Modo::Normal);
    assert!(s.mensagem.as_ref().is_some_and(|m| m.erro));
}

#[test]
fn prompt_sem_mudanca_nao_chama_o_motor() {
    let mut s = estado();
    aperta(&mut s, tecla('t'));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    aperta(&mut s, tecla('r'));
    aperta(&mut s, codigo(KeyCode::Backspace));
    digita(&mut s, "9");
    let (escrita, depois, ok) = executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert_eq!(
        escrita,
        Escrita::Deslocar {
            referencia: "/v/projects/alfa/tasks/a1.md".into(),
            pedido: Pedido::Rename("a9".into()),
        }
    );
    assert_eq!(
        depois,
        Depois::Task {
            projeto: "alfa".into(),
            id: "a9".into()
        }
    );
    assert_eq!(ok, "renomeada: a1 → a9");
}

#[test]
fn descricao_vazia_e_permitida_titulo_vazio_nao() {
    let mut s = estado();
    aperta(&mut s, tecla('i'));
    // A descrição já é vazia: nada muda.
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    aperta(&mut s, tecla('t'));
    aperta(&mut s, ctrl('u'));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert!(matches!(
        s.modo,
        Modo::Prompt {
            campo: Campo::Titulo,
            ..
        }
    ));
    assert_eq!(s.mensagem.as_ref().unwrap().texto, "não pode ficar vazio");
}

#[test]
fn releitura_mantem_a_task_pelo_id() {
    let mut s = estado();
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('j'));
    let mut d = dados_sinteticos();
    d.tasks.retain(|t| t.id != "a1"); // a task de cima sumiu
    aplica(
        &mut s,
        Evento::Dados {
            dados: d,
            depois: Depois::Manter,
            saida: String::new(),
        },
    );
    assert_eq!(s.task_atual().unwrap().id, "a2");
}

#[test]
fn releitura_seleciona_a_task_nova() {
    let mut s = estado();
    let mut d = dados_sinteticos();
    d.tasks.push(tarefa("beta", "b2", "todo", 1, None));
    aplica(
        &mut s,
        Evento::Dados {
            dados: d,
            depois: Depois::Task {
                projeto: "beta".into(),
                id: "b2".into(),
            },
            saida: String::new(),
        },
    );
    assert_eq!(s.projeto_atual().unwrap().nome, "beta");
    assert_eq!(s.task_atual().unwrap().id, "b2");
}

#[test]
fn novo_projeto_e_selecionado_pelo_stdout() {
    let mut s = estado();
    let mut d = dados_sinteticos();
    d.projetos.push(Projeto {
        nome: "gama".into(),
        ativas: 0,
    });
    aplica(
        &mut s,
        Evento::Dados {
            dados: d,
            depois: Depois::ProjetoDaSaida,
            saida: "gama\n".into(),
        },
    );
    assert_eq!(s.projeto_atual().unwrap().nome, "gama");
    assert!(s.task_atual().is_none());
    assert_eq!(aperta(&mut s, tecla('d')), []);
    assert!(s.mensagem.as_ref().unwrap().erro);
}

#[test]
fn ctrl_c_sai_de_qualquer_modo() {
    let mut s = estado();
    aperta(&mut s, tecla('n'));
    assert_eq!(aperta(&mut s, ctrl('c')), [Comando::Sair]);
}

// --- o que a fatia 7 acrescenta -------------------------------------------------

#[test]
fn q_e_ctrl_c_pedem_para_sair_e_na_ajuda_q_so_fecha() {
    let mut s = estado();
    assert_eq!(aperta(&mut s, tecla('q')), [Comando::Sair]);
    aperta(&mut s, tecla('?'));
    assert_eq!(s.modo, Modo::Ajuda);
    assert_eq!(aperta(&mut s, tecla('q')), []);
    assert_eq!(s.modo, Modo::Normal);
}

#[test]
fn enter_nos_projetos_so_passa_o_foco_e_e_abre_a_task_de_qualquer_painel() {
    let mut s = estado();
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.painel, Painel::Tasks);
    assert_eq!(
        aperta(&mut s, codigo(KeyCode::Enter)),
        [Comando::Abrir("/v/projects/alfa/tasks/a1.md".into())]
    );
    s.painel = Painel::Projetos;
    assert_eq!(
        aperta(&mut s, tecla('e')),
        [Comando::Abrir("/v/projects/alfa/tasks/a1.md".into())]
    );
    // Sem task, nada a abrir.
    let mut vazio = State::novo(Dados::default());
    assert_eq!(aperta(&mut vazio, tecla('e')), []);
    assert_eq!(aperta(&mut vazio, codigo(KeyCode::Enter)), []);
    assert_eq!(aperta(&mut vazio, codigo(KeyCode::Enter)), []);
}

#[test]
fn c_pede_para_abrir_o_current_e_r_maiusculo_rele_sem_gravar() {
    let mut s = estado();
    assert_eq!(aperta(&mut s, tecla('c')), [Comando::AbrirCurrent]);
    assert_eq!(
        aperta(&mut s, tecla('R')),
        [Comando::Reler {
            depois: Depois::Manter,
            saida: String::new()
        }]
    );
    assert_eq!(s.mensagem.as_ref().unwrap().texto, "relido");
    assert!(!s.mensagem.as_ref().unwrap().erro);
}

#[test]
fn n_maiusculo_cria_o_projeto_pelo_nome_aparado() {
    let mut s = estado();
    aperta(&mut s, tecla('N'));
    digita(&mut s, "  Gama ");
    let (escrita, depois, ok) = executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert_eq!(escrita, Escrita::NovoProjeto("Gama".into()));
    assert_eq!(depois, Depois::ProjetoDaSaida);
    assert_eq!(ok, "projeto criado");
}

#[test]
fn n_sem_projeto_pede_para_criar_um_antes() {
    let mut s = State::novo(Dados::default());
    assert_eq!(aperta(&mut s, tecla('n')), []);
    assert_eq!(
        s.mensagem.as_ref().unwrap().texto,
        "crie um projeto antes (N)"
    );
    assert_eq!(s.modo, Modo::Normal);
}

#[test]
fn teclas_de_task_sem_task_avisam_e_nao_abrem_modal() {
    let mut s = State::novo(Dados::default());
    for k in ['s', 't', 'i', 'r', 'a', 'd'] {
        s.mensagem = None;
        assert_eq!(aperta(&mut s, tecla(k)), []);
        assert_eq!(s.modo, Modo::Normal, "{k}");
        assert_eq!(
            s.mensagem.as_ref().unwrap().texto,
            "nenhuma task selecionada"
        );
    }
}

#[test]
fn nova_task_sem_titulo_ou_sem_id_recusa_na_barra_e_o_modal_fica() {
    let mut s = estado();
    aperta(&mut s, tecla('n'));
    aperta(&mut s, codigo(KeyCode::Tab));
    digita(&mut s, "x");
    aperta(&mut s, codigo(KeyCode::Tab));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.mensagem.as_ref().unwrap().texto, "falta o título");
    let Modo::NovaTask(nt) = &s.modo else {
        panic!("o modal fechou")
    };
    assert_eq!(nt.campo, 0);
    // Com título e sem id.
    digita(&mut s, "T");
    aperta(&mut s, codigo(KeyCode::Down));
    aperta(&mut s, ctrl('u'));
    aperta(&mut s, codigo(KeyCode::Down));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.mensagem.as_ref().unwrap().texto, "falta o id");
    let Modo::NovaTask(nt) = &s.modo else {
        panic!("o modal fechou")
    };
    assert_eq!(nt.campo, 1);
}

#[test]
fn feito_mostra_o_ok_esquece_o_modal_e_pede_a_releitura() {
    let mut s = estado();
    aperta(&mut s, tecla('t'));
    digita(&mut s, "x");
    executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert!(s.refazer.is_some());
    let comandos = aplica(
        &mut s,
        Evento::Feito {
            ok: "título atualizado".into(),
            depois: Depois::Manter,
            saida: "/v/x".into(),
        },
    );
    assert_eq!(
        comandos,
        [Comando::Reler {
            depois: Depois::Manter,
            saida: "/v/x".into()
        }]
    );
    assert_eq!(s.refazer, None);
    assert_eq!(s.modo, Modo::Normal);
    let m = s.mensagem.as_ref().unwrap();
    assert_eq!((m.texto.as_str(), m.erro), ("título atualizado", false));
}

#[test]
fn falha_sem_modal_em_voo_so_mostra_o_erro() {
    let mut s = estado();
    aplica(&mut s, Evento::Falhou("rebuild falhou: x".into()));
    assert_eq!(s.modo, Modo::Normal);
    let m = s.mensagem.as_ref().unwrap();
    assert_eq!((m.texto.as_str(), m.erro), ("rebuild falhou: x", true));
}

#[test]
fn o_preview_e_pedido_uma_vez_por_selecao_e_de_novo_depois_de_reler() {
    let mut s = estado();
    let pede = |s: &mut State, e: Evento| -> Vec<Comando> {
        let (novo, c) = update(std::mem::take(s), e);
        *s = novo;
        c
    };
    // Ao abrir (primeiro evento com dados), pede o da task selecionada.
    let c = pede(&mut s, Evento::Resize(100, 30));
    assert_eq!(
        c,
        [Comando::LerPreview("/v/projects/alfa/tasks/a1.md".into())]
    );
    // Sem mudar a seleção, não pede de novo.
    assert_eq!(pede(&mut s, Evento::Resize(80, 24)), []);
    // Chegou: a seleção que muda pede o seguinte.
    pede(
        &mut s,
        Evento::PreviewLido {
            caminho: "/v/projects/alfa/tasks/a1.md".into(),
            conteudo: "x".into(),
        },
    );
    s.painel = Painel::Tasks;
    let c = pede(&mut s, Evento::Tecla(tecla('j')));
    assert_eq!(
        c,
        [Comando::LerPreview("/v/projects/alfa/tasks/a2.md".into())]
    );
    // Reler esquece o preview: a mesma task é lida outra vez.
    let c = pede(
        &mut s,
        Evento::Dados {
            dados: dados_sinteticos(),
            depois: Depois::Manter,
            saida: String::new(),
        },
    );
    assert_eq!(
        c,
        [Comando::LerPreview("/v/projects/alfa/tasks/a2.md".into())]
    );
    // Sem task, nada a ler.
    let mut vazio = State::novo(Dados::default());
    assert_eq!(pede(&mut vazio, Evento::Resize(1, 1)), []);
}

#[test]
fn rolar_o_preview_para_ao_mudar_a_selecao_e_nao_passa_de_zero() {
    let mut s = estado();
    aperta(&mut s, ctrl('d'));
    assert_eq!(s.preview_scroll, 10);
    aperta(&mut s, ctrl('u'));
    aperta(&mut s, ctrl('u'));
    assert_eq!(s.preview_scroll, 0);
    aperta(&mut s, ctrl('d'));
    aperta(&mut s, tecla('j'));
    assert_eq!(s.preview_scroll, 0, "outra seleção volta ao topo");
}

#[test]
fn no_filtro_as_letras_sao_texto_e_as_setas_movem() {
    let mut s = estado();
    aperta(&mut s, tecla('/'));
    assert_eq!(s.painel, Painel::Tasks);
    assert_eq!(aperta(&mut s, tecla('q')), []);
    aperta(&mut s, codigo(KeyCode::Backspace));
    assert_eq!(s.filtro, "");
    aperta(&mut s, codigo(KeyCode::Down));
    assert_eq!(s.modo, Modo::Filtro);
    assert_eq!(s.task_idx, 1);
    aperta(&mut s, codigo(KeyCode::Up));
    assert_eq!(s.task_idx, 0);
    aperta(&mut s, codigo(KeyCode::F(1)));
    assert_eq!(s.modo, Modo::Filtro);
    aperta(&mut s, codigo(KeyCode::Esc));
    assert_eq!(s.modo, Modo::Normal);
}

#[test]
fn esc_fecha_todo_modal_e_a_ajuda_fecha_com_qualquer_tecla() {
    for abre in ["n", "t", "i", "r", "N", "s", "a", "d", "?", "/"] {
        let mut s = estado();
        s.painel = Painel::Tasks;
        for c in abre.chars() {
            aperta(&mut s, tecla(c));
        }
        assert_ne!(s.modo, Modo::Normal, "{abre} devia abrir um modo");
        assert_eq!(aperta(&mut s, codigo(KeyCode::Esc)), [], "{abre}");
        assert_eq!(s.modo, Modo::Normal, "{abre}");
    }
    let mut s = estado();
    aperta(&mut s, tecla('?'));
    aperta(&mut s, tecla('x'));
    assert_eq!(s.modo, Modo::Normal);
}

#[test]
fn id_sugerido_fora_do_modal_nao_faz_nada() {
    let mut s = estado();
    // Assenta o pedido do preview, que o primeiro evento faz.
    aplica(&mut s, Evento::Resize(100, 30));
    let antes = s.clone();
    assert_eq!(aplica(&mut s, Evento::IdSugerido("x".into())), []);
    assert_eq!(s, antes);
}

#[test]
fn pick_de_fase_na_mesma_fase_nao_chama_o_motor() {
    let mut s = estado();
    aperta(&mut s, tecla('s'));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.modo, Modo::Normal);
}

#[test]
fn pick_nao_sai_dos_limites_da_lista() {
    let mut s = estado();
    aperta(&mut s, tecla('s'));
    for _ in 0..10 {
        aperta(&mut s, tecla('j'));
    }
    let Modo::Pick { idx, opcoes, .. } = &s.modo else {
        panic!("sem o pick")
    };
    assert_eq!(*idx, opcoes.len() - 1);
    for _ in 0..10 {
        aperta(&mut s, tecla('k'));
    }
    let Modo::Pick { idx, .. } = &s.modo else {
        panic!("sem o pick")
    };
    assert_eq!(*idx, 0);
}

// --- a tecla `p` (fatia 10b) ------------------------------------------------------

/// `alfa` com `a1` no prazo dado (texto como está escrito, e os dias que o motor
/// calcularia).
fn estado_com_prazo(prazo: &str, dias: Option<i64>) -> State {
    let mut d = dados_sinteticos();
    d.tasks[0].prazo = Some(prazo.into());
    d.tasks[0].dias_para_o_prazo = dias;
    State::novo(d)
}

fn valor_do_prompt(s: &State) -> &str {
    let Modo::Prompt {
        campo: Campo::Prazo,
        valor,
    } = &s.modo
    else {
        panic!("sem o prompt do prazo: {:?}", s.modo)
    };
    valor
}

#[test]
fn p_abre_o_prompt_com_o_prazo_escrito_mesmo_invalido_ou_vazio_sem_prazo() {
    let mut s = estado();
    assert_eq!(aperta(&mut s, tecla('p')), []);
    assert_eq!(valor_do_prompt(&s), "");
    for escrito in ["2026-12-24", "amanhã"] {
        let mut s = estado_com_prazo(escrito, None);
        assert_eq!(aperta(&mut s, tecla('p')), []);
        assert_eq!(valor_do_prompt(&s), escrito);
    }
}

#[test]
fn p_sem_task_avisa_e_nao_abre_o_prompt() {
    let mut s = State::novo(Dados::default());
    assert_eq!(aperta(&mut s, tecla('p')), []);
    assert_eq!(s.modo, Modo::Normal);
    assert_eq!(
        s.mensagem.as_ref().unwrap().texto,
        "nenhuma task selecionada"
    );
}

#[test]
fn p_grava_o_prazo_digitado_aparado_pelo_set() {
    let mut s = estado();
    aperta(&mut s, tecla('p'));
    digita(&mut s, " 2026-12-24 ");
    let (escrita, depois, ok) = executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert_eq!(
        escrita,
        set_de(
            "/v/projects/alfa/tasks/a1.md",
            PedidoSet {
                prazo: Some("2026-12-24".into()),
                ..PedidoSet::default()
            }
        )
    );
    assert_eq!((depois, ok.as_str()), (Depois::Manter, "prazo: 2026-12-24"));
}

#[test]
fn p_com_o_prompt_vazio_limpa_o_prazo() {
    let mut s = estado_com_prazo("2026-12-24", Some(44));
    aperta(&mut s, tecla('p'));
    aperta(&mut s, ctrl('u'));
    let (escrita, depois, ok) = executar(aperta(&mut s, codigo(KeyCode::Enter)));
    assert_eq!(
        escrita,
        set_de(
            "/v/projects/alfa/tasks/a1.md",
            PedidoSet {
                prazo: Some(String::new()),
                ..PedidoSet::default()
            }
        )
    );
    assert_eq!((depois, ok.as_str()), (Depois::Manter, "prazo removido"));
}

#[test]
fn p_com_o_prazo_igual_ao_escrito_nao_pede_nada_e_fecha() {
    // Sem prazo e vazio também é igual.
    let mut s = estado();
    aperta(&mut s, tecla('p'));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.modo, Modo::Normal);
    assert_eq!(s.refazer, None);
    // O prazo inválido confirmado sem mudança também não grava.
    let mut s = estado_com_prazo("amanhã", None);
    aperta(&mut s, tecla('p'));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.modo, Modo::Normal);
    // Espaços em volta do mesmo texto não contam como mudança.
    let mut s = estado_com_prazo("2026-12-24", Some(44));
    aperta(&mut s, tecla('p'));
    digita(&mut s, "  ");
    assert_eq!(aperta(&mut s, codigo(KeyCode::Enter)), []);
    assert_eq!(s.modo, Modo::Normal);
}

#[test]
fn recusa_do_motor_devolve_o_prompt_do_prazo_com_o_digitado() {
    let mut s = estado();
    aperta(&mut s, tecla('p'));
    digita(&mut s, "2026-02-30");
    executar(aperta(&mut s, codigo(KeyCode::Enter)));
    aplica(&mut s, Evento::Falhou("data inválida: 2026-02-30".into()));
    assert_eq!(valor_do_prompt(&s), "2026-02-30");
    let m = s.mensagem.as_ref().unwrap();
    assert_eq!(
        (m.texto.as_str(), m.erro),
        ("data inválida: 2026-02-30", true)
    );
}

#[test]
fn p_numa_arquivada_abre_o_prompt_como_nas_outras_teclas_de_task() {
    let mut s = estado();
    aperta(&mut s, tecla('A'));
    s.painel = Painel::Tasks;
    aperta(&mut s, tecla('G'));
    assert_eq!(s.task_atual().unwrap().id, "a0");
    aperta(&mut s, tecla('p'));
    assert_eq!(valor_do_prompt(&s), "");
}

#[test]
fn esc_fecha_o_prompt_do_prazo() {
    let mut s = estado();
    aperta(&mut s, tecla('p'));
    assert_eq!(aperta(&mut s, codigo(KeyCode::Esc)), []);
    assert_eq!(s.modo, Modo::Normal);
}
