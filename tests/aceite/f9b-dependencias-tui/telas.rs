//! A marca `⛔ ` (spec §10.1, "Task bloqueada não sai do painel nem da TUI;
//! ganha marca"; §3.5, "a marca de bloqueada […] na linha certa"; f9b.2) e a
//! linha do `b` na ajuda (f9b.1). Em 100×30 e 60×15. Todo teste que afirma a
//! ausência da marca numa linha afirma antes a presença dela noutra da mesma
//! tela, para não passar sem a marca existir.

use crate::util::*;

/// Os ícones das fases no `status.json` default (`gerador::STATUS_JSON`).
const ICONE_TIP: &str = "󰌶";
const ICONE_QUESTION: &str = "󰘥";
const ICONE_DONE: &str = "󰄬";

/// A tela nos dois tamanhos da §3.5.
fn nos_dois_tamanhos(s: &freitask::tui::Sessao) -> Vec<Tela> {
    TAMANHOS.iter().map(|&(w, h)| tela_de(s, w, h)).collect()
}

/// Afirma, numa tela, quem tem a marca e quem não tem, e que a moldura do
/// painel das tasks ficou inteira.
fn marcas(t: &Tela, com: &[&str], sem: &[&str]) {
    for id in com {
        com_a_marca(t, id);
    }
    for id in sem {
        sem_a_marca(t, id);
    }
    moldura_inteira(t, t.tasks());
}

/// A posição (em bytes) de `agulha` no item, com pânico se não está.
fn pos(item: &str, agulha: &str) -> usize {
    item.find(agulha)
        .unwrap_or_else(|| panic!("{agulha:?} não está em {item:?}"))
}

// ===========================================================================
// Quando a marca aparece (§10.1, a semântica)
// ===========================================================================

#[test]
fn marca_colada_antes_do_id_da_task_com_bloqueadora_ativa() {
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, 0);
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &ALFA);
        marcas(
            &t,
            &["revisar", "homologar"],
            &["escrever", "medir", "fechar"],
        );
    }
    // Em `beta`, `quer-livre` espera `livre`, que não espera ninguém.
    aperta(&mut s, tecla('h'));
    aperta(&mut s, tecla('j'));
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &BETA);
        marcas(&t, &["quer-livre"], &["livre"]);
    }
}

#[test]
fn sem_marca_quando_a_unica_bloqueadora_esta_arquivada() {
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 1, 0);
    for t in nos_dois_tamanhos(&s) {
        marcas(&t, &["quer-livre"], &["quer-velha"]);
    }
}

#[test]
fn sem_marca_quando_a_bloqueadora_nao_existe() {
    // Um typo não pode esconder a task da frente (§10.1): ela segue na lista,
    // sem marca.
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 1, 0);
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &BETA);
        marcas(&t, &["quer-livre"], &["quer-sumida"]);
    }
}

#[test]
fn marca_quando_a_bloqueadora_esta_escondida_em_algum_dia_ou_adiada() {
    // `sonho` (Algum dia) e `adiada` (volta em 2026-12-01) não aparecem sem
    // `S`, e bloqueiam normalmente.
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 1, 0);
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &BETA);
        marcas(&t, &["quer-sonho", "quer-adiada"], &[]);
    }
    // Com `S`, as escondidas aparecem: a `adiada` espera `livre` e leva a
    // marca; o `sonho` não espera ninguém.
    aperta(&mut s, tecla('S'));
    for t in nos_dois_tamanhos(&s) {
        let mut ids: Vec<&str> = BETA.to_vec();
        ids.extend(["adiada", "sonho"]);
        ids_na_lista(&t, &ids);
        marcas(
            &t,
            &["quer-livre", "quer-sonho", "quer-adiada", "adiada"],
            &["quer-velha", "quer-sumida", "livre", "sonho"],
        );
    }
}

