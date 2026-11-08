//! Aceite da fatia 5 — Projeto e doctor (spec §5): `encerrar`, `reabrir`, os
//! 12 checks do `doctor` mais `temporario-orfao`, `--fix`, `--json` e
//! `--quiet`; I3 (fix idempotente), I6/D7 (o `doctor` sem `--fix` é leitura
//! pura), I7 e D3.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§0, §1.4, §1.5, §1.6, §2.1, §2.2, §2.4, §3.2 I3/I6/I7/I8/I9, §3.3, §3.4
//! D1/D3/D4/D5/D6/D7, §3.7), `docs/freitask.md` ("Encerrar / reabrir um
//! projeto", "Referências do Obsidian", "CLI", "`freitask doctor`") e
//! `docs/decisoes-da-reescrita.md` (7, 11, 21, 22, 27, 29, 34, 35). O texto
//! exato de stdout/stderr e dos arquivos foi medido no oráculo Lua
//! (`lua/freitask/cli.lua`) num HOME temporário; onde a spec manda divergir
//! (D1, D3, D4, D5, D6, D7, I7, saída 3, `FREITASK_TODAY` no histórico,
//! `temporario-orfao`), vale a spec. Nenhum valor esperado aqui foi gerado do
//! binário sob teste.
//!
//! Decisões tomadas aqui, onde a spec e o oráculo não fecham (repetidas no
//! relatório do rascunho):
//! - ordem dos achados no texto: `sync-conflict`, `fora-do-padrao` (cada um
//!   por caminho), `id-duplicado` (por id), depois task a task (ativas por
//!   caminho, depois arquivadas por caminho; em cada uma, `sem-callout` ou
//!   `status-0`, `bloco-dessincronizado`, `callout-vs-pasta`,
//!   `frontmatter-id`, `historico-ausente`, `prioridade-desconhecida`,
//!   `volta-em-invalido`), `link-pendurado` (por nota, linha e posição) e
//!   `temporario-orfao` por último. É a ordem do oráculo com o `pairs()` do
//!   `id-duplicado` fixado. O JSON é comparado ordenado por (kind, path, msg)
//!   (decisão 7); o `path` do `id-duplicado` é o da primeira cópia nessa ordem;
//! - `--fix` procura link pendurado depois de reparar, como hoje: o link da
//!   linha 2 de uma task reparada deixa de ser achado;
//! - `ok` do JSON é "não houve achado", mesmo reparado, como hoje; `findings`
//!   vazio é `[]` (o schema recusa o `{}` que o `vim.json.encode` daria);
//! - num HOME sem `ObsidianVault/`, o `doctor` diz `vault consistente` e sai 0
//!   sem criar o vault (a regra do `list` na fatia 2);
//! - o `doctor` sem `--fix` não toma o lock (é leitura, §1.5) e roda com ele
//!   ocupado; `encerrar`, `reabrir` e `doctor --fix` esperam até 5 s e saem 3;
//! - I7 no `doctor`: `sync-conflict` vale para `*.sync-conflict-*.md` em
//!   qualquer lugar sob `projects/`, inclusive fora de `tasks/` e dentro de
//!   `projects/_encerrados/` (que está sob `projects/`); os checks de task
//!   seguem sem ver `_encerrados/`, como hoje. Não se fixa conflito de outra
//!   extensão nem fora de `projects/`;
//! - D3: `encerrar` recusa (saída 1, nada movido) `*.sync-conflict-*` em
//!   qualquer lugar da pasta do projeto (I7: "`encerrar` o recusa") e os `.md`
//!   sob `tasks/` que o `doctor` chama de `fora-do-padrao`. A mensagem é
//!   `o projeto "<p>" tem arquivo fora do padrão; resolva antes de encerrar
//!   (`freitask doctor`):` seguida de uma linha `  <caminho> (<kind>)` por
//!   arquivo, caminho relativo ao vault, em ordem de caminho. Não se fixa
//!   arquivo não-`.md` em `tasks/`, nem qual recusa vence quando há também
//!   task ativa;
//! - `temporario-orfao` (warn): arquivo `.freitask.<nome>.<pid>.tmp` (pid em
//!   dígitos), procurado no vault inteiro, inclusive a raiz e pastas ocultas
//!   (`.freitask/`, onde mora o `status.json`), porque a gravação acontece
//!   na pasta do destino (§1.5). `msg`: `temporário de uma gravação
//!   interrompida — o arquivo de destino ficou como estava`. O `--fix` o
//!   apaga (`fixed: true`): é o resto de uma gravação que não chegou ao
//!   `rename`, então o destino está no estado anterior (I8) e apagar não
//!   perde nada gravado;
//! - `historico-ausente` com `--fix`: a marca `[reconstruído pelo doctor; data
//!   real desconhecida]` vai na linha acrescentada, no fim da seção
//!   `## Histórico` (`docs/freitask.md`), com o dia de `FREITASK_TODAY`. O
//!   oráculo a cola na última linha do arquivo quando há outra seção depois
//!   do histórico (bug; ver o relatório);
//! - D4 no `--fix`, no `encerrar` e no `reabrir`: CRLF, BOM e falta de `\n`
//!   final sobrevivem; linha acrescentada sai no fim de linha do arquivo, e
//!   sem `\n` final a última linha acrescentada também fica sem (a regra das
//!   fatias 3 e 4); o BOM antes do `---` não esconde o frontmatter;
//! - o aviso `freitask: N referência(s) atualizada(s) em M arquivo(s)` do
//!   `encerrar`/`reabrir` sai só com N > 0 e conta só os links reescritos:
//!   tabela conta (D1); código (D5) e `*.sync-conflict-*` (I7) não;
//! - link em frontmatter se reescreve (decisão 35); `daily/`, `CURRENT.md`
//!   (inclusive as Notas Avulsas) e anexo que não é `.md` não;
//! - "nada gravado" é: nenhum arquivo do vault mudou de bytes nem de mtime e
//!   nenhuma entrada apareceu ou sumiu; nos caminhos felizes, "só mudou X"
//!   olha só arquivos;
//! - fica de fora o que a spec não fecha: `.` e `..` em `encerrar`/`reabrir`
//!   (a D2 não os cita), nota oculta e symlink na reescrita (decisão 34), e
//!   `CURRENT.md` na varredura de `link-pendurado` (o oráculo o varre).
//!
//! Já passam contra o esqueleto: `schema_doctor_recusa_forma_errada` (só
//! confere o schema, sem binário). `encerrar_sem_projeto_sai_2` e
//! `reabrir_sem_projeto_sai_2` acertam o código mas não a mensagem.

mod util;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use util::*;

// ===========================================================================
// Fixtures comuns
// ===========================================================================

/// Um vault com `status.json` default e os arquivos dados, envelhecido (mtime
/// no passado), sem `CURRENT.md`.
fn home_pequeno(arquivos: &[(&str, &str)]) -> tempfile::TempDir {
    let home = home_com(arquivos);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects");
    envelhece(home.path());
    home
}

/// O painel de 2026-11-08 com a contagem e as seções dadas (formato fixado na
/// fatia 2).
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

/// O stdout de um comando que deu certo: o caminho absoluto e `\n`.
fn caminho(home: &Path, rel: &str) -> String {
    format!("{}\n", abs(home, rel))
}

/// Os bytes de cada ARQUIVO de uma foto (sem mtime).
fn bytes_de(foto: &Foto) -> BTreeMap<String, Vec<u8>> {
    foto.iter()
        .filter_map(|(k, v)| match v {
            Entrada::Arquivo(b, _) => Some((k.clone(), b.clone())),
            Entrada::Pasta => None,
        })
        .collect()
}

/// Como um arquivo de texto termina suas linhas e começa.
#[derive(Clone, Copy, Debug)]
enum Variante {
    Crlf,
    Bom,
    SemQuebraFinal,
}

/// Leva um texto escrito em LF, com `\n` final e sem BOM, para a variante.
fn na_variante(v: Variante, lf: &str) -> Vec<u8> {
    match v {
        Variante::Crlf => lf.replace('\n', "\r\n").into_bytes(),
        Variante::Bom => format!("\u{feff}{lf}").into_bytes(),
        Variante::SemQuebraFinal => lf
            .strip_suffix('\n')
            .expect("termina em \\n")
            .as_bytes()
            .to_vec(),
    }
}

// ===========================================================================
// Lock (spec §1.5, §2.4)
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

// ===========================================================================
// encerrar: o vault de partida
// ===========================================================================

const MOC: &str = "projects/alfa/alfa.md";
const SPEC: &str = "projects/alfa/spec.md";
const DEC: &str = "projects/alfa/decisoes/0001-escolha.md";
const LOG: &str = "projects/alfa/anexos/v/log.txt";
const V: &str = "projects/alfa/tasks/archived/done/v.md";
const W: &str = "projects/alfa/tasks/archived/dropped/w.md";
const B: &str = "projects/beta/tasks/b.md";
const NOTA: &str = "inbox/nota.md";
const NOTA_CONFLITO: &str = "inbox/nota.sync-conflict-20261101-120000-ABCDEFG.md";
const DAILY: &str = "daily/2026-11-01.md";
const CURRENT: &str = "CURRENT.md";

const MOC_F: &str = "projects/_encerrados/alfa/alfa.md";
const SPEC_F: &str = "projects/_encerrados/alfa/spec.md";
const DEC_F: &str = "projects/_encerrados/alfa/decisoes/0001-escolha.md";
const LOG_F: &str = "projects/_encerrados/alfa/anexos/v/log.txt";
const V_F: &str = "projects/_encerrados/alfa/tasks/archived/done/v.md";
const W_F: &str = "projects/_encerrados/alfa/tasks/archived/dropped/w.md";

const ALFA: &str = "projects/alfa";
const ALFA_F: &str = "projects/_encerrados/alfa";

/// O MOC: link no frontmatter (decisão 35), links path-qualificados para
/// dentro da pasta, um curto, um para a pasta sem a barra final e um para um
/// vizinho cujo nome começa por `alfa`.
const MOC_MD: &str = concat!(
    "---\n",
    "tipo: moc\n",
    "related: \"[[projects/alfa/spec|spec]]\"\n",
    "---\n",
    "\n",
    "# Alfa\n",
    "\n",
    "- spec: [[projects/alfa/spec]]\n",
    "- decisão: [[projects/alfa/decisoes/0001-escolha|0001]]\n",
    "- feita: [[projects/alfa/tasks/archived/done/v|v]] e [[v]]\n",
    "- largada: [[projects/alfa/tasks/archived/dropped/w#Notas|w]]\n",
    "- a pasta sem barra: [[projects/alfa]]\n",
    "- vizinho de prefixo: [[projects/alfa-b/nota]]\n",
);

const MOC_FECHADO: &str = concat!(
    "---\n",
    "tipo: moc\n",
    "related: \"[[projects/_encerrados/alfa/spec|spec]]\"\n",
    "---\n",
    "\n",
    "# Alfa\n",
    "\n",
    "- spec: [[projects/_encerrados/alfa/spec]]\n",
    "- decisão: [[projects/_encerrados/alfa/decisoes/0001-escolha|0001]]\n",
    "- feita: [[projects/_encerrados/alfa/tasks/archived/done/v|v]] e [[v]]\n",
    "- largada: [[projects/_encerrados/alfa/tasks/archived/dropped/w#Notas|w]]\n",
    "- a pasta sem barra: [[projects/alfa]]\n",
    "- vizinho de prefixo: [[projects/alfa-b/nota]]\n",
);

