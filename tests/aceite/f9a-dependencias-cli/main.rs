//! Aceite da fatia 9a — Dependências, parte da CLI (spec §5, §10.1; decisão
//! 54 do log da reescrita): `set --bloqueada-por` com as recusas, `rename`
//! que arrasta o id nas linhas `bloqueada-por` do vault, `delete` que deixa a
//! dependência pendurada, os campos `bloqueada_por`, `bloqueada`, `bloqueia`
//! no `list --json` e no `show --json` (mais `ciclo`), o `show` em texto, a
//! semântica (arquivada resolvida, inexistente não bloqueia, escondida
//! bloqueia, bloqueada não sai do painel) e os kinds `dependencia-pendurada`
//! e `dependencia-ciclica` do `doctor`. A TUI (tecla `b`, marca) é a 9b; o
//! prazo é a 10a; I10 fica no crate `vault` (decisões 36 e 47).
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§1.5, §2.1, §2.2, §2.4, §3.3, §3.7, §10.1), `docs/freitask.md` ("Modelo
//! mental", "Formato do bloco", "Encerrar / reabrir", "CLI", "`freitask
//! doctor`") e `docs/decisoes-da-reescrita.md` (18, 19, 29, 31, 54). Nenhum
//! valor esperado aqui foi gerado do binário sob teste.
//!
//! Decisões tomadas aqui, onde a spec não fecha o byte (repetidas no
//! relatório do rascunho):
//! - o `set --bloqueada-por` grava `bloqueada-por: [a, b]` (a forma da
//!   §10.1, crua e sem aspas pela decisão 18), com a mesma regra de
//!   frontmatter do `--volta-em` (fatia 3): chave nova no fim do frontmatter,
//!   frontmatter criado quando falta, linha trocada no lugar, `""` tira a
//!   linha; sem `--status`/`--title`/`--desc` o bloco fica byte a byte; o
//!   stdout é o caminho absoluto, como todo `set`;
//! - recusa: afirma-se o código, stdout vazio, stderr com o prefixo
//!   `freitask: ` e nada gravado; o texto da mensagem fica solto, menos o
//!   caminho do ciclo, que tem de aparecer como uma linha `x → y → … → x`
//!   fechada, com cada task do ciclo uma vez e as arestas todas no mesmo
//!   sentido (quem espera → bloqueadora, ou o contrário; o começo é livre);
//! - `bloqueia` e a lista do `bloqueia: N (…)` do `show` são comparadas como
//!   conjunto (a spec não dá a ordem); `bloqueada_por` e `bloqueada por:`,
//!   na ordem do arquivo ("como está escrito");
//! - "ativa" é a task fora de `archived/`, inclusive a escondida (o sentido
//!   de `docs/freitask.md`, "Encerrar": "task ativa, em qualquer fase,
//!   inclusive Algum dia"): a escondida que espera entra no `bloqueia`;
//! - o `bloqueia` de uma task arquivada lista as ativas que ainda a citam (a
//!   letra da §10.1: "array dos ids ativos que listam esta task");
//! - `show` em texto: o bloco, e embaixo, nessa ordem, `bloqueada por: …` e
//!   `bloqueia: N (…)`; linha em branco entre o bloco e elas fica solta, e a
//!   linha que falta quando um dos lados é vazio também; com ciclo, o stdout
//!   ainda começa pelo bloco, sai 1 e o stderr tem `CICLO: ` com o caminho;
//! - `show --json` de task em ciclo: `ciclo` é array com as tasks do ciclo
//!   (comparado como conjunto; repetir a primeira no fim é livre); o código
//!   de saída do `--json` com ciclo fica solto;
//! - `doctor`: afirma-se kind, level, `fixed: false` e quantos achados de
//!   cada kind há quando o vault tem um só problema; o `path` e o `msg` ficam
//!   soltos, e no ciclo também o número de achados (um por ciclo ou um por
//!   task);
//! - o aviso de referências do `rename` em stderr fica solto (a spec não diz
//!   se a linha `bloqueada-por` conta);
//! - `set <id> --bloqueada-por ""` numa task que está num ciclo escrito à mão
//!   é aceito (limpar não cria ciclo, e é como se corta a aresta que o
//!   `doctor` acusa);
//! - fica de fora o que a spec não fecha: id de projeto encerrado na lista,
//!   lista com id repetido ou com espaço em volta, escalar sem colchetes,
//!   auto-dependência escrita à mão, ciclo que passa por task arquivada,
//!   `bloqueada` de task arquivada, `ciclo` de quem só espera por um ciclo,
//!   escrita com outro ciclo já no vault, e `rename` em task de projeto
//!   encerrado.

mod util;

use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use util::*;

// ===========================================================================
// Fixtures
// ===========================================================================

/// O frontmatter com as linhas dadas, seguido da linha em branco; vazio se
/// não há linha.
fn fm(linhas: &[&str]) -> String {
    if linhas.is_empty() {
        return String::new();
    }
    let mut s = String::from("---\n");
    for l in linhas {
        s.push_str(l);
        s.push('\n');
    }
    s.push_str("---\n\n");
    s
}

/// O bloco de uma task ativa (linhas 1 e 2).
fn bloco(projeto: &str, id: &str, callout: &str, titulo: &str) -> String {
    format!("> [!{callout}] {titulo}\n> [[projects/{projeto}/tasks/{id}|{id}]]\n")
}

/// Uma task ativa: frontmatter (se houver) e bloco.
fn task(projeto: &str, id: &str, callout: &str, titulo: &str, frontmatter: &[&str]) -> String {
    fm(frontmatter) + &bloco(projeto, id, callout, titulo)
}

/// Uma task arquivada em `archived/<tipo>/`, com a entrada de histórico (o
/// `doctor` não acusa nada nela).
fn arquivada(
    projeto: &str,
    tipo: &str,
    id: &str,
    callout: &str,
    titulo: &str,
    frontmatter: &[&str],
) -> String {
    let glosa = match tipo {
        "done" => "feito",
        "dropped" => "abandonado",
        _ => "falhou",
    };
    format!(
        "{}> [!{callout}] {titulo}\n> [[projects/{projeto}/tasks/archived/{tipo}/{id}|{id}]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/{tipo}` ({glosa})\n",
        fm(frontmatter)
    )
}

/// `projects/<p>/tasks/<id>.md`.
fn rel(projeto: &str, id: &str) -> String {
    format!("projects/{projeto}/tasks/{id}.md")
}

/// Um vault com `status.json` default, `projects/` e os arquivos dados,
/// envelhecido, sem `CURRENT.md`.
fn home_pequeno(arquivos: &[(String, String)]) -> tempfile::TempDir {
    let pares: Vec<(&str, &str)> = arquivos
        .iter()
        .map(|(r, c)| (r.as_str(), c.as_str()))
        .collect();
    let home = home_com(&pares);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects");
    envelhece(home.path());
    home
}

/// Atalho: `(rel(p, id), task(p, id, callout, titulo, fm))`.
fn t(projeto: &str, id: &str, callout: &str, frontmatter: &[&str]) -> (String, String) {
    (
        rel(projeto, id),
        task(projeto, id, callout, &id.to_uppercase(), frontmatter),
    )
}

const CURRENT: &str = "CURRENT.md";

// ===========================================================================
// Leitura: helpers
// ===========================================================================

/// `list --json --archived`, saída 0, validado contra o schema.
fn lista(home: &Path) -> Value {
    let saida = roda(home, &["list", "--json", "--archived"]);
    ok(&saida);
    let v = json(&saida);
    valida("list", &v);
    v
}

/// A linha do `list --json` com o id dado (tem de haver exatamente uma).
fn linha(linhas: &Value, id: &str) -> Value {
    let achadas: Vec<&Value> = linhas
        .as_array()
        .expect("array")
        .iter()
        .filter(|l| l["id"] == json!(id))
        .collect();
    assert_eq!(achadas.len(), 1, "linhas com id {id}: {achadas:?}");
    achadas[0].clone()
}

/// Um array JSON de strings, na ordem.
fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap_or_else(|| panic!("não é array: {v}"))
        .iter()
        .map(|s| s.as_str().expect("string").to_owned())
        .collect()
}