#[test]
fn a_bloqueada_selecionada_tem_a_marca_na_linha_em_destaque() {
    // A marca não esconde nem reordena (§10.1): a ordem é a das fases, e a
    // linha em destaque da bloqueada traz a marca.
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, 1);
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &ALFA);
        selecionada(&t, "revisar");
        com_a_marca(&t, "revisar");
        assert!(
            t.destacada(t.tasks()).contains(&format!("{MARCA} revisar")),
            "a linha em destaque é a da bloqueada, com a marca:\n{}",
            t.texto()
        );
    }
}

// ===========================================================================
// Onde a marca fica na linha (f9b.2)
// ===========================================================================

#[test]
fn a_marca_vem_depois_do_icone_da_fase() {
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, 0);
    for t in nos_dois_tamanhos(&s) {
        for (id, icone) in [("revisar", ICONE_TIP), ("homologar", ICONE_QUESTION)] {
            com_a_marca(&t, id);
            let item = item_de(&t, id);
            assert!(
                pos(&item, icone) < pos(&item, MARCA),
                "o ícone da fase vem antes da marca: {item:?}"
            );
        }
    }
}

#[test]
fn a_arquivada_com_bloqueadora_ativa_leva_a_marca_depois_do_tipo_e_do_icone() {
    // `velha` está arquivada e espera `medir`, que está ativa: o `bloqueada`
    // do JSON dela é verdadeiro, e a TUI lê o mesmo campo (f9b.2).
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, 0);
    aperta(&mut s, tecla('A'));
    for t in nos_dois_tamanhos(&s) {
        let mut ids: Vec<&str> = ALFA.to_vec();
        ids.push("velha");
        ids_na_lista(&t, &ids);
        marcas(&t, &["revisar", "homologar", "velha"], &["medir"]);
        let item = item_de(&t, "velha");
        assert!(
            pos(&item, "[done] ") < pos(&item, MARCA),
            "o `[tipo] ` da arquivada vem antes da marca: {item:?}"
        );
        assert!(
            pos(&item, ICONE_DONE) < pos(&item, MARCA),
            "o ícone da fase vem antes da marca: {item:?}"
        );
    }
}

#[test]
fn a_prioridade_alta_bloqueada_tem_o_exclamacao_na_frente_e_a_marca_antes_do_id() {
    // `urgente` (delta) espera `livre`, de outro projeto.
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 2, 0);
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &["urgente", "dup"]);
        marcas(&t, &["urgente"], &["dup"]);
        let item = item_de(&t, "urgente");
        assert!(
            item.starts_with('!'),
            "a coluna `! ` da prioridade continua na frente: {item:?}"
        );
    }
}

#[test]
fn a_adiada_bloqueada_traz_a_marca_antes_do_id_e_a_volta_depois() {
    // Em 160 colunas, como na fatia 7: em 100 o painel do meio corta a linha
    // da adiada antes da data.
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 1, 0);
    aperta(&mut s, tecla('S'));
    let t = tela_de(&s, 160, 30);
    marcas(&t, &["adiada"], &["sonho"]);
    let item = item_de(&t, "adiada");
    let id = posicao_do_id(&item, "adiada").expect("id");
    assert!(
        id < pos(&item, "volta 2026-12-01"),
        "a data de volta fica depois do id: {item:?}"
    );
}

#[test]
fn a_marca_fica_fora_do_preview_dos_projetos_e_do_current_md() {
    // f9b.2: nada muda no preview nem no `CURRENT.md`.
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, 1);
    for t in nos_dois_tamanhos(&s) {
        com_a_marca(&t, "revisar");
        assert!(
            !preview(&t).contains(MARCA),
            "o preview da bloqueada não ganha a marca:\n{}",
            t.texto()
        );
        assert!(
            !t.texto_do(t.projetos()).contains(MARCA),
            "o painel dos projetos não ganha a marca:\n{}",
            t.texto()
        );
    }
    assert!(
        !le(home.path(), CURRENT).contains(MARCA),
        "o CURRENT.md não ganha a marca"
    );
}

// ===========================================================================
// A marca acompanha o `b`
// ===========================================================================

