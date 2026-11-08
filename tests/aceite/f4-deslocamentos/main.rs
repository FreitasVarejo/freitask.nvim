//! Aceite da fatia 4 — Links e deslocamentos (spec §5): o scanner de
//! wikilinks e a reescrita do vault vistos de fora, e os quatro comandos que
//! mudam uma task de endereço: `archive`, `unarchive`, `rename` e `move`.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§0, §1.4, §1.5, §1.6, §2.1, §2.4, §3.2 I1/I2/I5/I7, §3.3, §3.4 D1/D2/D4/D5,
//! §3.7) e `docs/freitask.md` ("Arquivar / desarquivar / deletar", "Rodapé
//! `## Histórico`", "Mover para outro projeto", "Referências do Obsidian
//! (`M.retarget_links`)", "CLI"). O texto exato de stdout/stderr e dos arquivos
//! gravados foi medido no oráculo Lua (`lua/freitask/cli.lua`) num HOME
//! temporário; onde a spec manda divergir (D1, D2, D4, D5, I7, saída 3,
//! `FREITASK_TODAY` na linha do histórico), vale a spec. Nenhum valor esperado
//! aqui foi gerado do binário sob teste.
//!
//! Decisões tomadas aqui, onde a spec e o oráculo não fecham (repetidas no
//! relatório do rascunho):
//! - o aviso `freitask: N referência(s) atualizada(s) em M arquivo(s)` em
//!   stderr é parte do texto de hoje (§2.1, byte a byte) e sai só quando N > 0;
//!   N conta os links efetivamente reescritos, então link em tabela (D1) entra
//!   na conta e link em código (D5) e em `*.sync-conflict-*` (I7) não;
//! - as recusas de `archive`/`unarchive` que o oráculo dá por `vim.notify`
//!   saem em duas linhas, como hoje: o motivo e `operação não concluída (veja
//!   as mensagens acima)`; as de `rename`/`move`, numa linha só;
//! - `archive` com tipo que não existe sai 1 (como hoje), não 2;
//! - D2 no `move`: a mensagem é a de reservado, como a fatia 3 fixou para
//!   `new` e `new-project`;
//! - D4: a linha que o comando acrescenta (`## Histórico`) sai no fim de linha
//!   do arquivo (CRLF num arquivo CRLF), e num arquivo sem `\n` final a última
//!   linha acrescentada também fica sem `\n` (a mesma regra da fatia 3); o BOM
//!   antes do `---` não impede o `rename` de achar o `id:` do frontmatter;
//! - a linha do `## Histórico` usa o dia de `FREITASK_TODAY` (§1.6), que o
//!   oráculo ignora ali (usa o relógio);
//! - "nada gravado" é: nenhum arquivo do vault mudou de bytes nem de mtime e
//!   nenhuma entrada apareceu ou sumiu; nos caminhos felizes, "só mudou X" olha
//!   só arquivos (pasta vazia que sobra ou nasce não é afirmada);
//! - fica de fora o que a spec não fecha: link para a própria task no corpo
//!   dela, link com `.md` no alvo ou com espaço em volta, e link dentro de
//!   frontmatter (§1.4 o põe fora do scanner, mas não há entrada em
//!   `divergencias.toml` para isso).

mod util;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use util::*;

// ===========================================================================
// Fixtures
// ===========================================================================

/// `x`: frontmatter com chaves alheias e `id:`, bloco com descrição e nota,
/// corpo com seção e checklist, sem `## Histórico`.
const X_MD: &str = concat!(
    "---\n",
    "aliases: []\n",
    "id: x\n",
    "tags:\n",
    "  - foo\n",
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

/// `x-2`: id com o prefixo de `x`; sem frontmatter; a nota do bloco cita `x`.
const X2_MD: &str = concat!(
    "> [!example] Xis dois\n",
    "> [[projects/alfa/tasks/x-2|x-2]]\n",
    "> depois de [[projects/alfa/tasks/x|x]]\n",
);

/// `velha`: arquivada em `done`, com histórico e um link para `x` no corpo.
const VELHA_MD: &str = concat!(
    "> [!done] Velha\n",
    "> [[projects/alfa/tasks/archived/done/velha|velha]]\n",
    "\n",
    "veio antes de [[projects/alfa/tasks/x|x]]\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
);

/// `z`: task de um projeto encerrado, com um link para `x` no corpo.
const Z_MD: &str = concat!(
    "> [!todo] Zeta\n",
    "> [[projects/_encerrados/omega/tasks/z|z]]\n",
    "\n",
    "ver [[projects/alfa/tasks/x|x]]\n",
);

/// A nota com todas as formas de link que a reescrita tem de entender, e as
/// que ela não pode tocar.
const NOTAS_MD: &str = concat!(
    "# Notas\n",
    "\n",
    "- caminho: [[projects/alfa/tasks/x|x]]\n",
    "- sem alias: [[projects/alfa/tasks/x]]\n",
    "- alias livre: [[projects/alfa/tasks/x|o xis]]\n",
    "- heading: [[projects/alfa/tasks/x#Notas Soltas|seção]]\n",
    "- bloco: [[projects/alfa/tasks/x^abc123]]\n",
    "- embed: ![[projects/alfa/tasks/x]]\n",
    "- curtos: [[x]], [[x|x]], [[x|Xis]], [[x#Notas Soltas]] e ![[x]]\n",
    "- vizinha: [[projects/alfa/tasks/x-2|x-2]] e [[x-2]]\n",
    "- pendurado: [[projects/alfa/tasks/fantasma|fantasma]]\n",
    "- homônimo: [[projects/beta/tasks/x|x]]\n",
    "- dois na linha: [[projects/alfa/tasks/x|x]] e [[projects/alfa/tasks/archived/done/velha|velha]]\n",
);

const SOLTA_MD: &str = "solta: [[projects/alfa/tasks/x|x]]\n";
const DAILY_MD: &str = "# Dia\n\n- [[projects/alfa/tasks/x|x]] e [[x]]\n";
const LOG_TXT: &str = "[[projects/alfa/tasks/x|x]] e [[x]]\n";

const X: &str = "projects/alfa/tasks/x.md";
const X2: &str = "projects/alfa/tasks/x-2.md";
const VELHA: &str = "projects/alfa/tasks/archived/done/velha.md";
const Z: &str = "projects/_encerrados/omega/tasks/z.md";
const NOTAS: &str = "projects/alfa/notas.md";
const SOLTA: &str = "inbox/solta.md";
const DAILY: &str = "daily/2026-11-01.md";
const LOG: &str = "projects/alfa/anexos/x/log.txt";
const CURRENT: &str = "CURRENT.md";

const X_DONE: &str = "projects/alfa/tasks/archived/done/x.md";
const X_DROPPED: &str = "projects/alfa/tasks/archived/dropped/x.md";
const NOVO: &str = "projects/alfa/tasks/novo.md";
const X_BETA: &str = "projects/beta/tasks/x.md";
const VELHA_ATIVA: &str = "projects/alfa/tasks/velha.md";

/// O vault base: `alfa` com `x` (todo) e `x-2` (example) ativas e `velha` em
/// `archived/done/`, mais `notas.md` e um anexo `.txt`; `beta` vazio; `omega`
/// encerrado; uma nota em `inbox/` e um `daily/`. `status.json` default, sem
/// `CURRENT.md`. Todos os arquivos envelhecidos (mtime no passado).
fn home_base() -> tempfile::TempDir {
    let home = home_com(&[
        (X, X_MD),
        (X2, X2_MD),
        (VELHA, VELHA_MD),
        (Z, Z_MD),
        (NOTAS, NOTAS_MD),
        (SOLTA, SOLTA_MD),
        (DAILY, DAILY_MD),
        (LOG, LOG_TXT),
    ]);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects/beta/tasks");
    envelhece(home.path());
    home
}

/// Um vault com `status.json`, `beta` vazio e os arquivos dados, envelhecido.
fn home_pequeno(arquivos: &[(&str, &str)]) -> tempfile::TempDir {
    let home = home_com(arquivos);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects/beta/tasks");
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

const SECAO_BETA_VAZIA: &str = "## beta\n\n_(sem tasks)_\n\n";

/// O painel do vault base, intocado.
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
            "> [!example] Xis dois\n",
            "> [[projects/alfa/tasks/x-2|x-2]]\n",
            "> depois de [[projects/alfa/tasks/x|x]]\n",
            "\n",
            "## beta\n",
            "\n",
            "_(sem tasks)_\n",
            "\n",
        ),
    )
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