/// Um array JSON de strings como conjunto.
fn conj(v: &Value) -> BTreeSet<String> {
    strs(v).into_iter().collect()
}

/// Afirma os três campos de dependência de uma linha.
fn deps(l: &Value, bloqueada_por: &[&str], bloqueada: bool, bloqueia: &[&str]) {
    assert_eq!(
        strs(&l["bloqueada_por"]),
        bloqueada_por.to_vec(),
        "bloqueada_por de {}",
        l["id"]
    );
    assert_eq!(l["bloqueada"], json!(bloqueada), "bloqueada de {}", l["id"]);
    assert_eq!(
        conj(&l["bloqueia"]),
        conjunto(bloqueia),
        "bloqueia de {}",
        l["id"]
    );
}

// ===========================================================================
// Ciclo: o caminho na mensagem
// ===========================================================================

/// Os ids da primeira linha de `texto` que tem `→`: o pedaço antes da
/// primeira seta dá a última palavra, o depois da última seta a primeira, e
/// cada id perde o que não é `[a-z0-9-]` nas pontas (crase, dois-pontos).
fn caminho_do_ciclo(texto: &str) -> Vec<String> {
    let linha = texto
        .lines()
        .find(|l| l.contains('→'))
        .unwrap_or_else(|| panic!("nenhum caminho com `→` em {texto:?}"));
    let pedacos: Vec<&str> = linha.split('→').collect();
    let ultimo = pedacos.len() - 1;
    pedacos
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let p = p.trim();
            let tok = if i == 0 {
                p.split_whitespace().last().unwrap_or("")
            } else if i == ultimo {
                p.split_whitespace().next().unwrap_or("")
            } else {
                p
            };
            tok.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .to_owned()
        })
        .collect()
}

/// Afirma que `caminho` é o ciclo de `membros` sobre as `arestas` (cada uma
/// `(quem espera, bloqueadora)`): fecha no começo, passa uma vez por cada
/// membro e anda sempre no mesmo sentido.
fn confere_ciclo(caminho: &[String], arestas: &[(&str, &str)], membros: &[&str]) {
    assert!(caminho.len() >= 3, "o caminho tem de fechar: {caminho:?}");
    assert_eq!(
        caminho.first(),
        caminho.last(),
        "o caminho volta ao começo: {caminho:?}"
    );
    let internos: Vec<&str> = caminho[..caminho.len() - 1]
        .iter()
        .map(String::as_str)
        .collect();
    let unicos: BTreeSet<&str> = internos.iter().copied().collect();
    assert_eq!(
        internos.len(),
        unicos.len(),
        "cada task do ciclo aparece uma vez: {caminho:?}"
    );
    assert_eq!(
        unicos,
        membros.iter().copied().collect::<BTreeSet<&str>>(),
        "as tasks do ciclo: {caminho:?}"
    );
    let pares: Vec<(&str, &str)> = caminho
        .windows(2)
        .map(|w| (w[0].as_str(), w[1].as_str()))
        .collect();
    let esperando = pares.iter().all(|p| arestas.contains(p));
    let bloqueando = pares.iter().all(|(u, v)| arestas.contains(&(*v, *u)));
    assert!(
        esperando || bloqueando,
        "o caminho {caminho:?} não segue as dependências {arestas:?}"
    );
}

// ===========================================================================
// Recusas e lock
// ===========================================================================

/// Roda e afirma uma recusa: o código dado, stdout vazio, stderr com o
/// prefixo `freitask: ` e nada gravado no vault.
fn recusa_sem_gravar(home: &Path, args: &[&str], codigo: i32) -> Saida {
    let antes = fotografa(home);
    let saida = roda(home, args);
    assert_eq!(saida.codigo, Some(codigo), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, "", "stdout tem de ficar vazio numa recusa");
    assert!(
        saida.stderr.starts_with("freitask: ") && saida.stderr.ends_with('\n'),
        "stderr: {:?}",
        saida.stderr
    );
    nada_mudou(&antes, &fotografa(home));
    saida
}

// ===========================================================================
// O vault da semântica
// ===========================================================================

