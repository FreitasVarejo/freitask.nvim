//! A tecla `p` (spec §10.2, "A TUI ganha a tecla `p`"; decisão f10b.1): o
//! prompt com o prazo como está escrito, o que `Enter` grava (os mesmos bytes
//! que o `set --prazo` da CLI grava, fixados no aceite da 10a), a recusa de
//! data inválida com a mensagem da CLI, o vazio que limpa, o igual que não
//! grava, `Esc` e `Ctrl-C`.

use crate::util::*;
use freitask::tui::Pedido;

/// `ESCREVER_MD` com o frontmatter só com esta linha (como o `set --prazo` o
/// cria antes do bloco, com a linha em branco: aceite da 10a,
/// `set_prazo_cria_o_frontmatter_quando_falta`).
fn escrever_com(linha: &str) -> String {
    com_frontmatter(&[linha], ESCREVER_MD)
}

/// Aperta `p` e afirma que o prompt do prazo abriu (sem pedir nada ao laço).
fn abre_o_prompt(s: &mut freitask::tui::Sessao) {
    aperta(s, tecla('p'));
    let t = tela(s);
    prompt_do_prazo(&t);
}

// ===========================================================================
// Grava
// ===========================================================================

#[test]
fn p_numa_task_sem_prazo_abre_o_prompt_vazio_e_enter_grava_o_prazo() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    let t = tela(&s);
    let prompt = prompt_do_prazo(&t);
    assert!(
        t.itens_do(prompt)
            .iter()
            .all(|l| !l.chars().any(|c| c.is_ascii_digit())),
        "sem prazo escrito, o prompt abre vazio: {}",
        t.texto()
    );

    digita(&mut s, "2026-12-01");
    enter(&mut s);

    assert_eq!(le(home.path(), ESCREVER), escrever_com("prazo: 2026-12-01"));
    so_a_task_e_no_maximo_o_painel(&antes, &fotografa(home.path()), ESCREVER);
    assert_eq!(
        le(home.path(), CURRENT),
        PAINEL_BASE,
        "o prazo não entra no painel"
    );
    let t = tela(&s);
    barra_de_ok(&t, "prazo: 2026-12-01");
    assert!(!t.contem(PROMPT_PRAZO), "o prompt fechou: {}", t.texto());
    // A lista já mostra o prazo gravado.
    let t = tela_larga(&s);
    let item = item_de(&t, "escrever-spec");
    assert!(item.contains("prazo 01/12"), "{item:?}");
}

#[test]
fn p_age_na_task_selecionada_com_o_foco_nos_projetos_e_acrescenta_a_chave_no_fim_do_frontmatter() {
    // `beta-urgente` já tem frontmatter (`prioridade: alta`): a chave nova
    // entra no fim dele (aceite da 10a, `set_prazo_acrescenta_a_chave_no_fim_
    // do_frontmatter`). O foco fica nos projetos, como as teclas de task da
    // fatia 7.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('j'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    digita(&mut s, "2026-11-09");
    enter(&mut s);
    assert_eq!(
        le(home.path(), BETA_URGENTE),
        "---\nprioridade: alta\nprazo: 2026-11-09\n---\n\n> [!example] Bê urgente\n> [[projects/beta/tasks/beta-urgente|beta-urgente]]\n"
    );
    so_a_task_e_no_maximo_o_painel(&antes, &fotografa(home.path()), BETA_URGENTE);
    barra_de_ok(&tela(&s), "prazo: 2026-11-09");
}

#[test]
fn p_regenera_o_painel_ao_gravar() {
    // Uma edição de fora, depois de abrir: o `rebuild` da gravação a leva ao
    // painel, como em toda tecla que grava (fatia 7).
    let home = home_base();
    let mut s = abre(home.path());
    arquivo(
        home.path(),
        MEDIR,
        "> [!example] Medir o tempo de novo\n> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\n",
    );
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    digita(&mut s, "2026-12-01");
    enter(&mut s);
    assert_eq!(le(home.path(), ESCREVER), escrever_com("prazo: 2026-12-01"));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[ESCREVER, CURRENT])
    );
    assert_eq!(
        le(home.path(), CURRENT),
        PAINEL_BASE.replacen(
            "> [!example] Medir o tempo\n",
            "> [!example] Medir o tempo de novo\n",
            1
        )
    );
    barra_de_ok(&tela(&s), "prazo: 2026-12-01");
}

