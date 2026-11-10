//! A tela (spec §3.5, camada 3, só as asserções semânticas: o snapshot é do
//! implementador). Em 100×30 e 60×15: os três painéis, a linha selecionada em
//! destaque e as outras não, o título que diz `+arquivadas`, a data de volta
//! na linha da adiada, a ajuda inteira, o vault vazio e o terminal estreito.

use crate::util::*;

/// As linhas da ajuda de hoje (`HELP` em `tui/src/ui.rs`): tecla e o que ela
/// faz.
const AJUDA: [(&str, &str); 19] = [
    ("j/k ↑/↓", "mover"),
    ("h/l ←/→ Tab", "trocar de painel"),
    ("g/G", "topo / fim"),
    ("Enter / e", "abrir a task no editor"),
    ("C-d / C-u", "rolar o preview"),
    ("/", "filtrar tasks (id e título)"),
    ("n", "nova task no projeto"),
    ("s", "trocar a fase"),
    ("t", "editar o título"),
    ("i", "editar a descrição"),
    ("r", "renomear o id (arquivo e links)"),
    ("a", "arquivar / desarquivar"),
    ("A", "mostrar / esconder arquivadas"),
    ("S", "mostrar / esconder Algum dia e adiadas"),
    ("d", "largar a task (arquiva como dropped)"),
    ("N", "novo projeto"),
    ("c", "abrir o CURRENT.md"),
    ("R", "reler o vault"),
    ("q / C-c", "sair"),
];

/// Afirma que os três painéis estão na linha de cima, da esquerda para a
/// direita: projetos, tasks, preview.
fn tres_paineis(t: &Tela, tasks: &str) {
    let (xp, yp) = t
        .acha("Projetos")
        .unwrap_or_else(|| panic!("sem o painel de projetos:\n{}", t.texto()));
    let (xt, yt) = t
        .acha(tasks)
        .unwrap_or_else(|| panic!("sem o painel {tasks:?}:\n{}", t.texto()));
    let (xv, yv) = t
        .acha("Preview")
        .unwrap_or_else(|| panic!("sem o painel do preview:\n{}", t.texto()));
    assert_eq!(
        (yp, yt, yv),
        (0, 0, 0),
        "os títulos ficam na moldura de cima"
    );
    assert!(
        xp < xt && xt < xv,
        "projetos | tasks | preview:\n{}",
        t.texto()
    );
}

#[test]
fn tela_100x30_tem_os_tres_paineis_a_lista_a_selecao_em_destaque_e_a_dica() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    tres_paineis(&t, "Tasks: alfa");

    let projetos = t.itens_do(t.projetos());
    assert_eq!(projetos.len(), 2, "{projetos:#?}");
    assert!(projetos[0].contains("alfa 3"), "{projetos:#?}");
    assert!(projetos[1].contains("beta 2"), "{projetos:#?}");
    projeto_selecionado(&t, "alfa");

    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
    selecionada(&t, "medir-tempo");

    let p = preview(&t);
    for esperado in ["Medir o tempo", "Em implementação", "medir-tempo"] {
        assert!(
            p.contains(esperado),
            "o preview não traz {esperado:?}:\n{p}"
        );
    }
    barra_com_a_dica(&t);
}

#[test]
fn tela_60x15_tem_os_tres_paineis_e_a_selecao_em_destaque() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('G'));
    let t = tela_de(&s, 60, 15);
    tres_paineis(&t, "Tasks");
    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
    selecionada(&t, "fechar-ciclo");
    projeto_selecionado(&t, "alfa");
    assert!(
        t.barra().starts_with("? ajuda"),
        "a dica começa na barra mesmo estreita: {:?}",
        t.barra()
    );
}

#[test]
fn o_preview_mostra_a_fase_a_descricao_e_o_arquivo_da_task() {
    let home = home_base();
    let s = abre(home.path());
    let t = tela(&s);
    let p = preview(&t);
    for esperado in [
        "Escrever a spec",
        "Não iniciada",
        "escrever-spec",
        "em curso",
        "nota um",
    ] {
        assert!(
            p.contains(esperado),
            "o preview não traz {esperado:?}:\n{p}"
        );
    }
}

#[test]
fn a_ajuda_cabe_inteira_em_100x30() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('?'));
    let t = tela(&s);
    let ajuda = t.painel("Teclas");
    let linhas = t.linhas_do(ajuda);
    for (k, d) in AJUDA {
        let linha = linhas
            .iter()
            .find(|l| l.contains(d) && l.contains(k) && l.find(k) < l.find(d))
            .unwrap_or_else(|| {
                panic!(
                    "a linha da ajuda {k:?} → {d:?} não está inteira:\n{}",
                    t.texto()
                )
            });
        assert!(!linha.is_empty());
    }
}