const SPEC_MD: &str = "# Spec\n\n## Escopo\n\nsó texto\n";

const DEC_MD: &str = "---\ntipo: decisao\n---\n\nvolta para o [[projects/alfa/alfa|MOC]]\n";
const DEC_FECHADA: &str =
    "---\ntipo: decisao\n---\n\nvolta para o [[projects/_encerrados/alfa/alfa|MOC]]\n";

/// Anexo que não é `.md`: muda de pasta junto e não tem link reescrito.
const LOG_TXT: &str = "[[projects/alfa/spec]]\n";

const V_MD: &str = concat!(
    "> [!done] Vê\n",
    "> [[projects/alfa/tasks/archived/done/v|v]]\n",
    "\n",
    "ver a [[projects/alfa/spec|spec]] e o anexo ![[projects/alfa/anexos/v/log.txt]]\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
);

const V_FECHADA: &str = concat!(
    "> [!done] Vê\n",
    "> [[projects/_encerrados/alfa/tasks/archived/done/v|v]]\n",
    "\n",
    "ver a [[projects/_encerrados/alfa/spec|spec]] e o anexo ![[projects/_encerrados/alfa/anexos/v/log.txt]]\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
);

const W_MD: &str = concat!(
    "> [!warning] Dabliu\n",
    "> [[projects/alfa/tasks/archived/dropped/w|w]]\n",
    "> travada\n",
    "\n",
    "## Notas\n",
    "\n",
    "nada\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-10-02 — arquivada em `archived/dropped` (abandonado)\n",
);

const W_FECHADA: &str = concat!(
    "> [!warning] Dabliu\n",
    "> [[projects/_encerrados/alfa/tasks/archived/dropped/w|w]]\n",
    "> travada\n",
    "\n",
    "## Notas\n",
    "\n",
    "nada\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-10-02 — arquivada em `archived/dropped` (abandonado)\n",
);

const B_MD: &str = concat!(
    "> [!todo] Bê\n",
    "> [[projects/beta/tasks/b|b]]\n",
    "\n",
    "depende de [[projects/alfa/tasks/archived/done/v|v]] (ver [[v]])\n",
);

const B_FECHADA: &str = concat!(
    "> [!todo] Bê\n",
    "> [[projects/beta/tasks/b|b]]\n",
    "\n",
    "depende de [[projects/_encerrados/alfa/tasks/archived/done/v|v]] (ver [[v]])\n",
);

/// A nota fora do projeto com cada forma de link que a reescrita tem de
/// entender, e as que ela não pode tocar (curtos, código).
const NOTA_MD: &str = r"---
fonte: '[[projects/alfa/alfa|alfa]]'
---

# Nota

- alias: [[projects/alfa/spec|a spec]]
- heading: [[projects/alfa/spec#Escopo]]
- bloco: [[projects/alfa/spec^abc123]]
- embed: ![[projects/alfa/spec]]
- anexo: [[projects/alfa/anexos/v/log.txt]]
- com .md: [[projects/alfa/spec.md|spec]]
- curtos: [[alfa]], [[spec]] e [[v]]
- no span: `[[projects/alfa/spec]]`

```
[[projects/alfa/spec]]
```

| link | nota |
| --- | --- |
| [[projects/alfa/spec\|spec]] | tabela |
";

const NOTA_FECHADA: &str = r"---
fonte: '[[projects/_encerrados/alfa/alfa|alfa]]'
---

# Nota

- alias: [[projects/_encerrados/alfa/spec|a spec]]
- heading: [[projects/_encerrados/alfa/spec#Escopo]]
- bloco: [[projects/_encerrados/alfa/spec^abc123]]
- embed: ![[projects/_encerrados/alfa/spec]]
- anexo: [[projects/_encerrados/alfa/anexos/v/log.txt]]
- com .md: [[projects/_encerrados/alfa/spec.md|spec]]
- curtos: [[alfa]], [[spec]] e [[v]]
- no span: `[[projects/alfa/spec]]`

```
[[projects/alfa/spec]]
```

| link | nota |
| --- | --- |
| [[projects/_encerrados/alfa/spec\|spec]] | tabela |
";

const CONFLITO_MD: &str = "[[projects/alfa/spec|spec]]\n";
const DAILY_MD: &str =
    "# Dia\n\n- [[projects/alfa/alfa|alfa]] e [[projects/alfa/tasks/archived/done/v|v]]\n";

/// O aviso do `encerrar` e do `reabrir` sobre o vault de partida: 19 links em
/// 6 arquivos (MOC 5, decisão 1, `v` 3, `w` 1, `b` 1, nota 8).
fn aviso_do_encerrar() -> String {
    aviso_refs(19, 6)
}

/// `alfa` só com tasks arquivadas (MOC, spec, decisão, anexo); `beta` com uma
/// task ativa que cita `alfa`; uma nota em `inbox/`, a cópia de conflito dela,
/// um `daily/`. Sem `CURRENT.md`.
fn home_encerrar() -> tempfile::TempDir {
    home_pequeno(&[
        (MOC, MOC_MD),
        (SPEC, SPEC_MD),
        (DEC, DEC_MD),
        (LOG, LOG_TXT),
        (V, V_MD),
        (W, W_MD),
        (B, B_MD),
        (NOTA, NOTA_MD),
        (NOTA_CONFLITO, CONFLITO_MD),
        (DAILY, DAILY_MD),
    ])
}

const SECAO_BETA: &str = "## beta\n\n> [!todo] Bê\n> [[projects/beta/tasks/b|b]]\n\n";

// ===========================================================================
// encerrar: caminho feliz
// ===========================================================================

#[test]
fn encerrar_move_a_pasta_inteira_e_imprime_o_caminho_novo() {
    let home = home_encerrar();
    let saida = roda(home.path(), &["encerrar", "alfa"]);
    sucesso(&saida, &caminho(home.path(), ALFA_F), &aviso_do_encerrar());
    assert!(!existe(home.path(), ALFA), "projects/alfa/ tinha de sair");
    for rel in [MOC, SPEC, DEC, LOG, V, W] {
        assert!(!existe(home.path(), rel), "{rel} ficou para trás");
    }
    for rel in [MOC_F, SPEC_F, DEC_F, LOG_F, V_F, W_F] {
        assert!(existe(home.path(), rel), "{rel} não chegou");
    }
}

#[test]
fn encerrar_reescreve_os_links_das_notas_da_propria_pasta() {
    // A linha 2 das tasks acompanha porque a reescrita alcança a pasta nova.
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert_eq!(le(home.path(), MOC_F), MOC_FECHADO);
    assert_eq!(le(home.path(), DEC_F), DEC_FECHADA);
    assert_eq!(le(home.path(), V_F), V_FECHADA);
    assert_eq!(le(home.path(), W_F), W_FECHADA);
    assert_eq!(le(home.path(), SPEC_F), SPEC_MD);
}

#[test]
fn encerrar_nao_reescreve_anexo_que_nao_e_markdown() {
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert_eq!(le(home.path(), LOG_F), LOG_TXT);
}

#[test]
fn encerrar_reescreve_os_links_path_qualificados_do_vault_e_deixa_os_curtos() {
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert_eq!(le(home.path(), B), B_FECHADA);
    assert_eq!(le(home.path(), NOTA), NOTA_FECHADA);
}

#[test]
fn encerrar_reescreve_link_no_frontmatter() {
    // Decisão 35: link em frontmatter é link de verdade.
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert!(
        le(home.path(), NOTA)
            .starts_with("---\nfonte: '[[projects/_encerrados/alfa/alfa|alfa]]'\n---\n")
    );
    assert!(
        le(home.path(), MOC_F).contains("\nrelated: \"[[projects/_encerrados/alfa/spec|spec]]\"\n")
    );
}

#[test]
fn encerrar_reescreve_link_em_tabela() {
    // D1: o `\|` do alias em tabela não atrapalha.
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert!(
        le(home.path(), NOTA).ends_with("| [[projects/_encerrados/alfa/spec\\|spec]] | tabela |\n")
    );
}

#[test]
fn encerrar_nao_reescreve_link_em_codigo() {
    // D5: bloco cercado e code span ficam como estavam.
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    let nota = le(home.path(), NOTA);
    assert!(nota.contains("- no span: `[[projects/alfa/spec]]`\n"));
    assert!(nota.contains("```\n[[projects/alfa/spec]]\n```\n"));
}

#[test]
fn encerrar_nao_toca_sync_conflict_nem_daily() {
    // I7 e `docs/freitask.md`: `*.sync-conflict-*` nunca tem link reescrito, e
    // `daily/` é histórico.
    let home = home_encerrar();
    let antes = fotografa(home.path());
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    let depois = fotografa(home.path());
    for rel in [NOTA_CONFLITO, DAILY, ".freitask/status.json"] {
        assert_eq!(antes.get(rel), depois.get(rel), "{rel} não podia mudar");
    }
}

#[test]
fn encerrar_so_grava_a_pasta_movida_as_notas_que_a_citam_e_o_current() {
    let home = home_encerrar();
    let antes = fotografa(home.path());
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[
            MOC, SPEC, DEC, LOG, V, W, MOC_F, SPEC_F, DEC_F, LOG_F, V_F, W_F, B, NOTA, CURRENT,
        ])
    );
}

#[test]
fn encerrar_nao_escreve_no_historico_das_tasks() {
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    // `v` e `w` só mudaram nos links: o `## Histórico` é o de antes.
    assert!(
        le(home.path(), V_F)
            .ends_with("## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n")
    );
    assert!(le(home.path(), W_F).ends_with(
        "## Histórico\n\n- 2026-10-02 — arquivada em `archived/dropped` (abandonado)\n"
    ));
}

#[test]
fn encerrar_regenera_o_current_sem_o_projeto() {
    let home = home_encerrar();
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert_eq!(le(home.path(), CURRENT), painel(1, 1, SECAO_BETA));
}

#[test]
fn encerrar_nao_reescreve_link_nas_notas_avulsas_do_current() {
    let home = home_encerrar();
    let avulsa = "- ver [[projects/alfa/alfa|alfa]]\n";
    let antes = painel(1, 2, &format!("## alfa\n\n_(sem tasks)_\n\n{SECAO_BETA}"));
    arquivo(home.path(), CURRENT, &antes.replacen("- \n", avulsa, 1));
    envelhece(home.path());
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    assert_eq!(
        le(home.path(), CURRENT),
        painel(1, 1, SECAO_BETA).replacen("- \n", avulsa, 1)
    );
}

#[test]
fn encerrar_projeto_so_com_tasks_vazio_move_a_pasta() {
    let home = home_pequeno(&[(B, B_MD)]);
    pasta(home.path(), "projects/gama/tasks");
    let saida = roda(home.path(), &["encerrar", "gama"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/_encerrados/gama"),
        "",
    );
    assert!(!existe(home.path(), "projects/gama"));
    assert!(
        vault(home.path())
            .join("projects/_encerrados/gama/tasks")
            .is_dir()
    );
}

#[test]
fn encerrar_sem_referencias_nao_avisa_nada_em_stderr() {
    let home = home_pequeno(&[
        (SPEC, SPEC_MD),
        (B, "> [!todo] Bê\n> [[projects/beta/tasks/b|b]]\n"),
    ]);
    pasta(home.path(), "projects/alfa/tasks");
    let saida = roda(home.path(), &["encerrar", "alfa"]);
    sucesso(&saida, &caminho(home.path(), ALFA_F), "");
    assert_eq!(le(home.path(), SPEC_F), SPEC_MD);
}

// ===========================================================================
// encerrar: recusas (saída 1, nada movido)
// ===========================================================================

/// `gama` com uma task ativa em cada situação que o painel trata diferente;
/// `so-um-dia` com uma task só, em Algum dia; `so-adiada` com uma task só,
/// adiada; `delta` sem `tasks/`; `omega` encerrado; `beta` com uma task.
fn home_recusas() -> tempfile::TempDir {
    let home = home_pequeno(&[
        (
            "projects/gama/tasks/zz.md",
            "---\nprioridade: alta\n---\n\n> [!warning] Alta\n> [[projects/gama/tasks/zz|zz]]\n",
        ),
        (
            "projects/gama/tasks/bog.md",
            "> [!bogus] Bog\n> [[projects/gama/tasks/bog|bog]]\n",
        ),
        ("projects/gama/tasks/sc.md", "sem callout\n"),
        (
            "projects/gama/tasks/adiada.md",
            "---\nvolta-em: 2027-01-01\n---\n\n> [!todo] Adiada\n> [[projects/gama/tasks/adiada|adiada]]\n",
        ),
        (
            "projects/gama/tasks/ex.md",
            "> [!example] Ex\n> [[projects/gama/tasks/ex|ex]]\n",
        ),
        (
            "projects/gama/tasks/um-dia.md",
            "> [!quote] Um dia\n> [[projects/gama/tasks/um-dia|um-dia]]\n",
        ),
        (
            "projects/gama/tasks/archived/done/f.md",
            "> [!done] Feita\n> [[projects/gama/tasks/archived/done/f|f]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
        ),
        ("projects/gama/gama.md", "# Gama\n"),
        (
            "projects/so-um-dia/tasks/u.md",
            "> [!quote] Talvez\n> [[projects/so-um-dia/tasks/u|u]]\n",
        ),
        (
            "projects/so-adiada/tasks/a.md",
            "---\nvolta-em: 2026-12-25\n---\n\n> [!tip] Depois\n> [[projects/so-adiada/tasks/a|a]]\n",
        ),
        ("projects/delta/delta.md", "# Delta, sem tasks/\n"),
        ("projects/_encerrados/omega/omega.md", "# Omega\n"),
        (B, "> [!todo] Bê\n> [[projects/beta/tasks/b|b]]\n"),
    ]);
    pasta(home.path(), "projects/_encerrados/omega/tasks");
    envelhece(home.path());
    home
}

fn recusa_sem_mexer(home: &Path, args: &[&str], codigo: i32, msg: &str) {
    let antes = fotografa(home);
    recusa(&roda(home, args), codigo, msg);
    nada_mudou(&antes, &fotografa(home));
}

#[test]
fn encerrar_recusa_projeto_com_task_ativa_e_lista_todas() {
    // Em qualquer fase, inclusive Algum dia, a adiada, o status 0 e o arquivo
    // sem callout; na ordem do painel (alta primeiro, depois fase e id).
    let home = home_recusas();
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "gama"],
        1,
        concat!(
            "o projeto \"gama\" tem task ativa; arquive-a antes de encerrar ",
            "(`freitask archive <id> <tipo>`):\n",
            "  zz (Bloqueada)\n",
            "  bog (Sem status)\n",
            "  sc (Sem status)\n",
            "  adiada (Não iniciada)\n",
            "  ex (Em implementação)\n",
            "  um-dia (Algum dia)",
        ),
    );
}

#[test]
fn encerrar_recusa_projeto_so_com_task_em_algum_dia() {
    let home = home_recusas();
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "so-um-dia"],
        1,
        concat!(
            "o projeto \"so-um-dia\" tem task ativa; arquive-a antes de encerrar ",
            "(`freitask archive <id> <tipo>`):\n",
            "  u (Algum dia)",
        ),
    );
}

