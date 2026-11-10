//! Cada tecla da tabela de `docs/freitask.md` ("Uso interativo (TUI)") e da
//! ajuda de hoje (`HELP` em `tui/src/ui.rs`), com a semântica de
//! `tui/src/app.rs`: o que a pessoa vê depois da tecla e o que o vault guarda.
//! Os textos da barra e dos modais são os da TUI de hoje; as mensagens de
//! recusa são as do motor (as que a CLI imprime em stderr, sem o
//! `freitask: `), já fixadas nas fatias 3 e 4 e medidas no oráculo.

use crate::util::*;
use freitask::tui::Pedido;
use ratatui::crossterm::event::KeyCode;

// ===========================================================================
// Navegação
// ===========================================================================

#[test]
fn j_k_e_setas_movem_a_selecao_na_lista_de_tasks_sem_dar_a_volta() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    selecionada(&tela(&s), "escrever-spec");

    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    selecionada(&t, "medir-tempo");
    assert!(preview(&t).contains("Medir o tempo"), "{}", t.texto());

    aperta(&mut s, especial(KeyCode::Down));
    let t = tela(&s);
    selecionada(&t, "fechar-ciclo");
    assert!(preview(&t).contains("Fechar o ciclo"), "{}", t.texto());

    // No fim, para.
    aperta(&mut s, especial(KeyCode::Down));
    selecionada(&tela(&s), "fechar-ciclo");

    aperta(&mut s, tecla('k'));
    selecionada(&tela(&s), "medir-tempo");
    aperta(&mut s, especial(KeyCode::Up));
    selecionada(&tela(&s), "escrever-spec");
    // No topo, para.
    aperta(&mut s, especial(KeyCode::Up));
    selecionada(&tela(&s), "escrever-spec");
}

#[test]
fn g_e_g_maiusculo_vao_ao_topo_e_ao_fim() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('G'));
    selecionada(&tela(&s), "fechar-ciclo");
    aperta(&mut s, tecla('g'));
    selecionada(&tela(&s), "escrever-spec");
}

#[test]
fn h_l_setas_e_tab_trocam_de_painel() {
    // Ao abrir o foco está nos projetos (como hoje): `j` troca de projeto. O
    // painel com foco se prova pelo que o `j`/`k` move. Com o foco nos
    // projetos, a task selecionada se lê no preview.
    let home = home_base();
    let mut s = abre(home.path());

    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    projeto_selecionado(&t, "beta");
    assert!(
        t.titulo_das_tasks().contains("Tasks: beta"),
        "{}",
        t.texto()
    );
    assert!(preview(&t).contains("Bê urgente"), "{}", t.texto());
    aperta(&mut s, tecla('k'));
    projeto_selecionado(&tela(&s), "alfa");

    // `l`: foco nas tasks.
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    projeto_selecionado(&t, "alfa");
    selecionada(&t, "medir-tempo");

    // `h`: de volta aos projetos.
    aperta(&mut s, tecla('h'));
    aperta(&mut s, tecla('j'));
    projeto_selecionado(&tela(&s), "beta");

    // `→`: tasks.
    aperta(&mut s, especial(KeyCode::Right));
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    projeto_selecionado(&t, "beta");
    selecionada(&t, "beta-um");

    // `←`: projetos.
    aperta(&mut s, especial(KeyCode::Left));
    aperta(&mut s, tecla('k'));
    projeto_selecionado(&tela(&s), "alfa");

    // `Tab` alterna: tasks…
    tab(&mut s);
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    projeto_selecionado(&t, "alfa");
    selecionada(&t, "medir-tempo");
    // …e de volta aos projetos.
    tab(&mut s);
    aperta(&mut s, tecla('j'));
    projeto_selecionado(&tela(&s), "beta");
}

#[test]
fn enter_no_painel_de_projetos_passa_o_foco_para_as_tasks_sem_abrir_nada() {
    let home = home_base();
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    enter(&mut s);
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    projeto_selecionado(&t, "alfa");
    selecionada(&t, "medir-tempo");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn trocar_de_projeto_volta_a_selecao_para_o_topo_da_lista() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('G'));
    selecionada(&tela(&s), "fechar-ciclo");
    aperta(&mut s, tecla('h'));
    aperta(&mut s, tecla('j'));
    aperta(&mut s, tecla('l'));
    let t = tela(&s);
    projeto_selecionado(&t, "beta");
    selecionada(&t, "beta-urgente");
    aperta(&mut s, tecla('h'));
    aperta(&mut s, tecla('k'));
    aperta(&mut s, tecla('l'));
    selecionada(&tela(&s), "escrever-spec");
}

