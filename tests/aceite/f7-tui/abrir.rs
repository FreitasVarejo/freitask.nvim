//! Abrir a TUI (`Sessao::abrir`): o `rebuild` antes da leitura, com o dia
//! dado em `hoje`, e a falha do `rebuild` que não impede a TUI de abrir
//! (`docs/freitask.md`, "Uso interativo (TUI)"; `tui/src/engine.rs`,
//! `Engine::open`).

use crate::util::*;
use freitask::tui::Sessao;

#[test]
fn abrir_regenera_o_current_md_no_dia_dado_sem_tocar_em_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    let _s = abre(home.path());
    assert_eq!(le(home.path(), CURRENT), PAINEL_BASE);
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[CURRENT]),
        "abrir só grava o painel"
    );
}

#[test]
fn abrir_num_dia_novo_guarda_o_painel_velho_em_daily_e_preserva_as_notas_avulsas() {
    // Medido no oráculo: o `rebuild` de um dia novo copia o painel velho para
    // `daily/<dia velho>.md` e leva a seção `## Notas Avulsas` para o novo.
    let home = home_base();
    let velho = concat!(
        "---\n",
        "date: 2026-11-09\n",
        "weekday: Monday\n",
        "---\n",
        "\n",
        "# Planning diário 09/11/26\n",
        "\n",
        "> Painel gerado automaticamente — 0 tasks em 0 projetos.\n",
        "\n",
        "## Notas Avulsas\n",
        "\n",
        "- comprar pão\n",
        "\n",
    );
    arquivo(home.path(), CURRENT, velho);
    let _s = abre(home.path());
    assert_eq!(le(home.path(), "daily/2026-11-09.md"), velho);
    assert_eq!(
        le(home.path(), CURRENT),
        PAINEL_BASE.replacen(
            "## Notas Avulsas\n\n- \n",
            "## Notas Avulsas\n\n- comprar pão\n",
            1
        )
    );
}

#[test]
fn abrir_noutro_dia_le_a_volta_em_contra_esse_dia() {
    // Em 2026-12-01 a `adiada-dezembro` já voltou: aparece sem `S`, sem a
    // data ao lado, e entra no painel. O `hoje` vale para a leitura e para o
    // `rebuild` (é o papel do `FREITASK_TODAY`).
    let home = home_base();
    let mut s = abre_no_dia(home.path(), "2026-12-01");
    aperta(&mut s, tecla('l'));
    let t = tela_de(&s, 160, 30);
    ids_na_lista(
        &t,
        &[
            "escrever-spec",
            "adiada-dezembro",
            "medir-tempo",
            "fechar-ciclo",
        ],
    );
    for item in itens_das_tasks(&t) {
        assert!(
            !item.contains("volta"),
            "nenhuma task está adiada em 2026-12-01: {item:?}"
        );
    }
    assert!(
        !t.titulo_das_tasks().contains("+algum-dia"),
        "o toggle do S continua desligado: {}",
        t.titulo_das_tasks()
    );
    let painel = concat!(
        "---\n",
        "date: 2026-12-01\n",
        "weekday: Tuesday\n",
        "---\n",
        "\n",
        "# Planning diário 01/12/26\n",
        "\n",
        "> Painel gerado automaticamente — 6 tasks em 2 projetos.\n",
        "\n",
        "## Notas Avulsas\n",
        "\n",
        "- \n",
        "\n",
        "## alfa\n",
        "\n",
        "> [!todo] Escrever a spec\n",
        "> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n",
        "> _em curso_\n",
        "> nota um\n",
        "\n",
        "> [!tip] Adiada até dezembro\n",
        "> [[projects/alfa/tasks/adiada-dezembro|adiada-dezembro]]\n",
        "\n",
        "> [!example] Medir o tempo\n",
        "> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\n",
        "\n",
        "> [!check] Fechar o ciclo\n",
        "> [[projects/alfa/tasks/fechar-ciclo|fechar-ciclo]]\n",
        "\n",
        "## beta\n",
        "\n",
        "> [!example] Bê urgente\n",
        "> [[projects/beta/tasks/beta-urgente|beta-urgente]]\n",
        "\n",
        "> [!todo] Bê um\n",
        "> [[projects/beta/tasks/beta-um|beta-um]]\n",
        "\n",
    );
    assert_eq!(le(home.path(), CURRENT), painel);
}

#[test]
fn abrir_com_rebuild_que_falha_abre_mesmo_assim_com_o_motivo_na_barra_em_vermelho() {
    // Um `CURRENT.md` que é pasta: o `rebuild` não consegue gravá-lo. A TUI
    // abre, lê as tasks e põe "rebuild falhou: <motivo>" na barra, como erro.
    // O texto do motivo é do motor e não se fixa aqui.
    let home = home_base();
    pasta(home.path(), CURRENT);
    let antes = fotografa(home.path());
    let s = Sessao::abrir(home.path(), HOJE)
        .unwrap_or_else(|e| panic!("a TUI tinha de abrir mesmo com o rebuild falhando: {e}"));
    let t = tela_de(&s, 300, 30);
    let barra = t.barra();
    assert!(
        barra.starts_with("rebuild falhou: ") && barra.len() > "rebuild falhou: ".len(),
        "barra: {barra:?}"
    );
    assert_eq!(t.cor_da_barra(), ratatui::style::Color::Red);
    ids_na_lista(&t, &["escrever-spec", "medir-tempo", "fechar-ciclo"]);
    nada_mudou(&antes, &fotografa(home.path()));
}
