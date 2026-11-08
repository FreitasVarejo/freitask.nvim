//! Aceite da fatia 3 — Escrita (spec §5): gravação com CAS, temporário,
//! `fsync` e lock vistos de fora; `new` (sem `--json`), `new-project`, `set`
//! com as flags de hoje (`--status`, `--title`, `--desc`, `--volta-em`) e
//! `delete`, e o efeito de cada um sobre o `CURRENT.md`.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§1.5, §1.6, §2.1, §2.4, §3.2 I6/I9, §3.3, §3.4 D2/D4/D6, §3.7) e
//! `docs/freitask.md` ("Formato do bloco", "Volta em", "Status 0",
//! "Arquivar / desarquivar / deletar", "Encerrar / reabrir", "CLI", "Para
//! agentes de IA"). O texto exato de stdout/stderr e dos arquivos gravados foi
//! medido no oráculo Lua (`lua/freitask/cli.lua`) num HOME temporário; onde a
//! spec manda divergir (D2, D4, D6, saída 3, lock), vale a spec. Nenhum valor
//! esperado aqui foi gerado do binário sob teste.
//!
//! Decisões tomadas aqui, onde a spec e o oráculo não fecham (repetidas no
//! relatório do rascunho):
//! - D2: a mensagem de `.` e `..` é a de reservado que já existe,
//!   `"<nome>" é um diretório reservado, não um projeto`, inclusive no
//!   `new-project`, em que o oráculo já recusava com `nome de projeto
//!   inválido: .` (o `kebab` esvazia o nome);
//! - saída 3 por lock: o texto não está na spec; afirma-se só stdout vazio,
//!   stderr de uma linha com o prefixo `freitask: ` e nada gravado;
//! - D4: linha que o comando acrescenta num arquivo CRLF sai em CRLF (inclusive
//!   o frontmatter criado pelo `--volta-em`); BOM antes do `---` não impede o
//!   frontmatter de ser reconhecido; a linha acrescentada depois de uma última
//!   linha sem `\n` fica, ela mesma, sem `\n`;
//! - "nada gravado" é: nenhum arquivo do vault mudou de bytes nem de mtime e
//!   nenhuma entrada apareceu ou sumiu (o mtime de pasta não é afirmado);
//! - `new-project` não regenera o `CURRENT.md`, como hoje;
//! - recusas são exercitadas num vault que já tem `projects/` e
//!   `.freitask/status.json`, para que "nada gravado" não dependa do que um
//!   comando recusado faz com a raiz que falta.

mod util;

use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use util::*;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// `x`: frontmatter com chaves alheias (inclusive uma lista em várias linhas),
/// bloco com descrição e nota, corpo com checklist e `## Histórico`.
const X_MD: &str = concat!(
    "---\n",
    "aliases: []\n",
    "id: x\n",
    "tags:\n",
    "  - foo\n",
    "criado: 2026-01-01\n",
    "---\n",
    "\n",
    "> [!todo] Xis\n",
    "> [[projects/alfa/tasks/x|x]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "\n",
    "## Notas Soltas\n",
    "\n",
    "- [ ] item\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-01-01 — desarquivada, de volta ao board\n",
);

/// `y`: só o bloco, sem descrição, sem frontmatter.
const Y_MD: &str = "> [!example] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n";

const VELHA_MD: &str = "> [!done] Velha\n> [[projects/alfa/tasks/archived/done/velha|velha]]\n";

const LARGADA_MD: &str =
    "> [!warning] Largada\n> [[projects/alfa/tasks/archived/dropped/largada|largada]]\n";

const Z_MD: &str = "> [!todo] Zeta\n> [[projects/_encerrados/omega/tasks/z|z]]\n";

/// Uma nota que cita `y` por caminho e por id.
const NOTAS_MD: &str = "# Notas\n\nver [[projects/alfa/tasks/y|y]] e [[y]]\n";

const X: &str = "projects/alfa/tasks/x.md";
const Y: &str = "projects/alfa/tasks/y.md";
const VELHA: &str = "projects/alfa/tasks/archived/done/velha.md";
const LARGADA: &str = "projects/alfa/tasks/archived/dropped/largada.md";
const NOTAS: &str = "projects/alfa/notas.md";
const CURRENT: &str = "CURRENT.md";

/// O vault base: `alfa` com `x` (todo) e `y` (example) ativas, `velha` em
/// `archived/done/` e `largada` em `archived/dropped/`, mais uma nota; `beta`
/// vazio; `omega` encerrado. `status.json` default, sem `CURRENT.md`. Todos os
/// arquivos envelhecidos (mtime no passado).
fn home_base() -> tempfile::TempDir {
    let home = home_com(&[
        (X, X_MD),
        (Y, Y_MD),
        (VELHA, VELHA_MD),
        (LARGADA, LARGADA_MD),
        ("projects/_encerrados/omega/tasks/z.md", Z_MD),
        (NOTAS, NOTAS_MD),
    ]);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects/beta/tasks");
    envelhece(home.path());
    home
}

/// O vault base com o `CURRENT.md` já regenerado e envelhecido.
fn home_base_com_painel() -> tempfile::TempDir {
    let home = home_base();
    ok(&roda(home.path(), &["rebuild"]));
    envelhece(home.path());
    home
}

/// Um vault só com a task `x` (conteúdo dado em bytes) em `alfa`.
fn home_so_com_x(bytes: &[u8]) -> tempfile::TempDir {
    let home = home_com_vault_vazio();
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    arquivo_bytes(home.path(), X, bytes);
    envelhece(home.path());
    home
}