/// Afirma a recusa de `archive`/`unarchive` como hoje: saída 1, stdout vazio
/// e duas linhas em stderr, o motivo e o fecho.
fn recusa_em_duas_linhas(saida: &Saida, msg: &str) {
    assert_eq!(saida.codigo, Some(1), "stderr: {:?}", saida.stderr);
    assert_eq!(
        saida.stderr,
        format!("freitask: {msg}\nfreitask: operação não concluída (veja as mensagens acima)\n"),
        "stderr"
    );
    assert_eq!(saida.stdout, "", "stdout tem de ficar vazio numa recusa");
}

/// O stdout de um comando que deu certo: o caminho absoluto novo e `\n`.
fn caminho(home: &Path, rel: &str) -> String {
    format!("{}\n", abs(home, rel))
}

// ===========================================================================
// archive: caminho feliz
// ===========================================================================

const X_ARQUIVADA_DONE: &str = concat!(
    "---\n",
    "aliases: []\n",
    "id: x\n",
    "tags:\n",
    "  - foo\n",
    "---\n",
    "\n",
    "> [!done] Xis\n",
    "> [[projects/alfa/tasks/archived/done/x|x]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "\n",
    "## Notas Soltas\n",
    "\n",
    "- [ ] item\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-11-08 — arquivada em `archived/done` (feito)\n",
);

const NOTAS_COM_X_ARQUIVADA: &str = concat!(
    "# Notas\n",
    "\n",
    "- caminho: [[projects/alfa/tasks/archived/done/x|x]]\n",
    "- sem alias: [[projects/alfa/tasks/archived/done/x]]\n",
    "- alias livre: [[projects/alfa/tasks/archived/done/x|o xis]]\n",
    "- heading: [[projects/alfa/tasks/archived/done/x#Notas Soltas|seção]]\n",
    "- bloco: [[projects/alfa/tasks/archived/done/x^abc123]]\n",
    "- embed: ![[projects/alfa/tasks/archived/done/x]]\n",
    "- curtos: [[x]], [[x|x]], [[x|Xis]], [[x#Notas Soltas]] e ![[x]]\n",
    "- vizinha: [[projects/alfa/tasks/x-2|x-2]] e [[x-2]]\n",
    "- pendurado: [[projects/alfa/tasks/fantasma|fantasma]]\n",
    "- homônimo: [[projects/beta/tasks/x|x]]\n",
    "- dois na linha: [[projects/alfa/tasks/archived/done/x|x]] e [[projects/alfa/tasks/archived/done/velha|velha]]\n",
);

#[test]
fn archive_done_move_a_task_e_imprime_o_caminho() {
    let home = home_base();
    let saida = roda(home.path(), &["archive", "x", "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(11, 5));
    assert!(!existe(home.path(), X));
    assert_eq!(le(home.path(), X_DONE), X_ARQUIVADA_DONE);
}

#[test]
fn fase_acompanha_a_pasta_ao_arquivar() {
    // `e3e4b56`: `archive done` grava `done`; `unarchive` devolve `check`.
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["archive", "x", "done"]));
    assert!(le(home.path(), X_DONE).contains("\n> [!done] Xis\n"));
    ok_ignorando_aviso(&roda(home.path(), &["unarchive", "x"]));
    assert_eq!(
        le(home.path(), X),
        concat!(
            "---\n",
            "aliases: []\n",
            "id: x\n",
            "tags:\n",
            "  - foo\n",
            "---\n",
            "\n",
            "> [!check] Xis\n",
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
            "- 2026-11-08 — arquivada em `archived/done` (feito)\n",
            "- 2026-11-08 — desarquivada, de volta ao board\n",
        )
    );
}

/// Saída 0 e stdout com um caminho; o stderr (o aviso de referências) não é o
/// que o teste prova.
fn ok_ignorando_aviso(saida: &Saida) {
    assert_eq!(saida.codigo, Some(0), "stderr: {:?}", saida.stderr);
    assert!(
        saida.stdout.ends_with(".md\n"),
        "stdout: {:?}",
        saida.stdout
    );
}

#[test]
fn archive_dropped_mantem_a_fase_e_glosa_abandonado() {
    let home = home_pequeno(&[(
        "projects/alfa/tasks/w.md",
        "> [!warning] Dabliu\n> [[projects/alfa/tasks/w|w]]\n> travada\n",
    )]);
    let saida = roda(home.path(), &["archive", "w", "dropped"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/archived/dropped/w.md"),
        "",
    );
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/archived/dropped/w.md"),
        concat!(
            "> [!warning] Dabliu\n",
            "> [[projects/alfa/tasks/archived/dropped/w|w]]\n",
            "> travada\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-11-08 — arquivada em `archived/dropped` (abandonado)\n",
        )
    );
}

#[test]
fn archive_failed_mantem_a_fase_e_glosa_falhou() {
    let home = home_pequeno(&[(
        "projects/alfa/tasks/w.md",
        "> [!example] Dabliu\n> [[projects/alfa/tasks/w|w]]\n",
    )]);
    let saida = roda(home.path(), &["archive", "w", "failed"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/archived/failed/w.md"),
        "",
    );
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/archived/failed/w.md"),
        concat!(
            "> [!example] Dabliu\n",
            "> [[projects/alfa/tasks/archived/failed/w|w]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-11-08 — arquivada em `archived/failed` (falhou)\n",
        )
    );
}

#[test]
fn archive_sem_referencias_nao_avisa_nada_em_stderr() {
    let home = home_pequeno(&[(
        "projects/alfa/tasks/w.md",
        "> [!todo] Dabliu\n> [[projects/alfa/tasks/w|w]]\n",
    )]);
    let saida = roda(home.path(), &["archive", "w", "done"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/archived/done/w.md"),
        "",
    );
}

#[test]
fn archive_reescreve_os_links_path_qualificados_e_deixa_os_curtos() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["archive", "x", "done"]));
    assert_eq!(le(home.path(), NOTAS), NOTAS_COM_X_ARQUIVADA);
}

#[test]
fn archive_reescreve_links_em_outras_tasks_ativas_arquivadas_e_encerradas() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["archive", "x", "done"]));
    assert_eq!(
        le(home.path(), X2),
        concat!(
            "> [!example] Xis dois\n",
            "> [[projects/alfa/tasks/x-2|x-2]]\n",
            "> depois de [[projects/alfa/tasks/archived/done/x|x]]\n",
        )
    );
    assert_eq!(
        le(home.path(), VELHA),
        VELHA_MD.replace(
            "veio antes de [[projects/alfa/tasks/x|x]]",
            "veio antes de [[projects/alfa/tasks/archived/done/x|x]]"
        )
    );
    assert_eq!(
        le(home.path(), Z),
        Z_MD.replace(
            "ver [[projects/alfa/tasks/x|x]]",
            "ver [[projects/alfa/tasks/archived/done/x|x]]"
        )
    );
    assert_eq!(
        le(home.path(), SOLTA),
        "solta: [[projects/alfa/tasks/archived/done/x|x]]\n"
    );
}

#[test]
fn archive_so_grava_a_task_as_notas_que_a_citam_e_o_current() {
    // I2 por exemplo: daily/, o anexo .txt, o status.json e a `x-2` só no que
    // a cita; nada mais muda de bytes nem de mtime.
    let home = home_base();
    let antes = fotografa(home.path());
    ok_ignorando_aviso(&roda(home.path(), &["archive", "x", "done"]));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[X, X_DONE, X2, VELHA, Z, NOTAS, SOLTA, CURRENT])
    );
    assert_eq!(le(home.path(), DAILY), DAILY_MD);
    assert_eq!(le(home.path(), LOG), LOG_TXT);
}