/// `alfa`: `a` sem dependência; `b` espera `a`; `c` espera a arquivada
/// `velha`; `d` espera `fantasma`, que não existe; `e` espera `algum` (Algum
/// dia) e `f` espera `adiada` (`volta-em` futura); `g` espera `velha`, `a` e
/// `fantasma`; `h` espera `velha` e `fantasma`; `algum` (escondida) espera
/// `a`; `velha` (arquivada) espera `a`. `beta`: `k` espera `a`.
fn home_semantica() -> tempfile::TempDir {
    home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "b", "todo", &["bloqueada-por: [a]"]),
        t("alfa", "c", "example", &["bloqueada-por: [velha]"]),
        t("alfa", "d", "todo", &["bloqueada-por: [fantasma]"]),
        t("alfa", "e", "todo", &["bloqueada-por: [algum]"]),
        t("alfa", "f", "todo", &["bloqueada-por: [adiada]"]),
        t(
            "alfa",
            "g",
            "todo",
            &["bloqueada-por: [velha, a, fantasma]"],
        ),
        t("alfa", "h", "todo", &["bloqueada-por: [velha, fantasma]"]),
        t("alfa", "algum", "quote", &["bloqueada-por: [a]"]),
        t("alfa", "adiada", "todo", &["volta-em: 2026-12-01"]),
        (
            "projects/alfa/tasks/archived/done/velha.md".into(),
            arquivada(
                "alfa",
                "done",
                "velha",
                "done",
                "Velha",
                &["bloqueada-por: [a]"],
            ),
        ),
        t("beta", "k", "todo", &["bloqueada-por: [a]"]),
    ])
}

// ===========================================================================
// list --json: os campos e a semântica
// ===========================================================================

#[test]
fn list_json_task_sem_dependencia_tem_listas_vazias_e_nao_esta_bloqueada() {
    let home = home_pequeno(&[t("alfa", "x", "todo", &["tags: [a]"])]);
    deps(&linha(&lista(home.path()), "x"), &[], false, &[]);
}

#[test]
fn list_json_bloqueadora_ativa_bloqueia() {
    let home = home_semantica();
    deps(&linha(&lista(home.path()), "b"), &["a"], true, &[]);
}

#[test]
fn list_json_bloqueadora_arquivada_esta_resolvida() {
    let home = home_semantica();
    deps(&linha(&lista(home.path()), "c"), &["velha"], false, &[]);
}

#[test]
fn list_json_bloqueadora_inexistente_nao_bloqueia_e_segue_listada() {
    // Um typo não pode sumir com a task da frente (§10.1).
    let home = home_semantica();
    deps(&linha(&lista(home.path()), "d"), &["fantasma"], false, &[]);
}

#[test]
fn list_json_bloqueadora_escondida_bloqueia_normalmente() {
    // Algum dia e adiada (`volta-em` futura) bloqueiam.
    let home = home_semantica();
    let linhas = lista(home.path());
    deps(&linha(&linhas, "e"), &["algum"], true, &[]);
    deps(&linha(&linhas, "f"), &["adiada"], true, &[]);
}

#[test]
fn list_json_bloqueada_por_vem_como_esta_escrita_e_basta_uma_bloqueadora_ativa() {
    let home = home_semantica();
    let linhas = lista(home.path());
    deps(&linha(&linhas, "g"), &["velha", "a", "fantasma"], true, &[]);
    deps(&linha(&linhas, "h"), &["velha", "fantasma"], false, &[]);
}

#[test]
fn list_json_bloqueia_lista_as_ativas_que_esperam_inclusive_escondida_e_de_outro_projeto() {
    // `velha` também cita `a`, mas está arquivada: não entra.
    let home = home_semantica();
    deps(
        &linha(&lista(home.path()), "a"),
        &[],
        false,
        &["algum", "b", "g", "k"],
    );
}

#[test]
fn list_json_bloqueia_de_task_arquivada_lista_as_ativas_que_a_citam() {
    // `bloqueada` de task arquivada fica solto: a spec não diz.
    let home = home_semantica();
    let velha = linha(&lista(home.path()), "velha");
    assert_eq!(strs(&velha["bloqueada_por"]), vec!["a"]);
    assert_eq!(conj(&velha["bloqueia"]), conjunto(&["c", "g", "h"]));
}

#[test]
fn list_json_bloqueia_da_escondida_conta_quem_espera_por_ela() {
    let home = home_semantica();
    let linhas = lista(home.path());
    deps(&linha(&linhas, "algum"), &["a"], true, &["e"]);
    deps(&linha(&linhas, "adiada"), &[], false, &["f"]);
}

#[test]
fn list_json_task_bloqueada_segue_na_lista_e_nao_fica_escondida() {
    let home = home_semantica();
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    let linhas = json(&saida);
    valida("list", &linhas);
    for id in ["b", "e", "f", "g", "k"] {
        let l = linha(&linhas, id);
        assert_eq!(l["bloqueada"], json!(true), "{id}");
        assert_eq!(l["hidden"], json!(false), "{id}");
    }
    // A escondida que espera continua escondida pelo motivo dela.
    assert_eq!(linha(&linhas, "algum")["hidden"], json!(true));
}

#[test]
fn list_json_le_a_lista_sem_espaco_depois_da_virgula() {
    // O celular escreve à mão; `[a,b]` é a mesma lista YAML.
    let home = home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "b", "todo", &[]),
        t("alfa", "m", "todo", &["bloqueada-por: [a,b]"]),
    ]);
    let linhas = lista(home.path());
    deps(&linha(&linhas, "m"), &["a", "b"], true, &[]);
    deps(&linha(&linhas, "a"), &[], false, &["m"]);
}

#[test]
fn list_json_lista_vazia_no_frontmatter_e_sem_dependencia() {
    let home = home_pequeno(&[t("alfa", "x", "todo", &["bloqueada-por: []"])]);
    deps(&linha(&lista(home.path()), "x"), &[], false, &[]);
}

#[test]
fn list_json_com_dependencias_passa_no_schema() {
    let home = home_semantica();
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    valida("list", &json(&saida));
    lista(home.path());
}

// ===========================================================================
// O painel não muda
// ===========================================================================

#[test]
fn painel_com_dependencias_e_byte_a_byte_o_painel_sem() {
    // Task bloqueada não sai do painel, e o `CURRENT.md` não ganha marca: o
    // bloco é o do arquivo (§10.1).
    let com = home_semantica();
    let sem = home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "b", "todo", &[]),
        t("alfa", "c", "example", &[]),
        t("alfa", "d", "todo", &[]),
        t("alfa", "e", "todo", &[]),
        t("alfa", "f", "todo", &[]),
        t("alfa", "g", "todo", &[]),
        t("alfa", "h", "todo", &[]),
        t("alfa", "algum", "quote", &[]),
        t("alfa", "adiada", "todo", &["volta-em: 2026-12-01"]),
        (
            "projects/alfa/tasks/archived/done/velha.md".into(),
            arquivada("alfa", "done", "velha", "done", "Velha", &[]),
        ),
        t("beta", "k", "todo", &[]),
    ]);
    ok(&roda(com.path(), &["rebuild"]));
    ok(&roda(sem.path(), &["rebuild"]));
    let painel = le(com.path(), CURRENT);
    assert_eq!(painel, le(sem.path(), CURRENT));
    assert!(
        painel.contains(&bloco("alfa", "b", "todo", "B")),
        "a bloqueada segue no painel:\n{painel}"
    );
    assert!(painel.contains(&bloco("beta", "k", "todo", "K")));
}