// ===========================================================================
// `/`: filtro por id ou título
// ===========================================================================

#[test]
fn barra_filtra_por_id_mostra_o_filtro_na_barra_e_esc_limpa() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('/'));
    digita(&mut s, "spec");
    let t = tela(&s);
    ids_na_lista(&t, &["escrever-spec"]);
    assert_eq!(t.barra(), "/spec", "a barra mostra o filtro sendo digitado");
    esc(&mut s);
    let t = tela(&s);
    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
    barra_com_a_dica(&t);
}

#[test]
fn filtro_casa_o_titulo_sem_diferenca_de_caixa() {
    // "o tempo" está só no título de `medir-tempo` ("Medir o tempo"), não no id.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('/'));
    digita(&mut s, "O TEMPO");
    ids_na_lista(&tela(&s), &["medir-tempo"]);
}

#[test]
fn enter_no_filtro_fixa_o_filtro_no_titulo_e_esc_depois_limpa() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('/'));
    digita(&mut s, "ciclo");
    enter(&mut s);
    let t = tela(&s);
    ids_na_lista(&t, &["fechar-ciclo"]);
    assert!(
        t.titulo_das_tasks().contains("/ciclo"),
        "o título do painel diz o filtro: {}",
        t.titulo_das_tasks()
    );
    barra_com_a_dica(&t);
    // Fora do modo de filtro, `j`/`k` voltam a navegar e `Esc` limpa.
    esc(&mut s);
    let t = tela(&s);
    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
    assert!(!t.titulo_das_tasks().contains("/ciclo"));
}

#[test]
fn no_filtro_as_letras_sao_texto_e_nao_teclas() {
    // `q` e `j` digitados no filtro entram no filtro: nem saem nem navegam.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('/'));
    aperta(&mut s, tecla('q'));
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    assert_eq!(t.barra(), "/qj");
    ids_na_lista(&t, &[]);
    assert!(
        preview(&t).contains("sem task selecionada"),
        "{}",
        t.texto()
    );
    esc(&mut s);
    assert_eq!(s.tecla(tecla('q')), Pedido::Sair);
}

// ===========================================================================
// `n`: task nova (título → id sugerido → descrição)
// ===========================================================================

#[test]
fn n_cria_a_task_todo_com_o_id_sugerido_a_partir_do_titulo() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('n'));
    let t = tela(&s);
    let modal = t.painel("Nova task em alfa");
    assert!(
        t.itens_do(modal).len() >= 3,
        "título, id e descrição: {}",
        t.texto()
    );

    digita(&mut s, "Ávida nova");
    tab(&mut s);
    let t = tela(&s);
    assert!(
        t.texto_do(t.painel("Nova task em alfa"))
            .contains("avida-nova"),
        "o id sugerido é o kebab do título: {}",
        t.texto()
    );
    tab(&mut s);
    digita(&mut s, "começando");
    enter(&mut s);

    const NOVA: &str = "projects/alfa/tasks/avida-nova.md";
    assert_eq!(
        le(home.path(), NOVA),
        "> [!todo] Ávida nova\n> [[projects/alfa/tasks/avida-nova|avida-nova]]\n> _começando_\n\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[NOVA, CURRENT])
    );
    assert!(
        le(home.path(), CURRENT).contains(
            "> [!todo] Ávida nova\n> [[projects/alfa/tasks/avida-nova|avida-nova]]\n> _começando_\n"
        ),
        "o painel é regenerado com a task nova"
    );
    let t = tela(&s);
    barra_de_ok(&t, "criada: avida-nova");
    assert!(!t.contem("Nova task em"), "o modal fechou: {}", t.texto());
    ids_na_lista(
        &t,
        &["avida-nova", "escrever-spec", "medir-tempo", "fechar-ciclo"],
    );
    selecionada(&t, "avida-nova");
}

