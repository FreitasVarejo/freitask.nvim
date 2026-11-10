//! A tecla `b` (spec §10.1, "A TUI ganha a tecla `b`, um prompt com a lista
//! atual"; f9b.1): o prompt, o que ele grava, as recusas do motor, `Esc`,
//! `C-u` e `Ctrl-C`. Cada teste afirma primeiro que o prompt abriu, para não
//! passar sem a tecla.

use crate::util::*;
use freitask::tui::Pedido;

// ===========================================================================
// Abrir o prompt
// ===========================================================================

#[test]
fn b_abre_o_prompt_com_a_moldura_e_a_lista_atual_preenchida() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    let t = tela(&s);
    assert_eq!(
        prompt_aberto(&t),
        "medir, fechar",
        "o campo vem com a lista do frontmatter, juntada por `, `:\n{}",
        t.texto()
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn b_numa_task_sem_a_chave_abre_o_prompt_vazio() {
    // `medir` tem frontmatter, sem `bloqueada-por`.
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "medir");
    aperta(&mut s, tecla('b'));
    let t = tela(&s);
    assert_eq!(prompt_aberto(&t), "", "o campo vem vazio:\n{}", t.texto());
}

#[test]
fn b_com_o_foco_nos_projetos_age_sobre_a_task_selecionada() {
    // Ao abrir, o foco está nos projetos e `escrever` é a task selecionada.
    let home = home_deps();
    let mut s = abre(home.path());
    let painel = le(home.path(), CURRENT);
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "");
    digita(&mut s, "fechar");
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        com_frontmatter_novo("fechar", ESCREVER_MD)
    );
    so_a_task_mudou(home.path(), &antes, ESCREVER, &painel);
    barra_de_ok(&tela(&s), "bloqueada-por atualizada");
}

#[test]
fn b_sem_task_selecionada_avisa_na_barra_e_nao_abre_o_prompt() {
    let home = tempfile::tempdir().expect("tempdir");
    arquivo(home.path(), ".freitask/status.json", gerador::STATUS_JSON);
    pasta(home.path(), "projects/vazio/tasks");
    let mut s = abre(home.path());
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    let t = tela(&s);
    barra_de_erro(&t, "nenhuma task selecionada");
    prompt_fechado(&t);
    // Com o foco nas tasks (vazias), o mesmo.
    esc(&mut s);
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('b'));
    let t = tela(&s);
    barra_de_erro(&t, "nenhuma task selecionada");
    prompt_fechado(&t);
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// Enter grava
// ===========================================================================

#[test]
fn enter_grava_a_lista_no_frontmatter_e_mostra_o_ok_em_verde() {
    // `escrever` não tem frontmatter: o `set` cria um só com a lista.
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "escrever");
    let painel = le(home.path(), CURRENT);
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "");
    digita(&mut s, "medir, fechar");
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        "---\nbloqueada-por: [medir, fechar]\n---\n\n> [!todo] Escrever a spec\n> [[projects/alfa/tasks/escrever|escrever]]\n> _em curso_\n"
    );
    so_a_task_mudou(home.path(), &antes, ESCREVER, &painel);
    let t = tela(&s);
    barra_de_ok(&t, "bloqueada-por atualizada");
    prompt_fechado(&t);
}

#[test]
fn enter_poe_a_chave_no_fim_do_frontmatter_que_ja_existe() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "medir");
    let painel = le(home.path(), CURRENT);
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "");
    digita(&mut s, "fechar");
    enter(&mut s);
    assert_eq!(
        le(home.path(), MEDIR),
        "---\ntags: [x]\nbloqueada-por: [fechar]\n---\n\n> [!example] Medir o tempo\n> [[projects/alfa/tasks/medir|medir]]\n"
    );
    so_a_task_mudou(home.path(), &antes, MEDIR, &painel);
    barra_de_ok(&tela(&s), "bloqueada-por atualizada");
}