/// O painel de 2026-11-08 com a contagem e as seções dadas (docs/freitask.md,
/// "CURRENT.md — o painel diário"; o formato é o fixado na fatia 2).
fn painel(tasks: usize, projetos: usize, secoes: &str) -> String {
    format!(
        concat!(
            "---\n",
            "date: 2026-11-08\n",
            "weekday: Sunday\n",
            "---\n",
            "\n",
            "# Planning diário 08/11/26\n",
            "\n",
            "> Painel gerado automaticamente — {} tasks em {} projetos.\n",
            "\n",
            "## Notas Avulsas\n",
            "\n",
            "- \n",
            "\n",
            "{}",
        ),
        tasks, projetos, secoes
    )
}

const SECAO_BETA_VAZIA: &str = "## beta\n\n_(sem tasks)_\n\n";

const SECAO_ALFA_X_Y: &str = concat!(
    "## alfa\n",
    "\n",
    "> [!todo] Xis\n",
    "> [[projects/alfa/tasks/x|x]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "\n",
    "> [!example] Ypsilon\n",
    "> [[projects/alfa/tasks/y|y]]\n",
    "\n",
);

/// O painel do vault base, intocado.
fn painel_base() -> String {
    painel(2, 2, &format!("{SECAO_ALFA_X_Y}{SECAO_BETA_VAZIA}"))
}

/// `X_MD` com a linha 1 do bloco trocada.
fn x_com_linha_1(linha: &str) -> String {
    X_MD.replace("> [!todo] Xis\n", &format!("{linha}\n"))
}

// ===========================================================================
// new
// ===========================================================================

#[test]
fn new_cria_a_task_e_imprime_o_caminho() {
    let home = home_base();
    let saida = roda(home.path(), &["new", "beta", "nova", "Nova task"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "projects/beta/tasks/nova.md"))
    );
    assert_eq!(
        le(home.path(), "projects/beta/tasks/nova.md"),
        "> [!todo] Nova task\n> [[projects/beta/tasks/nova|nova]]\n\n"
    );
}

#[test]
fn new_com_desc_grava_a_descricao_em_italico() {
    let home = home_base();
    let saida = roda(
        home.path(),
        &["new", "beta", "nova", "Nova task", "--desc", "começando"],
    );
    ok(&saida);
    assert_eq!(
        le(home.path(), "projects/beta/tasks/nova.md"),
        "> [!todo] Nova task\n> [[projects/beta/tasks/nova|nova]]\n> _começando_\n\n"
    );
}

#[test]
fn new_aceita_desc_com_igual() {
    let home = home_base();
    let saida = roda(
        home.path(),
        &["new", "beta", "nova", "Nova task", "--desc=começando"],
    );
    ok(&saida);
    assert_eq!(
        le(home.path(), "projects/beta/tasks/nova.md"),
        "> [!todo] Nova task\n> [[projects/beta/tasks/nova|nova]]\n> _começando_\n\n"
    );
}

#[test]
fn new_regenera_o_current() {
    let home = home_base();
    ok(&roda(
        home.path(),
        &["new", "beta", "nova", "Nova task", "--desc", "começando"],
    ));
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            3,
            2,
            &format!(
                "{SECAO_ALFA_X_Y}{}",
                concat!(
                    "## beta\n",
                    "\n",
                    "> [!todo] Nova task\n",
                    "> [[projects/beta/tasks/nova|nova]]\n",
                    "> _começando_\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn new_so_grava_a_task_e_o_current() {
    let home = home_base();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["new", "beta", "nova", "Nova task"]));
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&["projects/beta/tasks/nova.md", CURRENT])
    );
}

#[test]
fn new_em_projeto_inexistente_cria_o_projeto() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "gama", "primeira", "Primeira"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "projects/gama/tasks/primeira.md"))
    );
    assert_eq!(
        le(home.path(), "projects/gama/tasks/primeira.md"),
        "> [!todo] Primeira\n> [[projects/gama/tasks/primeira|primeira]]\n\n"
    );
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&[
            "projects/gama",
            "projects/gama/tasks",
            "projects/gama/tasks/primeira.md",
            CURRENT,
        ])
    );
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            3,
            3,
            &format!(
                "{SECAO_ALFA_X_Y}{SECAO_BETA_VAZIA}{}",
                concat!(
                    "## gama\n",
                    "\n",
                    "> [!todo] Primeira\n",
                    "> [[projects/gama/tasks/primeira|primeira]]\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn new_num_vault_vazio_cria_projects_e_status_json() {
    // Decisão 21 do log: comando que escreve cria `projects/` e
    // `.freitask/status.json` quando faltam, como o `ensure_root` de hoje.
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["new", "p", "x", "Xis"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "projects/p/tasks/x.md"))
    );
    assert_eq!(
        le(home.path(), "projects/p/tasks/x.md"),
        "> [!todo] Xis\n> [[projects/p/tasks/x|x]]\n\n"
    );
    let gravado: serde_json::Value =
        serde_json::from_str(&le(home.path(), ".freitask/status.json"))
            .expect("status.json é JSON");
    let default: serde_json::Value =
        serde_json::from_str(gerador::STATUS_JSON).expect("STATUS_JSON é JSON");
    assert_eq!(gravado, default);
    assert_eq!(
        fotografa(home.path()).keys().cloned().collect::<Vec<_>>(),
        vec![
            ".freitask".to_string(),
            ".freitask/status.json".to_string(),
            "CURRENT.md".to_string(),
            "projects".to_string(),
            "projects/p".to_string(),
            "projects/p/tasks".to_string(),
            "projects/p/tasks/x.md".to_string(),
        ]
    );
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            1,
            1,
            "## p\n\n> [!todo] Xis\n> [[projects/p/tasks/x|x]]\n\n"
        )
    );
}