#[test]
fn n_id_digitado_a_mao_nao_e_sobrescrito_pela_sugestao() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('n'));
    digita(&mut s, "Ávida nova");
    tab(&mut s);
    // "avida-nova" → "avida-" → "avida-x": o id foi mexido à mão.
    for _ in 0..4 {
        aperta(&mut s, especial(KeyCode::Backspace));
    }
    digita(&mut s, "x");
    // De volta ao título, que muda; ao avançar, o id fica o digitado.
    aperta(&mut s, especial(KeyCode::BackTab));
    digita(&mut s, " extra");
    tab(&mut s);
    let t = tela(&s);
    let modal = t.texto_do(t.painel("Nova task em alfa"));
    assert!(modal.contains("avida-x"), "{}", t.texto());
    assert!(!modal.contains("avida-nova-extra"), "{}", t.texto());
    tab(&mut s);
    enter(&mut s);
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/avida-x.md"),
        "> [!todo] Ávida nova extra\n> [[projects/alfa/tasks/avida-x|avida-x]]\n\n"
    );
    assert!(!existe(
        home.path(),
        "projects/alfa/tasks/avida-nova-extra.md"
    ));
}

#[test]
fn n_recusada_pelo_motor_mostra_a_mensagem_e_devolve_o_modal_preenchido() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('n'));
    digita(&mut s, "Outra");
    tab(&mut s);
    apaga(&mut s);
    digita(&mut s, "Id Ruim");
    tab(&mut s);
    enter(&mut s);

    let t = tela_de(&s, 250, 30);
    barra_de_erro(
        &t,
        "id precisa ser kebab-case: \"Id Ruim\" (seria \"id-ruim\")",
    );
    let modal = t.texto_do(t.painel("Nova task em alfa"));
    assert!(modal.contains("Outra"), "o título voltou: {}", t.texto());
    assert!(modal.contains("Id Ruim"), "o id voltou: {}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));

    // O modal que voltou serve para corrigir: o título continua lá.
    aperta(&mut s, especial(KeyCode::BackTab));
    apaga(&mut s);
    digita(&mut s, "outra");
    tab(&mut s);
    enter(&mut s);
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/outra.md"),
        "> [!todo] Outra\n> [[projects/alfa/tasks/outra|outra]]\n\n"
    );
    barra_de_ok(&tela(&s), "criada: outra");
}

#[test]
fn n_sem_titulo_recusa_na_barra_e_nao_grava() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('n'));
    tab(&mut s);
    digita(&mut s, "sem-titulo");
    tab(&mut s);
    enter(&mut s);
    let t = tela(&s);
    barra_de_erro(&t, "falta o título");
    assert!(t.contem("Nova task em alfa"), "o modal fica: {}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn n_sem_projeto_pede_para_criar_um_antes() {
    let home = tempfile::tempdir().expect("tempdir");
    pasta(home.path(), "");
    let mut s = abre(home.path());
    aperta(&mut s, tecla('n'));
    let t = tela(&s);
    barra_de_erro(&t, "crie um projeto antes (N)");
    assert!(!t.contem("Nova task"), "{}", t.texto());
}

// ===========================================================================
// `s`: fase
// ===========================================================================

/// As fases que o `set` aceita, na ordem do `status.json` default: todas
/// menos `done` (Arquivada) e o sentinela 0 (Sem status).
const FASES: [&str; 8] = [
    "Não iniciada",
    "Refinando",
    "Refinada",
    "Em implementação",
    "Em homologação",
    "Bloqueada",
    "Pronta",
    "Algum dia",
];

#[test]
fn s_oferece_so_as_fases_que_o_set_aceita_com_a_atual_em_destaque() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('s'));
    let t = tela(&s);
    let modal = t.painel("Fase de escrever-spec");
    let itens = t.itens_do(modal);
    assert_eq!(itens.len(), FASES.len(), "opções: {itens:#?}");
    for (item, fase) in itens.iter().zip(FASES) {
        assert!(item.contains(fase), "esperava {fase:?}, veio {item:?}");
    }
    assert!(t.destacada(modal).contains("Não iniciada"));
    assert!(!t.contem("Arquivada"), "{}", t.texto());
    assert!(!t.contem("Sem status"), "{}", t.texto());
    esc(&mut s);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn s_grava_o_callout_da_fase_escolhida_e_regenera_o_painel() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('s'));
    aperta_todas(&mut s, "jjj");
    let t = tela(&s);
    assert!(
        t.destacada(t.painel("Fase de escrever-spec"))
            .contains("Em implementação")
    );
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        "> [!example] Escrever a spec\n> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n> _em curso_\n> nota um\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[ESCREVER, CURRENT])
    );
    assert!(le(home.path(), CURRENT).contains("> [!example] Escrever a spec\n"));
    let t = tela(&s);
    let barra = t.barra();
    assert!(
        barra.starts_with("fase: ") && barra.ends_with("Em implementação"),
        "barra: {barra:?}"
    );
    assert_eq!(t.cor_da_barra(), ratatui::style::Color::Green);
    assert!(!t.contem("Fase de"), "o modal fechou: {}", t.texto());
}