#[test]
fn p_vem_preenchido_com_o_prazo_escrito_e_troca_a_data_no_lugar() {
    let home = home_base_com(&[(ESCREVER, &escrever_com("prazo: 2026-11-20"))]);
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    let t = tela(&s);
    assert!(
        t.texto_do(prompt_do_prazo(&t)).contains("2026-11-20"),
        "o campo vem com o prazo escrito: {}",
        t.texto()
    );
    apaga(&mut s);
    digita(&mut s, "2026-12-02");
    enter(&mut s);
    assert_eq!(le(home.path(), ESCREVER), escrever_com("prazo: 2026-12-02"));
    so_a_task_e_no_maximo_o_painel(&antes, &fotografa(home.path()), ESCREVER);
    barra_de_ok(&tela(&s), "prazo: 2026-12-02");
}

#[test]
fn p_vem_preenchido_com_o_prazo_invalido_para_consertar_ali() {
    // f10b.1: o prompt mostra o prazo como está escrito, mesmo inválido; é o
    // conserto do `prazo-invalido` (o doctor não adivinha a data).
    let home = home_base_com(&[(ESCREVER, &escrever_com("prazo: 2026-02-30"))]);
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    let t = tela(&s);
    assert!(
        t.texto_do(prompt_do_prazo(&t)).contains("2026-02-30"),
        "o campo vem com o prazo inválido como está escrito: {}",
        t.texto()
    );
    apaga(&mut s);
    digita(&mut s, "2026-03-02");
    enter(&mut s);
    assert_eq!(le(home.path(), ESCREVER), escrever_com("prazo: 2026-03-02"));
    so_a_task_e_no_maximo_o_painel(&antes, &fotografa(home.path()), ESCREVER);
    barra_de_ok(&tela(&s), "prazo: 2026-03-02");
}

#[test]
fn p_grava_a_data_aparada() {
    // f10b.1: `Enter` grava o texto aparado.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    abre_o_prompt(&mut s);
    digita(&mut s, "  2026-12-01  ");
    enter(&mut s);
    assert_eq!(le(home.path(), ESCREVER), escrever_com("prazo: 2026-12-01"));
    barra_de_ok(&tela(&s), "prazo: 2026-12-01");
}

// ===========================================================================
// Vazio limpa
// ===========================================================================

#[test]
fn p_vazio_limpa_o_prazo_e_tira_o_frontmatter_que_esvazia() {
    let home = home_base_com(&[(ESCREVER, &escrever_com("prazo: 2026-11-20"))]);
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    apaga(&mut s);
    enter(&mut s);
    assert_eq!(le(home.path(), ESCREVER), ESCREVER_MD);
    so_a_task_e_no_maximo_o_painel(&antes, &fotografa(home.path()), ESCREVER);
    let t = tela(&s);
    barra_de_ok(&t, "prazo removido");
    assert!(!t.contem(PROMPT_PRAZO), "o prompt fechou: {}", t.texto());
    let t = tela_larga(&s);
    let item = item_de(&t, "escrever-spec");
    assert!(!item.contains("prazo"), "o prazo saiu da linha: {item:?}");
}

#[test]
fn p_vazio_limpa_so_o_prazo_e_mantem_as_outras_chaves() {
    let home = home_base_com(&[(
        BETA_URGENTE,
        "---\nprioridade: alta\nprazo: 2026-11-20\n---\n\n> [!example] Bê urgente\n> [[projects/beta/tasks/beta-urgente|beta-urgente]]\n",
    )]);
    let mut s = abre(home.path());
    aperta(&mut s, tecla('j'));
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    apaga(&mut s);
    enter(&mut s);
    assert_eq!(le(home.path(), BETA_URGENTE), BETA_URGENTE_MD);
    so_a_task_e_no_maximo_o_painel(&antes, &fotografa(home.path()), BETA_URGENTE);
    barra_de_ok(&tela(&s), "prazo removido");
}

