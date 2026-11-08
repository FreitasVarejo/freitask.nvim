//! Aceite da fatia 1 — `core`: caminho, vocabulário, bloco, frontmatter,
//! histórico, data (spec §5). O que a fatia expõe pela CLI: `kebab`,
//! `statuses [--json]` e `version [--json]`, todos leituras puras (spec §1.6 e
//! I6), com `FREITASK_TODAY` aceito.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§1.6, §2.1, §2.2, §2.4) e `docs/freitask.md` ("Status", "Status 0",
//! "CLI"). Nenhum valor esperado aqui foi gerado da implementação.

mod util;

use serde_json::{Value, json};
use util::*;

// ---------------------------------------------------------------------------
// Valores esperados, escritos à mão a partir do contrato.
// ---------------------------------------------------------------------------

/// `statuses` em texto com o vocabulário default embutido: uma linha por fase,
/// `string.format("%d  %-10s %s", num, callout, title)`, precedido do
/// sentinela 0 (docs/freitask.md, "Status 0" e a tabela de "Status").
const STATUSES_DEFAULT_TEXTO: &str = "\
0  invalid    Sem status
1  todo       Não iniciada
2  abstract   Refinando
3  tip        Refinada
4  example    Em implementação
5  question   Em homologação
6  warning    Bloqueada
7  check      Pronta
8  quote      Algum dia
9  done       Arquivada
";

/// `statuses --json` com o vocabulário default: array em ordem crescente de
/// `num`, chaves `{num, callout, title, icon, hl_group, hidden}` (spec §2.2),
/// `hidden` só em `quote` (docs/freitask.md, "Algum dia é escondida por
/// padrão").
fn statuses_default_json() -> Value {
    json!([
        {"num": 0, "callout": "invalid",  "title": "Sem status",       "icon": "󰘸", "hl_group": "DiagnosticError", "hidden": false},
        {"num": 1, "callout": "todo",     "title": "Não iniciada",     "icon": "󰗡", "hl_group": "DiagnosticInfo",  "hidden": false},
        {"num": 2, "callout": "abstract", "title": "Refinando",        "icon": "󰨸", "hl_group": "DiagnosticInfo",  "hidden": false},
        {"num": 3, "callout": "tip",      "title": "Refinada",         "icon": "󰌶", "hl_group": "DiagnosticHint",  "hidden": false},
        {"num": 4, "callout": "example",  "title": "Em implementação", "icon": "󰉹", "hl_group": "DiagnosticHint",  "hidden": false},
        {"num": 5, "callout": "question", "title": "Em homologação",   "icon": "󰘥", "hl_group": "DiagnosticWarn",  "hidden": false},
        {"num": 6, "callout": "warning",  "title": "Bloqueada",        "icon": "󰀪", "hl_group": "DiagnosticWarn",  "hidden": false},
        {"num": 7, "callout": "check",    "title": "Pronta",           "icon": "󰄬", "hl_group": "DiagnosticOk",    "hidden": false},
        {"num": 8, "callout": "quote",    "title": "Algum dia",        "icon": "󱆨", "hl_group": "Comment",         "hidden": true},
        {"num": 9, "callout": "done",     "title": "Arquivada",        "icon": "󰄬", "hl_group": "DiagnosticOk",    "hidden": false},
    ])
}

/// Um `status.json` próprio, com só duas fases.
const STATUS_JSON_DUAS_FASES: &str = r#"{
  "1": { "title": "Fazer", "callout": "todo", "icon": "A", "hl_group": "Normal" },
  "2": { "title": "Feita", "callout": "check", "icon": "C", "hl_group": "DiagnosticOk" }
}"#;

const STATUSES_DUAS_FASES_TEXTO: &str = "\
0  invalid    Sem status
1  todo       Fazer
2  check      Feita
";

fn statuses_duas_fases_json() -> Value {
    json!([
        {"num": 0, "callout": "invalid", "title": "Sem status", "icon": "󰘸", "hl_group": "DiagnosticError", "hidden": false},
        {"num": 1, "callout": "todo",    "title": "Fazer",      "icon": "A", "hl_group": "Normal",          "hidden": false},
        {"num": 2, "callout": "check",   "title": "Feita",      "icon": "C", "hl_group": "DiagnosticOk",    "hidden": false},
    ])
}