#[test]
fn a_marca_aparece_logo_depois_de_gravar_pelo_b_e_some_depois_de_limpar() {
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, 0);
    // `fechar` está ativa e não espera ninguém.
    aperta(&mut s, tecla('b'));
    prompt_aberto(&tela(&s));
    digita(&mut s, "fechar");
    enter(&mut s);
    for t in nos_dois_tamanhos(&s) {
        marcas(&t, &["escrever"], &["fechar"]);
        selecionada(&t, "escrever");
    }

    aperta(&mut s, tecla('b'));
    assert_eq!(prompt_aberto(&tela(&s)), "fechar");
    aperta(&mut s, ctrl('u'));
    enter(&mut s);
    for t in nos_dois_tamanhos(&s) {
        marcas(&t, &["revisar"], &["escrever"]);
    }
    // O que sobra do frontmatter que só tinha a lista o aceite da 9a deixou
    // solto: aqui, só que a lista saiu e o resto do arquivo ficou.
    let texto = le(home.path(), ESCREVER);
    assert!(!texto.contains("bloqueada-por"), "{texto}");
    assert!(texto.ends_with(ESCREVER_MD), "{texto}");
}

#[test]
fn a_marca_some_quando_a_unica_bloqueadora_e_arquivada_pela_tui() {
    // `homologar` só espera `medir`; largar `medir` (`d`, `y`) a arquiva em
    // `dropped`, e bloqueadora arquivada está resolvida (§10.1).
    let home = home_deps();
    let mut s = abre(home.path());
    vai_para(&mut s, 0, em_alfa("medir"));
    let t = tela(&s);
    selecionada(&t, "medir");
    com_a_marca(&t, "homologar");
    aperta(&mut s, tecla('d'));
    aperta(&mut s, tecla('y'));
    for t in nos_dois_tamanhos(&s) {
        ids_na_lista(&t, &["escrever", "revisar", "homologar", "fechar"]);
        // `revisar` ainda espera `fechar`, ativa.
        marcas(&t, &["revisar"], &["homologar"]);
    }
}

// ===========================================================================
// A ajuda
// ===========================================================================

/// As linhas da ajuda que a fatia 7 fixou (tecla e o que ela faz).
const AJUDA_DA_7: [(&str, &str); 19] = [
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

/// A linha do `b` (f9b.1).
const AJUDA_DO_B: &str = "editar a lista bloqueada-por";

#[test]
fn a_ajuda_traz_a_linha_do_b_logo_depois_da_do_r_e_cabe_em_100x30() {
    let home = home_deps();
    let mut s = abre(home.path());
    aperta(&mut s, tecla('?'));
    let t = tela(&s);
    let ajuda = t.painel("Teclas");
    let linhas = t.linhas_do(ajuda);

    let i_b = linhas
        .iter()
        .position(|l| l.contains(AJUDA_DO_B))
        .unwrap_or_else(|| panic!("a ajuda não traz {AJUDA_DO_B:?} inteira:\n{}", t.texto()));
    let linha_b = &linhas[i_b];
    let rotulo = &linha_b[..linha_b.find(AJUDA_DO_B).expect("descrição")];
    assert_eq!(
        rotulo.trim(),
        "b",
        "o rótulo da linha é a tecla `b`: {linha_b:?}"
    );
    let i_r = linhas
        .iter()
        .position(|l| l.contains("renomear o id (arquivo e links)"))
        .unwrap_or_else(|| panic!("sem a linha do `r`:\n{}", t.texto()));
    assert_eq!(
        i_b,
        i_r + 1,
        "a linha do `b` vem logo depois da do `r`:\n{}",
        t.texto()
    );

    // E o resto continua inteiro em 100×30.
    for (k, d) in AJUDA_DA_7 {
        assert!(
            linhas
                .iter()
                .any(|l| l.contains(d) && l.contains(k) && l.find(k) < l.find(d)),
            "a linha da ajuda {k:?} → {d:?} não está inteira:\n{}",
            t.texto()
        );
    }
}