// ===========================================================================
// `t` e `i`: título e descrição
// ===========================================================================

#[test]
fn t_grava_o_titulo() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('t'));
    let t = tela(&s);
    assert!(
        t.texto_do(t.painel("Título")).contains("Escrever a spec"),
        "o campo vem com o título atual: {}",
        t.texto()
    );
    apaga(&mut s);
    digita(&mut s, "Escrever a spec nova");
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        "> [!todo] Escrever a spec nova\n> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n> _em curso_\n> nota um\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[ESCREVER, CURRENT])
    );
    assert!(le(home.path(), CURRENT).contains("> [!todo] Escrever a spec nova\n"));
    barra_de_ok(&tela(&s), "título atualizado");
}

#[test]
fn t_com_titulo_vazio_recusa_e_o_modal_fica() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('t'));
    // `Ctrl-U` limpa o campo (`app.rs`, `on_prompt`).
    aperta(&mut s, ctrl('u'));
    enter(&mut s);
    let t = tela(&s);
    barra_de_erro(&t, "não pode ficar vazio");
    assert!(t.contem("Título"), "o modal fica aberto: {}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn i_com_descricao_vazia_apaga_a_descricao() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('i'));
    let t = tela(&s);
    assert!(
        t.texto_do(t.painel("Descrição do estado"))
            .contains("em curso"),
        "o campo vem com a descrição atual: {}",
        t.texto()
    );
    apaga(&mut s);
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        "> [!todo] Escrever a spec\n> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n> nota um\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[ESCREVER, CURRENT])
    );
    barra_de_ok(&tela(&s), "descrição atualizada");
}

#[test]
fn i_grava_a_descricao_nova() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('i'));
    apaga(&mut s);
    digita(&mut s, "esperando revisão");
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        "> [!todo] Escrever a spec\n> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n> _esperando revisão_\n> nota um\n"
    );
    barra_de_ok(&tela(&s), "descrição atualizada");
}

#[test]
fn modais_confirmados_sem_mudanca_nao_gravam() {
    // `t`, `i`, `r` e `s` confirmados com o valor de antes não chamam o motor
    // (`app.rs`, `submit_prompt` e `on_pick`); `a` cancelado com `Esc` ou com
    // `q` (que no pick fecha, não sai) também não.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    for k in ['t', 'i', 'r', 's'] {
        aperta(&mut s, tecla(k));
        enter(&mut s);
        let t = tela(&s);
        for titulo in ["Título", "Descrição do estado", "Novo id", "Fase de"] {
            assert!(
                !t.contem(titulo),
                "{k}: o modal tinha de fechar: {}",
                t.texto()
            );
        }
    }
    aperta(&mut s, tecla('a'));
    esc(&mut s);
    aperta(&mut s, tecla('a'));
    aperta(&mut s, tecla('q'));
    assert!(!tela(&s).contem("Arquivar"), "{}", tela(&s).texto());
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `r`: renomear o id
// ===========================================================================

#[test]
fn r_renomeia_o_arquivo_e_reaponta_os_links_do_vault() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('j'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('r'));
    let t = tela(&s);
    assert!(
        t.texto_do(t.painel("Novo id")).contains("medir-tempo"),
        "o campo vem com o id atual: {}",
        t.texto()
    );
    apaga(&mut s);
    digita(&mut s, "medir-o-tempo");
    enter(&mut s);

    const NOVO: &str = "projects/alfa/tasks/medir-o-tempo.md";
    assert!(!existe(home.path(), MEDIR));
    assert_eq!(
        le(home.path(), NOVO),
        "> [!example] Medir o tempo\n> [[projects/alfa/tasks/medir-o-tempo|medir-o-tempo]]\n\n## Histórico\n\n- 2026-11-10 — renomeada de `medir-tempo` para `medir-o-tempo`\n"
    );
    assert_eq!(
        le(home.path(), NOTAS),
        "# Notas\n\nver [[projects/alfa/tasks/medir-o-tempo|medir-o-tempo]] e [[medir-o-tempo]]\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[MEDIR, NOVO, NOTAS, CURRENT])
    );
    let t = tela(&s);
    barra_de_ok(&t, "renomeada: medir-tempo → medir-o-tempo");
    selecionada(&t, "medir-o-tempo");
    assert!(preview(&t).contains("Medir o tempo"), "{}", t.texto());
}

