//! O prazo na tela (spec §10.2, "Tela"; decisões f10b.2 e f10b.4; §3.5 só
//! nas asserções semânticas: o snapshot é do implementador). A linha da task
//! traz `prazo DD/MM` depois do id e da `volta`, antes do título, em amarelo
//! até o prazo e em vermelho depois; a arquivada e o prazo inválido não o
//! mostram. A ajuda traz a tecla `p`. Nenhum tamanho de 20×5 a 80×24 entra
//! em pânico com o prompt aberto ou com prazo na lista.
//!
//! As linhas se leem em `LARGA` (160 colunas), como a `volta` na fatia 7.
//! Os testes de ausência leem, na mesma tela, uma linha que tem de mostrar o
//! prazo: sem ela, passariam numa TUI que não mostra prazo nenhum.

use crate::util::*;
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;

/// Afirma que todas as células não brancas de `trecho` na lista de tasks
/// têm a cor `cor`.
fn trecho_na_cor(t: &Tela, trecho: &str, cor: Color) {
    let cores = t.cores_de(t.tasks(), trecho);
    assert!(
        !cores.is_empty() && cores.iter().all(|c| *c == cor),
        "{trecho:?} devia estar todo em {cor:?}, está em {cores:?}; tela:\n{}",
        t.texto()
    );
}

// ===========================================================================
// A linha da task
// ===========================================================================

#[test]
fn prazo_vem_depois_do_id_e_antes_do_titulo_como_dia_e_mes() {
    let home = home_prazo();
    let s = abre(home.path());
    let t = tela_larga(&s);
    for (id, prazo, titulo) in [
        ("futura", "prazo 01/12", "Relatório anual"),
        ("hoje", "prazo 10/11", "Fechar contas"),
        ("ontem", "prazo 09/11", "Pagar boleto"),
        ("vencida", "prazo 15/10", "Mandar convite"),
        ("outro-ano", "prazo 05/01", "Virada"),
    ] {
        let item = item_de(&t, id);
        em_ordem(&item, &[id, prazo, titulo]);
        assert_eq!(
            item.matches("prazo").count(),
            1,
            "um prazo só na linha: {item:?}"
        );
    }
}

#[test]
fn prazo_de_outro_ano_nao_mostra_o_ano() {
    // f10b.2: §10.2 fixa `DD/MM`, nem num prazo de outro ano.
    let home = home_prazo();
    let s = abre(home.path());
    let t = tela_larga(&s);
    let item = item_de(&t, "outro-ano");
    assert!(item.contains("prazo 05/01"), "{item:?}");
    assert!(!item.contains("2027"), "{item:?}");
    assert!(!item.contains("05/01/"), "{item:?}");
}

#[test]
fn prazo_de_hoje_e_futuro_sai_em_amarelo_e_vencido_em_vermelho() {
    // f10b.2: amarelo enquanto faltam zero ou mais dias (o próprio dia
    // inclusive); vermelho de um dia vencido em diante.
    let home = home_prazo();
    let s = abre(home.path());
    let t = tela_larga(&s);
    trecho_na_cor(&t, "prazo 01/12", Color::Yellow);
    trecho_na_cor(&t, "prazo 10/11", Color::Yellow);
    trecho_na_cor(&t, "prazo 05/01", Color::Yellow);
    trecho_na_cor(&t, "prazo 09/11", Color::Red);
    trecho_na_cor(&t, "prazo 15/10", Color::Red);
}

#[test]
fn a_cor_do_prazo_conta_os_dias_a_partir_do_dia_da_sessao() {
    // O mesmo vault aberto em outros dias: `futura` (01/12) vence, `vencida`
    // (15/10) ainda não venceu.
    let home = home_prazo();
    let s = abre_no_dia(home.path(), "2026-12-02");
    let t = tela_larga(&s);
    trecho_na_cor(&t, "prazo 01/12", Color::Red);
    trecho_na_cor(&t, "prazo 05/01", Color::Yellow);

    let s = abre_no_dia(home.path(), "2026-10-01");
    let t = tela_larga(&s);
    trecho_na_cor(&t, "prazo 15/10", Color::Yellow);
    trecho_na_cor(&t, "prazo 09/11", Color::Yellow);
}

#[test]
fn adiada_com_prazo_mostra_a_volta_e_depois_o_prazo() {
    // Com `S`: a adiada (`volta-em: 2026-12-01`, `prazo: 2026-12-15`) e a de
    // Algum dia (prazo vencido) aparecem, cada uma com o seu prazo.
    let home = home_prazo();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('S'));
    let t = tela_larga(&s);
    let item = item_de(&t, "adiada");
    em_ordem(
        &item,
        &["adiada", "volta 2026-12-01", "prazo 15/12", "Retorno"],
    );
    trecho_na_cor(&t, "prazo 15/12", Color::Yellow);
    let item = item_de(&t, "algum");
    em_ordem(&item, &["algum", "prazo 01/11", "Talvez"]);
    trecho_na_cor(&t, "prazo 01/11", Color::Red);
}