#[test]
fn enter_acrescenta_um_id_a_lista_que_veio_preenchida_trocando_a_linha_no_lugar() {
    // O texto preenchido (`medir, fechar`) volta ao motor como está, mais o
    // que se digitou: a TUI não reinterpreta a lista (f9b.1).
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    let painel = le(home.path(), CURRENT);
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "medir, fechar");
    digita(&mut s, ", escrever");
    enter(&mut s);
    assert_eq!(
        le(home.path(), REVISAR),
        REVISAR_MD.replace(
            "bloqueada-por: [medir, fechar]\n",
            "bloqueada-por: [medir, fechar, escrever]\n"
        )
    );
    so_a_task_mudou(home.path(), &antes, REVISAR, &painel);
    barra_de_ok(&tela(&s), "bloqueada-por atualizada");
}

#[test]
fn enter_apara_o_texto_digitado() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "escrever");
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "");
    digita(&mut s, "   fechar   ");
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        com_frontmatter_novo("fechar", ESCREVER_MD)
    );
    barra_de_ok(&tela(&s), "bloqueada-por atualizada");
}

#[test]
fn enter_mantem_a_mesma_task_selecionada_e_o_preview_mostra_a_lista_nova() {
    // `homologar` é a quarta de `alfa`, com o foco nas tasks.
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "homologar");
    selecionada(&tela(&s), "homologar");
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "medir");
    aperta(&mut s, ctrl('u'));
    digita(&mut s, "fechar");
    enter(&mut s);
    assert_eq!(
        le(home.path(), HOMOLOGAR),
        HOMOLOGAR_MD.replace("bloqueada-por: [medir]\n", "bloqueada-por: [fechar]\n")
    );
    let t = tela(&s);
    barra_de_ok(&t, "bloqueada-por atualizada");
    ids_na_lista(&t, &ALFA);
    selecionada(&t, "homologar");
    assert!(
        preview(&t).contains("bloqueada-por: [fechar]"),
        "o preview é relido depois da escrita:\n{}",
        t.texto()
    );
    // A seleção segue de verdade: `j` vai para a seguinte.
    aperta(&mut s, tecla('j'));
    selecionada(&tela(&s), "fechar");
}

#[test]
fn as_letras_digitadas_no_prompt_sao_texto_e_nao_teclas() {
    // `q`, `j`, `k`, `h`, `l`, `b`, `?` e `/` digitados no prompt entram no
    // campo: nem saem, nem navegam, nem abrem outra coisa.
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "escrever");
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "");
    digita(&mut s, "qjkhlb?/");
    let t = tela(&s);
    assert_eq!(prompt_aberto(&t), "qjkhlb?/");
    selecionada(&t, "escrever");
}

// ===========================================================================
// Vazio limpa; igual não grava
// ===========================================================================

#[test]
fn prompt_esvaziado_limpa_a_lista_e_mantem_as_outras_chaves() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    let painel = le(home.path(), CURRENT);
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "medir, fechar");
    aperta(&mut s, ctrl('u'));
    enter(&mut s);
    assert_eq!(
        le(home.path(), REVISAR),
        "---\ntags: [x]\ncriado: 2026-01-01\n---\n\n> [!tip] Revisar o texto\n> [[projects/alfa/tasks/revisar|revisar]]\n"
    );
    so_a_task_mudou(home.path(), &antes, REVISAR, &painel);
    let t = tela(&s);
    barra_de_ok(&t, "bloqueada-por atualizada");
    prompt_fechado(&t);
}

#[test]
fn texto_igual_ao_preenchido_nao_grava_e_fecha_o_prompt() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "medir, fechar");
    enter(&mut s);
    prompt_fechado(&tela(&s));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn prompt_vazio_numa_task_sem_a_chave_nao_grava_nada() {
    // Nem frontmatter novo, nem `bloqueada-por: []`.
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "escrever");
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "");
    enter(&mut s);
    prompt_fechado(&tela(&s));
    nada_mudou(&antes, &fotografa(home.path()));
    assert_eq!(le(home.path(), ESCREVER), ESCREVER_MD);
}

// ===========================================================================
// Recusas do motor: barra vermelha, vault intacto, prompt de volta
// ===========================================================================