#[test]
fn r_para_um_id_que_ja_existe_mostra_a_recusa_e_devolve_o_modal() {
    // `velha-feita` está arquivada: o id continua ocupado (é o nome da branch).
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('j'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('r'));
    apaga(&mut s);
    digita(&mut s, "velha-feita");
    enter(&mut s);
    let t = tela_de(&s, 300, 30);
    barra_de_erro(
        &t,
        &format!(
            "já existe uma task com esse id: {}",
            abs(home.path(), VELHA).display()
        ),
    );
    assert!(
        t.texto_do(t.painel("Novo id")).contains("velha-feita"),
        "o modal volta com o que foi digitado: {}",
        t.texto()
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `a`: arquivar / desarquivar
// ===========================================================================

#[test]
fn a_arquiva_com_o_tipo_sugerido_ja_selecionado() {
    // `fechar-ciclo` está Pronta (`check`): o tipo sugerido é `done`.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('G'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('a'));
    let t = tela(&s);
    let modal = t.painel("Arquivar fechar-ciclo como");
    let itens = t.itens_do(modal);
    assert_eq!(itens.len(), 3, "{itens:#?}");
    for (item, tipo) in
        itens
            .iter()
            .zip(["done — feito", "dropped — abandonado", "failed — falhou"])
    {
        assert!(item.contains(tipo), "esperava {tipo:?}, veio {item:?}");
    }
    assert!(t.destacada(modal).contains("done — feito"));
    enter(&mut s);

    const ARQ: &str = "projects/alfa/tasks/archived/done/fechar-ciclo.md";
    assert!(!existe(home.path(), FECHAR));
    assert_eq!(
        le(home.path(), ARQ),
        "> [!done] Fechar o ciclo\n> [[projects/alfa/tasks/archived/done/fechar-ciclo|fechar-ciclo]]\n\n## Histórico\n\n- 2026-11-10 — arquivada em `archived/done` (feito)\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[FECHAR, ARQ, CURRENT])
    );
    assert!(!le(home.path(), CURRENT).contains("fechar-ciclo"));
    let t = tela(&s);
    barra_de_ok(&t, "arquivada como done: fechar-ciclo");
    ids_na_lista(&t, &["escrever-spec", "medir-tempo"]);
}

#[test]
fn a_numa_task_que_nao_esta_pronta_sugere_dropped_e_deixa_escolher_outro() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('a'));
    let t = tela(&s);
    let modal = t.painel("Arquivar escrever-spec como");
    assert!(t.destacada(modal).contains("dropped — abandonado"));
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    assert!(
        t.destacada(t.painel("Arquivar escrever-spec como"))
            .contains("failed — falhou")
    );
    enter(&mut s);
    assert!(!existe(home.path(), ESCREVER));
    assert_eq!(
        le(
            home.path(),
            "projects/alfa/tasks/archived/failed/escrever-spec.md"
        ),
        "> [!todo] Escrever a spec\n> [[projects/alfa/tasks/archived/failed/escrever-spec|escrever-spec]]\n> _em curso_\n> nota um\n\n## Histórico\n\n- 2026-11-10 — arquivada em `archived/failed` (falhou)\n"
    );
    barra_de_ok(&tela(&s), "arquivada como failed: escrever-spec");
}

#[test]
fn a_numa_arquivada_confirma_com_y_e_desarquiva() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('A'));
    aperta(&mut s, tecla('G'));
    selecionada(&tela(&s), "velha-feita");
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('a'));
    let t = tela(&s);
    t.painel("Confirmar");
    assert!(t.contem("Desarquivar velha-feita?"), "{}", t.texto());
    aperta(&mut s, tecla('y'));

    const ATIVA: &str = "projects/alfa/tasks/velha-feita.md";
    assert!(!existe(home.path(), VELHA));
    assert_eq!(
        le(home.path(), ATIVA),
        "> [!check] Velha feita\n> [[projects/alfa/tasks/velha-feita|velha-feita]]\n\n## Histórico\n\n- 2026-01-05 — arquivada em `archived/done` (feito)\n- 2026-11-10 — desarquivada, de volta ao board\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[VELHA, ATIVA, CURRENT])
    );
    assert!(le(home.path(), CURRENT).contains("> [!check] Velha feita\n"));
    let t = tela(&s);
    barra_de_ok(&t, "desarquivada: velha-feita");
    selecionada(&t, "velha-feita");
}