/// Um `status.json` próprio em que `quote` não é a última fase nem a 8: o
/// `hidden` acompanha o callout, não o número nem a posição.
const STATUS_JSON_COM_QUOTE_NO_MEIO: &str = r#"{
  "1": { "title": "Fazer", "callout": "todo", "icon": "A", "hl_group": "Normal" },
  "2": { "title": "Depois", "callout": "quote", "icon": "B", "hl_group": "Comment" },
  "3": { "title": "Feita", "callout": "check", "icon": "C", "hl_group": "DiagnosticOk" }
}"#;

fn ok(saida: &Saida) {
    assert_eq!(
        saida.codigo,
        Some(0),
        "código de saída; stderr: {:?}",
        saida.stderr
    );
    assert_eq!(
        saida.stderr, "",
        "stderr tem de estar vazio numa leitura que deu certo"
    );
}

// ---------------------------------------------------------------------------
// `freitask kebab "<texto>"`
// ---------------------------------------------------------------------------

#[test]
fn kebab_tira_acento_e_troca_espaco_por_hifen() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "Configuração da API"]);
    ok(&saida);
    assert_eq!(saida.stdout, "configuracao-da-api\n");
}

#[test]
fn kebab_dobra_minuscula_multibyte() {
    // 3b3c0d5: `Á` vira `a`, não some.
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "Ávida Tarefa"]);
    ok(&saida);
    assert_eq!(saida.stdout, "avida-tarefa\n");
}

#[test]
fn kebab_descarta_pontuacao() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "Fix: the thing!"]);
    ok(&saida);
    assert_eq!(saida.stdout, "fix-the-thing\n");
}

#[test]
fn kebab_apara_espacos_das_pontas_e_dobra_os_do_meio() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "  foo   bar  "]);
    ok(&saida);
    assert_eq!(saida.stdout, "foo-bar\n");
}

#[test]
fn kebab_apara_hifens_das_pontas_e_dobra_os_do_meio() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "--foo--bar--"]);
    ok(&saida);
    assert_eq!(saida.stdout, "foo-bar\n");
}

#[test]
fn kebab_troca_underscore_por_hifen() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "foo_bar"]);
    ok(&saida);
    assert_eq!(saida.stdout, "foo-bar\n");
}

#[test]
fn kebab_dobra_underscore_e_espaco_misturados_num_hifen_so() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "foo __ bar"]);
    ok(&saida);
    assert_eq!(saida.stdout, "foo-bar\n");
}

#[test]
fn kebab_combina_acento_maiuscula_espaco_e_underscore() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "Ávida Revisão_final"]);
    ok(&saida);
    assert_eq!(saida.stdout, "avida-revisao-final\n");
}

#[test]
fn kebab_so_imprime_o_id_e_a_quebra_de_linha() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "Configuração da API"]);
    ok(&saida);
    assert_eq!(saida.stdout.matches('\n').count(), 1, "uma linha só");
    assert!(saida.stdout.ends_with('\n'));
    assert_eq!(saida.stdout.trim_end_matches('\n'), "configuracao-da-api");
}

#[test]
fn kebab_recusa_texto_so_de_pontuacao_com_saida_1() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "!!!"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: o texto não rende um id: !!!\n");
}

#[test]
fn kebab_recusa_texto_vazio_com_saida_1() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", ""]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: o texto não rende um id: \n");
}

#[test]
fn kebab_sem_argumento_sai_2_e_diz_que_falta_o_texto() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab"]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "freitask: falta o texto\n");
}

#[test]
fn kebab_e_leitura_pura_num_home_sem_vault() {
    let home = home_sem_vault();
    let antes = fotografa(home.path());
    assert!(antes.is_empty(), "o HOME começa vazio");

    let saida = roda(home.path(), &["kebab", "Configuração da API"]);
    ok(&saida);

    assert!(
        !vault(home.path()).exists(),
        "kebab não cria ObsidianVault/"
    );
    assert_eq!(
        entradas(home.path()),
        Vec::<String>::new(),
        "kebab não cria nada no HOME"
    );
    assert_eq!(fotografa(home.path()), antes);
}

#[test]
fn kebab_recusado_tambem_e_leitura_pura() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["kebab", "!!!"]);
    assert_eq!(saida.codigo, Some(1));
    assert!(!vault(home.path()).exists());
    assert_eq!(entradas(home.path()), Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// `freitask statuses [--json]`
// ---------------------------------------------------------------------------

#[test]
fn statuses_sem_status_json_imprime_o_default_em_texto() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses"]);
    ok(&saida);
    assert_eq!(saida.stdout, STATUSES_DEFAULT_TEXTO);
}