#[test]
fn archive_regenera_o_current() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["archive", "x", "done"]));
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
                    "> [!example] Xis dois\n",
                    "> [[projects/alfa/tasks/x-2|x-2]]\n",
                    "> depois de [[projects/alfa/tasks/archived/done/x|x]]\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn archive_nao_reescreve_link_nas_notas_avulsas_do_current() {
    // `CURRENT.md` fica fora da varredura (docs/freitask.md, "Referências do
    // Obsidian"): é regenerado, e a seção à mão vai como estava.
    let home = home_base();
    let avulsa = "- ver [[projects/alfa/tasks/x|x]]\n";
    arquivo(
        home.path(),
        CURRENT,
        &painel_base().replacen("- \n", avulsa, 1),
    );
    envelhece(home.path());
    let saida = roda(home.path(), &["archive", "x", "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(11, 5));
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
                    "> [!example] Xis dois\n",
                    "> [[projects/alfa/tasks/x-2|x-2]]\n",
                    "> depois de [[projects/alfa/tasks/archived/done/x|x]]\n",
                    "\n",
                )
            )
        )
        .replacen("- \n", avulsa, 1)
    );
    assert!(!existe(home.path(), "daily/2026-11-08.md"));
}

#[test]
fn archive_acrescenta_no_fim_da_secao_historico_mesmo_com_secao_depois() {
    let h = concat!(
        "> [!todo] Agá\n",
        "> [[projects/alfa/tasks/h|h]]\n",
        "\n",
        "## Histórico\n",
        "\n",
        "- 2026-10-01 — desarquivada, de volta ao board\n",
        "\n",
        "## Depois\n",
        "\n",
        "texto\n",
    );
    let home = home_pequeno(&[("projects/alfa/tasks/h.md", h)]);
    ok(&roda(home.path(), &["archive", "h", "dropped"]));
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/archived/dropped/h.md"),
        concat!(
            "> [!todo] Agá\n",
            "> [[projects/alfa/tasks/archived/dropped/h|h]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-10-01 — desarquivada, de volta ao board\n",
            "- 2026-11-08 — arquivada em `archived/dropped` (abandonado)\n",
            "\n",
            "## Depois\n",
            "\n",
            "texto\n",
        )
    );
}

#[test]
fn historico_usa_o_dia_de_freitask_today() {
    // §1.6: `FREITASK_TODAY` fixa a linha do `## Histórico`.
    let home = home_pequeno(&[(
        "projects/alfa/tasks/w.md",
        "> [!todo] Dabliu\n> [[projects/alfa/tasks/w|w]]\n",
    )]);
    let o = comando(home.path())
        .env("FREITASK_TODAY", "2026-12-25")
        .args(["archive", "w", "dropped"])
        .output()
        .expect("rodar");
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/archived/dropped/w.md"),
        concat!(
            "> [!todo] Dabliu\n",
            "> [[projects/alfa/tasks/archived/dropped/w|w]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-12-25 — arquivada em `archived/dropped` (abandonado)\n",
        )
    );
}

#[test]
fn archive_por_caminho_relativo_resolve_id_ambiguo() {
    let home = home_base();
    let x_de_beta = "> [!todo] Xis de beta\n> [[projects/beta/tasks/x|x]]\n";
    arquivo(home.path(), X_BETA, x_de_beta);
    envelhece(home.path());
    let saida = roda(
        home.path(),
        &[
            "archive",
            "ObsidianVault/projects/beta/tasks/x.md",
            "dropped",
        ],
    );
    // O homônimo `[[projects/beta/tasks/x|x]]` de notas.md é o único link
    // para a `x` de beta.
    sucesso(
        &saida,
        &caminho(home.path(), "projects/beta/tasks/archived/dropped/x.md"),
        &aviso_refs(1, 1),
    );
    assert_eq!(le(home.path(), X), X_MD);
    assert_eq!(
        le(home.path(), NOTAS),
        NOTAS_MD.replace(
            "- homônimo: [[projects/beta/tasks/x|x]]",
            "- homônimo: [[projects/beta/tasks/archived/dropped/x|x]]"
        )
    );
}

#[test]
fn archive_por_caminho_absoluto() {
    let home = home_base();
    let saida = roda(home.path(), &["archive", &abs(home.path(), X), "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(11, 5));
    assert_eq!(le(home.path(), X_DONE), X_ARQUIVADA_DONE);
}

#[test]
fn archive_no_tipo_em_que_ja_esta_sai_0_sem_mexer_na_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["archive", "velha", "done"]);
    sucesso(&saida, &caminho(home.path(), VELHA), "");
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(
        mudou.is_subset(&conjunto(&[CURRENT])),
        "só o painel pode ter mudado: {mudou:?}"
    );
}

// ===========================================================================
// archive: recusas
// ===========================================================================

#[test]
fn archive_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["archive"]),
        2,
        "falta o id ou caminho da task",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_sem_tipo_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["archive", "x"]),
        2,
        "falta o tipo: done|dropped|failed",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_tipo_invalido_sai_1_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa_em_duas_linhas(
        &roda(home.path(), &["archive", "x", "misc"]),
        "tipo de arquivamento inválido: misc",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_id_inexistente_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["archive", "zzz", "done"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_nao_acha_task_de_projeto_encerrado() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["archive", "z", "done"]),
        1,
        "task não encontrada: z",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_id_ambiguo_sai_1_e_nao_grava() {
    let home = home_base();
    arquivo(
        home.path(),
        X_BETA,
        "> [!todo] Xis de beta\n> [[projects/beta/tasks/x|x]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["archive", "x", "done"]),
        1,
        "id \"x\" é ambíguo (2 arquivos); passe o caminho",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_recusa_destino_ocupado_e_nao_grava() {
    let home = home_base();
    arquivo(home.path(), X_DONE, VELHA_MD);
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa_em_duas_linhas(
        &roda(
            home.path(),
            &["archive", "ObsidianVault/projects/alfa/tasks/x.md", "done"],
        ),
        &format!("já existe um arquivo em {}", abs(home.path(), X_DONE)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_recusa_arquivo_que_nao_e_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa_em_duas_linhas(
        &roda(
            home.path(),
            &["archive", "ObsidianVault/projects/alfa/notas.md", "done"],
        ),
        &format!("não é um arquivo de task: {}", abs(home.path(), NOTAS)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// unarchive
// ===========================================================================

#[test]
fn unarchive_devolve_a_task_ao_board_e_imprime_o_caminho() {
    let home = home_base();
    let saida = roda(home.path(), &["unarchive", "velha"]);
    sucesso(
        &saida,
        &caminho(home.path(), VELHA_ATIVA),
        &aviso_refs(1, 1),
    );
    assert!(!existe(home.path(), VELHA));
    assert_eq!(
        le(home.path(), VELHA_ATIVA),
        concat!(
            "> [!check] Velha\n",
            "> [[projects/alfa/tasks/velha|velha]]\n",
            "\n",
            "veio antes de [[projects/alfa/tasks/x|x]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
            "- 2026-11-08 — desarquivada, de volta ao board\n",
        )
    );
}

#[test]
fn unarchive_reescreve_as_referencias_para_o_caminho_ativo() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["unarchive", "velha"]));
    assert_eq!(
        le(home.path(), NOTAS),
        NOTAS_MD.replace(
            "[[projects/alfa/tasks/archived/done/velha|velha]]",
            "[[projects/alfa/tasks/velha|velha]]"
        )
    );
}

#[test]
fn unarchive_so_grava_a_task_a_nota_que_a_cita_e_o_current() {
    let home = home_base();
    let antes = fotografa(home.path());
    ok_ignorando_aviso(&roda(home.path(), &["unarchive", "velha"]));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[VELHA, VELHA_ATIVA, NOTAS, CURRENT])
    );
}

#[test]
fn unarchive_regenera_o_current_com_a_task_de_volta() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["unarchive", "velha"]));
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            3,
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
                    "> [!example] Xis dois\n",
                    "> [[projects/alfa/tasks/x-2|x-2]]\n",
                    "> depois de [[projects/alfa/tasks/x|x]]\n",
                    "\n",
                    "> [!check] Velha\n",
                    "> [[projects/alfa/tasks/velha|velha]]\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn unarchive_de_dropped_mantem_a_fase() {
    let largada = concat!(
        "> [!warning] Largada\n",
        "> [[projects/alfa/tasks/archived/dropped/largada|largada]]\n",
        "\n",
        "## Histórico\n",
        "\n",
        "- 2026-10-01 — arquivada em `archived/dropped` (abandonado)\n",
    );
    let home = home_pequeno(&[("projects/alfa/tasks/archived/dropped/largada.md", largada)]);
    let saida = roda(home.path(), &["unarchive", "largada"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/largada.md"),
        "",
    );
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/largada.md"),
        concat!(
            "> [!warning] Largada\n",
            "> [[projects/alfa/tasks/largada|largada]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-10-01 — arquivada em `archived/dropped` (abandonado)\n",
            "- 2026-11-08 — desarquivada, de volta ao board\n",
        )
    );
}