#[test]
fn encerrar_recusa_projeto_so_com_task_adiada() {
    let home = home_recusas();
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "so-adiada"],
        1,
        concat!(
            "o projeto \"so-adiada\" tem task ativa; arquive-a antes de encerrar ",
            "(`freitask archive <id> <tipo>`):\n",
            "  a (Refinada)",
        ),
    );
}

#[test]
fn encerrar_recusa_projeto_que_nao_existe() {
    let home = home_recusas();
    let msg = format!(
        "o projeto \"nada\" não existe: não há {}/tasks/",
        abs(home.path(), "projects/nada")
    );
    recusa_sem_mexer(home.path(), &["encerrar", "nada"], 1, &msg);
}

#[test]
fn encerrar_recusa_pasta_de_projeto_sem_tasks() {
    // Projeto é a pasta com `tasks/` dentro; `delta` não é.
    let home = home_recusas();
    let msg = format!(
        "o projeto \"delta\" não existe: não há {}/tasks/",
        abs(home.path(), "projects/delta")
    );
    recusa_sem_mexer(home.path(), &["encerrar", "delta"], 1, &msg);
}

#[test]
fn encerrar_recusa_nome_reservado() {
    let home = home_recusas();
    for p in ["daily", "templates", "_encerrados"] {
        recusa_sem_mexer(
            home.path(),
            &["encerrar", p],
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
    }
}

#[test]
fn encerrar_recusa_nome_com_barra() {
    let home = home_recusas();
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "gama/tasks"],
        1,
        "projeto não pode conter '/': gama/tasks",
    );
}

#[test]
fn encerrar_recusa_projeto_ja_encerrado() {
    let home = home_recusas();
    let msg = format!(
        "o projeto \"omega\" já está encerrado: {} existe",
        abs(home.path(), "projects/_encerrados/omega")
    );
    recusa_sem_mexer(home.path(), &["encerrar", "omega"], 1, &msg);
}

#[test]
fn encerrar_sem_projeto_sai_2() {
    // Já passa contra o esqueleto só no código; a mensagem é a de hoje.
    let home = home_recusas();
    recusa_sem_mexer(home.path(), &["encerrar"], 2, "falta o projeto");
}

// ---------------------------------------------------------------------------
// D3: `*.sync-conflict-*` e arquivo fora do padrão
// ---------------------------------------------------------------------------

/// O cabeçalho da recusa da D3 (texto decidido aqui; ver o topo do arquivo).
fn recusa_d3(projeto: &str, linhas: &[&str]) -> String {
    let mut s = format!(
        "o projeto \"{projeto}\" tem arquivo fora do padrão; resolva antes de encerrar (`freitask doctor`):"
    );
    for l in linhas {
        s.push_str("\n  ");
        s.push_str(l);
    }
    s
}

#[test]
fn encerrar_recusa_sync_conflict_e_fora_do_padrao() {
    // D3 (spec §3.7): a pasta não se move e a mensagem lista os arquivos, por
    // caminho relativo ao vault, cada um com o kind do `doctor`.
    let home = home_encerrar();
    arquivo(
        home.path(),
        "projects/alfa/tasks/v.sync-conflict-20261101-120000-ABCDEFG.md",
        V_MD,
    );
    arquivo(
        home.path(),
        "projects/alfa/tasks/notas/solta.md",
        "> [!todo] Solta\n",
    );
    envelhece(home.path());
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "alfa"],
        1,
        &recusa_d3(
            "alfa",
            &[
                "projects/alfa/tasks/notas/solta.md (fora-do-padrao)",
                "projects/alfa/tasks/v.sync-conflict-20261101-120000-ABCDEFG.md (sync-conflict)",
            ],
        ),
    );
}

#[test]
fn encerrar_recusa_sync_conflict_fora_de_tasks() {
    // I7: `encerrar` recusa o conflito onde quer que esteja na pasta do
    // projeto, não só em `tasks/`.
    let home = home_encerrar();
    arquivo(
        home.path(),
        "projects/alfa/alfa.sync-conflict-20261101-120000-ABCDEFG.md",
        MOC_MD,
    );
    envelhece(home.path());
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "alfa"],
        1,
        &recusa_d3(
            "alfa",
            &["projects/alfa/alfa.sync-conflict-20261101-120000-ABCDEFG.md (sync-conflict)"],
        ),
    );
}

#[test]
fn encerrar_recusa_sync_conflict_de_task_arquivada() {
    let home = home_encerrar();
    arquivo(
        home.path(),
        "projects/alfa/tasks/archived/done/v.sync-conflict-20261101-120000-ABCDEFG.md",
        V_MD,
    );
    envelhece(home.path());
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "alfa"],
        1,
        &recusa_d3(
            "alfa",
            &[
                "projects/alfa/tasks/archived/done/v.sync-conflict-20261101-120000-ABCDEFG.md (sync-conflict)",
            ],
        ),
    );
}

#[test]
fn encerrar_recusa_tipo_de_arquivamento_que_nao_existe() {
    // `archived/<tipo>/` com tipo fora de done|dropped|failed é fora do padrão.
    let home = home_encerrar();
    arquivo(
        home.path(),
        "projects/alfa/tasks/archived/misc/m.md",
        "> [!done] Eme\n> [[projects/alfa/tasks/archived/misc/m|m]]\n",
    );
    envelhece(home.path());
    recusa_sem_mexer(
        home.path(),
        &["encerrar", "alfa"],
        1,
        &recusa_d3(
            "alfa",
            &["projects/alfa/tasks/archived/misc/m.md (fora-do-padrao)"],
        ),
    );
}

// ===========================================================================
// reabrir
// ===========================================================================

const CONFLITO_FECHADO: &str = "[[projects/_encerrados/alfa/spec|spec]]\n";

/// O vault de partida já com `alfa` encerrado: os arquivos como o `encerrar`
/// os deixa, e uma cópia de conflito que cita a pasta encerrada.
fn home_reabrir() -> tempfile::TempDir {
    home_pequeno(&[
        (MOC_F, MOC_FECHADO),
        (SPEC_F, SPEC_MD),
        (DEC_F, DEC_FECHADA),
        (LOG_F, LOG_TXT),
        (V_F, V_FECHADA),
        (W_F, W_FECHADA),
        (B, B_FECHADA),
        (NOTA, NOTA_FECHADA),
        (NOTA_CONFLITO, CONFLITO_FECHADO),
        (DAILY, DAILY_MD),
    ])
}

#[test]
fn reabrir_traz_a_pasta_de_volta_e_imprime_o_caminho() {
    let home = home_reabrir();
    let saida = roda(home.path(), &["reabrir", "alfa"]);
    sucesso(&saida, &caminho(home.path(), ALFA), &aviso_do_encerrar());
    assert!(!existe(home.path(), ALFA_F));
    for rel in [MOC, SPEC, DEC, LOG, V, W] {
        assert!(existe(home.path(), rel), "{rel} não voltou");
    }
}

#[test]
fn reabrir_reescreve_os_links_de_volta() {
    let home = home_reabrir();
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_do_encerrar(),
    );
    assert_eq!(le(home.path(), MOC), MOC_MD);
    assert_eq!(le(home.path(), DEC), DEC_MD);
    assert_eq!(le(home.path(), V), V_MD);
    assert_eq!(le(home.path(), W), W_MD);
    assert_eq!(le(home.path(), SPEC), SPEC_MD);
    assert_eq!(le(home.path(), LOG), LOG_TXT);
    assert_eq!(le(home.path(), B), B_MD);
    assert_eq!(le(home.path(), NOTA), NOTA_MD);
}