#[test]
fn new_recusa_id_ja_ocupado_por_task_ativa_com_saida_1_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "alfa", "x", "Outra"]);
    recusa(
        &saida,
        1,
        &format!("já existe uma task com esse id: {}", abs(home.path(), X)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_recusa_id_ocupado_por_task_arquivada_em_done() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "alfa", "velha", "Outra"]);
    recusa(
        &saida,
        1,
        &format!(
            "já existe uma task com esse id: {}",
            abs(home.path(), VELHA)
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_recusa_id_ocupado_por_task_arquivada_em_dropped() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "alfa", "largada", "Outra"]);
    recusa(
        &saida,
        1,
        &format!(
            "já existe uma task com esse id: {}",
            abs(home.path(), LARGADA)
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_aceita_id_que_existe_em_outro_projeto() {
    let home = home_base();
    let saida = roda(home.path(), &["new", "beta", "x", "Outro xis"]);
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "projects/beta/tasks/x.md"))
    );
    assert_eq!(
        le(home.path(), "projects/beta/tasks/x.md"),
        "> [!todo] Outro xis\n> [[projects/beta/tasks/x|x]]\n\n"
    );
    assert_eq!(le(home.path(), X), X_MD);
}

#[test]
fn new_recusa_projeto_encerrado_e_aponta_o_reabrir() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "omega", "w", "W"]);
    recusa(
        &saida,
        1,
        "o projeto \"omega\" está encerrado; reabra-o com `freitask reabrir omega`",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_recusa_projeto_reservado() {
    let home = home_base();
    let antes = fotografa(home.path());
    for p in ["daily", "templates", "_encerrados"] {
        let saida = roda(home.path(), &["new", p, "w", "W"]);
        recusa(
            &saida,
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_recusa_projeto_com_barra() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "a/b", "w", "W"]);
    recusa(&saida, 1, "projeto não pode conter '/': a/b");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_recusa_id_fora_de_kebab_sem_corrigir_em_silencio() {
    let home = home_base();
    let antes = fotografa(home.path());
    let casos = [
        ("Minha Task", "minha-task"),
        ("ação", "acao"),
        ("Minha-Task", "minha-task"),
    ];
    for (id, seria) in casos {
        let saida = roda(home.path(), &["new", "alfa", id, "T"]);
        recusa(
            &saida,
            1,
            &format!("id precisa ser kebab-case: \"{id}\" (seria \"{seria}\")"),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_com_id_invalido_nao_cria_o_projeto_novo() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "gama", "Bad Id", "T"]);
    recusa(
        &saida,
        1,
        "id precisa ser kebab-case: \"Bad Id\" (seria \"bad-id\")",
    );
    assert!(!existe(home.path(), "projects/gama"));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_sem_projeto_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(&roda(home.path(), &["new"]), 2, "falta o projeto");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(&roda(home.path(), &["new", "alfa"]), 2, "falta o id");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_sem_titulo_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new", "alfa", "w"]),
        2,
        "falta o título",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_desc_sem_valor_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new", "alfa", "w", "W", "--desc"]),
        2,
        "falta o valor de --desc",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// new-project
// ===========================================================================

#[test]
fn new_project_cria_a_pasta_tasks_em_kebab_e_imprime_o_nome() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new-project", "Meu Projeto"]);
    ok(&saida);
    assert_eq!(saida.stdout, "meu-projeto\n");
    assert!(
        vault(home.path())
            .join("projects/meu-projeto/tasks")
            .is_dir()
    );
    assert!(entradas(&vault(home.path()).join("projects/meu-projeto/tasks")).is_empty());
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&["projects/meu-projeto", "projects/meu-projeto/tasks"])
    );
}

#[test]
fn new_project_nao_regenera_o_current() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["new-project", "gama"]));
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&["projects/gama", "projects/gama/tasks"])
    );
    assert_eq!(le(home.path(), CURRENT), painel_base());
}