#[test]
fn unarchive_de_task_ativa_sai_0_sem_mexer_na_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["unarchive", "x"]);
    sucesso(&saida, &caminho(home.path(), X), "");
    let mudou = arquivos_mudados(&antes, &fotografa(home.path()));
    assert!(
        mudou.is_subset(&conjunto(&[CURRENT])),
        "só o painel pode ter mudado: {mudou:?}"
    );
}

#[test]
fn archive_e_unarchive_devolvem_os_arquivos_originais_menos_o_historico() {
    // I5 por exemplo.
    let home = home_base();
    let antes = bytes_de(&fotografa(home.path()));
    ok_ignorando_aviso(&roda(home.path(), &["archive", "x", "dropped"]));
    let saida = roda(home.path(), &["unarchive", "x"]);
    sucesso(&saida, &caminho(home.path(), X), &aviso_refs(11, 5));
    let mut depois = bytes_de(&fotografa(home.path()));
    assert_eq!(
        String::from_utf8(depois.remove(X).expect("x voltou")).expect("utf-8"),
        format!(
            "{X_MD}\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/dropped` (abandonado)\n- 2026-11-08 — desarquivada, de volta ao board\n"
        )
    );
    assert_eq!(
        String::from_utf8(depois.remove(CURRENT).expect("painel")).expect("utf-8"),
        painel_base()
    );
    let mut antes = antes;
    antes.remove(X);
    assert_eq!(
        depois, antes,
        "fora a task e o painel, tudo volta byte a byte"
    );
    assert!(!existe(home.path(), X_DROPPED));
}

#[test]
fn unarchive_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["unarchive"]),
        2,
        "falta o id ou caminho da task",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn unarchive_id_inexistente_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["unarchive", "zzz"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn unarchive_recusa_destino_ocupado_e_nao_grava() {
    let home = home_base();
    arquivo(
        home.path(),
        VELHA_ATIVA,
        "> [!todo] Velha nova\n> [[projects/alfa/tasks/velha|velha]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa_em_duas_linhas(
        &roda(
            home.path(),
            &[
                "unarchive",
                "ObsidianVault/projects/alfa/tasks/archived/done/velha.md",
            ],
        ),
        &format!("já existe um arquivo em {}", abs(home.path(), VELHA_ATIVA)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// rename
// ===========================================================================

const NOVO_MD: &str = concat!(
    "---\n",
    "aliases: []\n",
    "id: novo\n",
    "tags:\n",
    "  - foo\n",
    "---\n",
    "\n",
    "> [!todo] Xis\n",
    "> [[projects/alfa/tasks/novo|novo]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "\n",
    "## Notas Soltas\n",
    "\n",
    "- [ ] item\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-11-08 — renomeada de `x` para `novo`\n",
);

const NOTAS_COM_X_RENOMEADA: &str = concat!(
    "# Notas\n",
    "\n",
    "- caminho: [[projects/alfa/tasks/novo|novo]]\n",
    "- sem alias: [[projects/alfa/tasks/novo]]\n",
    "- alias livre: [[projects/alfa/tasks/novo|o xis]]\n",
    "- heading: [[projects/alfa/tasks/novo#Notas Soltas|seção]]\n",
    "- bloco: [[projects/alfa/tasks/novo^abc123]]\n",
    "- embed: ![[projects/alfa/tasks/novo]]\n",
    "- curtos: [[novo]], [[novo|novo]], [[novo|Xis]], [[novo#Notas Soltas]] e ![[novo]]\n",
    "- vizinha: [[projects/alfa/tasks/x-2|x-2]] e [[x-2]]\n",
    "- pendurado: [[projects/alfa/tasks/fantasma|fantasma]]\n",
    "- homônimo: [[projects/beta/tasks/x|x]]\n",
    "- dois na linha: [[projects/alfa/tasks/novo|novo]] e [[projects/alfa/tasks/archived/done/velha|velha]]\n",
);

#[test]
fn rename_move_o_arquivo_troca_o_id_do_frontmatter_e_o_link_da_linha_2() {
    let home = home_base();
    let saida = roda(home.path(), &["rename", "x", "novo"]);
    sucesso(&saida, &caminho(home.path(), NOVO), &aviso_refs(16, 5));
    assert!(!existe(home.path(), X));
    assert_eq!(le(home.path(), NOVO), NOVO_MD);
}

#[test]
fn rename_reescreve_links_curtos_e_path_qualificados_do_vault() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["rename", "x", "novo"]));
    assert_eq!(le(home.path(), NOTAS), NOTAS_COM_X_RENOMEADA);
    assert_eq!(
        le(home.path(), X2),
        X2_MD.replace(
            "> depois de [[projects/alfa/tasks/x|x]]",
            "> depois de [[projects/alfa/tasks/novo|novo]]"
        )
    );
    assert_eq!(
        le(home.path(), VELHA),
        VELHA_MD.replace(
            "veio antes de [[projects/alfa/tasks/x|x]]",
            "veio antes de [[projects/alfa/tasks/novo|novo]]"
        )
    );
    assert_eq!(
        le(home.path(), Z),
        Z_MD.replace(
            "ver [[projects/alfa/tasks/x|x]]",
            "ver [[projects/alfa/tasks/novo|novo]]"
        )
    );
    assert_eq!(
        le(home.path(), SOLTA),
        "solta: [[projects/alfa/tasks/novo|novo]]\n"
    );
}

#[test]
fn rename_alias_igual_ao_id_antigo_acompanha_e_o_livre_fica() {
    let nota =
        "[[projects/alfa/tasks/w|w]] [[projects/alfa/tasks/w|Dabliu]] [[w|w]] [[w|w de novo]]\n";
    let home = home_pequeno(&[
        (
            "projects/alfa/tasks/w.md",
            "> [!todo] Dabliu\n> [[projects/alfa/tasks/w|w]]\n",
        ),
        ("projects/alfa/nota.md", nota),
    ]);
    let saida = roda(home.path(), &["rename", "w", "ve"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/ve.md"),
        &aviso_refs(4, 1),
    );
    assert_eq!(
        le(home.path(), "projects/alfa/nota.md"),
        "[[projects/alfa/tasks/ve|ve]] [[projects/alfa/tasks/ve|Dabliu]] [[ve|ve]] [[ve|w de novo]]\n"
    );
}

#[test]
fn rename_nao_toca_link_de_task_com_prefixo_parecido() {
    let nota =
        "[[w-2]] [[projects/alfa/tasks/w-2|w-2]] [[ww]] [[projects/alfa/tasks/wa|wa]] [[w]]\n";
    let home = home_pequeno(&[
        (
            "projects/alfa/tasks/w.md",
            "> [!todo] Dabliu\n> [[projects/alfa/tasks/w|w]]\n",
        ),
        (
            "projects/alfa/tasks/w-2.md",
            "> [!todo] Dabliu 2\n> [[projects/alfa/tasks/w-2|w-2]]\n",
        ),
        ("projects/alfa/nota.md", nota),
    ]);
    let saida = roda(home.path(), &["rename", "w", "ve"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/ve.md"),
        &aviso_refs(1, 1),
    );
    assert_eq!(
        le(home.path(), "projects/alfa/nota.md"),
        "[[w-2]] [[projects/alfa/tasks/w-2|w-2]] [[ww]] [[projects/alfa/tasks/wa|wa]] [[ve]]\n"
    );
}

#[test]
fn rename_so_grava_a_task_as_notas_que_a_citam_e_o_current() {
    let home = home_base();
    let antes = fotografa(home.path());
    ok_ignorando_aviso(&roda(home.path(), &["rename", "x", "novo"]));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[X, NOVO, X2, VELHA, Z, NOTAS, SOLTA, CURRENT])
    );
    assert_eq!(le(home.path(), DAILY), DAILY_MD);
    assert_eq!(le(home.path(), LOG), LOG_TXT);
}

#[test]
fn rename_regenera_o_current() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["rename", "x", "novo"]));
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
                    "> [!todo] Xis\n",
                    "> [[projects/alfa/tasks/novo|novo]]\n",
                    "> _em curso_\n",
                    "> nota um\n",
                    "\n",
                    "> [!example] Xis dois\n",
                    "> [[projects/alfa/tasks/x-2|x-2]]\n",
                    "> depois de [[projects/alfa/tasks/novo|novo]]\n",
                    "\n",
                )
            )
        )
    );
}