#[test]
fn a_numa_arquivada_sem_y_nao_desarquiva() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('A'));
    aperta(&mut s, tecla('G'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('a'));
    aperta(&mut s, tecla('n'));
    let t = tela(&s);
    assert!(!t.contem("Confirmar"), "{}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `d`: largar (arquivar como dropped, pede `y`)
// ===========================================================================

#[test]
fn d_so_larga_com_y_e_arquiva_como_dropped() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('d'));
    let t = tela(&s);
    t.painel("Confirmar");
    assert!(
        t.contem("Largar escrever-spec? Ela vai para archived/dropped."),
        "{}",
        t.texto()
    );
    aperta(&mut s, tecla('n'));
    assert!(!tela(&s).contem("Confirmar"));
    nada_mudou(&antes, &fotografa(home.path()));

    aperta(&mut s, tecla('d'));
    aperta(&mut s, tecla('y'));
    const ARQ: &str = "projects/alfa/tasks/archived/dropped/escrever-spec.md";
    assert!(!existe(home.path(), ESCREVER));
    assert_eq!(
        le(home.path(), ARQ),
        "> [!todo] Escrever a spec\n> [[projects/alfa/tasks/archived/dropped/escrever-spec|escrever-spec]]\n> _em curso_\n> nota um\n\n## Histórico\n\n- 2026-11-10 — arquivada em `archived/dropped` (abandonado)\n"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[ESCREVER, ARQ, CURRENT])
    );
    barra_de_ok(&tela(&s), "largada: escrever-spec");
}

#[test]
fn d_numa_arquivada_recusa_e_nao_grava() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('A'));
    aperta(&mut s, tecla('G'));
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('d'));
    let t = tela(&s);
    barra_de_erro(&t, "velha-feita já está arquivada (a desarquiva)");
    assert!(!t.contem("Confirmar"), "{}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `N`: projeto novo
// ===========================================================================

#[test]
fn tecla_n_maiuscula_cria_o_projeto_e_o_seleciona() {
    // Como `freitask new-project`: o nome passa por kebab, e o painel não é
    // regenerado (decisão da fatia 3).
    let home = home_base();
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('N'));
    tela(&s).painel("Novo projeto");
    digita(&mut s, "Gama Nova");
    enter(&mut s);
    assert!(
        abs(home.path(), "projects/gama-nova/tasks").is_dir(),
        "projects/gama-nova/tasks/ tinha de existir"
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[])
    );
    let t = tela(&s);
    barra_de_ok(&t, "projeto criado");
    let projetos = t.itens_do(t.projetos());
    assert_eq!(projetos.len(), 3, "{projetos:#?}");
    for (item, nome) in projetos.iter().zip(["alfa", "beta", "gama-nova"]) {
        assert!(item.contains(nome), "esperava {nome:?}, veio {item:?}");
    }
    projeto_selecionado(&t, "gama-nova");
    assert!(
        t.titulo_das_tasks().contains("Tasks: gama-nova"),
        "{}",
        t.titulo_das_tasks()
    );
    ids_na_lista(&t, &[]);
}