#[test]
fn reabrir_nao_toca_sync_conflict_nem_daily() {
    let home = home_reabrir();
    let antes = fotografa(home.path());
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_do_encerrar(),
    );
    let depois = fotografa(home.path());
    for rel in [NOTA_CONFLITO, DAILY] {
        assert_eq!(antes.get(rel), depois.get(rel), "{rel} não podia mudar");
    }
}

#[test]
fn reabrir_so_grava_a_pasta_movida_as_notas_que_a_citam_e_o_current() {
    let home = home_reabrir();
    let antes = fotografa(home.path());
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_do_encerrar(),
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[
            MOC, SPEC, DEC, LOG, V, W, MOC_F, SPEC_F, DEC_F, LOG_F, V_F, W_F, B, NOTA, CURRENT,
        ])
    );
}

#[test]
fn reabrir_regenera_o_current_com_o_projeto() {
    let home = home_reabrir();
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_do_encerrar(),
    );
    assert_eq!(
        le(home.path(), CURRENT),
        painel(1, 2, &format!("## alfa\n\n_(sem tasks)_\n\n{SECAO_BETA}"))
    );
}

#[test]
fn reabrir_nao_reescreve_link_em_codigo() {
    // D5 na volta.
    let nota = concat!(
        "fora: [[projects/_encerrados/alfa/spec]] e `[[projects/_encerrados/alfa/spec]]`\n",
        "```\n",
        "[[projects/_encerrados/alfa/spec]]\n",
        "```\n",
    );
    let home = home_pequeno(&[(SPEC_F, SPEC_MD), (NOTA, nota)]);
    pasta(home.path(), "projects/_encerrados/alfa/tasks");
    envelhece(home.path());
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_refs(1, 1),
    );
    assert_eq!(
        le(home.path(), NOTA),
        nota.replacen(
            "fora: [[projects/_encerrados/alfa/spec]]",
            "fora: [[projects/alfa/spec]]",
            1
        )
    );
}

#[test]
fn encerrar_e_reabrir_devolvem_os_arquivos_originais_menos_o_current() {
    let home = home_encerrar();
    let antes = bytes_de(&fotografa(home.path()));
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_do_encerrar(),
    );
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_do_encerrar(),
    );
    let mut depois = bytes_de(&fotografa(home.path()));
    assert_eq!(
        String::from_utf8(depois.remove(CURRENT).expect("painel")).expect("utf-8"),
        painel(1, 2, &format!("## alfa\n\n_(sem tasks)_\n\n{SECAO_BETA}"))
    );
    assert_eq!(depois, antes, "fora o painel, tudo volta byte a byte");
}

#[test]
fn reabrir_recusa_projeto_que_nao_esta_encerrado() {
    let home = home_recusas();
    let msg = format!(
        "o projeto \"beta\" não está encerrado: não há {}",
        abs(home.path(), "projects/_encerrados/beta")
    );
    recusa_sem_mexer(home.path(), &["reabrir", "beta"], 1, &msg);
}

#[test]
fn reabrir_recusa_quando_a_pasta_de_destino_ja_existe() {
    let home = home_recusas();
    arquivo(home.path(), "projects/omega/rascunho.md", "# outro omega\n");
    envelhece(home.path());
    let msg = format!(
        "já existe {}: reabrir juntaria duas pastas num projeto só; decida antes o que fica",
        abs(home.path(), "projects/omega")
    );
    recusa_sem_mexer(home.path(), &["reabrir", "omega"], 1, &msg);
}

#[test]
fn reabrir_recusa_nome_reservado() {
    let home = home_recusas();
    recusa_sem_mexer(
        home.path(),
        &["reabrir", "daily"],
        1,
        "\"daily\" é um diretório reservado, não um projeto",
    );
}

#[test]
fn reabrir_sem_projeto_sai_2() {
    // Já passa contra o esqueleto só no código; a mensagem é a de hoje.
    let home = home_recusas();
    recusa_sem_mexer(home.path(), &["reabrir"], 2, "falta o projeto");
}

// ===========================================================================
// D4 em encerrar e reabrir
// ===========================================================================

const D4_V: &str = "> [!done] Vê\n> [[projects/alfa/tasks/archived/done/v|v]]\n\ncorpo\n";
const D4_NOTA: &str = "# Nota\n\n[[projects/alfa/spec|spec]] e [[spec]]\n";

fn d4_encerrar(v: Variante) {
    let home = home_pequeno(&[(SPEC, SPEC_MD)]);
    arquivo_bytes(home.path(), V, &na_variante(v, D4_V));
    arquivo_bytes(home.path(), NOTA, &na_variante(v, D4_NOTA));
    envelhece(home.path());
    sucesso(
        &roda(home.path(), &["encerrar", "alfa"]),
        &caminho(home.path(), ALFA_F),
        &aviso_refs(2, 2),
    );
    assert_eq!(
        le_bytes(home.path(), V_F),
        na_variante(
            v,
            "> [!done] Vê\n> [[projects/_encerrados/alfa/tasks/archived/done/v|v]]\n\ncorpo\n"
        )
    );
    assert_eq!(
        le_bytes(home.path(), NOTA),
        na_variante(
            v,
            "# Nota\n\n[[projects/_encerrados/alfa/spec|spec]] e [[spec]]\n"
        )
    );
}

fn d4_reabrir(v: Variante) {
    let home = home_pequeno(&[(SPEC_F, SPEC_MD)]);
    arquivo_bytes(
        home.path(),
        V_F,
        &na_variante(
            v,
            "> [!done] Vê\n> [[projects/_encerrados/alfa/tasks/archived/done/v|v]]\n\ncorpo\n",
        ),
    );
    arquivo_bytes(
        home.path(),
        NOTA,
        &na_variante(
            v,
            "# Nota\n\n[[projects/_encerrados/alfa/spec|spec]] e [[spec]]\n",
        ),
    );
    envelhece(home.path());
    sucesso(
        &roda(home.path(), &["reabrir", "alfa"]),
        &caminho(home.path(), ALFA),
        &aviso_refs(2, 2),
    );
    assert_eq!(le_bytes(home.path(), V), na_variante(v, D4_V));
    assert_eq!(le_bytes(home.path(), NOTA), na_variante(v, D4_NOTA));
}

#[test]
fn crlf_sobrevive_ao_encerrar() {
    d4_encerrar(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_encerrar() {
    d4_encerrar(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_encerrar() {
    d4_encerrar(Variante::SemQuebraFinal);
}

#[test]
fn crlf_sobrevive_ao_reabrir() {
    d4_reabrir(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_reabrir() {
    d4_reabrir(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_reabrir() {
    d4_reabrir(Variante::SemQuebraFinal);
}

// ===========================================================================
// Lock: encerrar, reabrir e doctor --fix esperam até 5 s e saem 3
// ===========================================================================

#[test]
fn encerrar_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_encerrar();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["encerrar", "alfa"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn reabrir_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_reabrir();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["reabrir", "alfa"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn encerrar_espera_o_lock_em_local_state_sem_xdg_e_sai_3_sem_gravar() {
    let home = home_encerrar();
    let antes = fotografa(home.path());
    let _lock = Lock::segura(&lock_sem_xdg(home.path()), 60);
    let t0 = Instant::now();
    let saida = roda(home.path(), &["encerrar", "alfa"]);
    saiu_3_depois_de_esperar(&saida, t0.elapsed());
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn doctor_fix_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_doctor();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["doctor", "--fix"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn doctor_sem_fix_nao_espera_o_lock() {
    // Leitura não toma o lock (spec §1.5): roda na hora e devolve o de sempre.
    let home = home_pequeno(&[(OK, OK_MD)]);
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["doctor"]);
    sucesso(&saida, CONSISTENTE, "");
    assert!(
        decorrido < Duration::from_secs(3),
        "o doctor sem --fix esperou o lock: {decorrido:?}"
    );
}

// ===========================================================================
// doctor: o vault de cada kind
// ===========================================================================

const OK: &str = "projects/alfa/tasks/ok.md";
const OK_MD: &str = "> [!todo] Ok\n> [[projects/alfa/tasks/ok|ok]]\n";

const MSG_CONFLITO: &str = "conflito do Syncthing — compare com o original e resolva à mão";
const MSG_FORA: &str =
    "não casa projects/<projeto>/tasks/[archived/<tipo>/]<id>.md — invisível ao freitask";
const MSG_SEM_CALLOUT: &str = "arquivo de task sem bloco de callout";
const MSG_BLOCO: &str = "bloco não corresponde ao caminho (link/id)";
const MSG_FM_ID: &str = "`id:` do frontmatter ≠ nome do arquivo";
const MSG_HISTORICO: &str = "está em archived/ sem entrada de histórico";
const MSG_TEMP: &str =
    "temporário de uma gravação interrompida — o arquivo de destino ficou como estava";

fn msg_link(linha: usize, alvo: &str) -> String {
    format!("linha {linha}: `[[{alvo}]]` não resolve")
}

/// Afirma o texto de um `doctor` sem `--fix` com um achado só, e o JSON do
/// mesmo vault.
fn um_achado(home: &Path, level: &str, kind: &str, path: &str, msg: &str) {
    let saida = roda(home, &["doctor"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}",
            achado(level, kind, path, msg, false),
            rodape(1, 1, true)
        )
    );
    let saida = roda(home, &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    assert_eq!(
        json(&saida),
        json!({"ok": false, "findings": [achado_json(level, kind, path, msg, false)]})
    );
}

/// Afirma o texto de um `doctor --fix` que reparou um achado só.
fn um_reparo(home: &Path, level: &str, kind: &str, path: &str, msg: &str) {
    let saida = roda(home, &["doctor", "--fix"]);
    sucesso(
        &saida,
        &format!(
            "{}{}",
            achado(level, kind, path, msg, true),
            rodape(1, 0, false)
        ),
        "",
    );
}

// ---------------------------------------------------------------------------
// sem achado
// ---------------------------------------------------------------------------

#[test]
fn doctor_num_vault_consistente_diz_e_sai_0() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        (
            "projects/alfa/tasks/archived/done/f.md",
            "> [!done] Feita\n> [[projects/alfa/tasks/archived/done/f|f]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
        ),
        (
            "inbox/n.md",
            "[[projects/alfa/tasks/ok|ok]] e [[projects/alfa/tasks/archived/done/f]]\n",
        ),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_json_num_vault_consistente_e_ok_com_findings_vazio() {
    let home = home_pequeno(&[(OK, OK_MD)]);
    let saida = roda(home.path(), &["doctor", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!({"ok": true, "findings": []}));
}

#[test]
fn doctor_quiet_num_vault_consistente_nao_imprime_nada_e_sai_0() {
    let home = home_pequeno(&[(OK, OK_MD)]);
    sucesso(&roda(home.path(), &["doctor", "--quiet"]), "", "");
}

// ---------------------------------------------------------------------------
// os 12 kinds de hoje
// ---------------------------------------------------------------------------

const CONFLITO_EM_TASKS: &str = "projects/alfa/tasks/ok.sync-conflict-20261101-120000-ABCDEFG.md";

#[test]
fn doctor_acusa_sync_conflict_em_tasks() {
    // A cópia de conflito não é task: não vira `id-duplicado` nem
    // `fora-do-padrao`.
    let home = home_pequeno(&[(OK, OK_MD), (CONFLITO_EM_TASKS, OK_MD)]);
    um_achado(
        home.path(),
        "error",
        "sync-conflict",
        CONFLITO_EM_TASKS,
        MSG_CONFLITO,
    );
}

#[test]
fn doctor_acusa_sync_conflict_de_task_arquivada() {
    let rel = "projects/alfa/tasks/archived/done/f.sync-conflict-20261101-120000-ABCDEFG.md";
    let home = home_pequeno(&[(OK, OK_MD), (rel, "conflito\n")]);
    um_achado(home.path(), "error", "sync-conflict", rel, MSG_CONFLITO);
}

#[test]
fn doctor_acusa_sync_conflict_fora_de_tasks() {
    // I7: onde quer que esteja sob `projects/`.
    let rel = "projects/alfa/alfa.sync-conflict-20261101-120000-ABCDEFG.md";
    let home = home_pequeno(&[(OK, OK_MD), (rel, "conflito\n")]);
    um_achado(home.path(), "error", "sync-conflict", rel, MSG_CONFLITO);
}

#[test]
fn doctor_acusa_sync_conflict_em_projeto_encerrado() {
    // I7: `projects/_encerrados/` também está sob `projects/`.
    let rel = "projects/_encerrados/omega/tasks/q.sync-conflict-20261101-120000-ABCDEFG.md";
    let home = home_pequeno(&[(OK, OK_MD), (rel, "conflito\n")]);
    um_achado(home.path(), "error", "sync-conflict", rel, MSG_CONFLITO);
}

#[test]
fn doctor_fix_nunca_toca_sync_conflict() {
    let home = home_pequeno(&[(OK, OK_MD), (CONFLITO_EM_TASKS, OK_MD)]);
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}",
            achado(
                "error",
                "sync-conflict",
                CONFLITO_EM_TASKS,
                MSG_CONFLITO,
                false
            ),
            rodape(1, 1, false)
        )
    );
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[CURRENT])
    );
}

#[test]
fn doctor_acusa_fora_do_padrao_em_tasks() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        ("projects/alfa/tasks/sub/solta.md", "> [!todo] Solta\n"),
        ("projects/alfa/tasks/archived/misc/m.md", "> [!done] Eme\n"),
        ("projects/alfa/tasks/archived/a.md", "> [!done] A\n"),
        ("projects/alfa/notas/fora-de-tasks.md", "nota comum\n"),
    ]);
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}{}{}",
            achado(
                "warn",
                "fora-do-padrao",
                "projects/alfa/tasks/archived/a.md",
                MSG_FORA,
                false
            ),
            achado(
                "warn",
                "fora-do-padrao",
                "projects/alfa/tasks/archived/misc/m.md",
                MSG_FORA,
                false
            ),
            achado(
                "warn",
                "fora-do-padrao",
                "projects/alfa/tasks/sub/solta.md",
                MSG_FORA,
                false
            ),
            rodape(3, 3, true)
        )
    );
}

#[test]
fn doctor_acusa_id_duplicado_no_primeiro_caminho() {
    // O caminho do achado é o da primeira cópia: ativas antes de arquivadas,
    // cada grupo por caminho.
    let home = home_pequeno(&[
        (OK, OK_MD),
        (
            "projects/beta/tasks/ok.md",
            "> [!todo] Ok beta\n> [[projects/beta/tasks/ok|ok]]\n",
        ),
        (
            "projects/alfa/tasks/archived/done/ok.md",
            "> [!done] Ok velho\n> [[projects/alfa/tasks/archived/done/ok|ok]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
        ),
    ]);
    um_achado(
        home.path(),
        "warn",
        "id-duplicado",
        OK,
        "id \"ok\" existe em 3 arquivos — `[[ok]]` fica ambíguo",
    );
}

#[test]
fn id_duplicado_sai_em_ordem_de_id() {
    // O oráculo itera um `pairs()` aqui; a ordem fica fixada pelo id.
    let home = home_pequeno(&[
        (
            "projects/alfa/tasks/bb.md",
            "> [!todo] Bê\n> [[projects/alfa/tasks/bb|bb]]\n",
        ),
        (
            "projects/beta/tasks/bb.md",
            "> [!todo] Bê\n> [[projects/beta/tasks/bb|bb]]\n",
        ),
        (
            "projects/alfa/tasks/aa.md",
            "> [!todo] Á\n> [[projects/alfa/tasks/aa|aa]]\n",
        ),
        (
            "projects/beta/tasks/aa.md",
            "> [!todo] Á\n> [[projects/beta/tasks/aa|aa]]\n",
        ),
    ]);
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}{}",
            achado(
                "warn",
                "id-duplicado",
                "projects/alfa/tasks/aa.md",
                "id \"aa\" existe em 2 arquivos — `[[aa]]` fica ambíguo",
                false
            ),
            achado(
                "warn",
                "id-duplicado",
                "projects/alfa/tasks/bb.md",
                "id \"bb\" existe em 2 arquivos — `[[bb]]` fica ambíguo",
                false
            ),
            rodape(2, 2, true)
        )
    );
}