#[test]
fn rename_de_task_arquivada_fica_na_mesma_pasta_de_arquivo() {
    let home = home_base();
    let saida = roda(home.path(), &["rename", "velha", "antiga"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/archived/done/antiga.md"),
        &aviso_refs(1, 1),
    );
    assert!(!existe(home.path(), VELHA));
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/archived/done/antiga.md"),
        concat!(
            "> [!done] Velha\n",
            "> [[projects/alfa/tasks/archived/done/antiga|antiga]]\n",
            "\n",
            "veio antes de [[projects/alfa/tasks/x|x]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
            "- 2026-11-08 — renomeada de `velha` para `antiga`\n",
        )
    );
    assert_eq!(
        le(home.path(), NOTAS),
        NOTAS_MD.replace(
            "[[projects/alfa/tasks/archived/done/velha|velha]]",
            "[[projects/alfa/tasks/archived/done/antiga|antiga]]"
        )
    );
}

#[test]
fn rename_sem_frontmatter_nao_cria_frontmatter() {
    let home = home_base();
    let saida = roda(home.path(), &["rename", "x-2", "dois"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/dois.md"),
        &aviso_refs(2, 1),
    );
    assert_eq!(
        le(home.path(), "projects/alfa/tasks/dois.md"),
        concat!(
            "> [!example] Xis dois\n",
            "> [[projects/alfa/tasks/dois|dois]]\n",
            "> depois de [[projects/alfa/tasks/x|x]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-11-08 — renomeada de `x-2` para `dois`\n",
        )
    );
    assert_eq!(
        le(home.path(), NOTAS),
        NOTAS_MD.replace(
            "- vizinha: [[projects/alfa/tasks/x-2|x-2]] e [[x-2]]",
            "- vizinha: [[projects/alfa/tasks/dois|dois]] e [[dois]]"
        )
    );
}

#[test]
fn rename_aceita_id_que_existe_em_outro_projeto() {
    let home = home_base();
    let y_de_beta = "> [!todo] Ypsilon\n> [[projects/beta/tasks/y|y]]\n";
    arquivo(home.path(), "projects/beta/tasks/y.md", y_de_beta);
    envelhece(home.path());
    let saida = roda(home.path(), &["rename", "x", "y"]);
    sucesso(
        &saida,
        &caminho(home.path(), "projects/alfa/tasks/y.md"),
        &aviso_refs(16, 5),
    );
    assert_eq!(le(home.path(), "projects/beta/tasks/y.md"), y_de_beta);
}

#[test]
fn rename_ida_e_volta_devolve_os_arquivos_originais_menos_o_historico() {
    // I5 por exemplo.
    let home = home_base();
    let antes = bytes_de(&fotografa(home.path()));
    ok_ignorando_aviso(&roda(home.path(), &["rename", "x", "novo"]));
    let saida = roda(home.path(), &["rename", "novo", "x"]);
    sucesso(&saida, &caminho(home.path(), X), &aviso_refs(16, 5));
    let mut depois = bytes_de(&fotografa(home.path()));
    assert_eq!(
        String::from_utf8(depois.remove(X).expect("x voltou")).expect("utf-8"),
        format!(
            "{X_MD}\n## Histórico\n\n- 2026-11-08 — renomeada de `x` para `novo`\n- 2026-11-08 — renomeada de `novo` para `x`\n"
        )
    );
    assert_eq!(
        String::from_utf8(depois.remove(CURRENT).expect("painel")).expect("utf-8"),
        painel_base()
    );
    let mut antes = antes;
    antes.remove(X);
    assert_eq!(
        depois, antes,
        "fora a task e o painel, tudo volta byte a byte"
    );
    assert!(!existe(home.path(), NOVO));
}

#[test]
fn rename_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename"]),
        2,
        "falta o id ou caminho da task",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_sem_novo_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(&roda(home.path(), &["rename", "x"]), 2, "falta o novo id");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_para_o_mesmo_id_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "x", "x"]),
        1,
        "a task já se chama x",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_recusa_id_fora_de_kebab_sem_corrigir_em_silencio() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "x", "Novo_Id"]),
        1,
        "id precisa ser kebab-case: \"Novo_Id\" (seria \"novo-id\")",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_recusa_id_ocupado_por_task_ativa() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "x", "x-2"]),
        1,
        &format!("já existe uma task com esse id: {}", abs(home.path(), X2)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_recusa_id_ocupado_por_task_arquivada() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "x", "velha"]),
        1,
        &format!(
            "já existe uma task com esse id: {}",
            abs(home.path(), VELHA)
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_id_inexistente_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "zzz", "novo"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_nao_acha_task_de_projeto_encerrado() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "z", "novo"]),
        1,
        "task não encontrada: z",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_recusa_arquivo_que_nao_e_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["rename", "ObsidianVault/projects/alfa/notas.md", "novo"],
        ),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), NOTAS)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_recusa_task_sem_callout() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/alfa/tasks/sem.md",
        "texto sem callout\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["rename", "sem", "com"]),
        1,
        &format!(
            "task sem callout (rode `freitask doctor`): {}",
            abs(home.path(), "projects/alfa/tasks/sem.md")
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// move
// ===========================================================================

const X_MOVIDA_MD: &str = concat!(
    "---\n",
    "aliases: []\n",
    "id: x\n",
    "tags:\n",
    "  - foo\n",
    "---\n",
    "\n",
    "> [!todo] Xis\n",
    "> [[projects/beta/tasks/x|x]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "\n",
    "## Notas Soltas\n",
    "\n",
    "- [ ] item\n",
    "\n",
    "## Histórico\n",
    "\n",
    "- 2026-11-08 — movida de `alfa` para `beta`\n",
);

const NOTAS_COM_X_MOVIDA: &str = concat!(
    "# Notas\n",
    "\n",
    "- caminho: [[projects/beta/tasks/x|x]]\n",
    "- sem alias: [[projects/beta/tasks/x]]\n",
    "- alias livre: [[projects/beta/tasks/x|o xis]]\n",
    "- heading: [[projects/beta/tasks/x#Notas Soltas|seção]]\n",
    "- bloco: [[projects/beta/tasks/x^abc123]]\n",
    "- embed: ![[projects/beta/tasks/x]]\n",
    "- curtos: [[x]], [[x|x]], [[x|Xis]], [[x#Notas Soltas]] e ![[x]]\n",
    "- vizinha: [[projects/alfa/tasks/x-2|x-2]] e [[x-2]]\n",
    "- pendurado: [[projects/alfa/tasks/fantasma|fantasma]]\n",
    "- homônimo: [[projects/beta/tasks/x|x]]\n",
    "- dois na linha: [[projects/beta/tasks/x|x]] e [[projects/alfa/tasks/archived/done/velha|velha]]\n",
);

#[test]
fn move_leva_a_task_para_o_projeto_e_registra_no_historico() {
    let home = home_base();
    let saida = roda(home.path(), &["move", "x", "beta"]);
    sucesso(&saida, &caminho(home.path(), X_BETA), &aviso_refs(11, 5));
    assert!(!existe(home.path(), X));
    assert_eq!(le(home.path(), X_BETA), X_MOVIDA_MD);
}

#[test]
fn move_reescreve_os_links_path_qualificados_e_deixa_os_curtos() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["move", "x", "beta"]));
    assert_eq!(le(home.path(), NOTAS), NOTAS_COM_X_MOVIDA);
    assert_eq!(
        le(home.path(), SOLTA),
        "solta: [[projects/beta/tasks/x|x]]\n"
    );
}

#[test]
fn move_so_grava_a_task_as_notas_que_a_citam_e_o_current() {
    let home = home_base();
    let antes = fotografa(home.path());
    ok_ignorando_aviso(&roda(home.path(), &["move", "x", "beta"]));
    assert_eq!(
        arquivos_mudados(&antes, &fotografa(home.path())),
        conjunto(&[X, X_BETA, X2, VELHA, Z, NOTAS, SOLTA, CURRENT])
    );
    assert_eq!(le(home.path(), DAILY), DAILY_MD);
    assert_eq!(le(home.path(), LOG), LOG_TXT);
}