#[test]
fn statuses_sem_vault_imprime_o_default_em_texto() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["statuses"]);
    ok(&saida);
    assert_eq!(saida.stdout, STATUSES_DEFAULT_TEXTO);
}

#[test]
fn statuses_texto_comeca_pelo_sentinela_0() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses"]);
    ok(&saida);
    assert_eq!(
        saida.stdout.lines().next(),
        Some("0  invalid    Sem status")
    );
}

#[test]
fn statuses_json_sem_status_json_devolve_o_default() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), statuses_default_json());
}

#[test]
fn statuses_json_sem_vault_devolve_o_default() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), statuses_default_json());
}

#[test]
fn statuses_json_vem_em_ordem_crescente_de_num() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    let nums: Vec<i64> = json(&saida)
        .as_array()
        .expect("array")
        .iter()
        .map(|o| o["num"].as_i64().expect("num inteiro"))
        .collect();
    assert_eq!(nums, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn statuses_json_cada_objeto_tem_exatamente_as_seis_chaves() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    for obj in json(&saida).as_array().expect("array") {
        let mut chaves: Vec<&str> = obj
            .as_object()
            .expect("objeto")
            .keys()
            .map(String::as_str)
            .collect();
        chaves.sort_unstable();
        assert_eq!(
            chaves,
            vec!["callout", "hidden", "hl_group", "icon", "num", "title"]
        );
    }
}

#[test]
fn statuses_json_esconde_so_algum_dia() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    let escondidos: Vec<String> = json(&saida)
        .as_array()
        .expect("array")
        .iter()
        .filter(|o| o["hidden"] == json!(true))
        .map(|o| o["callout"].as_str().expect("callout string").to_owned())
        .collect();
    assert_eq!(escondidos, vec!["quote".to_string()]);
}

#[test]
fn statuses_json_sentinela_0_e_invalid_sem_status() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida)[0],
        json!({"num": 0, "callout": "invalid", "title": "Sem status", "icon": "󰘸", "hl_group": "DiagnosticError", "hidden": false})
    );
}

#[test]
fn statuses_le_status_json_proprio_em_texto() {
    let home = home_com_status_json(STATUS_JSON_DUAS_FASES);
    let saida = roda(home.path(), &["statuses"]);
    ok(&saida);
    assert_eq!(saida.stdout, STATUSES_DUAS_FASES_TEXTO);
}

#[test]
fn statuses_json_le_status_json_proprio() {
    let home = home_com_status_json(STATUS_JSON_DUAS_FASES);
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), statuses_duas_fases_json());
}

#[test]
fn statuses_json_hidden_acompanha_o_callout_quote_e_nao_o_numero() {
    let home = home_com_status_json(STATUS_JSON_COM_QUOTE_NO_MEIO);
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida),
        json!([
            {"num": 0, "callout": "invalid", "title": "Sem status", "icon": "󰘸", "hl_group": "DiagnosticError", "hidden": false},
            {"num": 1, "callout": "todo",    "title": "Fazer",      "icon": "A", "hl_group": "Normal",          "hidden": false},
            {"num": 2, "callout": "quote",   "title": "Depois",     "icon": "B", "hl_group": "Comment",         "hidden": true},
            {"num": 3, "callout": "check",   "title": "Feita",      "icon": "C", "hl_group": "DiagnosticOk",    "hidden": false},
        ])
    );
}

#[test]
fn statuses_le_o_vocabulario_a_cada_execucao() {
    let home = home_com_vault_vazio();

    let primeira = roda(home.path(), &["statuses"]);
    ok(&primeira);
    assert_eq!(primeira.stdout, STATUSES_DEFAULT_TEXTO);

    escreve_status_json(home.path(), STATUS_JSON_DUAS_FASES);
    let segunda = roda(home.path(), &["statuses"]);
    ok(&segunda);
    assert_eq!(segunda.stdout, STATUSES_DUAS_FASES_TEXTO);

    escreve_status_json(
        home.path(),
        r#"{ "1": { "title": "Única", "callout": "todo", "icon": "A", "hl_group": "Normal" } }"#,
    );
    let terceira = roda(home.path(), &["statuses"]);
    ok(&terceira);
    assert_eq!(
        terceira.stdout,
        "0  invalid    Sem status\n1  todo       Única\n"
    );
}