#[test]
fn sem_prazo_com_prazo_vazio_ou_invalido_a_linha_nao_traz_prazo() {
    let home = home_prazo();
    let s = abre(home.path());
    let t = tela_larga(&s);
    assert!(
        item_de(&t, "futura").contains("prazo 01/12"),
        "a de prazo válido mostra: {}",
        t.texto()
    );
    for id in ["livre", "vazio", "invalida"] {
        let item = item_de(&t, id);
        assert!(!item.contains("prazo"), "{id}: {item:?}");
        assert!(!item.contains("30/02"), "{id}: {item:?}");
    }
}

#[test]
fn arquivada_com_prazo_nao_mostra_o_prazo() {
    // f10b.2: como não mostra a `volta`; com `A`, `feita` (prazo vencido)
    // aparece sem ele.
    let home = home_prazo();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('A'));
    let t = tela_larga(&s);
    assert!(
        t.titulo_das_tasks().contains("+arquivadas"),
        "{}",
        t.titulo_das_tasks()
    );
    assert!(
        item_de(&t, "futura").contains("prazo 01/12"),
        "a ativa segue mostrando: {}",
        t.texto()
    );
    let item = item_de(&t, "feita");
    assert!(item.contains("[done]"), "{item:?}");
    assert!(!item.contains("prazo"), "{item:?}");
    assert!(!item.contains("01/11"), "{item:?}");
}

// ===========================================================================
// A ajuda
// ===========================================================================

#[test]
fn a_ajuda_tem_a_linha_do_p_depois_da_do_i() {
    // f10b.4. A ajuda inteira e a quantidade de linhas dela não se fixam (a
    // fatia 9b acrescenta a tecla `b` em paralelo).
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('?'));
    let t = tela(&s);
    let linhas: Vec<String> = t
        .linhas_do(t.painel("Teclas"))
        .into_iter()
        .map(|l| l.trim().to_owned())
        .collect();
    let comeca_com = |tecla: &str, desc: &str| {
        linhas.iter().position(|l| {
            l.strip_prefix(tecla)
                .is_some_and(|resto| resto.starts_with(char::is_whitespace))
                && l.contains(desc)
        })
    };
    let p = comeca_com("p", "definir / limpar o prazo").unwrap_or_else(|| {
        panic!(
            "a ajuda não tem a linha `p` → \"definir / limpar o prazo\":\n{}",
            t.texto()
        )
    });
    let i = comeca_com("i", "editar a descrição")
        .unwrap_or_else(|| panic!("a ajuda perdeu a linha do `i`:\n{}", t.texto()));
    assert!(i < p, "a linha do `p` vem depois da do `i`:\n{}", t.texto());
}

// ===========================================================================
// Terminal estreito
// ===========================================================================

#[test]
fn terminal_estreito_nao_entra_em_panico_com_o_prompt_do_p_nem_com_prazo_na_lista() {
    let tamanhos = [
        (20, 5),
        (21, 6),
        (24, 7),
        (30, 8),
        (40, 10),
        (60, 15),
        (80, 24),
    ];
    // Este teste só afirma que nada entra em pânico: o que cada tecla pede ao
    // laço não importa aqui (é dos testes de `teclas`).
    let leva = |s: &mut freitask::tui::Sessao, teclas: &str, recusa: bool| {
        for c in teclas.chars() {
            let _ = s.tecla(tecla(c));
        }
        if recusa {
            let _ = s.tecla(especial(KeyCode::Enter));
        }
    };
    // O prompt do `p`: vazio, com texto, e de volta depois de uma recusa (a
    // barra com a mensagem longa, em vermelho).
    for (teclas, recusa) in [("p", false), ("lp2026-12", false), ("lpquando", true)] {
        let home = home_base();
        let mut s = abre(home.path());
        leva(&mut s, teclas, recusa);
        for (w, h) in tamanhos {
            let _ = tela_de(&s, w, h);
        }
    }
    // A lista com prazo (vencido, de hoje, futuro, inválido, na adiada e na
    // arquivada), em cada modo, e o prompt preenchido com um prazo escrito.
    for teclas in ["", "l", "lS", "lA", "lSA", "lj", "ljp"] {
        let home = home_prazo();
        let mut s = abre(home.path());
        leva(&mut s, teclas, false);
        for (w, h) in tamanhos {
            let _ = tela_de(&s, w, h);
        }
    }
}
