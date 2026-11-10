//! Aceite da fatia 10a — Prazo, parte da CLI (spec §5, §10.2; decisão 54 do
//! log): `prazo:` no frontmatter, `set --prazo AAAA-MM-DD` (`""` limpa; data
//! inválida sai 2 sem gravar nada do comando), `prazo` e `dias_para_o_prazo`
//! no `list --json` e no `show --json`, `prazo: … (em N dias)` / `(vencido há
//! N dias)` no `show` em texto, os kinds `prazo-invalido` e
//! `prazo-antes-da-volta` no `doctor`, I11 na parte da CLI e os dois testes
//! nomeados da §3.7 (`prazo_invalido_nao_conta_dias_e_o_doctor_acusa`,
//! `prazo_nao_muda_a_ordem_do_painel`). A TUI (tecla `p`, prazo na tela) é a
//! fatia 10b e não entra aqui.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§2.1, §2.2, §2.4, §3.2 I6/I11, §3.3, §3.7, §5, §10.2), `docs/freitask.md`
//! ("Volta em", "CLI", "`freitask doctor`"), `docs/decisoes-da-reescrita.md`
//! (17, 19, 31, 54) e os aceites congelados das fatias 2, 3 e 5, de onde vêm o
//! formato do painel, a mensagem de data inválida, o texto do `doctor` e os
//! fixtures de `set --volta-em` que o `--prazo` espelha. O comportamento do
//! `--volta-em` e do `volta-em-invalido` foi conferido no oráculo Lua
//! (`lua/freitask/`) num HOME temporário. Nenhum valor esperado aqui foi
//! gerado do binário sob teste. O dia fixo é `FREITASK_TODAY=2026-11-08`, um
//! domingo, salvo onde o teste diz outro.
//!
//! Decisões tomadas aqui, onde a spec não fecha (repetidas no relatório do
//! rascunho). Quando havia espelho medido no `--volta-em`, valeu o espelho:
//! - `set --prazo` com data inválida sai 2 com a mesma mensagem do
//!   `--volta-em`: `data inválida: "<valor>" (use AAAA-MM-DD, de um dia que
//!   existe; "" limpa)`; `--prazo` sem valor sai 2 com `falta o valor de
//!   --prazo` (decisão 19). Com `--prazo` e `--volta-em` ambos inválidos,
//!   não se fixa qual dos dois é acusado, só a saída 2, uma linha
//!   `freitask: data inválida: …` e nada gravado;
//! - `set --prazo` só mexe no frontmatter: acrescenta `prazo:` no fim dele (ou
//!   o cria antes do bloco, com a linha em branco), troca a data no lugar,
//!   `""` tira a linha e tira o frontmatter que esvazia; o bloco não é
//!   reserializado; nenhuma linha vai para o `## Histórico`; o stdout é o
//!   caminho absoluto da task, como no `--volta-em`; a mesma data, ou `""` sem
//!   o campo, não toca o arquivo; CRLF sobrevive (D4);
//! - prazo no passado e 29/02 de ano bissexto são datas válidas e se gravam
//!   (o `parse_date` só pede um dia que existe);
//! - no JSON, `prazo` é o texto do frontmatter (o inválido também); `prazo:`
//!   vazio conta como sem prazo, `prazo: null` (no oráculo, `meta.read` trata
//!   `volta-em:` vazio como ausente). `dias_para_o_prazo` é `null` nos três
//!   casos da I11 (ausente, vazio, inválido); `prazo` e `dias` de task
//!   arquivada não se fixam;
//! - `show` em texto: o bloco como hoje e, depois dele, uma linha
//!   `prazo: AAAA-MM-DD (em N dias)` ou `prazo: AAAA-MM-DD (vencido há N
//!   dias)`. Linha em branco entre os dois não se fixa. Só se fixa N ≥ 2: o
//!   singular (`1 dia`) e o prazo de hoje (`em 0 dias`?) a spec não diz.
//!   Prazo vazio imprime só o bloco, como sem prazo; prazo inválido sai 0, com
//!   o bloco, e nenhuma linha conta dias (o texto dela não se fixa);
//! - `doctor`: o `msg` dos dois kinds novos não se fixa (só que não é vazio e
//!   que o texto do `doctor` o imprime como os outros achados); fixa-se
//!   `level` `warn`, `kind`, `path` e `fixed: false` também com `--fix`, que
//!   não toca a task. Vazio não é achado (como o `volta-em:` vazio, medido no
//!   oráculo); prazo vencido também não. `prazo-antes-da-volta` só compara
//!   duas datas válidas (`prazo < volta-em`, estrito: no mesmo dia não é
//!   achado); com uma delas inválida sai só o `*-invalido` dela. Com
//!   `volta-em` já passada, não se fixa. Task arquivada não é olhada, como o
//!   `volta-em-invalido` hoje (`doctor_nao_olha_prioridade_nem_volta_em_de_
//!   task_arquivada`, fatia 5);
//! - a ordem em texto de dois achados da mesma task não se fixa; o JSON é
//!   comparado ordenado (decisão 7);
//! - I11 "nem esconde nem mostra": Algum dia e adiada seguem `hidden` com
//!   prazo vencido; a linha do `list --json` (fora `prazo`, `dias` e o
//!   prefixo do HOME no `path`) e a ordem são as mesmas com e sem prazo; o
//!   `CURRENT.md` é byte a byte o mesmo (§3.7), e o esperado está escrito à
//!   mão no formato fixado na fatia 2. O texto do `list` sem `--json` não se
//!   compara: a spec não diz se ele mostra o prazo;
//! - `--prazo=AAAA-MM-DD` vale (§2.1: `--x=valor` vale para toda flag);
//! - `--help`/`-h` vence `--prazo`, inclusive com data inválida: sai 0, imprime
//!   o trecho do `set` (só a primeira linha se fixa aqui; o resto é da fatia
//!   6) e não toca o vault (I6).
//!
//! Fica de fora: a TUI (10b), `--bloqueada-por` (9a), o diferencial e a saída
//! 3 por lock (fatia 3).

mod util;

use std::path::Path;

use serde_json::{Value, json};
use util::*;

// ===========================================================================
// Fixtures
// ===========================================================================

const CURRENT: &str = "CURRENT.md";

/// Um vault com `status.json` default, `projects/` e os arquivos dados,
/// envelhecido (mtime no passado), sem `CURRENT.md`.
fn home_pequeno(arquivos: &[(&str, &str)]) -> tempfile::TempDir {
    let home = home_com(arquivos);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects");
    envelhece(home.path());
    home
}

/// O mesmo, com os arquivos em `String`.
fn home_pequeno_s(arquivos: &[(String, String)]) -> tempfile::TempDir {
    let refs: Vec<(&str, &str)> = arquivos
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    home_pequeno(&refs)
}

