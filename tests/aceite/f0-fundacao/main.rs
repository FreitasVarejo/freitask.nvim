//! Fatia 0 (Fundação): as travas da spec §4 existem e funcionam.
//!
//! Estes testes não rodam o binário `freitask`: o objeto deles são os
//! artefatos do repo (`.claude/settings.json`, `scripts/trava-bash.sh`,
//! `.cargo/mutants.toml`, `Makefile`, `scripts/hash-aceite.sh`, a tag
//! `oraculo-lua`) e a lista `divergencias.toml` da raiz de `tests/aceite/`.
//! Ficam verdes desde já porque as travas nascem antes deles
//! (`docs/decisoes-da-reescrita.md`, item 10).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

fn raiz() -> PathBuf {
    PathBuf::from(
        std::env::var("FREITASK_REPO_RAIZ")
            .unwrap_or_else(|_| format!("{}/../..", env!("CARGO_MANIFEST_DIR"))),
    )
}

fn aceite_raiz() -> PathBuf {
    PathBuf::from(
        std::env::var("FREITASK_ACEITE_RAIZ")
            .unwrap_or_else(|_| format!("{}/tests/aceite", raiz().display())),
    )
}

fn ler(rel: &str) -> String {
    let caminho = raiz().join(rel);
    std::fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {}: {e}", caminho.display()))
}

// ---------------------------------------------------------------------------
// .claude/settings.json (spec §4, camada 3)
// ---------------------------------------------------------------------------

fn settings() -> Value {
    serde_json::from_str(&ler(".claude/settings.json"))
        .expect(".claude/settings.json tem de ser JSON válido")
}

fn deny() -> Vec<String> {
    settings()["permissions"]["deny"]
        .as_array()
        .expect("permissions.deny tem de ser um array")
        .iter()
        .map(|v| v.as_str().expect("entrada de deny é string").to_string())
        .collect()
}

fn afirma_deny_cobre(padrao: &str) {
    let lista = deny();
    for ferramenta in ["Edit", "Write"] {
        let esperado = format!("{ferramenta}({padrao})");
        assert!(
            lista.contains(&esperado),
            "permissions.deny não tem `{esperado}`; tem: {lista:?}"
        );
    }
}

#[test]
fn settings_json_e_json_valido_com_permissions_e_hooks() {
    let s = settings();
    assert!(s.is_object(), "settings.json tem de ser um objeto");
    assert!(s.get("permissions").is_some(), "falta `permissions`");
    assert!(s.get("hooks").is_some(), "falta `hooks`");
}

#[test]
fn settings_nega_edit_e_write_em_tests_aceite() {
    afirma_deny_cobre("tests/aceite/**");
}

#[test]
fn settings_nega_edit_e_write_em_snapshots() {
    afirma_deny_cobre("**/snapshots/**");
}

#[test]
fn settings_nega_edit_e_write_nos_goldens_do_diferencial() {
    afirma_deny_cobre("tests/diferencial/goldens/**");
}

#[test]
fn settings_tem_hook_pretooluse_em_bash_apontando_para_trava_bash() {
    let s = settings();
    let pre = s["hooks"]["PreToolUse"]
        .as_array()
        .expect("hooks.PreToolUse tem de ser um array");
    let de_bash: Vec<&Value> = pre
        .iter()
        .filter(|h| h["matcher"].as_str() == Some("Bash"))
        .collect();
    assert!(
        !de_bash.is_empty(),
        "nenhum PreToolUse com matcher `Bash`: {pre:?}"
    );
    let aponta = de_bash.iter().any(|h| {
        h["hooks"].as_array().is_some_and(|hs| {
            hs.iter().any(|x| {
                x["type"].as_str() == Some("command")
                    && x["command"]
                        .as_str()
                        .is_some_and(|c| c.contains("scripts/trava-bash.sh"))
            })
        })
    });
    assert!(
        aponta,
        "o PreToolUse de Bash não tem um hook `command` para scripts/trava-bash.sh: {de_bash:?}"
    );
}

// ---------------------------------------------------------------------------
// scripts/trava-bash.sh
// ---------------------------------------------------------------------------

struct Veredito {
    codigo: i32,
    stderr: String,
}

fn trava(comando: &str) -> Veredito {
    let entrada = serde_json::json!({ "tool_input": { "command": comando } }).to_string();
    trava_com_stdin(&entrada)
}