#[test]
fn move_regenera_o_current() {
    let home = home_base();
    ok_ignorando_aviso(&roda(home.path(), &["move", "x", "beta"]));
    assert_eq!(
        le(home.path(), CURRENT),
        painel(
            2,
            2,
            concat!(
                "## alfa\n",
                "\n",
                "> [!example] Xis dois\n",
                "> [[projects/alfa/tasks/x-2|x-2]]\n",
                "> depois de [[projects/beta/tasks/x|x]]\n",
                "\n",
                "## beta\n",
                "\n",
                "> [!todo] Xis\n",
                "> [[projects/beta/tasks/x|x]]\n",
                "> _em curso_\n",
                "> nota um\n",
                "\n",
            )
        )
    );
}

#[test]
fn move_de_task_arquivada_vai_para_o_mesmo_tipo_no_destino() {
    let home = home_base();
    let saida = roda(home.path(), &["move", "velha", "beta"]);
    let destino = "projects/beta/tasks/archived/done/velha.md";
    sucesso(&saida, &caminho(home.path(), destino), &aviso_refs(1, 1));
    assert!(!existe(home.path(), VELHA));
    assert_eq!(
        le(home.path(), destino),
        concat!(
            "> [!done] Velha\n",
            "> [[projects/beta/tasks/archived/done/velha|velha]]\n",
            "\n",
            "veio antes de [[projects/alfa/tasks/x|x]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-10-01 — arquivada em `archived/done` (feito)\n",
            "- 2026-11-08 — movida de `alfa` para `beta`\n",
        )
    );
    assert_eq!(
        le(home.path(), NOTAS),
        NOTAS_MD.replace(
            "[[projects/alfa/tasks/archived/done/velha|velha]]",
            "[[projects/beta/tasks/archived/done/velha|velha]]"
        )
    );
}

#[test]
fn move_por_caminho_absoluto() {
    let home = home_base();
    let saida = roda(home.path(), &["move", &abs(home.path(), X), "beta"]);
    sucesso(&saida, &caminho(home.path(), X_BETA), &aviso_refs(11, 5));
    assert_eq!(le(home.path(), X_BETA), X_MOVIDA_MD);
}

#[test]
fn move_sem_id_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move"]),
        2,
        "falta o id ou caminho da task",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_sem_projeto_sai_2() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move", "x"]),
        2,
        "falta o projeto de destino",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_projeto_que_nao_existe_e_sugere_new_project() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move", "x", "gama"]),
        1,
        "o projeto \"gama\" não existe; crie-o antes com `freitask new-project gama`",
    );
    nada_mudou(&antes, &fotografa(home.path()));
    assert!(!existe(home.path(), "projects/gama"));
}