#[test]
fn doctor_acusa_sem_callout() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        ("projects/alfa/tasks/sc.md", "# sem callout\n\ntexto\n"),
    ]);
    um_achado(
        home.path(),
        "error",
        "sem-callout",
        "projects/alfa/tasks/sc.md",
        MSG_SEM_CALLOUT,
    );
}

const DES: &str = "projects/alfa/tasks/des.md";
const DES_MD: &str = "> [!todo] Des\n> [[projects/alfa/tasks/des|outro]]\n> _em curso_\n> nota\n";
const DES_REPARADA: &str =
    "> [!todo] Des\n> [[projects/alfa/tasks/des|des]]\n> _em curso_\n> nota\n";

#[test]
fn doctor_acusa_bloco_dessincronizado() {
    let home = home_pequeno(&[(DES, DES_MD)]);
    um_achado(home.path(), "warn", "bloco-dessincronizado", DES, MSG_BLOCO);
}

#[test]
fn doctor_fix_reserializa_o_bloco_a_partir_do_caminho() {
    let home = home_pequeno(&[(DES, DES_MD)]);
    um_reparo(home.path(), "warn", "bloco-dessincronizado", DES, MSG_BLOCO);
    assert_eq!(le(home.path(), DES), DES_REPARADA);
}

#[test]
fn doctor_fix_leva_o_bloco_legado_ao_formato_atual() {
    // Decisão 11: o `--fix` normaliza o bloco legado, como hoje.
    let rel = "projects/alfa/tasks/leg.md";
    let home = home_pequeno(&[(
        rel,
        "> [!example] **Título legado**\n> [[leg]]\n> Branch: leg\n> nota livre\n\ncorpo\n",
    )]);
    um_reparo(home.path(), "warn", "bloco-dessincronizado", rel, MSG_BLOCO);
    assert_eq!(
        le(home.path(), rel),
        "> [!example] Título legado\n> [[projects/alfa/tasks/leg|leg]]\n> nota livre\n\ncorpo\n"
    );
}

const FM: &str = "projects/alfa/tasks/fm.md";
const FM_MD: &str = "---\naliases: []\nid: errado\ntags: [a]\n---\n\n> [!todo] Fm\n> [[projects/alfa/tasks/fm|fm]]\n";

#[test]
fn doctor_acusa_frontmatter_id() {
    let home = home_pequeno(&[(FM, FM_MD)]);
    um_achado(home.path(), "warn", "frontmatter-id", FM, MSG_FM_ID);
}

#[test]
fn doctor_fix_alinha_o_id_do_frontmatter_e_preserva_as_outras_chaves() {
    let home = home_pequeno(&[(FM, FM_MD)]);
    um_reparo(home.path(), "warn", "frontmatter-id", FM, MSG_FM_ID);
    assert_eq!(
        le(home.path(), FM),
        "---\naliases: []\nid: fm\ntags: [a]\n---\n\n> [!todo] Fm\n> [[projects/alfa/tasks/fm|fm]]\n"
    );
}

#[test]
fn doctor_nao_acusa_frontmatter_sem_id() {
    let home = home_pequeno(&[(
        FM,
        "---\ntags: [a]\n---\n\n> [!todo] Fm\n> [[projects/alfa/tasks/fm|fm]]\n",
    )]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

const H: &str = "projects/alfa/tasks/archived/done/h.md";
const H_MD: &str = "> [!done] Agá\n> [[projects/alfa/tasks/archived/done/h|h]]\n\ncorpo\n";

#[test]
fn doctor_acusa_historico_ausente() {
    let home = home_pequeno(&[(H, H_MD)]);
    um_achado(home.path(), "warn", "historico-ausente", H, MSG_HISTORICO);
}

#[test]
fn doctor_fix_escreve_o_historico_marcado_como_reconstruido_no_dia_de_freitask_today() {
    // §1.6: a linha usa o dia de `FREITASK_TODAY`, não o relógio.
    let home = home_pequeno(&[(H, H_MD)]);
    let saida = roda_no_dia(home.path(), "2026-01-02", &["doctor", "--fix"]);
    sucesso(
        &saida,
        &format!(
            "{}{}",
            achado("warn", "historico-ausente", H, MSG_HISTORICO, true),
            rodape(1, 0, false)
        ),
        "",
    );
    assert_eq!(
        le(home.path(), H),
        concat!(
            "> [!done] Agá\n",
            "> [[projects/alfa/tasks/archived/done/h|h]]\n",
            "\n",
            "corpo\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-01-02 — arquivada em `archived/done` (feito) [reconstruído pelo doctor; data real desconhecida]\n",
        )
    );
}

#[test]
fn doctor_fix_reconstroi_o_historico_de_dropped_e_failed_com_a_glosa() {
    let d = "projects/alfa/tasks/archived/dropped/d.md";
    let f = "projects/alfa/tasks/archived/failed/f.md";
    let home = home_pequeno(&[
        (
            d,
            "> [!warning] Dê\n> [[projects/alfa/tasks/archived/dropped/d|d]]\n",
        ),
        (
            f,
            "> [!example] Efe\n> [[projects/alfa/tasks/archived/failed/f|f]]\n",
        ),
    ]);
    let saida = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(saida.codigo, Some(0), "stderr: {:?}", saida.stderr);
    assert_eq!(
        le(home.path(), d),
        "> [!warning] Dê\n> [[projects/alfa/tasks/archived/dropped/d|d]]\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/dropped` (abandonado) [reconstruído pelo doctor; data real desconhecida]\n"
    );
    assert_eq!(
        le(home.path(), f),
        "> [!example] Efe\n> [[projects/alfa/tasks/archived/failed/f|f]]\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/failed` (falhou) [reconstruído pelo doctor; data real desconhecida]\n"
    );
}

#[test]
fn doctor_fix_acrescenta_no_fim_da_secao_historico_que_ja_existe() {
    // A seção existe (com outra entrada) e há uma seção depois dela.
    let home = home_pequeno(&[(
        H,
        concat!(
            "> [!done] Agá\n",
            "> [[projects/alfa/tasks/archived/done/h|h]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-09-01 — movida de `beta` para `alfa`\n",
            "\n",
            "## Depois\n",
            "\n",
            "texto\n",
        ),
    )]);
    um_reparo(home.path(), "warn", "historico-ausente", H, MSG_HISTORICO);
    assert_eq!(
        le(home.path(), H),
        concat!(
            "> [!done] Agá\n",
            "> [[projects/alfa/tasks/archived/done/h|h]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-09-01 — movida de `beta` para `alfa`\n",
            "- 2026-11-08 — arquivada em `archived/done` (feito) [reconstruído pelo doctor; data real desconhecida]\n",
            "\n",
            "## Depois\n",
            "\n",
            "texto\n",
        )
    );
}

#[test]
fn doctor_acusa_status_0_e_o_fix_nao_toca() {
    let rel = "projects/alfa/tasks/s0.md";
    let home = home_pequeno(&[(
        rel,
        "> [!bogus] Esse zero\n> [[projects/alfa/tasks/s0|s0]]\n",
    )]);
    um_achado(
        home.path(),
        "warn",
        "status-0",
        rel,
        "callout \"bogus\" não existe em status.json",
    );
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[CURRENT])
    );
}