fn trava_com_stdin(entrada: &str) -> Veredito {
    let script = raiz().join("scripts/trava-bash.sh");
    let mut filho = Command::new(&script)
        .current_dir(raiz())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("não consegui rodar {}: {e}", script.display()));
    filho
        .stdin
        .take()
        .expect("stdin")
        .write_all(entrada.as_bytes())
        .expect("escrever no stdin da trava");
    let saida = filho.wait_with_output().expect("esperar a trava");
    Veredito {
        codigo: saida.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&saida.stderr).to_string(),
    }
}

fn afirma_nega(comando: &str) {
    let v = trava(comando);
    assert_eq!(
        v.codigo, 2,
        "a trava devia sair 2 para `{comando}`; saiu {} (stderr: {:?})",
        v.codigo, v.stderr
    );
}

fn afirma_deixa_passar(comando: &str) {
    let v = trava(comando);
    assert_eq!(
        v.codigo, 0,
        "a trava devia sair 0 para `{comando}`; saiu {} (stderr: {:?})",
        v.codigo, v.stderr
    );
}

#[test]
fn trava_bash_nega_comando_que_menciona_tests_aceite() {
    afirma_nega("cat tests/aceite/f0-fundacao/main.rs");
}

#[test]
fn trava_bash_nega_cargo_insta_accept() {
    afirma_nega("cargo insta accept");
}

#[test]
fn trava_bash_nega_cargo_insta_review() {
    afirma_nega("cargo insta review");
}

#[test]
fn trava_bash_nega_insta_update_yes() {
    afirma_nega("INSTA_UPDATE=yes cargo test --workspace");
}

#[test]
fn trava_bash_nega_insta_force_pass() {
    afirma_nega("INSTA_FORCE_PASS=1 cargo test --workspace");
}

#[test]
fn trava_bash_nega_flag_accept() {
    afirma_nega("cargo insta test --accept");
}

#[test]
fn trava_bash_nega_sed_i_sobre_tests() {
    afirma_nega("sed -i 's/a/b/' tests/x.rs");
}

#[test]
fn trava_bash_nega_git_checkout_de_tests_aceite() {
    afirma_nega("git checkout -- tests/aceite");
}

#[test]
fn trava_bash_nega_git_restore_de_tests_aceite() {
    afirma_nega("git restore tests/aceite");
}

#[test]
fn trava_bash_explica_a_negacao_no_stderr() {
    let v = trava("cargo insta accept");
    assert_eq!(v.codigo, 2);
    assert!(
        v.stderr.contains("trava") && v.stderr.contains("§4"),
        "a razão da negação tem de ir para o stderr citando a trava e a §4; veio: {:?}",
        v.stderr
    );
}

#[test]
fn trava_bash_deixa_passar_cargo_test_workspace() {
    afirma_deixa_passar("cargo test --workspace");
}

#[test]
fn trava_bash_deixa_passar_make_aceite() {
    afirma_deixa_passar("make aceite");
}

#[test]
fn trava_bash_deixa_passar_ls_crates() {
    afirma_deixa_passar("ls crates");
}

#[test]
fn trava_bash_deixa_passar_hook_sem_comando() {
    let v = trava_com_stdin("{}");
    assert_eq!(
        v.codigo, 0,
        "sem `tool_input.command` a trava não tem o que negar; saiu {} (stderr: {:?})",
        v.codigo, v.stderr
    );
}

// ---------------------------------------------------------------------------
// .cargo/mutants.toml (spec §3.6)
// ---------------------------------------------------------------------------

#[test]
fn mutants_toml_existe_e_exclui_o_main_rs_do_binario() {
    let texto = ler(".cargo/mutants.toml");
    let tabela: toml::Table =
        toml::from_str(&texto).expect(".cargo/mutants.toml tem de ser TOML válido");
    let excluidos: Vec<&str> = tabela["exclude_globs"]
        .as_array()
        .expect("exclude_globs tem de ser um array")
        .iter()
        .map(|v| v.as_str().expect("glob é string"))
        .collect();
    assert!(
        excluidos.contains(&"crates/freitask/src/main.rs"),
        "exclude_globs não tem crates/freitask/src/main.rs: {excluidos:?}"
    );
}

// ---------------------------------------------------------------------------
// Makefile
// ---------------------------------------------------------------------------

fn makefile_tem_alvo(alvo: &str) {
    let prefixo = format!("{alvo}:");
    let tem = ler("Makefile").lines().any(|l| l.starts_with(&prefixo));
    assert!(tem, "o Makefile não tem o alvo `{alvo}`");
}