/// Abre o `b` na task `id` de `alfa`, troca o campo por `texto`, confirma e
/// afirma a recusa: a barra em vermelho com `mensagem`, nada gravado e o
/// prompt de volta com o que foi digitado.
fn recusa(id: &str, texto: &str, mensagem: &str) {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, id);
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    prompt_aberto(&tela(&s));
    aperta(&mut s, ctrl('u'));
    digita(&mut s, texto);
    enter(&mut s);
    let t = tela(&s);
    barra_de_erro(&t, mensagem);
    assert_eq!(
        prompt_aberto(&t),
        texto,
        "o prompt volta com o que foi digitado:\n{}",
        t.texto()
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn recusa_id_inexistente_na_barra_em_vermelho_sem_gravar() {
    // A lista inteira é recusada, mesmo com `fechar` valendo.
    recusa("escrever", "fechar, zzz", "task não encontrada: zzz");
}

#[test]
fn recusa_id_ambiguo_na_barra_em_vermelho_sem_gravar() {
    // `dup` existe em `delta` e em `gama`.
    recusa(
        "escrever",
        "dup",
        "id \"dup\" é ambíguo (2 arquivos); passe o caminho",
    );
}

#[test]
fn recusa_a_propria_task_na_barra_em_vermelho_sem_gravar() {
    recusa(
        "escrever",
        "escrever",
        "a task não pode ser bloqueada por si mesma: escrever",
    );
}

#[test]
fn recusa_ciclo_com_o_caminho_na_barra_em_vermelho_sem_gravar() {
    // `homologar` espera `medir`; `medir` esperar `homologar` fecha o ciclo.
    recusa(
        "medir",
        "homologar",
        "ciclo de dependência: medir → homologar → medir",
    );
}

#[test]
fn o_prompt_que_volta_da_recusa_serve_para_corrigir_e_gravar() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "escrever");
    aperta(&mut s, tecla('b'));
    prompt_aberto(&tela(&s));
    digita(&mut s, "fechar, zzz");
    enter(&mut s);
    barra_de_erro(&tela(&s), "task não encontrada: zzz");
    // Apaga o `, zzz` e confirma o resto.
    for _ in 0..", zzz".chars().count() {
        aperta(
            &mut s,
            especial(ratatui::crossterm::event::KeyCode::Backspace),
        );
    }
    assert_eq!(prompt_aberto(&tela(&s)), "fechar");
    enter(&mut s);
    assert_eq!(
        le(home.path(), ESCREVER),
        com_frontmatter_novo("fechar", ESCREVER_MD)
    );
    let t = tela(&s);
    barra_de_ok(&t, "bloqueada-por atualizada");
    prompt_fechado(&t);
}

// ===========================================================================
// Esc, C-u, Ctrl-C
// ===========================================================================

#[test]
fn esc_no_prompt_cancela_sem_gravar_e_volta_a_dica() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "medir, fechar");
    aperta(&mut s, ctrl('u'));
    digita(&mut s, "escrever");
    esc(&mut s);
    let t = tela(&s);
    prompt_fechado(&t);
    barra_com_a_dica(&t);
    selecionada(&t, "revisar");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn ctrl_u_limpa_o_campo_do_prompt() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "medir, fechar");
    aperta(&mut s, ctrl('u'));
    assert_eq!(prompt_aberto(&tela(&s)), "", "o C-u esvazia o campo");
    digita(&mut s, "escrever");
    enter(&mut s);
    assert_eq!(
        le(home.path(), REVISAR),
        REVISAR_MD.replace(
            "bloqueada-por: [medir, fechar]\n",
            "bloqueada-por: [escrever]\n"
        )
    );
}

#[test]
fn ctrl_c_no_prompt_sai_sem_gravar() {
    let home = home_deps();
    let mut s = abre(home.path());
    seleciona_em_alfa(&mut s, "revisar");
    let antes = fotografa(home.path());
    aperta(&mut s, tecla('b'));
    prompt_aberto(&tela(&s));
    digita(&mut s, ", escrever");
    assert_eq!(s.tecla(ctrl('c')), Pedido::Sair);
    nada_mudou(&antes, &fotografa(home.path()));
}