#[test]
fn tecla_n_maiuscula_com_projeto_que_ja_existe_mostra_a_recusa_e_devolve_o_modal() {
    let home = home_base();
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('N'));
    digita(&mut s, "alfa");
    enter(&mut s);
    let t = tela(&s);
    barra_de_erro(&t, "projeto já existe: alfa");
    assert!(
        t.texto_do(t.painel("Novo projeto")).contains("alfa"),
        "{}",
        t.texto()
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn tecla_n_maiuscula_com_nome_vazio_recusa() {
    let home = home_base();
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('N'));
    enter(&mut s);
    let t = tela(&s);
    barra_de_erro(&t, "não pode ficar vazio");
    assert!(t.contem("Novo projeto"), "{}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `c`: regenerar e abrir o CURRENT.md
// ===========================================================================

#[test]
fn c_regenera_o_current_md_e_pede_para_abri_lo() {
    let home = home_base();
    let mut s = abre(home.path());
    // Uma edição de fora, depois de abrir: só o `c` a leva ao painel.
    arquivo(
        home.path(),
        MEDIR,
        "> [!example] Medir o tempo de novo\n> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\n",
    );
    assert_eq!(le(home.path(), CURRENT), PAINEL_BASE);
    assert_eq!(
        s.tecla(tecla('c')),
        Pedido::Abrir(abs(home.path(), CURRENT))
    );
    assert_eq!(
        le(home.path(), CURRENT),
        PAINEL_BASE.replacen(
            "> [!example] Medir o tempo\n",
            "> [!example] Medir o tempo de novo\n",
            1
        )
    );
}

#[test]
fn c_com_rebuild_que_falha_mostra_o_erro_e_nao_pede_para_abrir() {
    let home = home_base();
    pasta(home.path(), CURRENT);
    let mut s = abre(home.path());
    esc(&mut s);
    barra_com_a_dica(&tela(&s));
    assert_eq!(s.tecla(tecla('c')), Pedido::Nada);
    let t = tela_de(&s, 300, 30);
    assert_eq!(
        t.cor_da_barra(),
        ratatui::style::Color::Red,
        "{}",
        t.barra()
    );
    assert!(!t.barra().contains("? ajuda"), "{}", t.barra());
}

// ===========================================================================
// `R`: reler
// ===========================================================================

#[test]
fn r_maiusculo_rele_o_vault_sem_gravar_nada() {
    let home = home_base();
    let mut s = abre(home.path());
    arquivo(
        home.path(),
        "projects/alfa/tasks/nova-de-fora.md",
        "> [!todo] Nova de fora\n> [[projects/alfa/tasks/nova-de-fora|nova-de-fora]]\n",
    );
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('R'));
    nada_mudou(&antes, &fotografa(home.path()));
    let t = tela(&s);
    barra_de_ok(&t, "relido");
    ids_na_lista(
        &t,
        &[
            "escrever-spec",
            "nova-de-fora",
            "medir-tempo",
            "fechar-ciclo",
        ],
    );
    assert!(
        t.itens_do(t.projetos())
            .iter()
            .any(|p| p.contains("alfa 4")),
        "a contagem do projeto acompanha: {}",
        t.texto()
    );
}

// ===========================================================================
// `?`, `q`, `Ctrl-C`, `Esc`
// ===========================================================================

#[test]
fn interrogacao_abre_a_ajuda_e_a_tecla_seguinte_so_a_fecha() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('?'));
    assert!(tela(&s).contem("Teclas"), "{}", tela(&s).texto());
    // Na ajuda, `q` fecha a ajuda e não sai.
    aperta(&mut s, tecla('q'));
    let t = tela(&s);
    assert!(!t.contem("Teclas"), "{}", t.texto());
    assert_eq!(s.tecla(tecla('q')), Pedido::Sair);
}

#[test]
fn q_pede_para_sair_sem_gravar() {
    let home = home_base();
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    assert_eq!(s.tecla(tecla('q')), Pedido::Sair);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn ctrl_c_sai_de_qualquer_modo_sem_gravar() {
    // Normal, filtro, task nova, cada prompt, cada pick, cada confirmação e a
    // ajuda.
    for abre_modo in ["", "/x", "nx", "tx", "ix", "rx", "s", "a", "d", "?", "Nx"] {
        let home = home_base();
        let mut s = abre(home.path());
        aperta(&mut s, tecla('l'));
        let antes = fotografa(home.path());
        aperta_todas(&mut s, abre_modo);
        assert_eq!(
            s.tecla(ctrl('c')),
            Pedido::Sair,
            "Ctrl-C depois de {abre_modo:?}"
        );
        nada_mudou(&antes, &fotografa(home.path()));
    }
}

#[test]
fn esc_fecha_cada_modal_sem_gravar_e_volta_a_dica() {
    let titulos = [
        "Nova task",
        "Título",
        "Descrição do estado",
        "Novo id",
        "Novo projeto",
        "Fase de",
        "Arquivar",
        "Confirmar",
        "Teclas",
    ];
    for abre_modo in ["nx", "tx", "ix", "rx", "Nx", "s", "a", "d", "?", "/x"] {
        let home = home_base();
        let mut s = abre(home.path());
        aperta(&mut s, tecla('l'));
        let antes = fotografa(home.path());
        aperta_todas(&mut s, abre_modo);
        esc(&mut s);
        let t = tela(&s);
        for titulo in titulos {
            assert!(
                !t.contem(titulo),
                "Esc depois de {abre_modo:?} deixou {titulo:?}: {}",
                t.texto()
            );
        }
        barra_com_a_dica(&t);
        ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
        nada_mudou(&antes, &fotografa(home.path()));
    }
}

#[test]
fn esc_limpa_a_mensagem_da_barra() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('A'));
    aperta(&mut s, tecla('G'));
    aperta(&mut s, tecla('d'));
    assert_eq!(tela(&s).cor_da_barra(), ratatui::style::Color::Red);
    esc(&mut s);
    barra_com_a_dica(&tela(&s));
}