#[test]
fn new_project_recusa_projeto_existente_com_saida_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new-project", "alfa"]),
        1,
        "projeto já existe: alfa",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_project_passa_o_nome_pelo_kebab_antes_de_ver_se_existe() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new-project", "Alfa"]),
        1,
        "projeto já existe: alfa",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_project_recusa_projeto_encerrado_e_aponta_o_reabrir() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new-project", "omega"]),
        1,
        "o projeto \"omega\" está encerrado; reabra-o com `freitask reabrir omega`",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_project_recusa_nome_reservado() {
    let home = home_base();
    let antes = fotografa(home.path());
    for p in ["daily", "templates"] {
        recusa(
            &roda(home.path(), &["new-project", p]),
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_project_recusa_nome_que_nao_rende_id() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new-project", "!!!"]),
        1,
        "nome de projeto inválido: !!!",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_project_sem_nome_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new-project"]),
        2,
        "falta o nome do projeto",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// D2: `.` e `..` não são projeto
// ===========================================================================

#[test]
fn ponto_e_ponto_ponto_nao_sao_projeto() {
    // O caso medido em §0: num vault sem `projects/_encerrados/`, o oráculo
    // grava `tasks/y.md` na raiz do vault com `new .. y`. O novo recusa com a
    // mensagem de reservado, saída 1, e não grava nada.
    let home = home_com(&[(Y, Y_MD)]);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    envelhece(home.path());
    let antes = fotografa(home.path());
    for p in [".", ".."] {
        recusa(
            &roda(home.path(), &["new", p, "w", "W"]),
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
        recusa(
            &roda(home.path(), &["new-project", p]),
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
    assert!(!existe(home.path(), "tasks"));
    assert!(!home.path().join("tasks").exists());
}

// ===========================================================================
// set: caminho feliz
// ===========================================================================

#[test]
fn set_status_troca_o_callout_e_preserva_frontmatter_corpo_e_historico() {
    let home = home_base();
    let saida = roda(home.path(), &["set", "x", "--status", "example"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), X)));
    assert_eq!(le(home.path(), X), x_com_linha_1("> [!example] Xis"));
}

#[test]
fn set_title_troca_so_o_titulo() {
    let home = home_base();
    ok(&roda(home.path(), &["set", "x", "--title", "Xis novo"]));
    assert_eq!(le(home.path(), X), x_com_linha_1("> [!todo] Xis novo"));
}

#[test]
fn set_title_e_desc_sao_aparados() {
    let home = home_base();
    ok(&roda(
        home.path(),
        &[
            "set",
            "x",
            "--status",
            "example",
            "--title",
            "  Xis novo  ",
            "--desc",
            "  outro estado ",
        ],
    ));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace(
            "> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n> _em curso_\n",
            "> [!example] Xis novo\n> [[projects/alfa/tasks/x|x]]\n> _outro estado_\n"
        )
    );
}

#[test]
fn set_desc_vazia_apaga_a_linha_da_descricao_e_mantem_a_nota() {
    let home = home_base();
    ok(&roda(home.path(), &["set", "x", "--desc", ""]));
    assert_eq!(le(home.path(), X), X_MD.replace("> _em curso_\n", ""));
}

#[test]
fn set_desc_em_task_sem_descricao_insere_a_linha_3() {
    let home = home_base();
    ok(&roda(home.path(), &["set", "y", "--desc", "agora"]));
    assert_eq!(
        le(home.path(), Y),
        "> [!example] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n> _agora_\n"
    );
}

#[test]
fn set_aceita_flags_com_igual() {
    let home = home_base();
    ok(&roda(
        home.path(),
        &["set", "y", "--status=tip", "--title=Bar", "--desc=foo"],
    ));
    assert_eq!(
        le(home.path(), Y),
        "> [!tip] Bar\n> [[projects/alfa/tasks/y|y]]\n> _foo_\n"
    );
}

#[test]
fn set_so_grava_a_task_e_o_current() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["set", "x", "--status", "example"]));
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&[X, CURRENT])
    );
    assert_eq!(le(home.path(), NOTAS), NOTAS_MD);
    assert_eq!(le(home.path(), Y), Y_MD);
}

#[test]
fn set_regenera_o_current() {
    let home = home_base();
    ok(&roda(home.path(), &["set", "x", "--status", "example"]));
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            2,
            2,
            &format!(
                "{}{SECAO_BETA_VAZIA}",
                concat!(
                    "## alfa\n",
                    "\n",
                    "> [!example] Xis\n",
                    "> [[projects/alfa/tasks/x|x]]\n",
                    "> _em curso_\n",
                    "> nota um\n",
                    "\n",
                    "> [!example] Ypsilon\n",
                    "> [[projects/alfa/tasks/y|y]]\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn set_id_do_caminho_manda_e_conserta_o_link_dessincronizado() {
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        "> [!example] Ypsilon\n> [[projects/alfa/tasks/outro|outro]]\n",
    );
    let saida = roda(home.path(), &["set", "y", "--title", "Ypsilon novo"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), Y)));
    assert_eq!(
        le(home.path(), Y),
        "> [!example] Ypsilon novo\n> [[projects/alfa/tasks/y|y]]\n"
    );
    assert!(!existe(home.path(), "projects/alfa/tasks/outro.md"));
}

#[test]
fn set_status_conserta_callout_desconhecido() {
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        "> [!todoo] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n",
    );
    ok(&roda(home.path(), &["set", "y", "--status", "todo"]));
    assert_eq!(
        le(home.path(), Y),
        "> [!todo] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n"
    );
}

#[test]
fn set_title_em_status_0_preserva_o_callout_digitado() {
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        "> [!todoo] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n",
    );
    ok(&roda(home.path(), &["set", "y", "--title", "Outro"]));
    assert_eq!(
        le(home.path(), Y),
        "> [!todoo] Outro\n> [[projects/alfa/tasks/y|y]]\n"
    );
}

#[test]
fn set_por_caminho_relativo_ao_cwd_resolve_id_ambiguo() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/beta/tasks/x.md",
        "> [!todo] Xis de beta\n> [[projects/beta/tasks/x|x]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    let saida = roda(
        home.path(),
        &[
            "set",
            "ObsidianVault/projects/beta/tasks/x.md",
            "--status",
            "tip",
        ],
    );
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "projects/beta/tasks/x.md"))
    );
    assert_eq!(
        le(home.path(), "projects/beta/tasks/x.md"),
        "> [!tip] Xis de beta\n> [[projects/beta/tasks/x|x]]\n"
    );
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&["projects/beta/tasks/x.md", CURRENT])
    );
}

#[test]
fn set_por_caminho_absoluto() {
    let home = home_base();
    let caminho = abs(home.path(), Y);
    let saida = roda(home.path(), &["set", &caminho, "--status", "question"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{caminho}\n"));
    assert_eq!(
        le(home.path(), Y),
        "> [!question] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n"
    );
}

// ===========================================================================
// set --volta-em
// ===========================================================================

#[test]
fn set_volta_em_acrescenta_a_chave_no_fim_do_frontmatter() {
    let home = home_base();
    let saida = roda(home.path(), &["set", "x", "--volta-em", "2026-12-01"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), X)));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace(
            "criado: 2026-01-01\n---\n",
            "criado: 2026-01-01\nvolta-em: 2026-12-01\n---\n"
        )
    );
}