fn makefile_exporta(variavel: &str, valor: &str) {
    let esperado = format!("export {variavel} := {valor}");
    let tem = ler("Makefile").lines().any(|l| {
        let normalizada = l.split_whitespace().collect::<Vec<_>>().join(" ");
        normalizada == esperado
    });
    assert!(tem, "o Makefile não tem `{esperado}`");
}

#[test]
fn makefile_tem_alvo_check() {
    makefile_tem_alvo("check");
}

#[test]
fn makefile_tem_alvo_test() {
    makefile_tem_alvo("test");
}

#[test]
fn makefile_tem_alvo_aceite() {
    makefile_tem_alvo("aceite");
}

#[test]
fn makefile_tem_alvo_diff() {
    makefile_tem_alvo("diff");
}

#[test]
fn makefile_tem_alvo_mutantes() {
    makefile_tem_alvo("mutantes");
}

#[test]
fn makefile_tem_alvo_portoes() {
    makefile_tem_alvo("portoes");
}

#[test]
fn makefile_exporta_insta_update_no() {
    makefile_exporta("INSTA_UPDATE", "no");
}

#[test]
fn makefile_exporta_insta_force_pass_0() {
    makefile_exporta("INSTA_FORCE_PASS", "0");
}

// ---------------------------------------------------------------------------
// scripts/hash-aceite.sh (spec §4, camada 2)
// ---------------------------------------------------------------------------

#[test]
fn hash_aceite_conferir_sai_0() {
    let saida = Command::new(raiz().join("scripts/hash-aceite.sh"))
        .arg("--conferir")
        .current_dir(raiz())
        .output()
        .expect("rodar scripts/hash-aceite.sh --conferir");
    assert_eq!(
        saida.status.code(),
        Some(0),
        "hash-aceite.sh --conferir falhou: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&saida.stdout),
        String::from_utf8_lossy(&saida.stderr)
    );
}

// ---------------------------------------------------------------------------
// git tag oraculo-lua (spec §3.4 e §5, fatia 0)
// ---------------------------------------------------------------------------

fn git(args: &[&str]) -> String {
    let saida = Command::new("git")
        .args(args)
        .current_dir(raiz())
        .output()
        .expect("rodar git");
    assert!(
        saida.status.success(),
        "git {args:?} falhou: {}",
        String::from_utf8_lossy(&saida.stderr)
    );
    String::from_utf8_lossy(&saida.stdout).trim().to_string()
}

#[test]
fn tag_oraculo_lua_existe() {
    assert_eq!(git(&["tag", "-l", "oraculo-lua"]), "oraculo-lua");
}

#[test]
fn tag_oraculo_lua_aponta_para_o_commit_de_partida() {
    let sha = git(&["rev-parse", "oraculo-lua^{commit}"]);
    assert!(
        sha.starts_with("5ee9721"),
        "oraculo-lua devia apontar para 5ee9721…; aponta para {sha}"
    );
}

// ---------------------------------------------------------------------------
// tests/aceite/divergencias.toml (spec §3.4)
// ---------------------------------------------------------------------------

fn divergencias() -> Vec<toml::Table> {
    let caminho = aceite_raiz().join("divergencias.toml");
    let texto = std::fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {}: {e}", caminho.display()));
    let tabela: toml::Table =
        toml::from_str(&texto).expect("divergencias.toml tem de ser TOML válido");
    tabela["divergencia"]
        .as_array()
        .expect("`[[divergencia]]` tem de ser um array de tabelas")
        .iter()
        .map(|v| v.as_table().expect("cada divergência é uma tabela").clone())
        .collect()
}

fn ids_esperados() -> BTreeSet<String> {
    (1..=10).map(|n| format!("D{n}")).collect()
}

fn texto_nao_vazio(entrada: &toml::Table, chave: &str) -> String {
    let id = entrada.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let v = entrada
        .get(chave)
        .unwrap_or_else(|| panic!("{id}: falta `{chave}`"));
    let s = v
        .as_str()
        .unwrap_or_else(|| panic!("{id}: `{chave}` tem de ser string, é {v:?}"));
    assert!(!s.trim().is_empty(), "{id}: `{chave}` está vazio");
    s.to_string()
}

#[test]
fn divergencias_toml_parseia_e_tem_ao_menos_uma_entrada() {
    assert!(
        !divergencias().is_empty(),
        "divergencias.toml sem nenhuma `[[divergencia]]`"
    );
}