// ===========================================================================
// show --json
// ===========================================================================

#[test]
fn show_json_traz_os_campos_de_dependencia_da_linha_do_list_e_ciclo_null() {
    let home = home_semantica();
    let saida = roda(home.path(), &["show", "g", "--json"]);
    ok(&saida);
    let v = json(&saida);
    valida("show", &v);
    let mut esperado = linha(&lista(home.path()), "g");
    esperado["ciclo"] = Value::Null;
    assert_eq!(v, esperado);
    deps(&v, &["velha", "a", "fantasma"], true, &[]);
}

#[test]
fn show_json_de_quem_bloqueia_traz_o_bloqueia() {
    let home = home_semantica();
    let saida = roda(home.path(), &["show", "a", "--json"]);
    ok(&saida);
    let v = json(&saida);
    valida("show", &v);
    deps(&v, &[], false, &["algum", "b", "g", "k"]);
    assert_eq!(v["ciclo"], Value::Null);
}

/// `alfa`: `a` espera `b`, `b` espera `a` (escrito à mão, como o celular
/// faria); `d` espera `a`.
fn home_ciclo_de_dois() -> tempfile::TempDir {
    home_pequeno(&[
        t("alfa", "a", "todo", &["bloqueada-por: [b]"]),
        t("alfa", "b", "example", &["bloqueada-por: [a]"]),
        t("alfa", "d", "todo", &["bloqueada-por: [a]"]),
    ])
}

/// `alfa`: `a` espera `b`, `b` espera `c`, `c` espera `a`, à mão; `d` espera
/// `a`.
fn home_ciclo_de_tres() -> tempfile::TempDir {
    home_pequeno(&[
        t("alfa", "a", "todo", &["bloqueada-por: [b]"]),
        t("alfa", "b", "todo", &["bloqueada-por: [c]"]),
        t("alfa", "c", "todo", &["bloqueada-por: [a]"]),
        t("alfa", "d", "todo", &["bloqueada-por: [a]"]),
    ])
}

#[test]
fn show_json_de_task_em_ciclo_traz_as_tasks_do_ciclo() {
    let home = home_ciclo_de_tres();
    let saida = roda(home.path(), &["show", "b", "--json"]);
    let v = json(&saida);
    valida("show", &v);
    assert!(v["ciclo"].is_array(), "ciclo: {}", v["ciclo"]);
    assert_eq!(conj(&v["ciclo"]), conjunto(&["a", "b", "c"]));
    deps(&v, &["c"], true, &["a"]);
}

// ===========================================================================
// show em texto
// ===========================================================================

/// `alfa`: `a` e `w` sem dependência; `t` (com descrição) espera `a` e `w`;
/// `u` espera `t`; `b2` e `b3` esperam `a`.
fn home_show() -> tempfile::TempDir {
    home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "w", "tip", &[]),
        (
            rel("alfa", "t"),
            fm(&["bloqueada-por: [a, w]"])
                + &bloco("alfa", "t", "example", "Tê")
                + "> _em curso_\n",
        ),
        t("alfa", "u", "todo", &["bloqueada-por: [t]"]),
        t("alfa", "b2", "todo", &["bloqueada-por: [a]"]),
        t("alfa", "b3", "todo", &["bloqueada-por: [a]"]),
    ])
}

/// As linhas não vazias do stdout depois do bloco (que tem de vir primeiro).
fn depois_do_bloco(stdout: &str, bloco: &str) -> Vec<String> {
    let resto = stdout
        .strip_prefix(bloco)
        .unwrap_or_else(|| panic!("o stdout tem de começar pelo bloco {bloco:?}:\n{stdout}"));
    resto
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn show_texto_imprime_bloqueada_por_e_bloqueia_embaixo_do_bloco() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "t"]);
    ok(&saida);
    let bloco_t = format!("{}> _em curso_\n", bloco("alfa", "t", "example", "Tê"));
    assert_eq!(
        depois_do_bloco(&saida.stdout, &bloco_t),
        vec![
            "bloqueada por: a, w".to_owned(),
            "bloqueia: 1 (u)".to_owned()
        ]
    );
}

#[test]
fn show_texto_bloqueia_conta_e_lista_quem_espera() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "a"]);
    ok(&saida);
    let resto = depois_do_bloco(&saida.stdout, &bloco("alfa", "a", "todo", "A"));
    let linhas: Vec<&String> = resto.iter().filter(|l| l.starts_with("bloqueia")).collect();
    assert_eq!(linhas.len(), 1, "uma linha `bloqueia`: {resto:?}");
    let ids = linhas[0]
        .strip_prefix("bloqueia: 3 (")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or_else(|| panic!("esperava `bloqueia: 3 (…)`: {:?}", linhas[0]));
    let ids: BTreeSet<String> = ids.split(", ").map(str::to_owned).collect();
    assert_eq!(ids, conjunto(&["b2", "b3", "t"]));
}

#[test]
fn show_texto_de_quem_so_espera_imprime_bloqueada_por() {
    let home = home_show();
    let saida = roda(home.path(), &["show", "u"]);
    ok(&saida);
    let resto = depois_do_bloco(&saida.stdout, &bloco("alfa", "u", "todo", "U"));
    assert!(
        resto.contains(&"bloqueada por: t".to_owned()),
        "linhas depois do bloco: {resto:?}"
    );
}