#[test]
fn statuses_nao_cria_status_json_num_vault_vazio() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses"]);
    ok(&saida);
    assert_eq!(
        entradas(&vault(home.path())),
        Vec::<String>::new(),
        "ObsidianVault/ segue vazio"
    );
    assert!(!vault(home.path()).join(".freitask").exists());
    assert!(!vault(home.path()).join(".freitask/status.json").exists());
    assert!(!vault(home.path()).join("projects").exists());
}

#[test]
fn statuses_json_nao_cria_status_json_num_vault_vazio() {
    let home = home_com_vault_vazio();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert_eq!(
        entradas(&vault(home.path())),
        Vec::<String>::new(),
        "ObsidianVault/ segue vazio"
    );
    assert!(!vault(home.path()).join(".freitask/status.json").exists());
}

#[test]
fn statuses_nao_cria_o_vault_num_home_sem_vault() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["statuses"]);
    ok(&saida);
    assert!(
        !vault(home.path()).exists(),
        "statuses não cria ObsidianVault/"
    );
    assert_eq!(
        entradas(home.path()),
        Vec::<String>::new(),
        "statuses não cria nada no HOME"
    );
}

#[test]
fn statuses_json_nao_cria_o_vault_num_home_sem_vault() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["statuses", "--json"]);
    ok(&saida);
    assert!(!vault(home.path()).exists());
    assert_eq!(entradas(home.path()), Vec::<String>::new());
}

#[test]
fn statuses_nao_muda_byte_nem_mtime_do_vault() {
    let home = home_com_status_json(STATUS_JSON_DUAS_FASES);
    let antes = fotografa(home.path());
    assert!(antes.contains_key(std::path::Path::new("ObsidianVault/.freitask/status.json")));

    ok(&roda(home.path(), &["statuses"]));
    ok(&roda(home.path(), &["statuses", "--json"]));

    assert_eq!(fotografa(home.path()), antes);
}

// ---------------------------------------------------------------------------
// `freitask version [--json]`
// ---------------------------------------------------------------------------

#[test]
fn version_imprime_a_versao_numa_linha_e_sai_0() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["version"]);
    ok(&saida);
    assert_eq!(saida.stdout, "freitask 2.0.0\n");
}

#[test]
fn version_json_devolve_freitask_e_json_1() {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["version", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!({"freitask": "2.0.0", "json": 1}));
}

#[test]
fn version_e_leitura_pura_num_home_sem_vault() {
    let home = home_sem_vault();
    ok(&roda(home.path(), &["version"]));
    ok(&roda(home.path(), &["version", "--json"]));
    assert!(
        !vault(home.path()).exists(),
        "version não cria ObsidianVault/"
    );
    assert_eq!(
        entradas(home.path()),
        Vec::<String>::new(),
        "version não cria nada no HOME"
    );
}

#[test]
fn version_num_vault_vazio_nao_cria_nada_nele() {
    let home = home_com_vault_vazio();
    ok(&roda(home.path(), &["version"]));
    assert_eq!(entradas(&vault(home.path())), Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// `FREITASK_TODAY`
// ---------------------------------------------------------------------------

#[test]
fn freitask_today_e_aceito_pelas_tres_leituras_sem_aviso() {
    // `roda` sempre passa FREITASK_TODAY=2026-11-08 (spec §1.6): nenhuma das
    // leituras pode recusá-lo nem reclamar em stderr.
    let home = home_com_vault_vazio();
    for args in [
        &["kebab", "Configuração da API"][..],
        &["statuses"][..],
        &["statuses", "--json"][..],
        &["version"][..],
        &["version", "--json"][..],
    ] {
        let saida = roda(home.path(), args);
        assert_eq!(
            saida.codigo,
            Some(0),
            "freitask {args:?} com FREITASK_TODAY={HOJE}"
        );
        assert_eq!(
            saida.stderr, "",
            "freitask {args:?} não avisa nada em stderr"
        );
        assert!(!saida.stdout.is_empty(), "freitask {args:?} imprime algo");
    }
}

#[test]
fn freitask_today_de_outro_dia_nao_muda_as_leituras_desta_fatia() {
    // O vocabulário e a versão não dependem do dia; só o painel e as adiadas
    // (fatias seguintes) dependem.
    let home = home_com_vault_vazio();
    let mut cmd = comando(home.path());
    cmd.env("FREITASK_TODAY", "2031-02-28").args(["statuses"]);
    let saida = cmd.output().expect("rodar");
    assert_eq!(saida.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(saida.stdout).expect("utf-8"),
        STATUSES_DEFAULT_TEXTO
    );
    assert_eq!(String::from_utf8(saida.stderr).expect("utf-8"), "");
}