const CV: &str = "projects/alfa/tasks/archived/done/cv.md";
const CV_MD: &str = concat!(
    "> [!todo] Cê vê\n",
    "> [[projects/alfa/tasks/archived/done/cv|cv]]\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
);

#[test]
fn doctor_acusa_callout_vs_pasta() {
    let home = home_pequeno(&[(CV, CV_MD)]);
    um_achado(
        home.path(),
        "warn",
        "callout-vs-pasta",
        CV,
        "está em archived/done/ com callout \"todo\"",
    );
}

#[test]
fn doctor_fix_grava_done_na_task_de_archived_done() {
    let home = home_pequeno(&[(CV, CV_MD)]);
    um_reparo(
        home.path(),
        "warn",
        "callout-vs-pasta",
        CV,
        "está em archived/done/ com callout \"todo\"",
    );
    assert_eq!(le(home.path(), CV), CV_MD.replacen("[!todo]", "[!done]", 1));
}

#[test]
fn callout_vs_pasta_so_vale_para_done() {
    // Em `dropped` e `failed` a fase de quando foi largada é informação.
    let home = home_pequeno(&[
        (
            "projects/alfa/tasks/archived/dropped/d.md",
            "> [!warning] Dê\n> [[projects/alfa/tasks/archived/dropped/d|d]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/dropped` (abandonado)\n",
        ),
        (
            "projects/alfa/tasks/archived/failed/f.md",
            "> [!example] Efe\n> [[projects/alfa/tasks/archived/failed/f|f]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/failed` (falhou)\n",
        ),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

const PR: &str = "projects/alfa/tasks/pr.md";

#[test]
fn doctor_acusa_prioridade_desconhecida() {
    // O kind tem 23 caracteres: o `%-22s` não corta, só não preenche.
    let home = home_pequeno(&[(
        PR,
        "---\nprioridade: Alta\n---\n\n> [!todo] Pê erre\n> [[projects/alfa/tasks/pr|pr]]\n",
    )]);
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        saida.stdout,
        concat!(
            "warn   prioridade-desconhecida projects/alfa/tasks/pr.md\n",
            "         prioridade \"Alta\" não é `alta` — a task conta como normal\n",
            "\n",
            "1 achado(s), 1 pendente(s).\n",
            "Repare o que for automático com: freitask doctor --fix\n",
        )
    );
    um_achado(
        home.path(),
        "warn",
        "prioridade-desconhecida",
        PR,
        "prioridade \"Alta\" não é `alta` — a task conta como normal",
    );
}

#[test]
fn doctor_acusa_volta_em_invalido() {
    // 30 de fevereiro não é um dia que existe.
    let home = home_pequeno(&[(
        PR,
        "---\nvolta-em: 2026-02-30\n---\n\n> [!todo] Pê erre\n> [[projects/alfa/tasks/pr|pr]]\n",
    )]);
    um_achado(
        home.path(),
        "warn",
        "volta-em-invalido",
        PR,
        "volta-em \"2026-02-30\" não é uma data AAAA-MM-DD — a task aparece, sem adiar",
    );
}

#[test]
fn doctor_nao_acusa_volta_em_valida_passada_nem_prioridade_alta() {
    let home = home_pequeno(&[(
        PR,
        "---\nprioridade: alta\nvolta-em: 2026-01-01\n---\n\n> [!todo] Pê erre\n> [[projects/alfa/tasks/pr|pr]]\n",
    )]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_nao_olha_prioridade_nem_volta_em_de_task_arquivada() {
    let home = home_pequeno(&[(
        "projects/alfa/tasks/archived/done/pr.md",
        "---\nprioridade: urgente\nvolta-em: amanhã\n---\n\n> [!done] Pê erre\n> [[projects/alfa/tasks/archived/done/pr|pr]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
    )]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_acusa_link_pendurado() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        (
            "inbox/n.md",
            "# N\n\nver [[projects/alfa/tasks/sumiu|sumiu]]\n",
        ),
    ]);
    um_achado(
        home.path(),
        "error",
        "link-pendurado",
        "inbox/n.md",
        &msg_link(3, "projects/alfa/tasks/sumiu"),
    );
}

#[test]
fn doctor_fix_nao_toca_link_pendurado() {
    let n = "inbox/n.md";
    let home = home_pequeno(&[
        (OK, OK_MD),
        (n, "ver [[projects/alfa/tasks/sumiu|sumiu]]\n"),
    ]);
    let saida = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}",
            achado(
                "error",
                "link-pendurado",
                n,
                &msg_link(1, "projects/alfa/tasks/sumiu"),
                false
            ),
            rodape(1, 1, false)
        )
    );
    assert_eq!(
        le(home.path(), n),
        "ver [[projects/alfa/tasks/sumiu|sumiu]]\n"
    );
}

// ---------------------------------------------------------------------------
// link-pendurado: o que conta como link (a regra do scanner)
// ---------------------------------------------------------------------------

const LINKS_MD: &str = r"# Links

pendurado: [[projects/alfa/tasks/fantasma|f]] e resolve: [[projects/alfa/tasks/ok|ok]]
curtos que não resolvem: [[nota-futura]] e [[fantasma]]
fora de tasks: [[projects/alfa/nota-que-nao-existe]]
no span: `[[projects/alfa/tasks/no-span]]` e depois do span: [[projects/alfa/tasks/depois-do-span]]
crase sem par ` e o link depois dela conta: [[projects/alfa/tasks/apos-crase]]
```
[[projects/alfa/tasks/na-cerca]]
```
  ```lua
[[projects/alfa/tasks/na-cerca-indentada]]
  ```
| tabela | [[projects/alfa/tasks/ok\|ok]] | [[projects/alfa/tasks/sumiu\|s]] |
sufixos: [[projects/alfa/tasks/ok#Seção]] [[projects/alfa/tasks/ok^abc]] [[projects/alfa/tasks/ok.md|ok]] ![[projects/alfa/tasks/ok]]
dois na linha: [[projects/alfa/tasks/um]] e [[projects/alfa/tasks/dois|d]]
encerrado: [[projects/_encerrados/omega/tasks/qualquer]]
arquivada: [[projects/alfa/tasks/archived/done/velha|velha]] e [[projects/alfa/tasks/archived/done/nao|nao]]
";

const VELHA: &str = "projects/alfa/tasks/archived/done/velha.md";
const VELHA_MD: &str = "> [!done] Velha\n> [[projects/alfa/tasks/archived/done/velha|velha]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n";

#[test]
fn link_pendurado_segue_a_regra_do_scanner() {
    // Bloco cercado (``` depois de espaços) e code span em par ficam de fora;
    // crase sem par é texto; `\|` em tabela separa o alias (D1); só
    // `projects/<p>/tasks/…` é cobrado; sufixos `#`/`^`, `.md` e embed
    // resolvem pelo alvo.
    let l = "inbox/links.md";
    let home = home_pequeno(&[(OK, OK_MD), (VELHA, VELHA_MD), (l, LINKS_MD)]);
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    let esperado = [
        (3, "projects/alfa/tasks/fantasma"),
        (6, "projects/alfa/tasks/depois-do-span"),
        (7, "projects/alfa/tasks/apos-crase"),
        (14, "projects/alfa/tasks/sumiu"),
        (16, "projects/alfa/tasks/um"),
        (16, "projects/alfa/tasks/dois"),
        (18, "projects/alfa/tasks/archived/done/nao"),
    ];
    let mut texto = String::new();
    for (linha, alvo) in esperado {
        texto.push_str(&achado(
            "error",
            "link-pendurado",
            l,
            &msg_link(linha, alvo),
            false,
        ));
    }
    texto.push_str(&rodape(7, 7, true));
    assert_eq!(saida.stdout, texto);
}