#[test]
fn show_texto_de_task_em_ciclo_avisa_em_stderr_e_sai_1() {
    let home = home_ciclo_de_tres();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["show", "a"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert!(
        saida.stdout.starts_with(&bloco("alfa", "a", "todo", "A")),
        "o bloco sai mesmo com ciclo:\n{}",
        saida.stdout
    );
    let linha_ciclo = saida
        .stderr
        .lines()
        .find(|l| l.contains("CICLO: "))
        .unwrap_or_else(|| panic!("stderr sem `CICLO: `: {:?}", saida.stderr));
    let depois = &linha_ciclo[linha_ciclo.find("CICLO: ").expect("CICLO") + "CICLO: ".len()..];
    confere_ciclo(
        &caminho_do_ciclo(depois),
        &[("a", "b"), ("b", "c"), ("c", "a")],
        &["a", "b", "c"],
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn show_texto_de_ciclo_de_duas_tasks_avisa_em_stderr_e_sai_1() {
    let home = home_ciclo_de_dois();
    let saida = roda(home.path(), &["show", "b"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert!(
        saida
            .stdout
            .starts_with(&bloco("alfa", "b", "example", "B"))
    );
    let linha_ciclo = saida
        .stderr
        .lines()
        .find(|l| l.contains("CICLO: "))
        .unwrap_or_else(|| panic!("stderr sem `CICLO: `: {:?}", saida.stderr));
    confere_ciclo(
        &caminho_do_ciclo(linha_ciclo),
        &[("a", "b"), ("b", "a")],
        &["a", "b"],
    );
}

// ===========================================================================
// set --bloqueada-por: o vault
// ===========================================================================

/// `x`: frontmatter com chaves alheias (inclusive uma lista em várias
/// linhas), bloco com descrição e nota, corpo com checklist.
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
);

/// `y`: só o bloco.
const Y_MD: &str = "> [!example] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n";

const X: &str = "projects/alfa/tasks/x.md";
const Y: &str = "projects/alfa/tasks/y.md";
const VELHA: &str = "projects/alfa/tasks/archived/done/velha.md";

/// `alfa`: `x`, `y`, `w` (todo), `algum` (Algum dia) e `velha` (arquivada);
/// `beta`: `q`. Nenhuma dependência.
fn home_set() -> tempfile::TempDir {
    home_pequeno(&[
        (X.into(), X_MD.into()),
        (Y.into(), Y_MD.into()),
        t("alfa", "w", "todo", &[]),
        t("alfa", "algum", "quote", &[]),
        (
            VELHA.into(),
            arquivada("alfa", "done", "velha", "done", "Velha", &[]),
        ),
        t("beta", "q", "todo", &[]),
    ])
}

/// O stdout de um `set` que deu certo: o caminho absoluto da task.
fn caminho(home: &Path, rel: &str) -> String {
    format!("{}\n", abs(home, rel))
}

// ===========================================================================
// set --bloqueada-por: caminho feliz
// ===========================================================================

#[test]
fn set_bloqueada_por_acrescenta_a_lista_no_fim_do_frontmatter() {
    let home = home_set();
    let saida = roda(home.path(), &["set", "x", "--bloqueada-por", "w,y"]);
    ok(&saida);
    assert_eq!(saida.stdout, caminho(home.path(), X));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace(
            "criado: 2026-01-01\n---\n",
            "criado: 2026-01-01\nbloqueada-por: [w, y]\n---\n"
        )
    );
}

#[test]
fn set_bloqueada_por_cria_o_frontmatter_quando_falta() {
    let home = home_set();
    let saida = roda(home.path(), &["set", "y", "--bloqueada-por", "w"]);
    ok(&saida);
    assert_eq!(saida.stdout, caminho(home.path(), Y));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nbloqueada-por: [w]\n---\n\n{Y_MD}")
    );
}

#[test]
fn set_bloqueada_por_substitui_a_lista_inteira_no_lugar() {
    let home = home_set();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nbloqueada-por: [velha]\n"),
    );
    ok(&roda(home.path(), &["set", "x", "--bloqueada-por", "w,y"]));
    assert_eq!(
        le(home.path(), X),
        X_MD.replace("id: x\n", "id: x\nbloqueada-por: [w, y]\n")
    );
}

#[test]
fn set_bloqueada_por_vazio_limpa_a_lista_e_mantem_as_outras_chaves() {
    let home = home_set();
    arquivo(
        home.path(),
        X,
        &X_MD.replace("id: x\n", "id: x\nbloqueada-por: [w, y]\n"),
    );
    let saida = roda(home.path(), &["set", "x", "--bloqueada-por", ""]);
    ok(&saida);
    assert_eq!(saida.stdout, caminho(home.path(), X));
    assert_eq!(le(home.path(), X), X_MD);
}

#[test]
fn set_bloqueada_por_vazio_tira_a_dependencia() {
    let home = home_set();
    arquivo(
        home.path(),
        Y,
        &format!("---\nbloqueada-por: [w]\n---\n\n{Y_MD}"),
    );
    ok(&roda(home.path(), &["set", "y", "--bloqueada-por", ""]));
    let texto = le(home.path(), Y);
    assert!(!texto.contains("bloqueada-por"), "{texto}");
    assert!(texto.ends_with(Y_MD), "{texto}");
    let linhas = lista(home.path());
    deps(&linha(&linhas, "y"), &[], false, &[]);
    deps(&linha(&linhas, "w"), &[], false, &[]);
}

#[test]
fn set_bloqueada_por_e_lido_de_volta_nos_dois_lados() {
    let home = home_set();
    ok(&roda(home.path(), &["set", "x", "--bloqueada-por", "w,y"]));
    let linhas = lista(home.path());
    deps(&linha(&linhas, "x"), &["w", "y"], true, &[]);
    deps(&linha(&linhas, "w"), &[], false, &["x"]);
    deps(&linha(&linhas, "y"), &[], false, &["x"]);
}

#[test]
fn set_bloqueada_por_aceita_task_arquivada_e_ela_nao_bloqueia() {
    let home = home_set();
    ok(&roda(
        home.path(),
        &["set", "y", "--bloqueada-por", "velha"],
    ));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nbloqueada-por: [velha]\n---\n\n{Y_MD}")
    );
    deps(&linha(&lista(home.path()), "y"), &["velha"], false, &[]);
}

#[test]
fn set_bloqueada_por_aceita_task_de_outro_projeto() {
    // O id é global (§10.1, "Fora").
    let home = home_set();
    ok(&roda(home.path(), &["set", "y", "--bloqueada-por", "q"]));
    let linhas = lista(home.path());
    deps(&linha(&linhas, "y"), &["q"], true, &[]);
    deps(&linha(&linhas, "q"), &[], false, &["y"]);
}

#[test]
fn set_bloqueada_por_aceita_task_escondida_e_ela_bloqueia() {
    let home = home_set();
    ok(&roda(
        home.path(),
        &["set", "y", "--bloqueada-por", "algum"],
    ));
    deps(&linha(&lista(home.path()), "y"), &["algum"], true, &[]);
}