#[test]
fn set_volta_em_cria_o_frontmatter_quando_falta() {
    let home = home_base();
    ok(&roda(
        home.path(),
        &["set", "y", "--volta-em", "2026-12-01"],
    ));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nvolta-em: 2026-12-01\n---\n\n{Y_MD}")
    );
}

#[test]
fn set_volta_em_troca_a_data_no_lugar() {
    let home = home_base();
    let com_data = X_MD.replace("id: x\n", "id: x\nvolta-em: 2026-11-20\n");
    arquivo(home.path(), X, &com_data);
    ok(&roda(
        home.path(),
        &["set", "x", "--volta-em", "2026-12-02"],
    ));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace("id: x\n", "id: x\nvolta-em: 2026-12-02\n")
    );
}

#[test]
fn set_volta_em_vazio_limpa_o_campo_e_mantem_as_outras_chaves() {
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nvolta-em: 2026-11-20\n"),
    );
    ok(&roda(home.path(), &["set", "x", "--volta-em", ""]));
    assert_eq!(le(home.path(), X), X_MD);
}

#[test]
fn set_volta_em_vazio_remove_o_frontmatter_que_esvazia() {
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        &format!("---\nvolta-em: 2026-12-01\n---\n\n{Y_MD}"),
    );
    ok(&roda(home.path(), &["set", "y", "--volta-em", ""]));
    assert_eq!(le(home.path(), Y), Y_MD);
}

#[test]
fn set_volta_em_so_mexe_no_frontmatter_e_nao_reserializa_o_bloco() {
    // O bloco com link dessincronizado e espaço sobrando fica byte a byte.
    let home = home_base();
    let bloco = "> [!example]   Ypsilon  \n> [[projects/alfa/tasks/outro|outro]]\n";
    arquivo(home.path(), Y, bloco);
    ok(&roda(
        home.path(),
        &["set", "y", "--volta-em", "2026-12-01"],
    ));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nvolta-em: 2026-12-01\n---\n\n{bloco}")
    );
}

#[test]
fn set_volta_em_tira_a_task_do_painel() {
    let home = home_base();
    ok(&roda(
        home.path(),
        &["set", "x", "--volta-em", "2026-12-01"],
    ));
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            1,
            2,
            &format!(
                "{}{SECAO_BETA_VAZIA}",
                "## alfa\n\n> [!example] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n\n"
            )
        )
    );
}

#[test]
fn set_volta_em_invalida_sai_2_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    for data in ["amanha", "2026-02-30", "2026-12-1", "01/12/2026"] {
        recusa(
            &roda(home.path(), &["set", "x", "--volta-em", data]),
            2,
            &format!(
                "data inválida: \"{data}\" (use AAAA-MM-DD, de um dia que existe; \"\" limpa)"
            ),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_status_valido_com_volta_em_invalida_nao_grava_nada() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["set", "x", "--status", "todo", "--volta-em", "amanha"],
        ),
        2,
        "data inválida: \"amanha\" (use AAAA-MM-DD, de um dia que existe; \"\" limpa)",
    );
    recusa(
        &roda(
            home.path(),
            &["set", "x", "--status", "example", "--volta-em", "amanha"],
        ),
        2,
        "data inválida: \"amanha\" (use AAAA-MM-DD, de um dia que existe; \"\" limpa)",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_volta_em_sem_valor_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x", "--volta-em"]),
        2,
        "falta o valor de --volta-em",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// set: fase, arquivadas e recusas
// ===========================================================================

#[test]
fn set_recusa_done_com_saida_1_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x", "--status", "done"]),
        1,
        "`done` é gravado por `freitask archive <id> done`, não por `set`",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_fase_desconhecida_sai_2_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x", "--status", "foo"]),
        2,
        "fase desconhecida: foo",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_recusa_trocar_a_fase_de_task_em_archived_done() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "velha", "--status", "todo"]),
        1,
        "task em archived/done/ fica `done`; desarquive antes (`freitask unarchive <id>`)",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_title_em_archived_done_mantem_done_e_a_pasta() {
    let home = home_base();
    let saida = roda(home.path(), &["set", "velha", "--title", "Velha nova"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), VELHA)));
    assert_eq!(
        le(home.path(), VELHA),
        "> [!done] Velha nova\n> [[projects/alfa/tasks/archived/done/velha|velha]]\n"
    );
    assert!(!existe(home.path(), "projects/alfa/tasks/velha.md"));
}

#[test]
fn set_em_task_arquivada_nao_a_desarquiva() {
    let home = home_base();
    let saida = roda(home.path(), &["set", "largada", "--status", "example"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), LARGADA)));
    assert_eq!(
        le(home.path(), LARGADA),
        "> [!example] Largada\n> [[projects/alfa/tasks/archived/dropped/largada|largada]]\n"
    );
    assert!(!existe(home.path(), "projects/alfa/tasks/largada.md"));
}