// ===========================================================================
// Recusa
// ===========================================================================

#[test]
fn p_com_data_invalida_mostra_a_recusa_da_cli_e_devolve_o_prompt_com_o_digitado() {
    // A mensagem é a que a CLI imprime em stderr, sem o `freitask: ` (aceite
    // da 10a). "quando" também prova que, no prompt, `q` é texto e não sai.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    for data in [
        "quando",
        "2026-02-30",
        "2027-02-29",
        "01/12/2026",
        "2026-12-1",
    ] {
        abre_o_prompt(&mut s);
        apaga(&mut s);
        digita(&mut s, data);
        enter(&mut s);
        let t = tela_larga(&s);
        barra_de_erro(&t, &msg_data(data));
        assert!(
            t.texto_do(prompt_do_prazo(&t)).contains(data),
            "o prompt volta com o que foi digitado ({data}): {}",
            t.texto()
        );
        nada_mudou(&antes, &fotografa(home.path()));
        esc(&mut s);
    }

    // O prompt que voltou serve para corrigir.
    abre_o_prompt(&mut s);
    digita(&mut s, "2026-13-01");
    enter(&mut s);
    barra_de_erro(&tela_larga(&s), &msg_data("2026-13-01"));
    apaga(&mut s);
    digita(&mut s, "2026-12-01");
    enter(&mut s);
    assert_eq!(le(home.path(), ESCREVER), escrever_com("prazo: 2026-12-01"));
    barra_de_ok(&tela(&s), "prazo: 2026-12-01");
}

// ===========================================================================
// Não grava
// ===========================================================================

#[test]
fn p_confirmado_igual_ao_que_esta_escrito_nao_grava() {
    // f10b.1, como `t`, `i` e `r`: sem prazo e vazio, a mesma data, e o
    // mesmo prazo inválido não chamam o motor; o prompt fecha.
    let home = home_base_com(&[
        (MEDIR, &com_frontmatter(&["prazo: 2026-11-20"], MEDIR_MD)),
        (FECHAR, &com_frontmatter(&["prazo: 2026-02-30"], FECHAR_MD)),
    ]);
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    for id in ["escrever-spec", "medir-tempo", "fechar-ciclo"] {
        abre_o_prompt(&mut s);
        enter(&mut s);
        let t = tela(&s);
        assert!(
            !t.contem(PROMPT_PRAZO),
            "{id}: o prompt tinha de fechar: {}",
            t.texto()
        );
        aperta(&mut s, tecla('j'));
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn esc_no_prompt_do_p_cancela_sem_gravar_e_volta_a_dica() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    digita(&mut s, "2026-12-01");
    esc(&mut s);
    let t = tela(&s);
    assert!(!t.contem(PROMPT_PRAZO), "{}", t.texto());
    barra_com_a_dica(&t);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn ctrl_c_no_prompt_do_p_sai_sem_gravar() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    let antes = fotografa(home.path());
    abre_o_prompt(&mut s);
    digita(&mut s, "2026-12-01");
    assert_eq!(s.tecla(ctrl('c')), Pedido::Sair);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn p_sem_task_selecionada_avisa_na_barra() {
    let home = tempfile::tempdir().expect("tempdir");
    arquivo(home.path(), ".freitask/status.json", gerador::STATUS_JSON);
    pasta(home.path(), "projects/vazio/tasks");
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('p'));
    let t = tela(&s);
    barra_de_erro(&t, "nenhuma task selecionada");
    assert!(!t.contem(PROMPT_PRAZO), "{}", t.texto());
    nada_mudou(&antes, &fotografa(home.path()));
}