#[test]
fn doctor_nao_acusa_link_de_tabela_que_resolve() {
    // D1: o alvo de `[[projects/alfa/tasks/ok\|ok]]` é `projects/alfa/tasks/ok`.
    let home = home_pequeno(&[
        (OK, OK_MD),
        (
            "inbox/t.md",
            "| t |\n| --- |\n| [[projects/alfa/tasks/ok\\|ok]] |\n",
        ),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_nao_procura_link_pendurado_em_daily() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        ("daily/2026-11-01.md", "[[projects/alfa/tasks/sumiu]]\n"),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn link_pendurado_em_arquivo_crlf_conta_a_linha_certa() {
    let n = "inbox/crlf.md";
    let home = home_pequeno(&[(OK, OK_MD)]);
    arquivo_bytes(
        home.path(),
        n,
        b"# N\r\n\r\nver [[projects/alfa/tasks/sumiu]]\r\n",
    );
    envelhece(home.path());
    um_achado(
        home.path(),
        "error",
        "link-pendurado",
        n,
        &msg_link(3, "projects/alfa/tasks/sumiu"),
    );
}

// ---------------------------------------------------------------------------
// projeto encerrado fica fora dos checks de task
// ---------------------------------------------------------------------------

#[test]
fn doctor_nao_ve_task_de_projeto_encerrado() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        // bloco dessincronizado (link curto, para não pendurar), `id:` errado,
        // status 0
        (
            "projects/_encerrados/omega/tasks/q.md",
            "---\nid: errado\n---\n\n> [!bogus] Q\n> [[q|outro]]\n",
        ),
        // sem histórico e com callout que contradiz a pasta
        (
            "projects/_encerrados/omega/tasks/archived/done/r.md",
            "> [!todo] R\n> [[projects/_encerrados/omega/tasks/archived/done/r|r]]\n",
        ),
        // id repetido com a ativa de `alfa`
        (
            "projects/_encerrados/omega/tasks/archived/done/ok.md",
            "> [!done] Ok\n> [[projects/_encerrados/omega/tasks/archived/done/ok|ok]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n",
        ),
        // fora do padrão e sem callout
        ("projects/_encerrados/omega/tasks/sub/s.md", "sem callout\n"),
    ]);
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

// ===========================================================================
// doctor: o vault com tudo, texto, JSON, códigos
// ===========================================================================

const CONFLITO_X: &str = "projects/alfa/tasks/x.sync-conflict-20261101-120000-ABCDEFG.md";
const SOLTA: &str = "projects/alfa/tasks/sub/solta.md";
const S0: &str = "projects/alfa/tasks/s0.md";
const SC: &str = "projects/alfa/tasks/sc.md";
const LINKS: &str = "inbox/links.md";

/// Um achado de cada kind de hoje, em arquivos separados (dois em `pr`).
fn home_doctor() -> tempfile::TempDir {
    home_pequeno(&[
        (OK, OK_MD),
        (
            "projects/beta/tasks/ok.md",
            "> [!example] Ok beta\n> [[projects/beta/tasks/ok|ok]]\n",
        ),
        (CONFLITO_X, OK_MD),
        (SOLTA, "> [!todo] Solta\n"),
        (DES, DES_MD),
        (FM, FM_MD),
        (
            PR,
            "---\nprioridade: Alta\nvolta-em: 2026-13-01\n---\n\n> [!todo] Pê erre\n> [[projects/alfa/tasks/pr|pr]]\n",
        ),
        (
            S0,
            "> [!bogus] Esse zero\n> [[projects/alfa/tasks/s0|s0]]\n",
        ),
        (SC, "sem callout\n"),
        (CV, CV_MD),
        (H, H_MD),
        (LINKS, "[[projects/alfa/tasks/fantasma|f]]\n"),
    ])
}

/// Os achados de `home_doctor`, na ordem do texto: `sync-conflict`,
/// `fora-do-padrao`, `id-duplicado`, depois task a task (ativas por caminho,
/// arquivadas por caminho; em cada uma, na ordem dos checks de hoje), e
/// `link-pendurado`. `reparado` marca os que o `--fix` repara.
fn achados_home_doctor(
    com_fix: bool,
) -> Vec<(&'static str, &'static str, &'static str, String, bool)> {
    vec![
        (
            "error",
            "sync-conflict",
            CONFLITO_X,
            MSG_CONFLITO.into(),
            false,
        ),
        ("warn", "fora-do-padrao", SOLTA, MSG_FORA.into(), false),
        (
            "warn",
            "id-duplicado",
            OK,
            "id \"ok\" existe em 2 arquivos — `[[ok]]` fica ambíguo".into(),
            false,
        ),
        (
            "warn",
            "bloco-dessincronizado",
            DES,
            MSG_BLOCO.into(),
            com_fix,
        ),
        ("warn", "frontmatter-id", FM, MSG_FM_ID.into(), com_fix),
        (
            "warn",
            "prioridade-desconhecida",
            PR,
            "prioridade \"Alta\" não é `alta` — a task conta como normal".into(),
            false,
        ),
        (
            "warn",
            "volta-em-invalido",
            PR,
            "volta-em \"2026-13-01\" não é uma data AAAA-MM-DD — a task aparece, sem adiar".into(),
            false,
        ),
        (
            "warn",
            "status-0",
            S0,
            "callout \"bogus\" não existe em status.json".into(),
            false,
        ),
        ("error", "sem-callout", SC, MSG_SEM_CALLOUT.into(), false),
        (
            "warn",
            "callout-vs-pasta",
            CV,
            "está em archived/done/ com callout \"todo\"".into(),
            com_fix,
        ),
        (
            "warn",
            "historico-ausente",
            H,
            MSG_HISTORICO.into(),
            com_fix,
        ),
        (
            "error",
            "link-pendurado",
            LINKS,
            msg_link(1, "projects/alfa/tasks/fantasma"),
            false,
        ),
    ]
}

fn texto_de(achados: &[(&str, &str, &str, String, bool)], com_dica: bool) -> String {
    let mut s = String::new();
    for (level, kind, path, msg, rep) in achados {
        s.push_str(&achado(level, kind, path, msg, *rep));
    }
    let pend = achados.iter().filter(|a| !a.4).count();
    s.push_str(&rodape(achados.len(), pend, com_dica));
    s
}

fn json_de(achados: &[(&str, &str, &str, String, bool)]) -> Vec<Value> {
    let mut v: Vec<Value> = achados
        .iter()
        .map(|(level, kind, path, msg, rep)| achado_json(level, kind, path, msg, *rep))
        .collect();
    v.sort_by_key(|f| {
        (
            f["kind"].as_str().unwrap_or("").to_owned(),
            f["path"].as_str().unwrap_or("").to_owned(),
            f["msg"].as_str().unwrap_or("").to_owned(),
        )
    });
    v
}

#[test]
fn doctor_texto_lista_cada_achado_na_ordem_com_contagem_e_dica() {
    let home = home_doctor();
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    assert_eq!(saida.stdout, texto_de(&achados_home_doctor(false), true));
}

#[test]
fn doctor_json_traz_level_kind_path_msg_e_fixed_de_cada_achado() {
    let home = home_doctor();
    let saida = roda(home.path(), &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    let v = json(&saida);
    assert_eq!(v["ok"], json!(false));
    assert_eq!(achados_ordenados(&v), json_de(&achados_home_doctor(false)));
}

#[test]
fn doctor_quiet_com_achado_nao_imprime_nada_e_sai_1() {
    let home = home_doctor();
    let saida = roda(home.path(), &["doctor", "--quiet"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(saida.stdout, "");
    assert_eq!(saida.stderr, "");
}

#[test]
fn doctor_fix_marca_o_que_reparou_e_sai_1_com_o_que_sobra() {
    // Sem a dica do `--fix`: quem pediu `--fix` já o usou.
    let home = home_doctor();
    let saida = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stderr, "");
    assert_eq!(saida.stdout, texto_de(&achados_home_doctor(true), false));
}

#[test]
fn doctor_fix_json_marca_fixed_nos_reparados() {
    let home = home_doctor();
    let saida = roda(home.path(), &["doctor", "--fix", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let v = json(&saida);
    assert_eq!(v["ok"], json!(false));
    assert_eq!(achados_ordenados(&v), json_de(&achados_home_doctor(true)));
}

#[test]
fn doctor_fix_so_grava_o_que_reparou_e_o_current() {
    // D6: o que não se repara não é regravado (nem o mtime muda).
    let home = home_doctor();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--fix", "--quiet"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[DES, FM, CV, H, CURRENT])
    );
    assert_eq!(le(home.path(), DES), DES_REPARADA);
}

#[test]
fn doctor_fix_que_resolve_tudo_sai_0() {
    let home = home_pequeno(&[(DES, DES_MD), (FM, FM_MD), (CV, CV_MD), (H, H_MD)]);
    let saida = roda(home.path(), &["doctor", "--fix"]);
    sucesso(
        &saida,
        &format!(
            "{}{}{}{}{}",
            achado("warn", "bloco-dessincronizado", DES, MSG_BLOCO, true),
            achado("warn", "frontmatter-id", FM, MSG_FM_ID, true),
            achado(
                "warn",
                "callout-vs-pasta",
                CV,
                "está em archived/done/ com callout \"todo\"",
                true
            ),
            achado("warn", "historico-ausente", H, MSG_HISTORICO, true),
            rodape(4, 0, false)
        ),
        "",
    );
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn doctor_fix_quiet_que_resolve_tudo_sai_0_sem_imprimir() {
    let home = home_pequeno(&[(DES, DES_MD)]);
    sucesso(&roda(home.path(), &["doctor", "--fix", "--quiet"]), "", "");
    assert_eq!(le(home.path(), DES), DES_REPARADA);
}

#[test]
fn doctor_fix_json_que_resolve_tudo_sai_0_com_ok_false() {
    // `ok` é "não houve achado", como hoje: um achado reparado conta.
    let home = home_pequeno(&[(DES, DES_MD)]);
    let saida = roda(home.path(), &["doctor", "--fix", "--json"]);
    ok(&saida);
    assert_eq!(
        json(&saida),
        json!({"ok": false, "findings": [achado_json("warn", "bloco-dessincronizado", DES, MSG_BLOCO, true)]})
    );
}

#[test]
fn doctor_fix_regenera_o_current() {
    let home = home_pequeno(&[(DES, DES_MD)]);
    um_reparo(home.path(), "warn", "bloco-dessincronizado", DES, MSG_BLOCO);
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            1,
            1,
            "## alfa\n\n> [!todo] Des\n> [[projects/alfa/tasks/des|des]]\n> _em curso_\n> nota\n\n"
        )
    );
}

#[test]
fn doctor_fix_repara_a_task_arquivada_com_mv_e_o_link_da_linha_2_some_do_relatorio() {
    // Arquivar com `mv` é dano reparável: o bloco aponta para o caminho
    // antigo (o que também pendura o link da linha 2), o callout não é `done`
    // e falta o histórico. O `--fix` repara os três antes de procurar link
    // pendurado, e o link da linha 2 já resolve.
    let m = "projects/alfa/tasks/archived/done/m.md";
    let home = home_pequeno(&[(m, "> [!example] Eme\n> [[projects/alfa/tasks/m|m]]\n")]);

    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}{}{}{}",
            achado("warn", "bloco-dessincronizado", m, MSG_BLOCO, false),
            achado(
                "warn",
                "callout-vs-pasta",
                m,
                "está em archived/done/ com callout \"example\"",
                false
            ),
            achado("warn", "historico-ausente", m, MSG_HISTORICO, false),
            achado(
                "error",
                "link-pendurado",
                m,
                &msg_link(2, "projects/alfa/tasks/m"),
                false
            ),
            rodape(4, 4, true)
        )
    );

    let saida = roda(home.path(), &["doctor", "--fix"]);
    sucesso(
        &saida,
        &format!(
            "{}{}{}{}",
            achado("warn", "bloco-dessincronizado", m, MSG_BLOCO, true),
            achado(
                "warn",
                "callout-vs-pasta",
                m,
                "está em archived/done/ com callout \"example\"",
                true
            ),
            achado("warn", "historico-ausente", m, MSG_HISTORICO, true),
            rodape(3, 0, false)
        ),
        "",
    );
    assert_eq!(
        le(home.path(), m),
        concat!(
            "> [!done] Eme\n",
            "> [[projects/alfa/tasks/archived/done/m|m]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-11-08 — arquivada em `archived/done` (feito) [reconstruído pelo doctor; data real desconhecida]\n",
        )
    );
}

// ===========================================================================
// D4 no --fix: CRLF, BOM e falta de `\n` final fora do trecho reparado
// ===========================================================================

const X_ARQ: &str = "projects/alfa/tasks/archived/done/x.md";

/// Em `archived/done/` com o bloco apontando para o caminho ativo, o callout
/// `todo`, o `id:` errado e sem histórico; corpo com espaço no fim da linha.
const D4_FIX: &str = "---\nid: errado\ntags: [a]\n---\n\n> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n> nota\n\ncorpo  \n";
const D4_FIX_REPARADA: &str = "---\nid: x\ntags: [a]\n---\n\n> [!done] Xis\n> [[projects/alfa/tasks/archived/done/x|x]]\n> nota\n\ncorpo  \n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/done` (feito) [reconstruído pelo doctor; data real desconhecida]\n";

fn d4_fix(v: Variante) {
    let home = home_pequeno(&[]);
    arquivo_bytes(home.path(), X_ARQ, &na_variante(v, D4_FIX));
    envelhece(home.path());
    let saida = roda(home.path(), &["doctor", "--fix", "--quiet"]);
    sucesso(&saida, "", "");
    assert_eq!(
        le_bytes(home.path(), X_ARQ),
        na_variante(v, D4_FIX_REPARADA)
    );
}

