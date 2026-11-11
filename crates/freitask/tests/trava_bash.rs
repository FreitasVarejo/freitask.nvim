//! `scripts/trava-bash.sh` (spec §4, camada 3): o que o hook deixa passar de
//! propósito e o que ele nega.
//!
//! Os casos de "passa" afirmam o **limite registrado** do hook: ele casa texto
//! na linha de comando, não o caminho resolvido, e por isso glob, aspas,
//! variável e `cd` seguido de caminho curto escrevem no aceite congelado sem
//! serem negados. A decisão foi registrar o limite (spec §4, item 3, e
//! `docs/decisoes-da-reescrita.md`), não fechá-lo; quem cobre essas escritas
//! são as camadas 2 e 5. Se um destes testes passar a falhar porque o hook
//! ficou mais estrito, o limite mudou e a §4 tem de acompanhar.
//!
//! O aceite congelado já cobre a negação básica (`tests/aceite/f0-fundacao`);
//! este arquivo fica fora dele de propósito, porque o congelado não se toca.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn raiz() -> PathBuf {
    PathBuf::from(format!("{}/../..", env!("CARGO_MANIFEST_DIR")))
}

/// O código de saída da trava para um comando, montado como o hook o recebe.
fn codigo(comando: &str) -> i32 {
    let entrada = serde_json::json!({ "tool_input": { "command": comando } }).to_string();
    let script = raiz().join("scripts/trava-bash.sh");
    // Por `bash`, e não pelo bit de execução: a cópia de trabalho do
    // cargo-mutants (27.1) não o preserva, e o baseline da mutação quebrava
    // aqui (decisão 70).
    let mut filho = Command::new("bash")
        .arg(&script)
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
    saida.status.code().unwrap_or(-1)
}

fn afirma_passa(comandos: &[&str]) {
    for c in comandos {
        assert_eq!(codigo(c), 0, "a trava devia deixar passar `{c}`");
    }
}

fn afirma_nega(comandos: &[&str]) {
    for c in comandos {
        assert_eq!(codigo(c), 2, "a trava devia negar `{c}`");
    }
}

// ---------------------------------------------------------------------------
// Limite registrado na §4, camada 3: o hook casa texto, não caminho resolvido.
// ---------------------------------------------------------------------------

#[test]
fn limite_glob_passa() {
    afirma_passa(&[
        "cp x tests/ac*e/f10a/util.rs",
        "echo 'testes 0' > tests/ac*e/contagem.baseline",
        "rm -rf tests/ac*e/f10a",
        "cat tests/acei?e/f0-fundacao/main.rs",
        "cat tests/[a]ceite/f0-fundacao/main.rs",
        "cat tes*/aceite/f0-fundacao/main.rs",
    ]);
}

#[test]
fn limite_aspas_e_barra_invertida_passam() {
    afirma_passa(&[
        r#"cat tests/ace""ite/f0-fundacao/main.rs"#,
        "cat 'tests/ace'ite/f0-fundacao/main.rs",
        r"cat tests/ace\ite/f0-fundacao/main.rs",
    ]);
}

#[test]
fn limite_variavel_passa() {
    afirma_passa(&[
        "d=tests/ace; cp x ${d}ite/f10a/util.rs",
        "a=aceite; cp x tests/$a/f10a/util.rs",
    ]);
}

#[test]
fn limite_barras_duplas_e_ponto_passam() {
    afirma_passa(&[
        "cat tests//aceite/f0-fundacao/main.rs",
        "cat tests/./aceite/f0-fundacao/main.rs",
    ]);
}

#[test]
fn limite_cd_seguido_de_caminho_curto_passa() {
    afirma_passa(&[
        "cd tests && cp x aceite/f10a/util.rs",
        "cd tests; echo 'testes 0' > aceite/contagem.baseline",
        "cd tests && sed -i 's/a/b/' aceite/f10a/util.rs",
    ]);
}

#[test]
fn limite_sed_in_place_e_perl_pi_passam() {
    afirma_passa(&[
        "sed --in-place 's/a/b/' tests/ac*e/f10a/util.rs",
        "perl -pi -e 's/a/b/' tests/ac*e/f10a/util.rs",
    ]);
}

#[test]
fn limite_find_exec_e_substituicao_de_comando_passam() {
    afirma_passa(&[
        r"find tests -path '*ceite*' -name util.rs -exec cp x {} \;",
        r#"cp x "$(printf tests/ace)ite/f10a/util.rs""#,
    ]);
}

#[test]
fn limite_git_checkout_com_glob_passa() {
    afirma_passa(&["git checkout HEAD -- tests/ac*e"]);
}

// ---------------------------------------------------------------------------
// Exceção: chamada única de `freitask` não é negada pelo texto.
// ---------------------------------------------------------------------------

#[test]
fn freitask_new_com_o_texto_no_titulo_passa() {
    afirma_passa(&[r#"freitask new workflow-ia x "Trava: o hook nega o texto tests/aceite""#]);
}

#[test]
fn freitask_set_com_o_texto_na_descricao_passa() {
    afirma_passa(&[r#"freitask set x --desc "o hook nega o texto tests/aceite""#]);
}

// ---------------------------------------------------------------------------
// O que continua negado.
// ---------------------------------------------------------------------------

#[test]
fn freitask_encadeado_com_escrita_no_aceite_e_negado() {
    afirma_nega(&[
        r#"freitask set x --desc "tests/aceite" && cp /tmp/x.rs tests/aceite/f10a/util.rs"#,
    ]);
}

#[test]
fn freitask_com_redirecionamento_para_o_aceite_e_negado() {
    afirma_nega(&[r#"freitask set x --desc "a" > tests/aceite/x"#]);
}

#[test]
fn leitura_literal_do_aceite_e_negada() {
    afirma_nega(&["cat tests/aceite/f0-fundacao/main.rs"]);
}

#[test]
fn a_excecao_do_freitask_so_vale_para_o_texto_do_aceite() {
    afirma_nega(&[r#"freitask set x --desc "cargo insta accept""#]);
}