/// Uma task: frontmatter com as linhas dadas (nenhum se vazio) e o bloco.
fn task(fm: &[&str], bloco: &str) -> String {
    if fm.is_empty() {
        return bloco.to_owned();
    }
    let mut s = String::from("---\n");
    for l in fm {
        s.push_str(l);
        s.push('\n');
    }
    s.push_str("---\n\n");
    s.push_str(bloco);
    s
}

/// O bloco mínimo de uma task ativa.
fn bloco(callout: &str, titulo: &str, projeto: &str, id: &str) -> String {
    format!("> [!{callout}] {titulo}\n> [[projects/{projeto}/tasks/{id}|{id}]]\n")
}

/// `projects/<p>/tasks/<id>.md`.
fn rel(projeto: &str, id: &str) -> String {
    format!("projects/{projeto}/tasks/{id}.md")
}

/// A linha do `list --json` com esse `id` (tem de haver exatamente uma).
fn linha<'a>(linhas: &'a Value, id: &str) -> &'a Value {
    let achadas: Vec<&Value> = linhas
        .as_array()
        .unwrap_or_else(|| panic!("o list --json tem de ser array: {linhas}"))
        .iter()
        .filter(|l| l["id"] == json!(id))
        .collect();
    assert_eq!(achadas.len(), 1, "linhas com id {id}: {linhas}");
    achadas[0]
}

/// Os ids do `list --json`, na ordem.
fn ids(linhas: &Value) -> Vec<String> {
    linhas
        .as_array()
        .expect("array")
        .iter()
        .map(|l| l["id"].as_str().expect("id string").to_owned())
        .collect()
}

/// `list --json`: saída 0, stderr vazio, valida contra o schema.
fn list_json(home: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["list", "--json"];
    args.extend_from_slice(extra);
    let saida = roda(home, &args);
    ok(&saida);
    let v = json(&saida);
    valida("list", &v);
    v
}

/// `show <id> --json`: saída 0, stderr vazio, valida contra o schema.
fn show_json(home: &Path, id: &str) -> Value {
    let saida = roda(home, &["show", id, "--json"]);
    ok(&saida);
    let v = json(&saida);
    valida("show", &v);
    v
}

/// A mensagem de data inválida do `set` (a do `--volta-em`, medida no oráculo
/// e congelada na fatia 3).
fn msg_data(valor: &str) -> String {
    format!("data inválida: \"{valor}\" (use AAAA-MM-DD, de um dia que existe; \"\" limpa)")
}

// ---------------------------------------------------------------------------
// O vault do `set` (o da fatia 3, sem as arquivadas)
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

const X: &str = "projects/alfa/tasks/x.md";
const Y: &str = "projects/alfa/tasks/y.md";

/// `alfa` com `x` (todo) e `y` (example); `beta` vazio; `status.json` default;
/// sem `CURRENT.md`; tudo envelhecido.
fn home_base() -> tempfile::TempDir {
    let home = home_com(&[(X, X_MD), (Y, Y_MD)]);
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

/// O painel de 2026-11-08 com a contagem e as seções dadas (formato fixado
/// na fatia 2).
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

/// O painel do vault base.
fn painel_base() -> String {
    painel(
        2,
        2,
        concat!(
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
            "## beta\n",
            "\n",
            "_(sem tasks)_\n",
            "\n",
        ),
    )
}

/// `X_MD` com a linha dada acrescentada no fim do frontmatter.
fn x_com(linha: &str) -> String {
    X_MD.replace(
        "criado: 2026-01-01\n---\n",
        &format!("criado: 2026-01-01\n{linha}\n---\n"),
    )
}

// ===========================================================================
// set --prazo: grava
// ===========================================================================

#[test]
fn set_prazo_acrescenta_a_chave_no_fim_do_frontmatter() {
    let home = home_base();
    let saida = roda(home.path(), &["set", "x", "--prazo", "2026-12-01"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), X)));
    // Só a linha nova: nada no `## Histórico`, bloco e corpo intactos.
    assert_eq!(le(home.path(), X), x_com("prazo: 2026-12-01"));
}

#[test]
fn set_prazo_cria_o_frontmatter_quando_falta() {
    let home = home_base();
    let saida = roda(home.path(), &["set", "y", "--prazo", "2026-12-01"]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), Y)));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nprazo: 2026-12-01\n---\n\n{Y_MD}")
    );
}

#[test]
fn set_prazo_com_igual_tambem_vale() {
    // §2.1: `--x=valor` segue valendo para toda flag.
    let home = home_base();
    ok(&roda(home.path(), &["set", "y", "--prazo=2026-12-01"]));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nprazo: 2026-12-01\n---\n\n{Y_MD}")
    );
}

#[test]
fn set_prazo_troca_a_data_no_lugar() {
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nprazo: 2026-11-20\n"),
    );
    ok(&roda(home.path(), &["set", "x", "--prazo", "2026-12-02"]));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace("id: x\n", "id: x\nprazo: 2026-12-02\n")
    );
}

#[test]
fn set_prazo_troca_um_prazo_invalido_por_uma_data() {
    // É o conserto do `prazo-invalido` (o doctor não adivinha a data).
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nprazo: 2026-02-30\n"),
    );
    ok(&roda(home.path(), &["set", "x", "--prazo", "2026-12-02"]));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace("id: x\n", "id: x\nprazo: 2026-12-02\n")
    );
    let linhas = list_json(home.path(), &[]);
    assert_eq!(linha(&linhas, "x")["prazo"], json!("2026-12-02"));
    assert_eq!(linha(&linhas, "x")["dias_para_o_prazo"], json!(24));
}

#[test]
fn set_prazo_vazio_limpa_o_campo_e_mantem_as_outras_chaves() {
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nprazo: 2026-11-20\n"),
    );
    let saida = roda(home.path(), &["set", "x", "--prazo", ""]);
    ok(&saida);
    assert_eq!(saida.stdout, format!("{}\n", abs(home.path(), X)));
    assert_eq!(le(home.path(), X), X_MD);
}

#[test]
fn set_prazo_vazio_remove_o_frontmatter_que_esvazia() {
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        &format!("---\nprazo: 2026-12-01\n---\n\n{Y_MD}"),
    );
    ok(&roda(home.path(), &["set", "y", "--prazo", ""]));
    assert_eq!(le(home.path(), Y), Y_MD);
}

#[test]
fn set_prazo_vazio_nao_tira_a_volta_em() {
    // Cada campo tem o seu escritor: limpar o prazo não apaga a data de volta.
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        &format!("---\nvolta-em: 2026-12-01\nprazo: 2026-11-20\n---\n\n{Y_MD}"),
    );
    ok(&roda(home.path(), &["set", "y", "--prazo", ""]));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nvolta-em: 2026-12-01\n---\n\n{Y_MD}")
    );
}