#[test]
fn set_bloqueada_por_so_mexe_no_frontmatter_e_nao_reserializa_o_bloco() {
    // O bloco com link dessincronizado e espaço sobrando fica byte a byte.
    let home = home_set();
    let torto = "> [!example]   Ypsilon  \n> [[projects/alfa/tasks/outro|outro]]\n";
    arquivo(home.path(), Y, torto);
    ok(&roda(home.path(), &["set", "y", "--bloqueada-por", "w"]));
    assert_eq!(
        le(home.path(), Y),
        format!("---\nbloqueada-por: [w]\n---\n\n{torto}")
    );
}

#[test]
fn set_bloqueada_por_junto_com_status_grava_os_dois() {
    let home = home_set();
    ok(&roda(
        home.path(),
        &["set", "y", "--status", "tip", "--bloqueada-por", "w"],
    ));
    assert_eq!(
        le(home.path(), Y),
        "---\nbloqueada-por: [w]\n---\n\n> [!tip] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n"
    );
}

#[test]
fn set_bloqueada_por_so_grava_a_task_e_o_painel_fica_igual() {
    let home = home_set();
    ok(&roda(home.path(), &["rebuild"]));
    envelhece(home.path());
    let painel = le(home.path(), CURRENT);
    let antes = fotografa(home.path());
    ok(&roda(home.path(), &["set", "y", "--bloqueada-por", "w,x"]));
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(mudou.contains(Y), "a task tinha de mudar: {mudou:?}");
    assert!(
        mudou.is_subset(&conjunto(&[Y, CURRENT])),
        "só a task e o painel podem mudar: {mudou:?}"
    );
    assert_eq!(le(home.path(), CURRENT), painel);
}

#[test]
fn set_bloqueada_por_vazio_corta_um_ciclo_escrito_a_mao() {
    let home = home_ciclo_de_dois();
    ok(&roda(home.path(), &["set", "a", "--bloqueada-por", ""]));
    let linhas = lista(home.path());
    deps(&linha(&linhas, "a"), &[], false, &["b", "d"]);
    deps(&linha(&linhas, "b"), &["a"], true, &[]);
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.stdout, CONSISTENTE);
    assert_eq!(saida.codigo, Some(0));
}

// ===========================================================================
// set --bloqueada-por: recusas
// ===========================================================================

#[test]
fn set_bloqueada_por_recusa_id_inexistente_e_nao_grava() {
    let home = home_set();
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "zzz"], 1);
}

#[test]
fn set_bloqueada_por_recusa_a_lista_inteira_se_um_id_nao_existe() {
    let home = home_set();
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "w,zzz"], 1);
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "zzz,w"], 1);
}

#[test]
fn set_bloqueada_por_recusa_id_ambiguo_e_nao_grava() {
    let home = home_set();
    arquivo(
        home.path(),
        &rel("alfa", "dup"),
        &bloco("alfa", "dup", "todo", "Dup"),
    );
    arquivo(
        home.path(),
        &rel("beta", "dup"),
        &bloco("beta", "dup", "todo", "Dup"),
    );
    envelhece(home.path());
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "dup"], 1);
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "w,dup"], 1);
}

#[test]
fn set_bloqueada_por_recusa_a_propria_task_e_nao_grava() {
    let home = home_set();
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "x"], 1);
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por", "w,x"], 1);
}

#[test]
fn set_bloqueada_por_recusa_ciclo_e_nao_grava() {
    // §3.7. `b` espera `c` e `c` espera `a`; `a` esperar `b` fecha
    // `a → b → c → a`.
    let home = home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "b", "todo", &["bloqueada-por: [c]"]),
        t("alfa", "c", "todo", &["bloqueada-por: [a]"]),
        t("alfa", "w", "todo", &[]),
    ]);
    let saida = recusa_sem_gravar(home.path(), &["set", "a", "--bloqueada-por", "b"], 1);
    confere_ciclo(
        &caminho_do_ciclo(&saida.stderr),
        &[("a", "b"), ("b", "c"), ("c", "a")],
        &["a", "b", "c"],
    );
    // Com outro id válido na lista, a recusa é a mesma.
    let saida = recusa_sem_gravar(home.path(), &["set", "a", "--bloqueada-por", "w,b"], 1);
    confere_ciclo(
        &caminho_do_ciclo(&saida.stderr),
        &[("a", "w"), ("a", "b"), ("b", "c"), ("c", "a")],
        &["a", "b", "c"],
    );
    deps(&linha(&lista(home.path()), "a"), &[], false, &["c"]);
}

#[test]
fn set_bloqueada_por_recusa_ciclo_de_duas_tasks_e_nao_grava() {
    let home = home_pequeno(&[
        t("alfa", "p", "todo", &["bloqueada-por: [q]"]),
        t("beta", "q", "todo", &[]),
    ]);
    let saida = recusa_sem_gravar(home.path(), &["set", "q", "--bloqueada-por", "p"], 1);
    confere_ciclo(
        &caminho_do_ciclo(&saida.stderr),
        &[("p", "q"), ("q", "p")],
        &["p", "q"],
    );
}

#[test]
fn set_bloqueada_por_recusado_com_status_valido_nao_grava_nada() {
    let home = home_set();
    recusa_sem_gravar(
        home.path(),
        &["set", "x", "--status", "example", "--bloqueada-por", "zzz"],
        1,
    );
    recusa_sem_gravar(
        home.path(),
        &["set", "x", "--title", "Outro", "--bloqueada-por", "x"],
        1,
    );
}

#[test]
fn set_bloqueada_por_sem_valor_sai_2() {
    let home = home_set();
    recusa_sem_gravar(home.path(), &["set", "x", "--bloqueada-por"], 2);
}