#[test]
fn com_a_maiuscula_o_titulo_diz_mais_arquivadas_e_a_arquivada_vem_com_o_tipo() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    assert!(!tela(&s).titulo_das_tasks().contains("+arquivadas"));
    aperta(&mut s, tecla('A'));
    let t = tela(&s);
    assert!(
        t.titulo_das_tasks().contains("+arquivadas"),
        "{}",
        t.titulo_das_tasks()
    );
    ids_na_lista(
        &t,
        &[
            "escrever-spec",
            "medir-tempo",
            "fechar-ciclo",
            "velha-feita",
        ],
    );
    let velha = itens_das_tasks(&t)
        .into_iter()
        .find(|l| l.contains("velha-feita"))
        .expect("velha-feita");
    assert!(
        velha.find("[done]") < velha.find("velha-feita") && velha.contains("[done]"),
        "a arquivada vem prefixada com o tipo: {velha:?}"
    );
    aperta(&mut s, tecla('A'));
    let t = tela(&s);
    assert!(!t.titulo_das_tasks().contains("+arquivadas"));
    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
}

#[test]
fn com_s_maiusculo_aparecem_algum_dia_e_a_adiada_com_a_data_de_volta_na_linha_dela() {
    // Em 160 colunas, para a linha da adiada caber inteira no painel do meio.
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('l'));
    aperta(&mut s, tecla('S'));
    let t = tela_de(&s, 160, 30);
    assert!(
        t.titulo_das_tasks().contains("+algum-dia/adiadas"),
        "{}",
        t.titulo_das_tasks()
    );
    ids_na_lista(
        &t,
        &[
            "escrever-spec",
            "adiada-dezembro",
            "medir-tempo",
            "fechar-ciclo",
            "sonho-distante",
        ],
    );
    for item in itens_das_tasks(&t) {
        if item.contains("adiada-dezembro") {
            assert!(
                item.contains("volta 2026-12-01"),
                "a data de volta fica na linha da adiada: {item:?}"
            );
        } else {
            assert!(!item.contains("volta"), "só a adiada ganha data: {item:?}");
        }
    }
    aperta(&mut s, tecla('j'));
    let t = tela_de(&s, 160, 30);
    selecionada(&t, "adiada-dezembro");
    assert!(
        preview(&t).contains("volta em 2026-12-01"),
        "o preview da adiada diz quando ela volta:\n{}",
        t.texto()
    );
    aperta(&mut s, tecla('S'));
    let t = tela(&s);
    assert!(!t.titulo_das_tasks().contains("+algum-dia"));
    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
}

#[test]
fn prioridade_alta_vem_primeiro_com_a_marca() {
    let home = home_base();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('j'));
    let t = tela(&s);
    ids_na_lista(&t, &["beta-urgente", "beta-um"]);
    let itens = itens_das_tasks(&t);
    assert!(
        itens[0].starts_with('!'),
        "a de prioridade alta tem o `!` na frente: {:?}",
        itens[0]
    );
    assert!(!itens[1].contains('!'), "a normal não: {:?}", itens[1]);
}

#[test]
fn vault_vazio_abre_com_os_tres_paineis_e_sem_task() {
    let home = tempfile::tempdir().expect("tempdir");
    pasta(home.path(), "");
    let s = abre(home.path());
    let t = tela(&s);
    tres_paineis(&t, "Tasks");
    assert_eq!(t.itens_do(t.projetos()), Vec::<String>::new());
    ids_na_lista(&t, &[]);
    assert!(
        preview(&t).contains("sem task selecionada"),
        "{}",
        t.texto()
    );
    barra_com_a_dica(&t);
}

#[test]
fn terminal_estreito_nao_entra_em_panico_em_nenhum_modo() {
    let tamanhos = [
        (20, 5),
        (21, 6),
        (24, 7),
        (30, 8),
        (40, 10),
        (60, 15),
        (80, 24),
    ];
    for abre_modo in [
        "", "l", "?", "nÁvida", "ls", "la", "ld", "lt", "li", "lr", "N", "/x", "lAG", "lS",
    ] {
        let home = home_base();
        let mut s = abre(home.path());
        aperta_todas(&mut s, abre_modo);
        for (w, h) in tamanhos {
            let _ = tela_de(&s, w, h);
        }
    }
}