#[test]
fn set_titulo_vazio_sai_2_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    for t in ["", "   "] {
        recusa(
            &roda(home.path(), &["set", "x", "--title", t]),
            2,
            "o título não pode ficar vazio",
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_sem_flags_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x"]),
        2,
        "nada a mudar: passe --status, --title, --desc e/ou --volta-em",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set"]),
        2,
        "falta o id ou caminho da task",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_id_inexistente_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "zzz", "--status", "todo"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_nao_acha_task_de_projeto_encerrado() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "z", "--title", "A"]),
        1,
        "task não encontrada: z",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_id_ambiguo_sai_1_e_nao_grava() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/beta/tasks/x.md",
        "> [!todo] Xis de beta\n> [[projects/beta/tasks/x|x]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x", "--title", "Foo"]),
        1,
        "id \"x\" é ambíguo (2 arquivos); passe o caminho",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_em_arquivo_que_nao_e_task_sai_1_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    for flag in [["--title", "Foo"], ["--volta-em", "2026-12-01"]] {
        recusa(
            &roda(
                home.path(),
                &[
                    "set",
                    "ObsidianVault/projects/alfa/notas.md",
                    flag[0],
                    flag[1],
                ],
            ),
            1,
            &format!("não é um arquivo de task: {}", abs(home.path(), NOTAS)),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_em_sync_conflict_sai_1_e_nao_grava() {
    let home = home_base();
    let conflito = "projects/alfa/tasks/x.sync-conflict-20260101-000000-ABCDEFG.md";
    arquivo(home.path(), conflito, X_MD);
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["set", &abs(home.path(), conflito), "--status", "example"],
        ),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), conflito)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_em_task_sem_callout_sai_1_e_nao_grava() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/alfa/tasks/sem.md",
        "texto sem callout\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "sem", "--status", "example"]),
        1,
        &format!(
            "task sem callout (rode `freitask doctor`): {}",
            abs(home.path(), "projects/alfa/tasks/sem.md")
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_status_sem_valor_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x", "--status"]),
        2,
        "falta o valor de --status",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_flag_desconhecida_sai_2_e_nao_grava() {
    // A mensagem é a do clap (spec §2.1); só o código e o "nada gravado" são
    // contrato aqui.
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["set", "x", "--foo", "bar"]);
    assert_eq!(saida.codigo, Some(2), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, "");
    assert!(!saida.stderr.is_empty());
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// D4: CRLF, BOM e ausência de `\n` final sobrevivem ao `set`
// ===========================================================================

#[test]
fn crlf_sobrevive_ao_set() {
    let original = concat!(
        "---\r\n",
        "aliases: []\r\n",
        "id: x\r\n",
        "---\r\n",
        "\r\n",
        "> [!todo] Xis\r\n",
        "> [[projects/alfa/tasks/x|x]]\r\n",
        "> _em curso_\r\n",
        "> nota um\r\n",
        "\r\n",
        "## Notas\r\n",
        "\r\n",
        "- [ ] item\r\n",
    );
    let home = home_so_com_x(original.as_bytes());
    let saida = roda(
        home.path(),
        &["set", "x", "--status", "example", "--title", "Xis novo"],
    );
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), X)));
    assert_eq!(
        String::from_utf8(le_bytes(home.path(), X)).expect("utf-8"),
        original.replace("> [!todo] Xis\r\n", "> [!example] Xis novo\r\n")
    );
}

#[test]
fn crlf_sobrevive_ao_set_desc() {
    let original = "> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n> nota\r\n\r\ncorpo\r\n";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(home.path(), &["set", "x", "--desc", "novo"]));
    assert_eq!(
        String::from_utf8(le_bytes(home.path(), X)).expect("utf-8"),
        "> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n> _novo_\r\n> nota\r\n\r\ncorpo\r\n"
    );
}

#[test]
fn crlf_sobrevive_ao_set_volta_em_no_frontmatter() {
    let original = "---\r\nid: x\r\n---\r\n\r\n> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(
        home.path(),
        &["set", "x", "--volta-em", "2026-12-01"],
    ));
    assert_eq!(
        String::from_utf8(le_bytes(home.path(), X)).expect("utf-8"),
        "---\r\nid: x\r\nvolta-em: 2026-12-01\r\n---\r\n\r\n> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n"
    );
}

#[test]
fn crlf_sem_frontmatter_ganha_frontmatter_em_crlf() {
    let original = "> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(
        home.path(),
        &["set", "x", "--volta-em", "2026-12-01"],
    ));
    assert_eq!(
        String::from_utf8(le_bytes(home.path(), X)).expect("utf-8"),
        "---\r\nvolta-em: 2026-12-01\r\n---\r\n\r\n> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n"
    );
}

#[test]
fn bom_sobrevive_ao_set() {
    let original = "\u{feff}> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n\ncorpo\n";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(home.path(), &["set", "x", "--status", "example"]));
    assert_eq!(
        le_bytes(home.path(), X),
        "\u{feff}> [!example] Xis\n> [[projects/alfa/tasks/x|x]]\n\ncorpo\n".as_bytes()
    );
}

#[test]
fn bom_antes_do_frontmatter_sobrevive_ao_set() {
    let original = "\u{feff}---\nid: x\n---\n\n> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(home.path(), &["set", "x", "--title", "Xis novo"]));
    assert_eq!(
        le_bytes(home.path(), X),
        "\u{feff}---\nid: x\n---\n\n> [!todo] Xis novo\n> [[projects/alfa/tasks/x|x]]\n".as_bytes()
    );
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_set() {
    let original = "> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n\nCorpo sem quebra";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(home.path(), &["set", "x", "--status", "example"]));
    assert_eq!(
        le_bytes(home.path(), X),
        "> [!example] Xis\n> [[projects/alfa/tasks/x|x]]\n\nCorpo sem quebra".as_bytes()
    );
}

#[test]
fn bloco_no_fim_sem_quebra_final_continua_sem_quebra_depois_do_set() {
    let original = "> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]";
    let home = home_so_com_x(original.as_bytes());
    ok(&roda(home.path(), &["set", "x", "--desc", "novo"]));
    assert_eq!(
        le_bytes(home.path(), X),
        "> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n> _novo_".as_bytes()
    );
}