#[test]
fn move_recusa_projeto_encerrado_e_aponta_o_reabrir() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move", "x", "omega"]),
        1,
        "o projeto \"omega\" está encerrado; reabra-o com `freitask reabrir omega`",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_projeto_reservado() {
    let home = home_base();
    let antes = fotografa(home.path());
    for p in ["daily", "templates", "_encerrados"] {
        recusa(
            &roda(home.path(), &["move", "x", p]),
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_ponto_e_ponto_ponto_como_projeto() {
    // D2: `.` e `..` são reservados também no `move`.
    let home = home_base();
    let antes = fotografa(home.path());
    for p in [".", ".."] {
        recusa(
            &roda(home.path(), &["move", "x", p]),
            1,
            &format!("\"{p}\" é um diretório reservado, não um projeto"),
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_projeto_com_barra() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move", "x", "beta/tasks"]),
        1,
        "projeto não pode conter '/': beta/tasks",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_o_projeto_em_que_a_task_ja_esta() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move", "x", "alfa"]),
        1,
        "a task já está no projeto alfa",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_id_ocupado_no_destino_por_task_ativa() {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/beta/tasks/velha.md",
        "> [!todo] Velha de beta\n> [[projects/beta/tasks/velha|velha]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &[
                "move",
                "ObsidianVault/projects/alfa/tasks/archived/done/velha.md",
                "beta",
            ],
        ),
        1,
        &format!(
            "já existe uma task com esse id em beta: {}",
            abs(home.path(), "projects/beta/tasks/velha.md")
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_id_ocupado_no_destino_por_task_arquivada() {
    let home = home_base();
    let ocupada = "projects/beta/tasks/archived/failed/x.md";
    arquivo(
        home.path(),
        ocupada,
        "> [!todo] Xis antiga\n> [[projects/beta/tasks/archived/failed/x|x]]\n",
    );
    envelhece(home.path());
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["move", "ObsidianVault/projects/alfa/tasks/x.md", "beta"],
        ),
        1,
        &format!(
            "já existe uma task com esse id em beta: {}",
            abs(home.path(), ocupada)
        ),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_id_inexistente_sai_1() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["move", "zzz", "beta"]),
        1,
        "task não encontrada: zzz",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_arquivo_que_nao_e_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["move", "ObsidianVault/projects/alfa/notas.md", "beta"],
        ),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), NOTAS)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// D1: `[[…\|alias]]` em tabela acompanha o deslocamento
// ===========================================================================

const TABELA: &str = "projects/alfa/tabela.md";

const TABELA_MD: &str = r"# Tabela

| task | nota |
| --- | --- |
| [[projects/alfa/tasks/x\|x]] | caminho |
| [[projects/alfa/tasks/x\|o xis]] | alias livre |
| [[projects/alfa/tasks/x#Notas\|seção]] | heading |
| [[x\|x]] | curto |
| [[projects/alfa/tasks/x-2\|x-2]] | vizinha |
";

const XS_MD: &str = "> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n";
const X2S_MD: &str = "> [!todo] Xis dois\n> [[projects/alfa/tasks/x-2|x-2]]\n";

#[test]
fn link_em_tabela_acompanha_o_archive_o_move_e_o_rename() {
    let home = home_pequeno(&[(X, XS_MD), (X2, X2S_MD), (TABELA, TABELA_MD)]);

    let saida = roda(home.path(), &["archive", "x", "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(3, 1));
    assert_eq!(
        le(home.path(), TABELA),
        r"# Tabela

| task | nota |
| --- | --- |
| [[projects/alfa/tasks/archived/done/x\|x]] | caminho |
| [[projects/alfa/tasks/archived/done/x\|o xis]] | alias livre |
| [[projects/alfa/tasks/archived/done/x#Notas\|seção]] | heading |
| [[x\|x]] | curto |
| [[projects/alfa/tasks/x-2\|x-2]] | vizinha |
"
    );

    let saida = roda(home.path(), &["move", "x", "beta"]);
    let x_beta_done = "projects/beta/tasks/archived/done/x.md";
    sucesso(
        &saida,
        &caminho(home.path(), x_beta_done),
        &aviso_refs(3, 1),
    );
    assert_eq!(
        le(home.path(), TABELA),
        r"# Tabela

| task | nota |
| --- | --- |
| [[projects/beta/tasks/archived/done/x\|x]] | caminho |
| [[projects/beta/tasks/archived/done/x\|o xis]] | alias livre |
| [[projects/beta/tasks/archived/done/x#Notas\|seção]] | heading |
| [[x\|x]] | curto |
| [[projects/alfa/tasks/x-2\|x-2]] | vizinha |
"
    );

    let saida = roda(home.path(), &["rename", "x", "novo"]);
    let novo_beta_done = "projects/beta/tasks/archived/done/novo.md";
    sucesso(
        &saida,
        &caminho(home.path(), novo_beta_done),
        &aviso_refs(4, 1),
    );
    assert_eq!(
        le(home.path(), TABELA),
        r"# Tabela

| task | nota |
| --- | --- |
| [[projects/beta/tasks/archived/done/novo\|novo]] | caminho |
| [[projects/beta/tasks/archived/done/novo\|o xis]] | alias livre |
| [[projects/beta/tasks/archived/done/novo#Notas\|seção]] | heading |
| [[novo\|novo]] | curto |
| [[projects/alfa/tasks/x-2\|x-2]] | vizinha |
"
    );
    assert_eq!(
        le(home.path(), novo_beta_done),
        concat!(
            "> [!done] Xis\n",
            "> [[projects/beta/tasks/archived/done/novo|novo]]\n",
            "\n",
            "## Histórico\n",
            "\n",
            "- 2026-11-08 — arquivada em `archived/done` (feito)\n",
            "- 2026-11-08 — movida de `alfa` para `beta`\n",
            "- 2026-11-08 — renomeada de `x` para `novo`\n",
        )
    );
}

#[test]
fn link_em_tabela_acompanha_o_unarchive() {
    let tabela = r"| task |
| --- |
| [[projects/alfa/tasks/archived/done/x\|x]] |
";
    let x_done = "> [!done] Xis\n> [[projects/alfa/tasks/archived/done/x|x]]\n";
    let home = home_pequeno(&[(X_DONE, x_done), (TABELA, tabela)]);
    let saida = roda(home.path(), &["unarchive", "x"]);
    sucesso(&saida, &caminho(home.path(), X), &aviso_refs(1, 1));
    assert_eq!(
        le(home.path(), TABELA),
        r"| task |
| --- |
| [[projects/alfa/tasks/x\|x]] |
"
    );
}

// ===========================================================================
// D5: link dentro de bloco cercado e de code span não é reescrito
// ===========================================================================

const CODIGO: &str = "projects/alfa/codigo.md";

const CODIGO_MD: &str = concat!(
    "# Código\n",
    "\n",
    "fora: [[projects/alfa/tasks/x|x]] e no span: `[[projects/alfa/tasks/x|x]]`\n",
    "\n",
    "```\n",
    "[[projects/alfa/tasks/x|x]]\n",
    "[[x]]\n",
    "```\n",
    "\n",
    "curto fora: [[x]] e no span: `[[x]]`\n",
);

#[test]
fn link_em_codigo_nao_e_reescrito_pelo_archive() {
    let home = home_pequeno(&[(X, XS_MD), (CODIGO, CODIGO_MD)]);
    let saida = roda(home.path(), &["archive", "x", "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(1, 1));
    assert_eq!(
        le(home.path(), CODIGO),
        CODIGO_MD.replacen(
            "fora: [[projects/alfa/tasks/x|x]]",
            "fora: [[projects/alfa/tasks/archived/done/x|x]]",
            1
        )
    );
}

#[test]
fn link_em_codigo_nao_e_reescrito_pelo_rename() {
    let home = home_pequeno(&[(X, XS_MD), (CODIGO, CODIGO_MD)]);
    let saida = roda(home.path(), &["rename", "x", "novo"]);
    sucesso(&saida, &caminho(home.path(), NOVO), &aviso_refs(2, 1));
    assert_eq!(
        le(home.path(), CODIGO),
        concat!(
            "# Código\n",
            "\n",
            "fora: [[projects/alfa/tasks/novo|novo]] e no span: `[[projects/alfa/tasks/x|x]]`\n",
            "\n",
            "```\n",
            "[[projects/alfa/tasks/x|x]]\n",
            "[[x]]\n",
            "```\n",
            "\n",
            "curto fora: [[novo]] e no span: `[[x]]`\n",
        )
    );
}

#[test]
fn link_em_codigo_nao_e_reescrito_pelo_move() {
    let home = home_pequeno(&[(X, XS_MD), (CODIGO, CODIGO_MD)]);
    let saida = roda(home.path(), &["move", "x", "beta"]);
    sucesso(&saida, &caminho(home.path(), X_BETA), &aviso_refs(1, 1));
    assert_eq!(
        le(home.path(), CODIGO),
        CODIGO_MD.replacen(
            "fora: [[projects/alfa/tasks/x|x]]",
            "fora: [[projects/beta/tasks/x|x]]",
            1
        )
    );
}

#[test]
fn link_em_codigo_nao_e_reescrito_pelo_unarchive() {
    let codigo = concat!(
        "fora: [[projects/alfa/tasks/archived/done/x|x]]\n",
        "```\n",
        "[[projects/alfa/tasks/archived/done/x|x]]\n",
        "```\n",
        "span: `[[projects/alfa/tasks/archived/done/x|x]]`\n",
    );
    let x_done = "> [!done] Xis\n> [[projects/alfa/tasks/archived/done/x|x]]\n";
    let home = home_pequeno(&[(X_DONE, x_done), (CODIGO, codigo)]);
    let saida = roda(home.path(), &["unarchive", "x"]);
    sucesso(&saida, &caminho(home.path(), X), &aviso_refs(1, 1));
    assert_eq!(
        le(home.path(), CODIGO),
        concat!(
            "fora: [[projects/alfa/tasks/x|x]]\n",
            "```\n",
            "[[projects/alfa/tasks/archived/done/x|x]]\n",
            "```\n",
            "span: `[[projects/alfa/tasks/archived/done/x|x]]`\n",
        )
    );
}

// ===========================================================================
// I7: `*.sync-conflict-*` nunca tem link reescrito nem é tratado como task
// ===========================================================================

const NOTA: &str = "projects/alfa/nota.md";
const NOTA_CONFLITO: &str = "projects/alfa/nota.sync-conflict-20261101-120000-ABCDEFG.md";
const X_CONFLITO: &str = "projects/alfa/tasks/x.sync-conflict-20261101-120000-ABCDEFG.md";
const NOTA_COM_X: &str = "[[projects/alfa/tasks/x|x]] e [[x]]\n";

/// `x`, uma nota que a cita, a cópia de conflito da nota e a cópia de
/// conflito da própria task, ao lado dela em `tasks/`.
fn home_com_conflitos() -> tempfile::TempDir {
    home_pequeno(&[
        (X, XS_MD),
        (NOTA, NOTA_COM_X),
        (NOTA_CONFLITO, NOTA_COM_X),
        (X_CONFLITO, XS_MD),
    ])
}

/// As duas cópias de conflito ficam onde estavam, byte a byte e com o mesmo
/// mtime.
fn conflitos_intactos(antes: &Foto, depois: &Foto) {
    for rel in [NOTA_CONFLITO, X_CONFLITO] {
        assert_eq!(
            antes.get(rel),
            depois.get(rel),
            "{rel} não podia ter mudado nem sumido"
        );
    }
}

#[test]
fn sync_conflict_nao_tem_link_reescrito_nem_sai_do_lugar_no_archive() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["archive", "x", "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(1, 1));
    conflitos_intactos(&antes, &fotografa(home.path()));
    assert_eq!(
        le(home.path(), NOTA),
        "[[projects/alfa/tasks/archived/done/x|x]] e [[x]]\n"
    );
}

#[test]
fn sync_conflict_nao_tem_link_reescrito_nem_sai_do_lugar_no_rename() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["rename", "x", "novo"]);
    sucesso(&saida, &caminho(home.path(), NOVO), &aviso_refs(2, 1));
    conflitos_intactos(&antes, &fotografa(home.path()));
    assert_eq!(
        le(home.path(), NOTA),
        "[[projects/alfa/tasks/novo|novo]] e [[novo]]\n"
    );
}

#[test]
fn sync_conflict_nao_tem_link_reescrito_nem_sai_do_lugar_no_move() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["move", "x", "beta"]);
    sucesso(&saida, &caminho(home.path(), X_BETA), &aviso_refs(1, 1));
    conflitos_intactos(&antes, &fotografa(home.path()));
    assert_eq!(
        le(home.path(), NOTA),
        "[[projects/beta/tasks/x|x]] e [[x]]\n"
    );
}

#[test]
fn sync_conflict_nao_tem_link_reescrito_no_unarchive() {
    let x_done = "> [!done] Xis\n> [[projects/alfa/tasks/archived/done/x|x]]\n";
    let cita = "[[projects/alfa/tasks/archived/done/x|x]]\n";
    let home = home_pequeno(&[(X_DONE, x_done), (NOTA, cita), (NOTA_CONFLITO, cita)]);
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["unarchive", "x"]);
    sucesso(&saida, &caminho(home.path(), X), &aviso_refs(1, 1));
    assert_eq!(
        antes.get(NOTA_CONFLITO),
        fotografa(home.path()).get(NOTA_CONFLITO)
    );
    assert_eq!(le(home.path(), NOTA), "[[projects/alfa/tasks/x|x]]\n");
}

#[test]
fn archive_recusa_sync_conflict() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    recusa_em_duas_linhas(
        &roda(
            home.path(),
            &["archive", &abs(home.path(), X_CONFLITO), "done"],
        ),
        &format!("não é um arquivo de task: {}", abs(home.path(), X_CONFLITO)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn unarchive_recusa_sync_conflict() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    recusa_em_duas_linhas(
        &roda(home.path(), &["unarchive", &abs(home.path(), X_CONFLITO)]),
        &format!("não é um arquivo de task: {}", abs(home.path(), X_CONFLITO)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_recusa_sync_conflict() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["rename", &abs(home.path(), X_CONFLITO), "novo"],
        ),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), X_CONFLITO)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_recusa_sync_conflict() {
    let home = home_com_conflitos();
    let antes = fotografa(home.path());
    recusa(
        &roda(
            home.path(),
            &["move", &abs(home.path(), X_CONFLITO), "beta"],
        ),
        1,
        &format!("não é um arquivo de task: {}", abs(home.path(), X_CONFLITO)),
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// D4: CRLF, BOM e falta de `\n` final sobrevivem, na task e nas notas
// ===========================================================================

/// Como um arquivo de texto termina suas linhas e começa.
#[derive(Clone, Copy)]
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

/// Ativa, em LF: frontmatter com `id:`, bloco e corpo.
const D4_ATIVA: &str = "---\nid: x\n---\n\n> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n\ncorpo\n";

/// Arquivada em `done`, em LF, com histórico.
const D4_ARQUIVADA: &str = "---\nid: x\n---\n\n> [!done] Xis\n> [[projects/alfa/tasks/archived/done/x|x]]\n\ncorpo\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n";

/// Monta o vault da D4: a task (em `rel`, no texto `lf`) e uma nota que a cita
/// por `alvo`, as duas na variante.
fn home_d4(v: Variante, rel: &str, lf: &str, alvo: &str) -> tempfile::TempDir {
    let home = home_pequeno(&[]);
    arquivo_bytes(home.path(), rel, &na_variante(v, lf));
    arquivo_bytes(
        home.path(),
        NOTA,
        &na_variante(v, &format!("# Nota\n\n[[{alvo}|x]] e [[x]]\n")),
    );
    envelhece(home.path());
    home
}

fn d4_archive(v: Variante) {
    let home = home_d4(v, X, D4_ATIVA, "projects/alfa/tasks/x");
    let saida = roda(home.path(), &["archive", "x", "done"]);
    sucesso(&saida, &caminho(home.path(), X_DONE), &aviso_refs(1, 1));
    assert_eq!(
        le_bytes(home.path(), X_DONE),
        na_variante(
            v,
            "---\nid: x\n---\n\n> [!done] Xis\n> [[projects/alfa/tasks/archived/done/x|x]]\n\ncorpo\n\n## Histórico\n\n- 2026-11-08 — arquivada em `archived/done` (feito)\n"
        )
    );
    assert_eq!(
        le_bytes(home.path(), NOTA),
        na_variante(
            v,
            "# Nota\n\n[[projects/alfa/tasks/archived/done/x|x]] e [[x]]\n"
        )
    );
}

fn d4_unarchive(v: Variante) {
    let home = home_d4(
        v,
        X_DONE,
        D4_ARQUIVADA,
        "projects/alfa/tasks/archived/done/x",
    );
    let saida = roda(home.path(), &["unarchive", "x"]);
    sucesso(&saida, &caminho(home.path(), X), &aviso_refs(1, 1));
    assert_eq!(
        le_bytes(home.path(), X),
        na_variante(
            v,
            "---\nid: x\n---\n\n> [!check] Xis\n> [[projects/alfa/tasks/x|x]]\n\ncorpo\n\n## Histórico\n\n- 2026-10-01 — arquivada em `archived/done` (feito)\n- 2026-11-08 — desarquivada, de volta ao board\n"
        )
    );
    assert_eq!(
        le_bytes(home.path(), NOTA),
        na_variante(v, "# Nota\n\n[[projects/alfa/tasks/x|x]] e [[x]]\n")
    );
}

fn d4_rename(v: Variante) {
    let home = home_d4(v, X, D4_ATIVA, "projects/alfa/tasks/x");
    let saida = roda(home.path(), &["rename", "x", "novo"]);
    sucesso(&saida, &caminho(home.path(), NOVO), &aviso_refs(2, 1));
    assert_eq!(
        le_bytes(home.path(), NOVO),
        na_variante(
            v,
            "---\nid: novo\n---\n\n> [!todo] Xis\n> [[projects/alfa/tasks/novo|novo]]\n\ncorpo\n\n## Histórico\n\n- 2026-11-08 — renomeada de `x` para `novo`\n"
        )
    );
    assert_eq!(
        le_bytes(home.path(), NOTA),
        na_variante(
            v,
            "# Nota\n\n[[projects/alfa/tasks/novo|novo]] e [[novo]]\n"
        )
    );
}

fn d4_move(v: Variante) {
    let home = home_d4(v, X, D4_ATIVA, "projects/alfa/tasks/x");
    let saida = roda(home.path(), &["move", "x", "beta"]);
    sucesso(&saida, &caminho(home.path(), X_BETA), &aviso_refs(1, 1));
    assert_eq!(
        le_bytes(home.path(), X_BETA),
        na_variante(
            v,
            "---\nid: x\n---\n\n> [!todo] Xis\n> [[projects/beta/tasks/x|x]]\n\ncorpo\n\n## Histórico\n\n- 2026-11-08 — movida de `alfa` para `beta`\n"
        )
    );
    assert_eq!(
        le_bytes(home.path(), NOTA),
        na_variante(v, "# Nota\n\n[[projects/beta/tasks/x|x]] e [[x]]\n")
    );
}

#[test]
fn crlf_sobrevive_ao_archive() {
    d4_archive(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_archive() {
    d4_archive(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_archive() {
    d4_archive(Variante::SemQuebraFinal);
}

#[test]
fn crlf_sobrevive_ao_unarchive() {
    d4_unarchive(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_unarchive() {
    d4_unarchive(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_unarchive() {
    d4_unarchive(Variante::SemQuebraFinal);
}

#[test]
fn crlf_sobrevive_ao_rename() {
    d4_rename(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_rename() {
    d4_rename(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_rename() {
    d4_rename(Variante::SemQuebraFinal);
}

#[test]
fn crlf_sobrevive_ao_move() {
    d4_move(Variante::Crlf);
}

#[test]
fn bom_sobrevive_ao_move() {
    d4_move(Variante::Bom);
}

#[test]
fn falta_de_quebra_final_sobrevive_ao_move() {
    d4_move(Variante::SemQuebraFinal);
}

// ===========================================================================
// Lock (spec §1.5, §2.4): os quatro comandos esperam até 5 s e saem 3
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
fn archive_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_base();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["archive", "x", "done"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn unarchive_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_base();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["unarchive", "velha"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_base();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["rename", "x", "novo"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_espera_o_lock_e_sai_3_sem_gravar() {
    let home = home_base();
    let antes = fotografa(home.path());
    let (saida, decorrido) = roda_com_lock_em_xdg(home.path(), &["move", "x", "beta"]);
    saiu_3_depois_de_esperar(&saida, decorrido);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn move_espera_o_lock_em_local_state_sem_xdg_e_sai_3_sem_gravar() {
    let home = home_base();
    let antes = fotografa(home.path());
    let _lock = Lock::segura(&lock_sem_xdg(home.path()), 60);
    let t0 = Instant::now();
    let saida = roda(home.path(), &["move", "x", "beta"]);
    saiu_3_depois_de_esperar(&saida, t0.elapsed());
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn rename_espera_o_lock_ser_liberado_e_grava() {
    let home = home_base();
    let xdg = tempfile::tempdir().expect("tempdir");
    // Solta sozinho depois de 2 s, dentro da janela de 5 s.
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 2);
    let t0 = Instant::now();
    let saida = roda_com_xdg(home.path(), xdg.path(), &["rename", "x", "novo"]);
    let decorrido = t0.elapsed();
    sucesso(&saida, &caminho(home.path(), NOVO), &aviso_refs(16, 5));
    assert!(
        decorrido >= Duration::from_millis(1500),
        "não esperou o lock: {decorrido:?}"
    );
    assert_eq!(le(home.path(), NOVO), NOVO_MD);
}