// ===========================================================================
// `C-d` / `C-u`: rolar o preview
// ===========================================================================

/// Os números `NN` das linhas `linha NN` visíveis no preview.
fn linhas_visiveis(t: &Tela) -> Vec<u32> {
    let mut v: Vec<u32> = t
        .linhas_do(t.preview())
        .iter()
        .filter_map(|l| {
            let i = l.find("linha ")?;
            l[i + "linha ".len()..]
                .chars()
                .take(2)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn ctrl_d_e_ctrl_u_rolam_o_preview() {
    let home = tempfile::tempdir().expect("tempdir");
    arquivo(home.path(), ".freitask/status.json", gerador::STATUS_JSON);
    let mut corpo = String::from("> [!todo] Longa\n> [[projects/alfa/tasks/longa|longa]]\n\n");
    for i in 1..=60 {
        corpo.push_str(&format!("linha {i:02}\n"));
    }
    arquivo(home.path(), "projects/alfa/tasks/longa.md", &corpo);
    let mut s = abre(home.path());

    let antes = linhas_visiveis(&tela(&s));
    assert!(
        antes.first() == Some(&1) && antes.last() < Some(&60),
        "o preview começa no topo e não cabe inteiro: {antes:?}"
    );
    aperta(&mut s, ctrl('d'));
    let depois = linhas_visiveis(&tela(&s));
    assert!(
        depois.first() > antes.first() && depois.last() > antes.last(),
        "Ctrl-D rola para baixo: antes {antes:?}, depois {depois:?}"
    );
    aperta(&mut s, ctrl('u'));
    assert_eq!(linhas_visiveis(&tela(&s)), antes, "Ctrl-U volta para cima");
}

// ===========================================================================
// `Enter` / `e`: abrir a task no editor
// ===========================================================================

#[test]
fn enter_e_e_pedem_para_abrir_a_task_selecionada_sem_gravar() {
    let home = home_base();
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    // `e` abre a task selecionada mesmo com o foco nos projetos.
    assert_eq!(
        s.tecla(tecla('e')),
        Pedido::Abrir(abs(home.path(), ESCREVER))
    );
    enter(&mut s); // nos projetos, Enter só passa para as tasks
    aperta(&mut s, tecla('j'));
    assert_eq!(
        s.tecla(especial(KeyCode::Enter)),
        Pedido::Abrir(abs(home.path(), MEDIR))
    );
    aperta(&mut s, tecla('A'));
    aperta(&mut s, tecla('G'));
    assert_eq!(s.tecla(tecla('e')), Pedido::Abrir(abs(home.path(), VELHA)));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn enter_e_e_sem_task_nao_pedem_nada() {
    let home = tempfile::tempdir().expect("tempdir");
    arquivo(home.path(), ".freitask/status.json", gerador::STATUS_JSON);
    pasta(home.path(), "projects/vazio/tasks");
    let mut s = abre(home.path());
    aperta(&mut s, tecla('e'));
    aperta(&mut s, tecla('l'));
    aperta(&mut s, especial(KeyCode::Enter));
    aperta(&mut s, tecla('e'));
}

#[test]
fn teclas_de_task_sem_task_selecionada_avisam_na_barra() {
    let home = tempfile::tempdir().expect("tempdir");
    arquivo(home.path(), ".freitask/status.json", gerador::STATUS_JSON);
    pasta(home.path(), "projects/vazio/tasks");
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    for k in ['s', 't', 'i', 'r', 'a', 'd'] {
        esc(&mut s);
        aperta(&mut s, tecla(k));
        let t = tela(&s);
        barra_de_erro(&t, "nenhuma task selecionada");
        for titulo in [
            "Título",
            "Descrição do estado",
            "Novo id",
            "Fase de",
            "Arquivar",
            "Confirmar",
        ] {
            assert!(!t.contem(titulo), "{k}: {}", t.texto());
        }
    }
    nada_mudou(&antes, &fotografa(home.path()));
}