#[test]
fn crlf_sobrevive_ao_doctor_fix() {
    d4_fix(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_doctor_fix() {
    d4_fix(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_doctor_fix() {
    d4_fix(Variante::SemQuebraFinal);
}

/// Task ativa e arquivada sincronizadas, com frontmatter e histórico.
const D4_ATIVA: &str =
    "---\nid: y\n---\n\n> [!todo] Ípsilon\n> [[projects/alfa/tasks/y|y]]\n> _em curso_\n\ncorpo\n";
const D4_ARQUIVADA: &str = "---\nid: z\n---\n\n> [!done] Zê\n> [[projects/alfa/tasks/archived/done/z|z]]\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n";

fn d4_sem_achado(v: Variante) {
    let home = home_pequeno(&[]);
    arquivo_bytes(
        home.path(),
        "projects/alfa/tasks/y.md",
        &na_variante(v, D4_ATIVA),
    );
    arquivo_bytes(
        home.path(),
        "projects/alfa/tasks/archived/done/z.md",
        &na_variante(v, D4_ARQUIVADA),
    );
    envelhece(home.path());
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
}

#[test]
fn task_sincronizada_em_crlf_nao_e_achado() {
    d4_sem_achado(Variante::Crlf);
}

#[test]
fn task_sincronizada_com_bom_nao_e_achado() {
    d4_sem_achado(Variante::Bom);
}

#[test]
fn task_sincronizada_sem_quebra_final_nao_e_achado() {
    d4_sem_achado(Variante::SemQuebraFinal);
}

// ===========================================================================
// I3: `doctor --fix` idempotente
// ===========================================================================

#[test]
fn fix_e_idempotente() {
    // fix(fix(v)) == fix(v): a segunda passagem não muda byte nem mtime e só
    // relata o que não se repara.
    let home = home_doctor();
    arquivo_bytes(home.path(), X_ARQ, &na_variante(Variante::Crlf, D4_FIX));
    arquivo(
        home.path(),
        "projects/alfa/tasks/.freitask.ok.md.999999999.tmp",
        "meio",
    );
    envelhece(home.path());
    let primeira = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(primeira.codigo, Some(1), "stderr: {:?}", primeira.stderr);
    let depois_do_primeiro = fotografa(home.path());

    let segunda = roda(home.path(), &["doctor", "--fix"]);
    assert_eq!(segunda.codigo, Some(1), "stderr: {:?}", segunda.stderr);
    assert_eq!(segunda.stderr, "");
    nada_mudou(&depois_do_primeiro, &fotografa(home.path()));
    let pendentes: Vec<_> = achados_home_doctor(false)
        .into_iter()
        .filter(|a| {
            !matches!(
                a.1,
                "bloco-dessincronizado"
                    | "frontmatter-id"
                    | "callout-vs-pasta"
                    | "historico-ausente"
            )
        })
        .collect();
    assert_eq!(segunda.stdout, texto_de(&pendentes, false));
}

#[test]
fn depois_do_fix_so_sobram_os_erros_por_desenho() {
    // I3: com só achados reparáveis e os três `error` que não se reparam,
    // depois do `--fix` o `doctor` só relata `sync-conflict`,
    // `link-pendurado` e `sem-callout`.
    let home = home_pequeno(&[
        (OK, OK_MD),
        (CONFLITO_X, OK_MD),
        (SC, "sem callout\n"),
        (LINKS, "[[projects/alfa/tasks/fantasma|f]]\n"),
        (DES, DES_MD),
        (FM, FM_MD),
        (CV, CV_MD),
        (H, H_MD),
        ("inbox/.freitask.links.md.999999999.tmp", "meio"),
    ]);
    arquivo_bytes(home.path(), X_ARQ, &na_variante(Variante::Bom, D4_FIX));
    envelhece(home.path());
    let saida = roda(home.path(), &["doctor", "--fix", "--quiet"]);
    assert_eq!(saida.codigo, Some(1));
    let saida = roda(home.path(), &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1));
    let v = json(&saida);
    assert_eq!(
        achados_ordenados(&v),
        vec![
            achado_json(
                "error",
                "link-pendurado",
                LINKS,
                &msg_link(1, "projects/alfa/tasks/fantasma"),
                false
            ),
            achado_json("error", "sem-callout", SC, MSG_SEM_CALLOUT, false),
            achado_json("error", "sync-conflict", CONFLITO_X, MSG_CONFLITO, false),
        ]
    );
}

// ===========================================================================
// D7 / I6: `doctor` sem `--fix` é leitura pura
// ===========================================================================

#[test]
fn doctor_sem_fix_nao_cria_nada() {
    // D7 (spec §3.7): num vault sem `.freitask/` e sem `projects/`, nenhuma
    // das três formas cria nada.
    let home = home_com_vault_vazio();
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
    let saida = roda(home.path(), &["doctor", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!({"ok": true, "findings": []}));
    sucesso(&roda(home.path(), &["doctor", "--quiet"]), "", "");
    assert_eq!(entradas(&vault(home.path())), Vec::<String>::new());
}

#[test]
fn doctor_sem_fix_num_home_sem_vault_nao_cria_o_vault() {
    let home = home_sem_vault();
    sucesso(&roda(home.path(), &["doctor"]), CONSISTENTE, "");
    let saida = roda(home.path(), &["doctor", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!({"ok": true, "findings": []}));
    sucesso(&roda(home.path(), &["doctor", "--quiet"]), "", "");
    assert!(
        !vault(home.path()).exists(),
        "ObsidianVault/ não foi criado"
    );
}

#[test]
fn leitura_em_vault_do_trabalho_vazio_sai_sem_escrever() {
    // O caso do WSL (`check.sh:762`): vault com notas e sem `.freitask/` nem
    // `projects/`; o healthcheck roda `doctor --json` e `list --json`.
    let home = home_com(&[("notas/reuniao.md", "# Reunião\n\nsem tasks aqui\n")]);
    envelhece(home.path());
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!({"ok": true, "findings": []}));
    sucesso(&roda(home.path(), &["doctor", "--quiet"]), "", "");
    let saida = roda(home.path(), &["list", "--json"]);
    ok(&saida);
    assert_eq!(json(&saida), json!([]));
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn doctor_sem_fix_nao_muda_byte_nem_mtime() {
    // I6: com achados reparáveis, temporário órfão e arquivo em CRLF.
    let home = home_doctor();
    arquivo_bytes(home.path(), X_ARQ, &na_variante(Variante::Crlf, D4_FIX));
    arquivo(
        home.path(),
        "projects/alfa/tasks/.freitask.ok.md.999999999.tmp",
        "meio",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    for args in [
        &["doctor"][..],
        &["doctor", "--json"],
        &["doctor", "--quiet"],
    ] {
        let saida = roda(home.path(), args);
        assert_eq!(saida.codigo, Some(1), "{args:?}: {}", saida.stderr);
        nada_mudou(&antes, &fotografa(home.path()));
    }
}

// ===========================================================================
// temporario-orfao (spec §1.5, §2.2)
// ===========================================================================

const TMP: &str = "projects/alfa/tasks/.freitask.ok.md.999999999.tmp";

#[test]
fn doctor_acusa_temporario_orfao() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        (TMP, "> [!todo] Ok\n> [[projects/alfa/tasks/ok|o"),
    ]);
    um_achado(home.path(), "warn", "temporario-orfao", TMP, MSG_TEMP);
}

#[test]
fn doctor_acusa_temporario_orfao_em_qualquer_pasta_do_vault() {
    // Onde uma gravação pode acontecer: pasta de task, nota fora de projeto,
    // a raiz (o painel), `daily/` e `.freitask/` (o `status.json`).
    let temporarios = [
        TMP,
        "inbox/.freitask.nota.md.999999999.tmp",
        ".freitask.CURRENT.md.999999999.tmp",
        "daily/.freitask.2026-11-07.md.999999999.tmp",
        ".freitask/.freitask.status.json.999999999.tmp",
    ];
    let home = home_pequeno(&[(OK, OK_MD)]);
    for t in temporarios {
        arquivo(home.path(), t, "meio");
    }
    envelhece(home.path());
    let saida = roda(home.path(), &["doctor", "--json"]);
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    let mut esperado: Vec<Value> = temporarios
        .iter()
        .map(|t| achado_json("warn", "temporario-orfao", t, MSG_TEMP, false))
        .collect();
    esperado.sort_by_key(|f| f["path"].as_str().unwrap_or("").to_owned());
    assert_eq!(achados_ordenados(&json(&saida)), esperado);
}

#[test]
fn doctor_fix_apaga_o_temporario_orfao_e_nao_toca_o_destino() {
    let home = home_pequeno(&[(OK, OK_MD), (TMP, "meio")]);
    let antes = fotografa(home.path());
    um_reparo(home.path(), "warn", "temporario-orfao", TMP, MSG_TEMP);
    assert!(!existe(home.path(), TMP));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[TMP, CURRENT])
    );
    assert_eq!(le(home.path(), OK), OK_MD);
}

#[test]
fn temporario_orfao_vem_depois_dos_outros_achados() {
    let home = home_pequeno(&[
        (OK, OK_MD),
        (TMP, "meio"),
        (LINKS, "[[projects/alfa/tasks/fantasma|f]]\n"),
        (SC, "sem callout\n"),
    ]);
    let saida = roda(home.path(), &["doctor"]);
    assert_eq!(saida.codigo, Some(1));
    assert_eq!(
        saida.stdout,
        format!(
            "{}{}{}{}",
            achado("error", "sem-callout", SC, MSG_SEM_CALLOUT, false),
            achado(
                "error",
                "link-pendurado",
                LINKS,
                &msg_link(1, "projects/alfa/tasks/fantasma"),
                false
            ),
            achado("warn", "temporario-orfao", TMP, MSG_TEMP, false),
            rodape(3, 3, true)
        )
    );
}

// ===========================================================================
// schema/doctor.json
// ===========================================================================

#[test]
fn schema_doctor_valida_a_saida_com_achados() {
    let home = home_doctor();
    valida("doctor", &json(&roda(home.path(), &["doctor", "--json"])));
}

#[test]
fn schema_doctor_valida_a_saida_do_fix() {
    let home = home_doctor();
    valida(
        "doctor",
        &json(&roda(home.path(), &["doctor", "--fix", "--json"])),
    );
}

#[test]
fn schema_doctor_valida_a_saida_sem_achado() {
    let home = home_pequeno(&[(OK, OK_MD)]);
    valida("doctor", &json(&roda(home.path(), &["doctor", "--json"])));
}

#[test]
fn schema_doctor_valida_temporario_orfao() {
    let home = home_pequeno(&[(OK, OK_MD), (TMP, "meio")]);
    valida("doctor", &json(&roda(home.path(), &["doctor", "--json"])));
}

#[test]
fn schema_doctor_recusa_forma_errada() {
    let esquema = schema("doctor");
    let validador = jsonschema::validator_for(&esquema).expect("schema válido");
    let certo = achado_json(
        "warn",
        "status-0",
        S0,
        "callout \"bogus\" não existe em status.json",
        false,
    );
    assert!(validador.is_valid(&json!({"ok": false, "findings": [certo.clone()]})));
    assert!(validador.is_valid(&json!({"ok": true, "findings": []})));

    let mut a_mais = certo.clone();
    a_mais["linha"] = json!(1);
    assert!(!validador.is_valid(&json!({"ok": false, "findings": [a_mais]})));
    let mut a_menos = certo.clone();
    a_menos.as_object_mut().expect("objeto").remove("fixed");
    assert!(!validador.is_valid(&json!({"ok": false, "findings": [a_menos]})));
    let mut nivel = certo.clone();
    nivel["level"] = json!("info");
    assert!(!validador.is_valid(&json!({"ok": false, "findings": [nivel]})));
    let mut kind = certo.clone();
    kind["kind"] = json!("inventado");
    assert!(!validador.is_valid(&json!({"ok": false, "findings": [kind]})));
    let mut absoluto = certo.clone();
    absoluto["path"] = json!("/home/x/ObsidianVault/projects/alfa/tasks/s0.md");
    assert!(!validador.is_valid(&json!({"ok": false, "findings": [absoluto]})));
    // `findings` vazio é array, não objeto (`vim.json.encode({})` dá `{}`).
    assert!(!validador.is_valid(&json!({"ok": true, "findings": {}})));
    assert!(!validador.is_valid(&json!({"findings": []})));
    assert!(!validador.is_valid(&json!({"ok": true, "findings": [], "extra": 1})));
}