#[test]
fn set_prazo_nao_mexe_na_volta_em_nem_a_volta_em_no_prazo() {
    let home = home_base();
    arquivo(
        home.path(),
        Y,
        &format!("---\nprazo: 2026-11-20\n---\n\n{Y_MD}"),
    );
    ok(&roda(
        home.path(),
        &["set", "y", "--volta-em", "2026-12-01"],
    ));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nprazo: 2026-11-20\nvolta-em: 2026-12-01\n---\n\n{Y_MD}")
    );
    ok(&roda(home.path(), &["set", "y", "--prazo", "2026-12-24"]));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nprazo: 2026-12-24\nvolta-em: 2026-12-01\n---\n\n{Y_MD}")
    );
}

#[test]
fn set_prazo_e_volta_em_juntos_gravam_os_dois() {
    // As duas chaves já existem: cada uma troca no lugar.
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace(
            "id: x\n",
            "id: x\nvolta-em: 2026-11-20\nprazo: 2026-11-21\n",
        ),
    );
    ok(&roda(
        home.path(),
        &[
            "set",
            "x",
            "--volta-em",
            "2026-12-01",
            "--prazo",
            "2026-12-20",
        ],
    ));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace(
            "id: x\n",
            "id: x\nvolta-em: 2026-12-01\nprazo: 2026-12-20\n"
        )
    );
}

#[test]
fn set_prazo_com_titulo_grava_os_dois() {
    let home = home_base();
    ok(&roda(
        home.path(),
        &[
            "set",
            "y",
            "--title",
            "Ypsilon novo",
            "--prazo",
            "2026-12-01",
        ],
    ));
    assert_eq!(
        le(home.path(), Y),
        "---\nprazo: 2026-12-01\n---\n\n> [!example] Ypsilon novo\n> [[projects/alfa/tasks/y|y]]\n"
    );
}

#[test]
fn set_prazo_so_mexe_no_frontmatter_e_nao_reserializa_o_bloco() {
    // O bloco com link dessincronizado e espaço sobrando fica byte a byte.
    let home = home_base();
    let bloco = "> [!example]   Ypsilon  \n> [[projects/alfa/tasks/outro|outro]]\n";
    arquivo(home.path(), Y, bloco);
    ok(&roda(home.path(), &["set", "y", "--prazo", "2026-12-01"]));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nprazo: 2026-12-01\n---\n\n{bloco}")
    );
}

#[test]
fn set_prazo_aceita_data_passada_e_29_de_fevereiro_bissexto() {
    let home = home_base();
    ok(&roda(home.path(), &["set", "x", "--prazo", "2026-01-01"]));
    assert_eq!(le(home.path(), X), x_com("prazo: 2026-01-01"));
    ok(&roda(home.path(), &["set", "x", "--prazo", "2028-02-29"]));
    assert_eq!(le(home.path(), X), x_com("prazo: 2028-02-29"));
}

#[test]
fn set_prazo_nao_muda_o_painel() {
    // O prazo mora no frontmatter, que o painel nunca copiou, e não muda a
    // ordem nem esconde: vencido ou não, o painel é o de antes.
    let home = home_base();
    ok(&roda(home.path(), &["set", "y", "--prazo", "2026-11-01"]));
    assert_eq!(le(home.path(), CURRENT), painel_base());
    ok(&roda(home.path(), &["set", "x", "--prazo", "2027-03-01"]));
    assert_eq!(le(home.path(), CURRENT), painel_base());
}

#[test]
fn set_prazo_so_grava_a_task_e_o_painel() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["set", "x", "--prazo", "2026-12-01"]));
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(
        mudou == conjunto(&[X]) || mudou == conjunto(&[X, CURRENT]),
        "só a task (e no máximo o painel) podia mudar: {mudou:?}"
    );
    assert_eq!(le(home.path(), CURRENT), painel_base());
}

#[test]
fn set_prazo_vazio_sem_o_campo_nao_toca_o_arquivo() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["set", "y", "--prazo", ""]));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_prazo_com_a_mesma_data_nao_toca_o_arquivo() {
    let home = home_base();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nprazo: 2026-12-01\n"),
    );
    ok(&roda(home.path(), &["rebuild"]));
    envelhece(home.path());
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["set", "x", "--prazo", "2026-12-01"]));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn crlf_sobrevive_ao_set_prazo() {
    let home = home_com_vault_vazio();
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    arquivo(
        home.path(),
        X,
        "---\r\nid: x\r\n---\r\n\r\n> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n",
    );
    arquivo(
        home.path(),
        Y,
        "> [!example] Ypsilon\r\n> [[projects/alfa/tasks/y|y]]\r\n",
    );
    envelhece(home.path());
    ok(&roda(home.path(), &["set", "x", "--prazo", "2026-12-01"]));
    ok(&roda(home.path(), &["set", "y", "--prazo", "2026-12-01"]));
    assert_eq!(
        String::from_utf8(le_bytes(home.path(), X)).expect("utf-8"),
        "---\r\nid: x\r\nprazo: 2026-12-01\r\n---\r\n\r\n> [!todo] Xis\r\n> [[projects/alfa/tasks/x|x]]\r\n"
    );
    assert_eq!(
        String::from_utf8(le_bytes(home.path(), Y)).expect("utf-8"),
        "---\r\nprazo: 2026-12-01\r\n---\r\n\r\n> [!example] Ypsilon\r\n> [[projects/alfa/tasks/y|y]]\r\n"
    );
}

// ===========================================================================
// set --prazo: recusas, sem gravar nada do comando
// ===========================================================================