// ===========================================================================
// D6: escrita sem mudança não grava (mtime igual)
// ===========================================================================

#[test]
fn rebuild_sem_mudanca_nao_grava() {
    let home = home_base_com_painel();
    assert_eq!(le(home.path(), CURRENT), painel_base());
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["rebuild"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), CURRENT)));
    nada_mudou(&antes, &fotografa(home.path()));
    assert_eq!(mtime(home.path(), CURRENT), antigo());
}

#[test]
fn set_sem_mudanca_nao_toca_o_mtime_da_task_nem_do_current() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let saida = roda(
        home.path(),
        &[
            "set", "x", "--status", "todo", "--title", "Xis", "--desc", "em curso",
        ],
    );
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), X)));
    nada_mudou(&antes, &fotografa(home.path()));
    assert_eq!(mtime(home.path(), X), antigo());
    assert_eq!(mtime(home.path(), CURRENT), antigo());
}

#[test]
fn set_volta_em_vazio_sem_o_campo_nao_toca_o_arquivo() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["set", "y", "--volta-em", ""]));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_volta_em_com_a_mesma_data_nao_toca_o_arquivo() {
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nvolta-em: 2026-12-01\n"),
    );
    ok(&roda(home.path(), &["rebuild"]));
    envelhece(home.path());
    let antes = fotografa(home.path());
    ok(&roda(
        home.path(),
        &["set", "x", "--volta-em", "2026-12-01"],
    ));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_em_arquivada_nao_regrava_o_current() {
    // O painel não mostra arquivada: a task muda, o CURRENT.md não.
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(
        home.path(),
        &["set", "velha", "--title", "Velha nova"],
    ));
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&[VELHA])
    );
    assert_eq!(mtime(home.path(), CURRENT), antigo());
}

// ===========================================================================
// delete
// ===========================================================================

#[test]
fn delete_apaga_a_task_e_imprime_o_caminho() {
    let home = home_base();
    let saida = roda(home.path(), &["delete", "y"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), Y)));
    assert!(!existe(home.path(), Y));
}

#[test]
fn delete_regenera_o_current() {
    let home = home_base_com_painel();
    ok(&roda(home.path(), &["delete", "y"]));
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            1,
            2,
            &format!(
                "{}{SECAO_BETA_VAZIA}",
                concat!(
                    "## alfa\n",
                    "\n",
                    "> [!todo] Xis\n",
                    "> [[projects/alfa/tasks/x|x]]\n",
                    "> _em curso_\n",
                    "> nota um\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn delete_deixa_as_referencias_penduradas() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["delete", "y"]));
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&[Y, CURRENT])
    );
    assert_eq!(le(home.path(), NOTAS), NOTAS_MD);
}

#[test]
fn delete_apaga_task_arquivada_pelo_id() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["delete", "velha"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), VELHA)));
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&[VELHA])
    );
}

#[test]
fn delete_por_caminho_resolve_id_ambiguo() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/beta/tasks/x.md",
        "> [!todo] Xis de beta\n> [[projects/beta/tasks/x|x]]\n",
    );
    envelhece(home.path());
    let saida = roda(
        home.path(),
        &["delete", "ObsidianVault/projects/beta/tasks/x.md"],
    );
    ok(&saida);
    assert_eq!(
        saida.stdout,
        format!("{}\n", abs(home.path(), "projects/beta/tasks/x.md"))
    );
    assert!(!existe(home.path(), "projects/beta/tasks/x.md"));
    assert_eq!(le(home.path(), X), X_MD);
}

#[test]
fn delete_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["delete"]),
        2,
        "falta o id ou caminho da task",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn delete_id_inexistente_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["delete", "zzz"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn delete_nao_acha_task_de_projeto_encerrado() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["delete", "z"]),
        1,
        "task não encontrada: z",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn delete_id_ambiguo_sai_1_e_nao_apaga_nada() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/beta/tasks/x.md",
        "> [!todo] Xis de beta\n> [[projects/beta/tasks/x|x]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["delete", "x"]),
        1,
        "id \"x\" é ambíguo (2 arquivos); passe o caminho",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn delete_recusa_arquivo_que_nao_e_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["delete", "ObsidianVault/projects/alfa/notas.md"],
        ),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), NOTAS)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn delete_recusa_sync_conflict() {
    let home = home_base();
    let conflito = "projects/alfa/tasks/x.sync-conflict-20260101-000000-ABCDEFG.md";
    arquivo(home.path(), conflito, X_MD);
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["delete", &abs(home.path(), conflito)]),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), conflito)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// Lock (spec §1.5, §2.4): escrita espera até 5 s e sai 3; leitura não espera
// ===========================================================================

/// Afirma a saída 3 por lock: esperou uns 5 s, stdout vazio, uma linha em
/// stderr com o prefixo `freitask: `.
fn saiu_3_depois_de_esperar(saida: &Saida, decorrido: Duration) {
    assert_eq!(saida.codigo, Some(3), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, "");
    assert!(
        saida.stderr.starts_with("freitask: ") && saida.stderr.ends_with('\n'),
        "stderr: {:?}",
        saida.stderr
    );
    assert_eq!(
        saida.stderr.matches('\n').count(),
        1,
        "stderr: {:?}",
        saida.stderr
    );
    assert!(
        decorrido >= Duration::from_millis(4500),
        "saiu cedo demais: {decorrido:?} (devia esperar o lock até 5 s)"
    );
    assert!(
        decorrido <= Duration::from_secs(15),
        "esperou demais: {decorrido:?}"
    );
}