#[test]
fn set_bloqueada_por_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_set();
    let antes = fotografa(home.path());
    let xdg = tempfile::tempdir().expect("tempdir");
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 60);
    let t0 = Instant::now();
    let saida = roda_com_xdg(
        home.path(),
        xdg.path(),
        &["set", "y", "--bloqueada-por", "w"],
    );
    let decorrido = t0.elapsed();
    assert_eq!(saida.codigo, Some(3), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, "");
    assert!(
        saida.stderr.starts_with("freitask: ") && saida.stderr.ends_with('\n'),
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
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// rename arrasta, delete deixa pendurada
// ===========================================================================

const A: &str = "projects/alfa/tasks/a.md";
const NOVO: &str = "projects/alfa/tasks/novo.md";
const B: &str = "projects/alfa/tasks/b.md";
const C: &str = "projects/alfa/tasks/c.md";
const E: &str = "projects/beta/tasks/e.md";
const FORA: &str = "projects/beta/tasks/fora.md";
const V: &str = "projects/alfa/tasks/archived/done/v.md";

/// `b`: a linha `bloqueada-por` entre chaves alheias.
const B_MD: &str = concat!(
    "---\n",
    "tags: [x]\n",
    "bloqueada-por: [a, c]\n",
    "prioridade: alta\n",
    "---\n",
    "\n",
    "> [!todo] Bê\n",
    "> [[projects/alfa/tasks/b|b]]\n",
    "> _esperando_\n",
    "\n",
    "corpo\n",
);

/// `alfa`: `a`, `b` (espera `a` e `c`), `c`, `v` arquivada (espera `a`).
/// `beta`: `e` (espera `fora` e `a`), `fora`. Nenhum link além da linha 2.
fn home_rename() -> tempfile::TempDir {
    home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        (B.into(), B_MD.into()),
        t("alfa", "c", "todo", &[]),
        (
            V.into(),
            arquivada("alfa", "done", "v", "done", "Vê", &["bloqueada-por: [a]"]),
        ),
        t("beta", "e", "example", &["bloqueada-por: [fora, a]"]),
        t("beta", "fora", "todo", &[]),
    ])
}

#[test]
fn rename_arrasta_a_dependencia_nas_outras_tasks() {
    // §3.7: o id muda em todas as linhas `bloqueada-por` do vault, ativas e
    // arquivadas, de qualquer projeto; o resto de cada arquivo fica igual.
    let home = home_rename();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["rename", "a", "novo"]);
    assert_eq!(saida.codigo, Some(0), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, caminho(home.path(), NOVO));
    assert_eq!(
        le(home.path(), B),
        B_MD.replace("bloqueada-por: [a, c]", "bloqueada-por: [novo, c]")
    );
    assert_eq!(
        le(home.path(), V),
        arquivada(
            "alfa",
            "done",
            "v",
            "done",
            "Vê",
            &["bloqueada-por: [novo]"]
        )
    );
    assert_eq!(
        le(home.path(), E),
        task(
            "beta",
            "e",
            "example",
            "E",
            &["bloqueada-por: [fora, novo]"]
        )
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[A, NOVO, B, V, E, CURRENT])
    );
    assert_eq!(le(home.path(), C), task("alfa", "c", "todo", "C", &[]));
    assert_eq!(
        le(home.path(), FORA),
        task("beta", "fora", "todo", "FORA", &[])
    );
}

#[test]
fn rename_mantem_a_dependencia_de_pe_e_o_doctor_nao_acusa_nada() {
    let home = home_rename();
    sucesso_doctor(home.path());
    assert_eq!(roda(home.path(), &["rename", "a", "novo"]).codigo, Some(0));
    let linhas = lista(home.path());
    deps(&linha(&linhas, "novo"), &[], false, &["b", "e"]);
    deps(&linha(&linhas, "b"), &["novo", "c"], true, &[]);
    deps(&linha(&linhas, "e"), &["fora", "novo"], true, &[]);
    sucesso_doctor(home.path());
}

#[test]
fn rename_de_bloqueadora_arquivada_tambem_arrasta() {
    // `c`, `g` e `h` esperam a arquivada `velha`; a ordem de cada lista fica.
    let home = home_semantica();
    assert_eq!(
        roda(home.path(), &["rename", "velha", "antiga"]).codigo,
        Some(0)
    );
    assert_eq!(
        le(home.path(), &rel("alfa", "c")),
        task("alfa", "c", "example", "C", &["bloqueada-por: [antiga]"])
    );
    assert_eq!(
        le(home.path(), &rel("alfa", "g")),
        task(
            "alfa",
            "g",
            "todo",
            "G",
            &["bloqueada-por: [antiga, a, fantasma]"]
        )
    );
    assert_eq!(
        le(home.path(), &rel("alfa", "h")),
        task(
            "alfa",
            "h",
            "todo",
            "H",
            &["bloqueada-por: [antiga, fantasma]"]
        )
    );
    let linhas = lista(home.path());
    deps(&linha(&linhas, "c"), &["antiga"], false, &[]);
    assert_eq!(
        conj(&linha(&linhas, "antiga")["bloqueia"]),
        conjunto(&["c", "g", "h"])
    );
}

#[test]
fn rename_arrasta_o_id_de_quem_e_esperado_pela_bloqueadora_renomeada() {
    // `c` é esperada por `b`; renomear `c` troca só o segundo id da lista.
    let home = home_rename();
    assert_eq!(roda(home.path(), &["rename", "c", "cc"]).codigo, Some(0));
    assert_eq!(
        le(home.path(), B),
        B_MD.replace("bloqueada-por: [a, c]", "bloqueada-por: [a, cc]")
    );
}

#[test]
fn rename_nao_toca_id_parecido_na_lista() {
    let home = home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "a-2", "todo", &[]),
        t("alfa", "aa", "todo", &[]),
        t("alfa", "m", "todo", &["bloqueada-por: [a-2, aa, a]"]),
    ]);
    assert_eq!(roda(home.path(), &["rename", "a", "novo"]).codigo, Some(0));
    assert_eq!(
        le(home.path(), &rel("alfa", "m")),
        task(
            "alfa",
            "m",
            "todo",
            "M",
            &["bloqueada-por: [a-2, aa, novo]"]
        )
    );
}

#[test]
fn delete_nao_toca_quem_dependia_e_a_dependencia_fica_pendurada() {
    // §10.1: `delete` não toca ninguém; o `doctor` acusa, como no
    // `link-pendurado`.
    let home = home_pequeno(&[
        (B.into(), B_MD.into()),
        t("alfa", "a", "todo", &[]),
        t("alfa", "c", "todo", &[]),
        t("alfa", "s", "todo", &["bloqueada-por: [a]"]),
    ]);
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["delete", "a"]);
    assert_eq!(saida.codigo, Some(0), "stderr: {:?}", saida.stderr);
    assert!(!existe(home.path(), A));
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(
        mudou.is_subset(&conjunto(&[A, CURRENT])),
        "delete não toca quem dependia: {mudou:?}"
    );
    let linhas = lista(home.path());
    // A inexistente não bloqueia; `b` segue bloqueada pela `c`, que existe.
    deps(&linha(&linhas, "s"), &["a"], false, &[]);
    deps(&linha(&linhas, "b"), &["a", "c"], true, &[]);
    let (saida, v) = doctor_json(home.path());
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(v["ok"], json!(false));
    let pendurados = do_kind(&v, "dependencia-pendurada");
    assert!(!pendurados.is_empty(), "`b` e `s` citam a `a` apagada: {v}");
    for f in &pendurados {
        assert_eq!(f["level"], json!("warn"), "{f}");
        assert_eq!(f["fixed"], json!(false), "{f}");
    }
    assert!(do_kind(&v, "dependencia-ciclica").is_empty(), "{v}");
}