#[test]
fn divergencias_tem_exatamente_d1_a_d10() {
    let entradas = divergencias();
    let ids: Vec<String> = entradas.iter().map(|e| texto_nao_vazio(e, "id")).collect();
    assert_eq!(
        ids.len(),
        10,
        "esperava 10 entradas, há {}: {ids:?}",
        ids.len()
    );
    let conjunto: BTreeSet<String> = ids.iter().cloned().collect();
    assert_eq!(conjunto, ids_esperados(), "ids fora do esperado: {ids:?}");
}

#[test]
fn divergencias_cada_entrada_tem_id_motivo_teste_cenario_e_onde() {
    for e in divergencias() {
        texto_nao_vazio(&e, "id");
        texto_nao_vazio(&e, "motivo");
        texto_nao_vazio(&e, "cenario");
        texto_nao_vazio(&e, "onde");
        let id = e["id"].as_str().unwrap_or("?");
        let teste = e.get("teste").unwrap_or_else(|| {
            panic!("{id}: falta `teste` (pode ser \"\" quando a §3.7 não o nomeia)")
        });
        assert!(
            teste.is_str(),
            "{id}: `teste` tem de ser string, é {teste:?}"
        );
        if let Some(d) = e.get("detalhe") {
            let s = d
                .as_str()
                .unwrap_or_else(|| panic!("{id}: `detalhe` tem de ser string, é {d:?}"));
            assert!(
                !s.trim().is_empty(),
                "{id}: `detalhe` presente mas vazio; omita-o"
            );
        }
        if let Some(o) = e.get("observavel") {
            assert!(o.is_bool(), "{id}: `observavel` tem de ser bool, é {o:?}");
        }
    }
}

#[test]
fn divergencias_teste_e_o_nome_da_tabela_da_secao_3_7() {
    let esperado: Vec<(&str, &str)> = vec![
        ("D1", "link_em_tabela_acompanha_o_archive_o_move_e_o_rename"),
        ("D2", "ponto_e_ponto_ponto_nao_sao_projeto"),
        ("D3", "encerrar_recusa_sync_conflict_e_fora_do_padrao"),
        ("D4", "crlf_sobrevive_ao_set"),
        ("D5", ""),
        ("D6", "rebuild_sem_mudanca_nao_grava"),
        ("D7", "doctor_sem_fix_nao_cria_nada"),
        ("D8", "help_por_subcomando_nao_executa"),
        ("D9", ""),
        ("D10", "new_json_devolve_caminho_projeto_e_id"),
    ];
    let entradas = divergencias();
    for (id, teste) in esperado {
        let e = entradas
            .iter()
            .find(|e| e["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("falta {id}"));
        assert_eq!(
            e["teste"].as_str(),
            Some(teste),
            "{id}: `teste` devia ser {teste:?} (spec §3.7)"
        );
    }
}

#[test]
fn divergencias_nao_observaveis_sao_so_d6_e_d9() {
    let nao_observaveis: BTreeSet<String> = divergencias()
        .iter()
        .filter(|e| e.get("observavel").and_then(|v| v.as_bool()) == Some(false))
        .map(|e| e["id"].as_str().unwrap_or("?").to_string())
        .collect();
    let esperado: BTreeSet<String> = ["D6", "D9"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        nao_observaveis, esperado,
        "só D6 (mtime) e D9 (ordem de chaves) são invisíveis ao harness"
    );
}

#[test]
fn divergencias_cenario_e_onde_sao_regex_sem_barra_de_fechamento_solta() {
    // Sem a crate `regex` no rascunho, a conferência fica nos sinais mais
    // comuns de regex quebrada: parênteses e colchetes desbalanceados.
    fn balanceado(s: &str, abre: char, fecha: char) -> bool {
        let mut n = 0i32;
        let mut escapado = false;
        for c in s.chars() {
            if escapado {
                escapado = false;
                continue;
            }
            if c == '\\' {
                escapado = true;
            } else if c == abre {
                n += 1;
            } else if c == fecha {
                n -= 1;
                if n < 0 {
                    return false;
                }
            }
        }
        n == 0
    }
    for e in divergencias() {
        let id = e["id"].as_str().unwrap_or("?");
        for chave in ["cenario", "onde", "detalhe"] {
            let Some(s) = e.get(chave).and_then(|v| v.as_str()) else {
                continue;
            };
            assert!(
                balanceado(s, '(', ')'),
                "{id}: `{chave}` com parênteses desbalanceados: {s}"
            );
            assert!(
                balanceado(s, '[', ']'),
                "{id}: `{chave}` com colchetes desbalanceados: {s}"
            );
        }
    }
}