/// Roda o comando com o lock em `$XDG_RUNTIME_DIR/freitask.lock` segurado de
/// fora e devolve a saída e o tempo que levou.
fn roda_com_lock_em_xdg(home: &Path, args: &[&str]) -> (Saida, Duration) {
    let xdg = tempfile::tempdir().expect("tempdir");
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 60);
    let t0 = Instant::now();
    let saida = roda_com_xdg(home, xdg.path(), args);
    (saida, t0.elapsed())
}

#[test]
fn set_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let (saida, decorrido) =
        roda_com_lock_em_xdg(home.path(), &["set", "x", "--status", "example"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_espera_o_lock_em_local_state_sem_xdg_e_sai_3_sem_gravar() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let _lock = Lock::segura(&lock_sem_xdg(home.path()), 60);
    let t0 = Instant::now();
    let saida = roda(home.path(), &["new", "beta", "nova", "Nova"]);
    saiu_3_depois_de_esperar(&saida, t0.elapsed());
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn delete_espera_o_lock_e_sai_3_sem_apagar() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["delete", "y"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_project_espera_o_lock_e_sai_3_sem_criar() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["new-project", "gama"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rebuild_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_base();
    // Sem CURRENT.md: um rebuild que não esperasse o criaria.
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["rebuild"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
    assert!(!existe(home.path(), CURRENT));
}

#[test]
fn escrita_espera_o_lock_ser_liberado_e_grava() {
    let home = home_base();
    let xdg = tempfile::tempdir().expect("tempdir");
    // Solta sozinho depois de 2 s, dentro da janela de 5 s.
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 2);
    let t0 = Instant::now();
    let saida = roda_com_xdg(
        home.path(),
        xdg.path(),
        &["set", "x", "--status", "example"],
    );
    let decorrido = t0.elapsed();
    ok(&saida);
    assert!(
        decorrido >= Duration::from_millis(1500),
        "não esperou o lock: {decorrido:?}"
    );
    assert_eq!(le(home.path(), X), x_com_linha_1("> [!example] Xis"));
}

#[test]
fn list_json_nao_espera_o_lock() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["list", "--json"]);
    ok(&saida);
    assert!(
        decorrido < Duration::from_secs(2),
        "a leitura esperou o lock: {decorrido:?}"
    );
    let linhas = json(&saida);
    let ids: Vec<&str> = linhas
        .as_array()
        .expect("array")
        .iter()
        .map(|l| l["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, vec!["x", "y"]);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn list_json_sem_xdg_tambem_nao_espera_o_lock() {
    let home = home_base_com_painel();
    let _lock = Lock::segura(&lock_sem_xdg(home.path()), 60);
    let t0 = Instant::now();
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert!(
        t0.elapsed() < Duration::from_secs(2),
        "a leitura esperou o lock: {:?}",
        t0.elapsed()
    );
}

// ===========================================================================
// §3.7: dois freitask em paralelo não perdem escrita
// ===========================================================================

fn task_paralela(i: usize, desc: Option<&str>) -> String {
    let mut s = format!("> [!todo] Tarefa {i}\n> [[projects/alfa/tasks/t{i}|t{i}]]\n");
    if let Some(d) = desc {
        s.push_str(&format!("> _{d}_\n"));
    }
    s.push_str("> nota\n\n## Corpo\n\n- [ ] passo\n");
    s
}

#[test]
fn dois_freitask_em_paralelo_nao_perdem_escrita() {
    let home = home_com_vault_vazio();
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    for i in 1..=8 {
        arquivo(
            home.path(),
            &format!("projects/alfa/tasks/t{i}.md"),
            &task_paralela(i, None),
        );
    }
    for rodada in 1..=3 {
        let desc = format!("rodada {rodada}");
        let mut filhos: Vec<(String, Child)> = Vec::new();
        for i in 1..=8 {
            let id = format!("t{i}");
            let filho = comando(home.path())
                .args(["set", &id, "--desc", &desc])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("spawn set");
            filhos.push((format!("set {id}"), filho));
        }
        let filho = comando(home.path())
            .args(["rebuild"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn rebuild");
        filhos.push(("rebuild".to_string(), filho));
        for (nome, filho) in filhos {
            let o = filho.wait_with_output().expect("esperar");
            assert_eq!(
                o.status.code(),
                Some(0),
                "rodada {rodada}, {nome}: stderr {:?}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
        for i in 1..=8 {
            assert_eq!(
                le(home.path(), &format!("projects/alfa/tasks/t{i}.md")),
                task_paralela(i, Some(&desc)),
                "rodada {rodada}: t{i} perdeu a escrita ou foi truncada"
            );
        }
        assert_eq!(
            temporarios(home.path()),
            Vec::<String>::new(),
            "rodada {rodada}: sobrou temporário"
        );
    }

    // O painel final é o mesmo de um rebuild isolado depois de tudo, e é o
    // esperado escrito aqui.
    let mut secao = String::from("## alfa\n\n");
    for i in 1..=8 {
        secao.push_str(&format!(
            "> [!todo] Tarefa {i}\n> [[projects/alfa/tasks/t{i}|t{i}]]\n> _rodada 3_\n> nota\n\n"
        ));
    }
    let esperado = painel(8, 1, &secao);
    assert_eq!(le(home.path(), CURRENT), esperado);
    let depois_do_paralelo = le_bytes(home.path(), CURRENT);
    ok(&roda(home.path(), &["rebuild"]));
    assert_eq!(le_bytes(home.path(), CURRENT), depois_do_paralelo);
    assert_eq!(temporarios(home.path()), Vec::<String>::new());
}