// ===========================================================================
// doctor
// ===========================================================================

/// `doctor --json`, validado contra o schema.
fn doctor_json(home: &Path) -> (Saida, Value) {
    let saida = roda(home, &["doctor", "--json"]);
    assert_eq!(saida.stderr, "", "stderr do doctor --json");
    let v = json(&saida);
    valida("doctor", &v);
    (saida, v)
}

/// Os achados de um kind.
fn do_kind(v: &Value, kind: &str) -> Vec<Value> {
    v["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|f| f["kind"] == json!(kind))
        .cloned()
        .collect()
}

/// Os kinds dos achados, sem repetição.
fn kinds(v: &Value) -> BTreeSet<String> {
    v["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .map(|f| f["kind"].as_str().expect("kind").to_owned())
        .collect()
}

/// `doctor` sai 0 com `vault consistente`.
fn sucesso_doctor(home: &Path) {
    let saida = roda(home, &["doctor"]);
    assert_eq!(saida.codigo, Some(0), "stdout: {}", saida.stdout);
    assert_eq!(saida.stdout, CONSISTENTE);
    assert_eq!(saida.stderr, "");
}

/// O começo da linha de um achado no texto do `doctor` (`%-6s %-22s `,
/// formato da fatia 5).
fn cabeca(level: &str, kind: &str) -> String {
    format!("{level:<6} {kind:<22} ")
}

/// `alfa`: `a` e `b`, que espera `fantasma` (não existe em lugar nenhum).
fn home_pendurada() -> tempfile::TempDir {
    home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "b", "todo", &["bloqueada-por: [fantasma]"]),
    ])
}

#[test]
fn doctor_acusa_dependencia_pendurada_como_warn() {
    let home = home_pendurada();
    let (saida, v) = doctor_json(home.path());
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(v["ok"], json!(false));
    let achados = v["findings"].as_array().expect("findings");
    assert_eq!(achados.len(), 1, "{v}");
    assert_eq!(achados[0]["kind"], json!("dependencia-pendurada"));
    assert_eq!(achados[0]["level"], json!("warn"));
    assert_eq!(achados[0]["fixed"], json!(false));

    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stderr, "");
    assert!(
        saida
            .stdout
            .starts_with(&cabeca("warn", "dependencia-pendurada")),
        "{}",
        saida.stdout
    );
    assert!(
        saida.stdout.contains("\n1 achado(s), 1 pendente(s).\n"),
        "{}",
        saida.stdout
    );
}

#[test]
fn doctor_fix_nao_repara_dependencia_pendurada() {
    // Não se adivinha o id (§10.1).
    let home = home_pendurada();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--fix", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    valida("doctor", &v);
    let achados = do_kind(&v, "dependencia-pendurada");
    assert_eq!(achados.len(), 1, "{v}");
    assert_eq!(achados[0]["fixed"], json!(false));
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(
        mudou.is_subset(&conjunto(&[CURRENT])),
        "o --fix não toca a task: {mudou:?}"
    );
}

#[test]
fn doctor_acusa_ciclo_escrito_a_mao_como_error() {
    // O ciclo que o celular escreve sem passar pelo `set` aparece aqui.
    let home = home_ciclo_de_tres();
    let (saida, v) = doctor_json(home.path());
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(v["ok"], json!(false));
    assert_eq!(kinds(&v), conjunto(&["dependencia-ciclica"]), "{v}");
    for f in v["findings"].as_array().expect("findings") {
        assert_eq!(f["level"], json!("error"), "{f}");
        assert_eq!(f["fixed"], json!(false), "{f}");
    }

    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1));
    assert!(
        saida
            .stdout
            .starts_with(&cabeca("error", "dependencia-ciclica")),
        "{}",
        saida.stdout
    );
}

#[test]
fn doctor_acusa_ciclo_de_duas_tasks() {
    let home = home_ciclo_de_dois();
    let (saida, v) = doctor_json(home.path());
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(kinds(&v), conjunto(&["dependencia-ciclica"]), "{v}");
    for f in v["findings"].as_array().expect("findings") {
        assert_eq!(f["level"], json!("error"), "{f}");
    }
}

#[test]
fn doctor_fix_nao_corta_o_ciclo() {
    // Escolher que aresta cortar é decisão editorial (§10.1).
    let home = home_ciclo_de_tres();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--fix", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    valida("doctor", &v);
    let achados = do_kind(&v, "dependencia-ciclica");
    assert!(!achados.is_empty(), "{v}");
    for f in &achados {
        assert_eq!(f["fixed"], json!(false), "{f}");
    }
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(
        mudou.is_subset(&conjunto(&[CURRENT])),
        "o --fix não toca as tasks: {mudou:?}"
    );
}

#[test]
fn doctor_nao_acusa_dependencia_de_ativa_escondida_ou_arquivada() {
    let home = home_pequeno(&[
        t("alfa", "a", "todo", &[]),
        t("alfa", "algum", "quote", &[]),
        t("alfa", "adiada", "todo", &["volta-em: 2026-12-01"]),
        (
            VELHA.into(),
            arquivada("alfa", "done", "velha", "done", "Velha", &[]),
        ),
        t(
            "alfa",
            "b",
            "todo",
            &["bloqueada-por: [a, algum, adiada, velha]"],
        ),
        t("beta", "k", "todo", &["bloqueada-por: [b]"]),
    ]);
    sucesso_doctor(home.path());
}

#[test]
fn doctor_acha_pendurada_e_ciclo_no_mesmo_vault() {
    let home = home_pequeno(&[
        t("alfa", "a", "todo", &["bloqueada-por: [b]"]),
        t("alfa", "b", "todo", &["bloqueada-por: [a]"]),
        t("alfa", "c", "todo", &["bloqueada-por: [sumiu]"]),
    ]);
    let (saida, v) = doctor_json(home.path());
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        kinds(&v),
        conjunto(&["dependencia-ciclica", "dependencia-pendurada"]),
        "{v}"
    );
    assert_eq!(do_kind(&v, "dependencia-pendurada").len(), 1, "{v}");
}