#[test]
fn set_prazo_invalido_sai_2_e_nao_grava() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    for data in [
        "amanha",
        "2026-02-30",
        "2027-02-29",
        "2026-13-01",
        "2026-12-1",
        "01/12/2026",
    ] {
        recusa(
            &roda(home.path(), &["set", "x", "--prazo", data]),
            2,
            &msg_data(data),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_prazo_invalido_com_outra_flag_valida_nao_grava_nada() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let casos: [&[&str]; 6] = [
        &["set", "x", "--status", "example", "--prazo", "amanha"],
        &["set", "x", "--status", "todo", "--prazo", "amanha"],
        &["set", "x", "--title", "Outro", "--prazo", "amanha"],
        &["set", "x", "--desc", "outra", "--prazo", "amanha"],
        &["set", "x", "--volta-em", "2026-12-01", "--prazo", "amanha"],
        &["set", "x", "--prazo", "amanha", "--volta-em", "2026-12-01"],
    ];
    for args in casos {
        recusa(&roda(home.path(), args), 2, &msg_data("amanha"));
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_volta_em_invalida_com_prazo_valido_nao_grava_o_prazo() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["set", "x", "--prazo", "2026-12-01", "--volta-em", "amanha"],
        ),
        2,
        &msg_data("amanha"),
    );
    recusa(
        &roda(
            home.path(),
            &[
                "set",
                "x",
                "--volta-em",
                "2026-02-30",
                "--prazo",
                "2026-12-01",
            ],
        ),
        2,
        &msg_data("2026-02-30"),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_prazo_e_volta_em_invalidos_saem_2_sem_gravar() {
    // Qual das duas datas é acusada não se fixa.
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    let saida = roda(
        home.path(),
        &["set", "x", "--volta-em", "amanha", "--prazo", "depois"],
    );
    assert_eq!(saida.codigo, Some(2), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, "");
    assert!(
        saida.stderr == format!("freitask: {}\n", msg_data("amanha"))
            || saida.stderr == format!("freitask: {}\n", msg_data("depois")),
        "stderr: {:?}",
        saida.stderr
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_prazo_sem_valor_sai_2() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "x", "--prazo"]),
        2,
        "falta o valor de --prazo",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_status_done_com_prazo_valido_recusa_e_nao_grava() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["set", "x", "--status", "done", "--prazo", "2026-12-01"],
        ),
        1,
        "`done` é gravado por `freitask archive <id> done`, não por `set`",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_prazo_em_id_inexistente_sai_1() {
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["set", "zzz", "--prazo", "2026-12-01"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn set_prazo_com_help_nao_executa_nem_toca_o_vault() {
    // I6 e §2.1: `--help` é resolvido antes de abrir o vault e vence as
    // outras flags, inclusive uma data inválida.
    let home = home_base_com_painel();
    let antes = fotografa(home.path());
    for args in [
        ["set", "x", "--prazo", "2026-12-01", "--help"],
        ["set", "x", "--prazo", "amanha", "-h"],
        ["set", "x", "--help", "--prazo", "2026-12-01"],
    ] {
        let saida = roda(home.path(), &args);
        assert_eq!(
            saida.codigo,
            Some(0),
            "{args:?}; stderr: {:?}",
            saida.stderr
        );
        assert_eq!(saida.stderr, "", "{args:?}");
        assert!(
            saida.stdout.starts_with("  freitask set <id|caminho>"),
            "{args:?}: o trecho do `set`, e não o caminho da task:\n{}",
            saida.stdout
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// list --json e show --json: prazo e dias_para_o_prazo (I11)
// ===========================================================================

/// Um vault com uma task `t` em `p` com o frontmatter dado.
fn home_uma_task(fm: &[&str]) -> tempfile::TempDir {
    home_pequeno_s(&[(rel("p", "t"), task(fm, &bloco("todo", "Tê", "p", "t")))])
}

/// `prazo` e `dias_para_o_prazo` da task `t` no `list --json` e no
/// `show --json` (que têm de concordar), num dia dado.
fn prazo_e_dias_no_dia(home: &Path, dia: &str) -> (Value, Value) {
    let saida = roda_no_dia(home, dia, &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    valida("list", &linhas);
    let l = linha(&linhas, "t").clone();
    let saida = roda_no_dia(home, dia, &["show", "t", "--json"]);
    ok(&saida);
    let s = json(&saida);
    valida("show", &s);
    assert_eq!(s["prazo"], l["prazo"], "show e list discordam do prazo");
    assert_eq!(
        s["dias_para_o_prazo"], l["dias_para_o_prazo"],
        "show e list discordam dos dias"
    );
    (l["prazo"].clone(), l["dias_para_o_prazo"].clone())
}

#[test]
fn list_json_prazo_futuro_conta_os_dias_que_faltam() {
    let home = home_uma_task(&["prazo: 2026-11-20"]);
    assert_eq!(
        prazo_e_dias_no_dia(home.path(), HOJE),
        (json!("2026-11-20"), json!(12))
    );
}

#[test]
fn list_json_prazo_de_hoje_e_zero_dias() {
    let home = home_uma_task(&["prazo: 2026-11-08"]);
    assert_eq!(
        prazo_e_dias_no_dia(home.path(), HOJE),
        (json!("2026-11-08"), json!(0))
    );
}

#[test]
fn list_json_prazo_vencido_da_dias_negativos() {
    let home = home_uma_task(&["prazo: 2026-11-01"]);
    assert_eq!(
        prazo_e_dias_no_dia(home.path(), HOJE),
        (json!("2026-11-01"), json!(-7))
    );
}

#[test]
fn list_json_dias_para_o_prazo_e_a_diferenca_de_calendario() {
    // I11: prazo − hoje em dias, atravessando mês, ano e 29 de fevereiro.
    let casos = [
        ("2026-11-08", "2026-11-09", 1),
        ("2026-11-08", "2027-01-01", 54),
        ("2026-12-31", "2027-01-01", 1),
        ("2027-01-01", "2026-12-31", -1),
        ("2028-02-28", "2028-03-01", 2),
        ("2027-02-28", "2027-03-01", 1),
        ("2026-12-31", "2026-01-01", -364),
        ("2026-01-01", "2027-01-01", 365),
        ("2028-01-01", "2029-01-01", 366),
        ("2028-03-01", "2028-02-29", -1),
    ];
    for (hoje, prazo, dias) in casos {
        let linha_fm = format!("prazo: {prazo}");
        let home = home_uma_task(&[&linha_fm]);
        assert_eq!(
            prazo_e_dias_no_dia(home.path(), hoje),
            (json!(prazo), json!(dias)),
            "hoje {hoje}, prazo {prazo}"
        );
    }
}

#[test]
fn list_json_prazo_vazio_conta_como_sem_prazo() {
    let home = home_uma_task(&["prazo:"]);
    assert_eq!(
        prazo_e_dias_no_dia(home.path(), HOJE),
        (Value::Null, Value::Null)
    );
}

#[test]
fn list_json_sem_prazo_e_null_e_null() {
    let home = home_uma_task(&["prioridade: alta"]);
    assert_eq!(
        prazo_e_dias_no_dia(home.path(), HOJE),
        (Value::Null, Value::Null)
    );
}

#[test]
fn list_json_prazo_nao_esconde_nem_mostra_task() {
    // Algum dia e adiada seguem escondidas com prazo vencido; a task comum
    // com prazo vencido segue à mostra. O prazo aparece no JSON de todas.
    let home = home_pequeno_s(&[
        (
            rel("p", "comum"),
            task(
                &["prazo: 2026-11-01"],
                &bloco("todo", "Comum", "p", "comum"),
            ),
        ),
        (
            rel("p", "algum"),
            task(
                &["prazo: 2026-11-01"],
                &bloco("quote", "Algum", "p", "algum"),
            ),
        ),
        (
            rel("p", "adiada"),
            task(
                &["volta-em: 2026-12-01", "prazo: 2026-11-01"],
                &bloco("todo", "Adiada", "p", "adiada"),
            ),
        ),
        (
            rel("p", "longe"),
            task(
                &["prazo: 2027-11-08"],
                &bloco("todo", "Longe", "p", "longe"),
            ),
        ),
    ]);
    let linhas = list_json(home.path(), &[]);
    assert_eq!(linhas.as_array().expect("array").len(), 4);
    let comum = linha(&linhas, "comum");
    assert_eq!(comum["hidden"], json!(false));
    assert_eq!(comum["dias_para_o_prazo"], json!(-7));
    let algum = linha(&linhas, "algum");
    assert_eq!(algum["hidden"], json!(true));
    assert_eq!(algum["prazo"], json!("2026-11-01"));
    assert_eq!(algum["dias_para_o_prazo"], json!(-7));
    let adiada = linha(&linhas, "adiada");
    assert_eq!(adiada["hidden"], json!(true));
    assert_eq!(adiada["volta_em"], json!("2026-12-01"));
    assert_eq!(adiada["prazo"], json!("2026-11-01"));
    assert_eq!(adiada["dias_para_o_prazo"], json!(-7));
    let longe = linha(&linhas, "longe");
    assert_eq!(longe["hidden"], json!(false));
    assert_eq!(longe["dias_para_o_prazo"], json!(365));
}

#[test]
fn show_json_com_prazo_e_a_linha_do_list_mais_ciclo() {
    let home = home_pequeno_s(&[(
        rel("p", "t"),
        task(
            &["id: t", "prazo: 2026-11-20", "tags: [x]"],
            "> [!example] Tê\n> [[projects/p/tasks/t|t]]\n> _em curso_\n",
        ),
    )]);
    let linhas = list_json(home.path(), &[]);
    let mut esperado = linha(&linhas, "t").clone();
    assert_eq!(esperado["prazo"], json!("2026-11-20"));
    assert_eq!(esperado["dias_para_o_prazo"], json!(12));
    let s = show_json(home.path(), "t");
    esperado["ciclo"] = s["ciclo"].clone();
    assert_eq!(s, esperado);
}

// ===========================================================================
// show em texto
// ===========================================================================

const SHOW_BLOCO: &str = "> [!todo] Tê\n> [[projects/p/tasks/t|t]]\n> _em curso_\n> nota\n";

/// O vault do `show` em texto: a task `t` com o bloco `SHOW_BLOCO`, o
/// frontmatter dado e um corpo.
fn home_show(fm: &[&str]) -> tempfile::TempDir {
    home_pequeno_s(&[(
        rel("p", "t"),
        task(fm, SHOW_BLOCO) + "\n## Notas\n\n- [ ] um\n",
    )])
}

/// As linhas não vazias que o `show` imprime depois do bloco (o bloco tem de
/// vir primeiro, como hoje).
fn depois_do_bloco(saida: &Saida) -> Vec<String> {
    assert!(
        saida.stdout.starts_with(SHOW_BLOCO),
        "o show começa pelo bloco:\n{}",
        saida.stdout
    );
    assert!(saida.stdout.ends_with('\n'), "stdout: {:?}", saida.stdout);
    saida.stdout[SHOW_BLOCO.len()..]
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn show_texto_imprime_o_prazo_e_os_dias_que_faltam() {
    let home = home_show(&["prazo: 2026-11-20"]);
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    assert_eq!(depois_do_bloco(&saida), ["prazo: 2026-11-20 (em 12 dias)"]);
}

#[test]
fn show_texto_conta_os_dias_atravessando_o_ano() {
    let home = home_show(&["prazo: 2027-01-01"]);
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    assert_eq!(depois_do_bloco(&saida), ["prazo: 2027-01-01 (em 54 dias)"]);
}

#[test]
fn show_texto_prazo_vencido_diz_ha_quantos_dias() {
    let home = home_show(&["prazo: 2026-11-01"]);
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    assert_eq!(
        depois_do_bloco(&saida),
        ["prazo: 2026-11-01 (vencido há 7 dias)"]
    );
}

#[test]
fn show_texto_conta_a_partir_de_freitask_today() {
    let home = home_show(&["prazo: 2026-11-20"]);
    let saida = roda_no_dia(home.path(), "2026-11-30", &["show", "t"]);
    ok(&saida);
    assert_eq!(
        depois_do_bloco(&saida),
        ["prazo: 2026-11-20 (vencido há 10 dias)"]
    );
}

#[test]
fn show_texto_de_adiada_com_prazo_tambem_imprime_o_prazo() {
    let home = home_show(&["volta-em: 2026-12-01", "prazo: 2026-12-10"]);
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    assert_eq!(depois_do_bloco(&saida), ["prazo: 2026-12-10 (em 32 dias)"]);
}

#[test]
fn show_texto_com_prazo_vazio_imprime_so_o_bloco() {
    let home = home_show(&["prazo:"]);
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    assert_eq!(saida.stdout, SHOW_BLOCO);
}

#[test]
fn show_texto_sem_prazo_segue_imprimindo_so_o_bloco() {
    let home = home_show(&["prioridade: alta", "volta-em: 2026-11-01"]);
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    assert_eq!(saida.stdout, SHOW_BLOCO);
}

// ===========================================================================
// doctor: prazo-invalido e prazo-antes-da-volta
// ===========================================================================

const PR: &str = "projects/alfa/tasks/pr.md";

/// A task `pr` de `alfa` com o frontmatter dado.
fn pr_md(fm: &[&str]) -> String {
    task(fm, "> [!todo] Pê erre\n> [[projects/alfa/tasks/pr|pr]]\n")
}

/// Afirma um `doctor` (texto e `--json`) com exatamente um achado `warn` do
/// `kind` dado em `path`, pendente; devolve o `msg`, que não se fixa aqui, e
/// confere que o texto o imprime no formato de todo achado.
fn um_achado_warn(home: &Path, kind: &str, path: &str) -> String {
    let saida = roda(home, &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    let v = json(&saida);
    valida("doctor", &v);
    assert_eq!(v["ok"], json!(false), "{v}");
    let achados = v["findings"].as_array().expect("findings");
    assert_eq!(achados.len(), 1, "um achado só: {v}");
    let f = &achados[0];
    assert_eq!(f["level"], json!("warn"), "{v}");
    assert_eq!(f["kind"], json!(kind), "{v}");
    assert_eq!(f["path"], json!(path), "{v}");
    assert_eq!(f["fixed"], json!(false), "{v}");
    let msg = f["msg"].as_str().expect("msg string").to_owned();
    assert!(!msg.is_empty());

    let saida = roda(home, &["doctor"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}",
            achado("warn", kind, path, &msg, false),
            rodape(1, 1, true)
        )
    );
    msg
}

/// Os achados do `doctor --json` como (level, kind, path, fixed), ordenados;
/// o `msg` não se fixa.
fn achados_sem_msg(v: &Value) -> Vec<(String, String, String, bool)> {
    let mut out: Vec<(String, String, String, bool)> = v["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("findings: {v}"))
        .iter()
        .map(|f| {
            assert!(f["msg"].as_str().is_some_and(|m| !m.is_empty()), "msg: {f}");
            (
                f["level"].as_str().expect("level").to_owned(),
                f["kind"].as_str().expect("kind").to_owned(),
                f["path"].as_str().expect("path").to_owned(),
                f["fixed"].as_bool().expect("fixed"),
            )
        })
        .collect();
    out.sort();
    out
}

fn tupla(level: &str, kind: &str, path: &str, fixed: bool) -> (String, String, String, bool) {
    (level.into(), kind.into(), path.into(), fixed)
}

#[test]
fn doctor_acusa_prazo_invalido() {
    let home = home_pequeno(&[(PR, &pr_md(&["prazo: 2026-02-30"]))]);
    um_achado_warn(home.path(), "prazo-invalido", PR);
}

#[test]
fn doctor_acusa_prazo_invalido_em_cada_forma_que_nao_e_data() {
    for valor in [
        "2026-12-1",
        "01/12/2026",
        "2026-13-01",
        "2027-02-29",
        "amanhã",
    ] {
        let linha_fm = format!("prazo: {valor}");
        let home = home_pequeno(&[(PR, &pr_md(&[&linha_fm]))]);
        um_achado_warn(home.path(), "prazo-invalido", PR);
    }
}

#[test]
fn doctor_nao_acusa_prazo_valido_vencido_nem_vazio() {
    let home = home_pequeno(&[
        (PR, &pr_md(&["prazo: 2026-01-01"])),
        (
            "projects/alfa/tasks/vazio.md",
            &task(
                &["prazo:"],
                "> [!todo] Vazio\n> [[projects/alfa/tasks/vazio|vazio]]\n",
            ),
        ),
        (
            "projects/alfa/tasks/bissexto.md",
            &task(
                &["prazo: 2028-02-29", "prioridade: alta"],
                "> [!todo] Bi\n> [[projects/alfa/tasks/bissexto|bissexto]]\n",
            ),
        ),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
    let saida = roda(home.path(), &["doctor", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!({"ok": true, "findings": []}));
}

#[test]
fn doctor_fix_nao_repara_prazo_invalido() {
    // Sem `--fix`: não se adivinha a data. A task fica byte a byte.
    let original = pr_md(&["prazo: 2026-02-30"]);
    let home = home_pequeno(&[(PR, &original)]);
    let saida = roda(home.path(), &["doctor", "--fix", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    valida("doctor", &v);
    assert_eq!(
        achados_sem_msg(&v),
        vec![tupla("warn", "prazo-invalido", PR, false)]
    );
    assert_eq!(le(home.path(), PR), original);
    assert_eq!(mtime(home.path(), PR), antigo());
}

#[test]
fn doctor_acusa_prazo_antes_da_volta() {
    let home = home_pequeno(&[(PR, &pr_md(&["volta-em: 2026-12-01", "prazo: 2026-11-20"]))]);
    um_achado_warn(home.path(), "prazo-antes-da-volta", PR);
}

#[test]
fn doctor_acusa_prazo_antes_da_volta_com_o_prazo_ja_vencido() {
    // Vencido não é achado por si; antes da volta é.
    let home = home_pequeno(&[(PR, &pr_md(&["prazo: 2026-11-01", "volta-em: 2026-11-15"]))]);
    um_achado_warn(home.path(), "prazo-antes-da-volta", PR);
}

#[test]
fn doctor_acusa_prazo_antes_da_volta_atravessando_o_ano() {
    let home = home_pequeno(&[(PR, &pr_md(&["volta-em: 2027-01-01", "prazo: 2026-12-31"]))]);
    um_achado_warn(home.path(), "prazo-antes-da-volta", PR);
}

#[test]
fn doctor_nao_acusa_prazo_no_dia_da_volta_nem_depois() {
    let home = home_pequeno(&[
        (PR, &pr_md(&["volta-em: 2026-12-01", "prazo: 2026-12-01"])),
        (
            "projects/alfa/tasks/depois.md",
            &task(
                &["prazo: 2026-12-02", "volta-em: 2026-12-01"],
                "> [!todo] Depois\n> [[projects/alfa/tasks/depois|depois]]\n",
            ),
        ),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_com_uma_data_invalida_nao_compara_prazo_e_volta() {
    // `2026-12-1` como texto é "menor" que `2026-12-05`; como data não é nada.
    let home = home_pequeno(&[
        (
            "projects/alfa/tasks/a.md",
            &task(
                &["volta-em: 2026-12-05", "prazo: 2026-12-1"],
                "> [!todo] A\n> [[projects/alfa/tasks/a|a]]\n",
            ),
        ),
        (
            "projects/alfa/tasks/b.md",
            &task(
                &["volta-em: 2026-12-1", "prazo: 2026-11-20"],
                "> [!todo] B\n> [[projects/alfa/tasks/b|b]]\n",
            ),
        ),
    ]);
    let saida = roda(home.path(), &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    valida("doctor", &v);
    assert_eq!(
        achados_sem_msg(&v),
        vec![
            tupla("warn", "prazo-invalido", "projects/alfa/tasks/a.md", false),
            tupla(
                "warn",
                "volta-em-invalido",
                "projects/alfa/tasks/b.md",
                false
            ),
        ]
    );
}

#[test]
fn doctor_acusa_prazo_invalido_e_volta_em_invalido_na_mesma_task() {
    let home = home_pequeno(&[(PR, &pr_md(&["volta-em: 2026-13-01", "prazo: 2026-02-30"]))]);
    let saida = roda(home.path(), &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    valida("doctor", &v);
    assert_eq!(
        achados_sem_msg(&v),
        vec![
            tupla("warn", "prazo-invalido", PR, false),
            tupla("warn", "volta-em-invalido", PR, false),
        ]
    );
    // O `volta-em-invalido` segue com o texto de hoje.
    let msgs: Vec<&str> = v["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|f| f["kind"] == json!("volta-em-invalido"))
        .map(|f| f["msg"].as_str().expect("msg"))
        .collect();
    assert_eq!(
        msgs,
        ["volta-em \"2026-13-01\" não é uma data AAAA-MM-DD — a task aparece, sem adiar"]
    );
}

#[test]
fn doctor_fix_nao_repara_prazo_antes_da_volta() {
    // Escolher qual das duas datas mudar é decisão editorial.
    let original = pr_md(&["volta-em: 2026-12-01", "prazo: 2026-11-20"]);
    let home = home_pequeno(&[(PR, &original)]);
    let saida = roda(home.path(), &["doctor", "--fix", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    valida("doctor", &v);
    assert_eq!(
        achados_sem_msg(&v),
        vec![tupla("warn", "prazo-antes-da-volta", PR, false)]
    );
    assert_eq!(le(home.path(), PR), original);
    assert_eq!(mtime(home.path(), PR), antigo());
}

#[test]
fn doctor_nao_olha_prazo_de_task_arquivada() {
    // Como o `volta-em-invalido`: arquivada não é adiada nem tem prazo a vigiar.
    let home = home_pequeno(&[
        (
            "projects/alfa/tasks/archived/done/pr.md",
            &task(
                &["prazo: 2026-02-30"],
                "> [!done] Pê erre\n> [[projects/alfa/tasks/archived/done/pr|pr]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
            ),
        ),
        (
            "projects/alfa/tasks/archived/dropped/qr.md",
            &task(
                &["volta-em: 2026-12-01", "prazo: 2026-11-20"],
                "> [!todo] Quê erre\n> [[projects/alfa/tasks/archived/dropped/qr|qr]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/dropped` (abandonado)\n",
            ),
        ),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_com_achado_de_prazo_sem_fix_nao_grava_nada() {
    // I6: o `doctor` sem `--fix` é leitura pura, com ou sem achado.
    let home = home_pequeno(&[
        (PR, &pr_md(&["prazo: 2026-02-30"])),
        (
            "projects/alfa/tasks/qr.md",
            &task(
                &["volta-em: 2026-12-01", "prazo: 2026-11-20"],
                "> [!todo] Quê erre\n> [[projects/alfa/tasks/qr|qr]]\n",
            ),
        ),
    ]);
    let antes = fotografa(home.path());
    for args in [
        &["doctor"][..],
        &["doctor", "--json"][..],
        &["doctor", "--quiet"][..],
    ] {
        let saida = roda(home.path(), args);
        assert_eq!(saida.codigo, Some(1), "{args:?}: stderr {:?}", saida.stderr);
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// Leitura pura (I6)
// ===========================================================================

#[test]
fn leituras_com_prazo_nao_gravam_nada() {
    let home = home_pequeno(&[
        (PR, &pr_md(&["prazo: 2026-11-20"])),
        (
            "projects/alfa/tasks/inv.md",
            &task(
                &["prazo: 2026-02-30"],
                "> [!todo] Inv\n> [[projects/alfa/tasks/inv|inv]]\n",
            ),
        ),
    ]);
    let antes = fotografa(home.path());
    for args in [
        &["list"][..],
        &["list", "--json"][..],
        &["list", "--json", "--archived"][..],
        &["show", "pr"][..],
        &["show", "pr", "--json"][..],
        &["show", "inv"][..],
        &["show", "inv", "--json"][..],
    ] {
        ok(&roda(home.path(), args));
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// I11 e os dois testes nomeados da §3.7
// ===========================================================================

/// Os prazos inválidos do teste nomeado: (id, valor).
const INVALIDOS: [(&str, &str); 5] = [
    ("curta", "2026-12-1"),
    ("barras", "01/12/2026"),
    ("fevereiro", "2026-02-30"),
    ("mes", "2026-13-01"),
    ("texto", "amanhã"),
];

#[test]
fn prazo_invalido_nao_conta_dias_e_o_doctor_acusa() {
    let arquivos: Vec<(String, String)> = INVALIDOS
        .iter()
        .map(|(id, valor)| {
            let linha_fm = format!("prazo: {valor}");
            (
                rel("alfa", id),
                task(&[&linha_fm], &bloco("todo", id, "alfa", id)),
            )
        })
        .collect();
    let home = home_pequeno_s(&arquivos);
    let antes = fotografa(home.path());

    // O JSON traz o texto como está e não conta dias; a task não se esconde.
    let linhas = list_json(home.path(), &[]);
    assert_eq!(linhas.as_array().expect("array").len(), INVALIDOS.len());
    for (id, valor) in INVALIDOS {
        let l = linha(&linhas, id);
        assert_eq!(l["prazo"], json!(valor), "{id}");
        assert_eq!(l["dias_para_o_prazo"], Value::Null, "{id}");
        assert_eq!(l["hidden"], json!(false), "{id}");
        let s = show_json(home.path(), id);
        assert_eq!(s["prazo"], json!(valor), "{id}");
        assert_eq!(s["dias_para_o_prazo"], Value::Null, "{id}");

        // O `show` em texto sai 0, imprime o bloco e não conta dias.
        let saida = roda(home.path(), &["show", id]);
        assert_eq!(saida.codigo, Some(0), "{id}: stderr {:?}", saida.stderr);
        let b = bloco("todo", id, "alfa", id);
        assert!(saida.stdout.starts_with(&b), "{id}:\n{}", saida.stdout);
        assert!(
            !saida.stdout.contains(" dias)") && !saida.stdout.contains("vencido"),
            "{id}: prazo inválido não conta dias:\n{}",
            saida.stdout
        );
    }

    // O doctor acusa cada um, como warn, sem reparo.
    let saida = roda(home.path(), &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    let v = json(&saida);
    valida("doctor", &v);
    assert_eq!(v["ok"], json!(false));
    let mut esperado: Vec<_> = INVALIDOS
        .iter()
        .map(|(id, _)| tupla("warn", "prazo-invalido", &rel("alfa", id), false))
        .collect();
    esperado.sort();
    assert_eq!(achados_sem_msg(&v), esperado);

    // O texto imprime cada achado com o msg do JSON e a contagem.
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    for f in v["findings"].as_array().expect("findings") {
        let bloco_achado = achado(
            "warn",
            "prazo-invalido",
            f["path"].as_str().expect("path"),
            f["msg"].as_str().expect("msg"),
            false,
        );
        assert!(
            saida.stdout.contains(&bloco_achado),
            "faltou {bloco_achado:?} em:\n{}",
            saida.stdout
        );
    }
    assert!(
        saida.stdout.ends_with(&rodape(5, 5, true)),
        "rodapé:\n{}",
        saida.stdout
    );

    // Nada disso grava (I6).
    nada_mudou(&antes, &fotografa(home.path()));
}

/// O vault do teste do painel, com ou sem `prazo:` nas tasks. Com prazo:
/// vencido, de hoje, futuro, inválido, vazio, em Algum dia, em adiada, em
/// arquivada; uma task fica sem prazo nos dois.
fn home_do_painel(com_prazo: bool) -> tempfile::TempDir {
    let fm = |base: &[&'static str], prazo: &'static str| -> Vec<&'static str> {
        let mut v = base.to_vec();
        if com_prazo {
            v.push(prazo);
        }
        v
    };
    let arquivos: Vec<(String, String)> = vec![
        (
            rel("alfa", "a"),
            task(
                &fm(&[], "prazo: 2026-11-01"),
                &bloco("todo", "A", "alfa", "a"),
            ),
        ),
        (
            rel("alfa", "b"),
            task(
                &fm(&[], "prazo: 2026-11-08"),
                &bloco("example", "B", "alfa", "b"),
            ),
        ),
        (
            rel("alfa", "c-alta"),
            task(
                &fm(&["prioridade: alta"], "prazo: 2027-01-01"),
                &bloco("check", "C", "alfa", "c-alta"),
            ),
        ),
        (
            rel("alfa", "d"),
            task(
                &fm(&["id: d"], "prazo: 2026-02-30"),
                "> [!todo] D\n> [[projects/alfa/tasks/d|d]]\n> _em curso_\n> nota\n",
            ),
        ),
        (
            rel("alfa", "e"),
            task(
                &fm(&[], "prazo: 2026-11-01"),
                &bloco("quote", "E", "alfa", "e"),
            ),
        ),
        (
            rel("alfa", "f"),
            task(
                &fm(&["volta-em: 2026-12-01"], "prazo: 2026-11-10"),
                &bloco("todo", "F", "alfa", "f"),
            ),
        ),
        (
            rel("alfa", "g"),
            task(&fm(&[], "prazo:"), &bloco("tip", "G", "alfa", "g")),
        ),
        (
            rel("alfa", "h"),
            task(
                &fm(&["volta-em: 2026-11-08"], "prazo: 2026-11-09"),
                &bloco("abstract", "H", "alfa", "h"),
            ),
        ),
        (
            "projects/alfa/tasks/archived/done/v.md".into(),
            task(
                &fm(&[], "prazo: 2026-11-09"),
                "> [!done] V\n> [[projects/alfa/tasks/archived/done/v|v]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
            ),
        ),
        (rel("beta", "y"), bloco("warning", "Y", "beta", "y")),
        (
            rel("beta", "z"),
            task(
                &fm(&[], "prazo: 2026-11-09"),
                &bloco("todo", "Z", "beta", "z"),
            ),
        ),
        (
            rel("gama", "q"),
            task(
                &fm(&[], "prazo: 2026-10-01"),
                &bloco("quote", "Q", "gama", "q"),
            ),
        ),
    ];
    home_pequeno_s(&arquivos)
}

/// O painel dos dois vaults acima: alta primeiro, depois fase, depois id; sem
/// Algum dia nem adiada; `gama` só com escondida sai `_(sem tasks)_`.
fn painel_do_vault_do_painel() -> String {
    painel(
        8,
        3,
        concat!(
            "## alfa\n",
            "\n",
            "> [!check] C\n",
            "> [[projects/alfa/tasks/c-alta|c-alta]]\n",
            "\n",
            "> [!todo] A\n",
            "> [[projects/alfa/tasks/a|a]]\n",
            "\n",
            "> [!todo] D\n",
            "> [[projects/alfa/tasks/d|d]]\n",
            "> _em curso_\n",
            "> nota\n",
            "\n",
            "> [!abstract] H\n",
            "> [[projects/alfa/tasks/h|h]]\n",
            "\n",
            "> [!tip] G\n",
            "> [[projects/alfa/tasks/g|g]]\n",
            "\n",
            "> [!example] B\n",
            "> [[projects/alfa/tasks/b|b]]\n",
            "\n",
            "## beta\n",
            "\n",
            "> [!todo] Z\n",
            "> [[projects/beta/tasks/z|z]]\n",
            "\n",
            "> [!warning] Y\n",
            "> [[projects/beta/tasks/y|y]]\n",
            "\n",
            "## gama\n",
            "\n",
            "_(sem tasks)_\n",
            "\n",
        ),
    )
}

#[test]
fn prazo_nao_muda_a_ordem_do_painel() {
    let sem = home_do_painel(false);
    let com = home_do_painel(true);
    ok(&roda(sem.path(), &["rebuild"]));
    ok(&roda(com.path(), &["rebuild"]));
    let painel_sem = le_bytes(sem.path(), CURRENT);
    let painel_com = le_bytes(com.path(), CURRENT);
    assert_eq!(
        String::from_utf8(painel_com.clone()).expect("utf-8"),
        String::from_utf8(painel_sem.clone()).expect("utf-8"),
        "o CURRENT.md com prazo tem de ser byte a byte o sem prazo"
    );
    assert_eq!(painel_com, painel_sem);
    assert_eq!(
        String::from_utf8(painel_com).expect("utf-8"),
        painel_do_vault_do_painel()
    );
}

#[test]
fn set_prazo_nao_reordena_o_painel_do_vault_inteiro() {
    // Gravar prazo pela CLI regenera o painel com a mesma ordem.
    let home = home_do_painel(false);
    ok(&roda(home.path(), &["set", "b", "--prazo", "2026-01-01"]));
    ok(&roda(home.path(), &["set", "y", "--prazo", "2026-11-09"]));
    assert_eq!(le(home.path(), CURRENT), painel_do_vault_do_painel());
}

/// O `list --json` sem `prazo`, sem `dias_para_o_prazo` e com o `path` sem o
/// HOME, para comparar dois vaults.
fn sem_prazo_e_sem_home(linhas: &Value, home: &Path) -> Value {
    let prefixo = home.to_str().expect("utf-8").to_owned();
    let mut v = linhas.clone();
    for l in v.as_array_mut().expect("array") {
        let o = l.as_object_mut().expect("objeto");
        o.remove("prazo");
        o.remove("dias_para_o_prazo");
        let p = o["path"].as_str().expect("path").to_owned();
        assert!(p.starts_with(&prefixo), "path fora do HOME: {p}");
        o.insert("path".into(), json!(p[prefixo.len()..].to_owned()));
    }
    v
}

#[test]
fn list_json_prazo_nao_muda_a_ordem_nem_o_conjunto_nem_o_hidden() {
    // I11: com ou sem prazo, a ordem, o conjunto e o resto de cada linha (o
    // `hidden` inclusive) são os de hoje.
    let sem = home_do_painel(false);
    let com = home_do_painel(true);
    for extra in [&[][..], &["--archived"][..]] {
        let l_sem = list_json(sem.path(), extra);
        let l_com = list_json(com.path(), extra);
        assert_eq!(ids(&l_com), ids(&l_sem), "ordem, {extra:?}");
        assert_eq!(
            sem_prazo_e_sem_home(&l_com, com.path()),
            sem_prazo_e_sem_home(&l_sem, sem.path()),
            "{extra:?}"
        );
    }
    // A ordem é a de hoje, escrita à mão: alta, fase, id; escondidas na
    // posição da sua fase.
    let l = list_json(com.path(), &[]);
    assert_eq!(
        ids(&l),
        ["c-alta", "a", "d", "f", "h", "g", "b", "e", "z", "y", "q"]
    );
    // E o prazo de quem tem chega no JSON.
    assert_eq!(linha(&l, "a")["dias_para_o_prazo"], json!(-7));
    assert_eq!(linha(&l, "b")["dias_para_o_prazo"], json!(0));
    assert_eq!(linha(&l, "c-alta")["dias_para_o_prazo"], json!(54));
    assert_eq!(linha(&l, "d")["dias_para_o_prazo"], Value::Null);
    assert_eq!(linha(&l, "f")["dias_para_o_prazo"], json!(2));
    assert_eq!(linha(&l, "g")["prazo"], Value::Null);
    assert_eq!(linha(&l, "y")["prazo"], Value::Null);
}
