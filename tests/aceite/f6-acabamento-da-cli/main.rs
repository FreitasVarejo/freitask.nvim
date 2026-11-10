//! Aceite da fatia 6 — Acabamento da CLI (spec §5): `--help`/`-h` por
//! subcomando (D8), `new --json` e `help --json` (D10, com os schemas
//! `schema/new.json` e `schema/help.json`), saída 3 no `new --json`, o texto do
//! usage idêntico ao de hoje e `freitask` sem argumentos fora de terminal
//! saindo 2.
//!
//! Fontes: `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`
//! (§1.5, §2.1, §2.2, §2.3, §2.4, §3.2 I6, §3.3, §3.4 D8/D10, §3.7),
//! `docs/freitask.md` ("CLI", "Para agentes de IA") e
//! `docs/decisoes-da-reescrita.md` (15, 16, 19, 24, 29, 31). Nenhum valor
//! esperado aqui foi gerado do binário sob teste.
//!
//! De onde vem cada texto:
//! - o usage (`CABECALHO`, os `TRECHO_*`, `RODAPE`): medido no oráculo
//!   (`nvim --clean -l lua/freitask/cli.lua help` num HOME temporário), 5483
//!   bytes de stdout: o `USAGE` do `cli.lua` mais o `\n` do `say()`. O mesmo
//!   texto sai em `freitask` sem argumentos (saída 2), `--help` e `-h` no
//!   topo (saída 0), e depois de `freitask: comando desconhecido: <x>\n\n` em
//!   stderr (saída 2). A decisão 15 manda o usage ficar byte a byte o de hoje,
//!   sem `show`, `version` nem `help --json`;
//! - a mensagem `já existe uma task com esse id: …`, a de reservado e
//!   `falta o título`: as do oráculo, já fixadas na fatia 3;
//! - o formato de `new --json` e de `help --json`: spec §2.2.
//!
//! Decisões tomadas aqui, onde a spec, o doc, o log e o oráculo não fecham
//! (repetidas no relatório do rascunho):
//! - "o trecho do usage do subcomando" (§2.1, D8) é o parágrafo do usage de
//!   hoje que o documenta, recortado entre as linhas em branco, cada linha com
//!   o seu `\n` e sem linha em branco no fim. Quem divide parágrafo divide o
//!   trecho: `archive`, `unarchive` e `rename` imprimem os três; `encerrar` e
//!   `reabrir`, os dois; `tui`, o parágrafo que começa no `freitask` sem
//!   argumentos. O texto é o de hoje mesmo onde envelheceu ("tratado pelo
//!   shim", o `new` sem `[--json]`, o `set` sem `--prazo`): é o usage que a
//!   decisão 15 congela;
//! - `show`, `version` e `help` não estão no usage de hoje. Do trecho deles só
//!   se fixa a primeira linha, a assinatura da §2.1 com o recuo de dois
//!   espaços do usage (`  freitask show <id|caminho> [--json]`, `  freitask
//!   version [--json]`, `  freitask help [--json]`), e o formato do resto: ao
//!   menos uma linha de descrição, todas com seis espaços de recuo, sem linha
//!   em branco. A prosa não tem fonte; inventá-la aqui obrigaria o binário a
//!   copiar um texto que ninguém aprovou;
//! - `--help`/`-h` valem em qualquer posição depois do subcomando e vencem os
//!   outros argumentos: `delete y --help`, `doctor --fix --help`,
//!   `new beta nova Nova --help` imprimem o trecho e não executam (o clap
//!   resolve antes de abrir o vault, §2.1). Também não esperam o lock;
//! - `freitask --help` e `freitask -h` sem subcomando imprimem o usage
//!   inteiro e saem 0, como hoje (o clap não pode trocá-lo pelo dele);
//! - `freitask tui` fora de terminal faz o mesmo que `freitask` sem
//!   argumentos: o usage em stdout, stderr vazio, saída 2 (§2.4 e o texto do
//!   usage: "Sem terminal — script, agente, pipe — cai neste usage com exit
//!   2"). O oráculo não tem o caso (o shim chama o `freitask-tui`);
//! - `xyz --help` com `xyz` desconhecido continua sendo "comando
//!   desconhecido", com o usage em stderr e saída 2, como hoje;
//! - `current --help` e `tui --help` rodam com um `PATH` que só tem um `nvim`
//!   falso e com prazo de 10 s, para provar que não abrem editor nem tela;
//! - `help --json`: a lista de subcomandos é exatamente a da §2.1 (21, com
//!   `tui` e `current`, que são subcomandos do clap). `writes` é `true` para
//!   quem escreve (§1.5: `new`, `set`, `archive`, `unarchive`, `rename`,
//!   `move`, `encerrar`, `reabrir`, `delete`, `new-project`, `rebuild`) e
//!   `false` para as leituras puras (`show`, `list`, `projects`, `statuses`,
//!   `kebab`, `version`, `help`); o de `doctor` (escreve só com `--fix`),
//!   `tui` e `current` não se fixa. As flags vão sem `--` e se comparam como
//!   conjunto (a ordem não é contrato). `value_flags`: as seis do `set` da
//!   §2.2, `desc` no `new`, e nenhuma nos outros. `bool_flags`: as do usage e
//!   da §2.1 (`json`, `archived`, `fix`, `quiet`, e `json` no `new`, `show`,
//!   `version` e `help`); `help` pode ou não constar, porque a §2.1 dá
//!   `--help` a todo subcomando e a spec não diz se ele entra na lista.
//!   `positionals`: o número é o do usage; o primeiro de quem recebe
//!   `<id|caminho>` se chama `ref`, como no exemplo da §2.2; os outros nomes
//!   não se fixam. O schema é fechado (`additionalProperties: false`), como
//!   os das fatias anteriores: campo novo é mudança de schema;
//! - `new --json`: o `path` é absoluto, o mesmo que o `new` imprime sem
//!   `--json`; `project` e `id` como gravados; uma linha com `\n`. Recusado,
//!   sai com a mensagem e o código de hoje e stdout vazio (nada de JSON). Com
//!   o lock ocupado, sai 3 depois de uns 5 s, sem JSON e sem gravar;
//! - saída 3 por conflito de CAS não se provoca de fora sem corrida; fica com
//!   a I9 (§3.2), no crate `vault`. A saída 3 por lock dos outros comandos de
//!   escrita já está nas fatias 3 a 5;
//! - flag desconhecida: só no `doctor`, em que hoje o oráculo a ignora e roda;
//!   a mensagem é a do clap (decisão 19), e o contrato é a saída 2, stdout
//!   vazio, a flag citada em stderr e nada gravado;
//! - os testes da §3.7 desta fatia (`help_por_subcomando_nao_executa`,
//!   `new_json_devolve_caminho_projeto_e_id`) moram aqui, pela decisão 24.
//!
//! Contra o binário de hoje (fim da fatia 5) falham os 7 de `new --json`
//! que precisam do comando (ele sai 2, "ainda não foi portado") e o
//! `tui_fora_de_terminal_sai_2_com_o_usage_byte_a_byte` (sai 2, mas sem o
//! usage em stdout). Já passam, e por quê:
//! - `--help`/`-h` por subcomando (os 21 `*_help_*`, os `*_com_help_*`,
//!   `help_por_subcomando_*`, `help_de_subcomando_que_escreve_nao_espera_o_lock`)
//!   e os `help_json_*`: o binário já os tem desde as fatias 1 e 2 (decisões
//!   15, 16 e 31 do log citam o `usage::EXTRAS`, a checagem de `--help` no
//!   `kebab` e a gramática do `help --json`). São contrato da fatia 6 (§5) e
//!   ficam congelados aqui; nenhum aceite anterior os fixava byte a byte;
//! - o usage byte a byte (`help_imprime_o_usage_de_hoje_byte_a_byte`,
//!   `help_e_h_sem_subcomando_*`, `sem_argumentos_*`, `comando_desconhecido_*`):
//!   a fatia 2 portou o texto; o aceite dela só conferia a primeira linha e
//!   o começo da mensagem;
//! - `flag_desconhecida_no_doctor_*`: o clap já recusa (decisão 19);
//! - `new_json_sem_titulo_sai_2_sem_json`: a checagem de argumento vem antes
//!   da recusa do `--json` não portado (decisão 29);
//! - `schema_help_recusa_forma_errada`, `schema_new_recusa_forma_errada`: só
//!   conferem o schema, sem binário;
//! - `schema_help_valida_a_saida_de_help_json`: o `help --json` já existe.
mod util;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use util::*;

// ===========================================================================
// O usage de hoje, byte a byte (medido no oráculo: `freitask help`)
// ===========================================================================
//
// Cada `TRECHO_*` é um parágrafo do usage do oráculo (`USAGE` em
// `lua/freitask/cli.lua`), recortado entre as linhas em branco, com o `\n` de
// cada linha e sem a linha em branco que o separa do seguinte. `usage()` os
// junta de volta na ordem do oráculo; `freitask help` imprime `usage()` e mais
// um `\n` (o `say()` do Lua soma um `\n` ao texto, que já termina em `\n`).

const CABECALHO: &str = "freitask — gestão de tasks do vault do Obsidian\n";

const TRECHO_TUI: &str = concat!(
    "  freitask            (sem argumentos, num terminal)\n",
    "  freitask tui\n",
    "      Abre a TUI (freitask-tui). Sem terminal — script, agente, pipe — cai\n",
    "      neste usage com exit 2. Tratado pelo shim em bash, antes de chegar aqui.\n",
);

const TRECHO_CURRENT: &str = concat!(
    "  freitask current\n",
    "      Abre o CURRENT.md no Neovim interativo (também tratado pelo shim).\n",
);

const TRECHO_DOCTOR: &str = concat!(
    "  freitask doctor [--fix] [--json] [--quiet]\n",
    "      Verifica a integridade do vault. Read-only por padrão; --fix repara o\n",
    "      que é derivável do caminho (nunca inventa datas nem resolve conflitos).\n",
);

const TRECHO_ARCHIVE: &str = concat!(
    "  freitask archive   <id|caminho> <done|dropped|failed>\n",
    "  freitask unarchive <id|caminho>\n",
    "  freitask rename    <id|caminho> <novo-id>\n",
    "      Movem/renomeiam mantendo os invariantes: wikilink da linha 2, `id:` do\n",
    "      frontmatter, referências do vault inteiro e o log em `## Histórico`.\n",
    "      `archive <id> done` também grava o callout `done`, e desarquivar uma\n",
    "      task `done` a devolve como `check` (Pronta).\n",
);

const TRECHO_MOVE: &str = concat!(
    "  freitask move <id|caminho> <projeto>\n",
    "      Leva a task para projects/<projeto>/tasks/ — ou para o mesmo\n",
    "      archived/<tipo>/ de lá, se estava arquivada: o tipo e a fase não mudam.\n",
    "      Reescreve o wikilink da linha 2 e as referências path-qualificadas do\n",
    "      vault, registra a mudança no `## Histórico` e deixa como estão os links\n",
    "      curtos (só o id), que seguem valendo. Recusa projeto que não existe (crie\n",
    "      com `freitask new-project`), projeto encerrado (reabra com `freitask\n",
    "      reabrir`), projeto reservado, task que já está nele e id já ocupado lá\n",
    "      (inclusive arquivado).\n",
);

const TRECHO_ENCERRAR: &str = concat!(
    "  freitask encerrar <projeto>\n",
    "  freitask reabrir  <projeto>\n",
    "      Encerrar leva a pasta projects/<projeto>/ inteira para\n",
    "      projects/_encerrados/<projeto>/, que fica fora do painel, do list, do\n",
    "      projects e do doctor; reabrir a traz de volta. Os dois reescrevem os\n",
    "      wikilinks path-qualificados do vault que apontam para dentro dela (menos\n",
    "      daily/ e CURRENT.md), deixam como estão os links curtos e não escrevem no\n",
    "      `## Histórico` das tasks. Encerrar recusa projeto com task ativa em\n",
    "      qualquer fase, inclusive Algum dia, e as lista; recusa também projeto\n",
    "      que não existe, nome reservado (daily, templates, _encerrados) e nome já\n",
    "      encerrado. Reabrir recusa o que não está encerrado e o nome que já\n",
    "      existe em projects/. Um nome encerrado não volta por outra porta: new,\n",
    "      new-project e move o recusam e apontam o reabrir.\n",
);

const TRECHO_NEW: &str = concat!(
    "  freitask new <projeto> <id> \"<título>\" [--desc \"<estado>\"]\n",
    "      Cria uma task em projects/<projeto>/tasks/<id>.md e regenera o painel.\n",
    "      O <id> é o nome do arquivo E o da branch git: precisa vir em kebab-case,\n",
    "      e não é corrigido em silêncio. Recusa projeto reservado e id já existente\n",
    "      (inclusive arquivado). Projeto que ainda não existe é criado; o\n",
    "      encerrado não: reabra-o com `freitask reabrir`.\n",
);

const TRECHO_REBUILD: &str = concat!(
    "  freitask rebuild\n",
    "      Regenera o CURRENT.md sem tocar em task nenhuma. É o que falta depois que\n",
    "      um agente sem shell (o claude.ai pelo MCP) criou o arquivo na mão.\n",
);

const TRECHO_SET: &str = concat!(
    "  freitask set <id|caminho> [--status <fase>] [--title \"<título>\"] [--desc \"<estado>\"]\n",
    "                            [--volta-em AAAA-MM-DD]\n",
    "      Troca a fase (o tipo do callout), o título e/ou a descrição em itálico,\n",
    "      reserializando o bloco pelo mesmo código de sempre. `--desc \"\"` apaga a\n",
    "      descrição. Recusa `done` (quem grava `done` é `archive <id> done`).\n",
    "      `--volta-em` adia a task até o dia dado, que é o primeiro em que ela volta\n",
    "      a aparecer (grava `volta-em:` no frontmatter; a fase não muda); `\"\"`\n",
    "      limpa. Recusa o que não é uma data AAAA-MM-DD de um dia que existe, e não\n",
    "      escreve nada, mesmo que outra flag fosse válida.\n",
);

const TRECHO_DELETE: &str = concat!(
    "  freitask delete <id|caminho>\n",
    "      Apaga a task, sem perguntar — quem confirma é quem chama. Referências a\n",
    "      ela ficam penduradas de propósito: o `doctor` as aponta.\n",
);

const TRECHO_NEW_PROJECT: &str = concat!(
    "  freitask new-project <nome>\n",
    "      Cria projects/<nome-em-kebab>/tasks/. Projeto já existente ou encerrado\n",
    "      é erro.\n",
);

const TRECHO_KEBAB: &str = concat!(
    "  freitask kebab \"<texto>\"\n",
    "      Imprime o id que o motor derivaria do texto (acentos dobrados,\n",
    "      kebab-case). É como a TUI sugere o id de uma task nova sem ter uma\n",
    "      segunda cópia da regra.\n",
);

const TRECHO_LIST: &str = concat!(
    "  freitask list [--json] [--archived]\n",
    "      Lista as tasks, as de prioridade alta primeiro. O JSON traz, por task:\n",
    "      project, id, path, archived, status (título da fase), status_num,\n",
    "      callout, title, desc, suggested_archive, hidden (o que o painel e a TUI\n",
    "      escondem por padrão: a fase Algum dia ou a task adiada, com `volta-em`\n",
    "      num dia que vem; só task ativa), prioridade (\"alta\" ou null) e volta_em\n",
    "      (o AAAA-MM-DD em que a adiada volta, ou null para quem não está adiada).\n",
    "      A task adiada segue na lista.\n",
);

const TRECHO_PROJECTS: &str = concat!(
    "  freitask projects [--json]\n",
    "      Lista os projetos (inclusive os sem task ativa; os encerrados não) e\n",
    "      quantas tasks ativas cada um tem, as que o painel mostra (as escondidas\n",
    "      não contam).\n",
);

const TRECHO_STATUSES: &str = concat!(
    "  freitask statuses [--json]\n",
    "      O vocabulário de fases do .freitask/status.json, em ordem, com ícone,\n",
    "      grupo de highlight e hidden (a fase que o painel e a TUI escondem por\n",
    "      padrão); o 0 é o sentinela \"Sem status\".\n",
);

const RODAPE: &str = concat!(
    "NUNCA mova nem renomeie um arquivo de task com `mv`: as referências do vault\n",
    "ficam penduradas e o histórico não é escrito. E prefira `new` a escrever o\n",
    "markdown na mão: o bloco é serializado pelo mesmo código de todo o resto.\n",
);

/// O texto inteiro do usage de hoje (sem o `\n` que o `say()` acrescenta).
fn usage() -> String {
    [
        CABECALHO,
        TRECHO_TUI,
        TRECHO_CURRENT,
        TRECHO_DOCTOR,
        TRECHO_ARCHIVE,
        TRECHO_MOVE,
        TRECHO_ENCERRAR,
        TRECHO_NEW,
        TRECHO_REBUILD,
        TRECHO_SET,
        TRECHO_DELETE,
        TRECHO_NEW_PROJECT,
        TRECHO_KEBAB,
        TRECHO_LIST,
        TRECHO_PROJECTS,
        TRECHO_STATUSES,
        RODAPE,
    ]
    .join("\n")
}

/// O stdout de `freitask help` (e de `freitask` sem argumentos fora de
/// terminal): o usage e uma linha em branco.
fn stdout_do_help() -> String {
    format!("{}\n", usage())
}

/// O trecho do usage que `freitask <sub> --help` imprime, para os
/// subcomandos que o usage de hoje documenta. `show`, `version` e `help` não
/// estão no usage de hoje (decisão 15 do log) e ficam em `assinatura_extra`.
fn trecho_de(sub: &str) -> &'static str {
    match sub {
        "tui" => TRECHO_TUI,
        "current" => TRECHO_CURRENT,
        "doctor" => TRECHO_DOCTOR,
        "archive" | "unarchive" | "rename" => TRECHO_ARCHIVE,
        "move" => TRECHO_MOVE,
        "encerrar" | "reabrir" => TRECHO_ENCERRAR,
        "new" => TRECHO_NEW,
        "rebuild" => TRECHO_REBUILD,
        "set" => TRECHO_SET,
        "delete" => TRECHO_DELETE,
        "new-project" => TRECHO_NEW_PROJECT,
        "kebab" => TRECHO_KEBAB,
        "list" => TRECHO_LIST,
        "projects" => TRECHO_PROJECTS,
        "statuses" => TRECHO_STATUSES,
        outro => panic!("sem trecho no usage de hoje: {outro}"),
    }
}

/// A primeira linha do trecho dos três subcomandos que não estão no usage de
/// hoje: a linha da spec (§2.1) com o recuo de dois espaços do usage.
fn assinatura_extra(sub: &str) -> &'static str {
    match sub {
        "show" => "  freitask show <id|caminho> [--json]\n",
        "version" => "  freitask version [--json]\n",
        "help" => "  freitask help [--json]\n",
        outro => panic!("não é extra: {outro}"),
    }
}

/// Os subcomandos da spec (§2.1), sem o `freitask` sem argumentos.
const SUBCOMANDOS: [&str; 21] = [
    "tui",
    "current",
    "new",
    "set",
    "show",
    "archive",
    "unarchive",
    "rename",
    "move",
    "encerrar",
    "reabrir",
    "delete",
    "new-project",
    "list",
    "projects",
    "statuses",
    "kebab",
    "rebuild",
    "doctor",
    "version",
    "help",
];

// ===========================================================================
// Fixtures
// ===========================================================================

const X: &str = "projects/alfa/tasks/x.md";
const Y: &str = "projects/alfa/tasks/y.md";
const VELHA: &str = "projects/alfa/tasks/archived/done/velha.md";
const NOTAS: &str = "projects/alfa/notas.md";
const CURRENT: &str = "CURRENT.md";

const X_MD: &str = "> [!todo] Xis\n> [[projects/alfa/tasks/x|x]]\n> _em curso_\n> nota um\n";
const Y_MD: &str = "> [!example] Ypsilon\n> [[projects/alfa/tasks/y|y]]\n";
const VELHA_MD: &str = "> [!done] Velha\n> [[projects/alfa/tasks/archived/done/velha|velha]]\n";
const NOTAS_MD: &str = "# Notas\n\nver [[projects/alfa/tasks/y|y]] e [[y]]\n";

/// `alfa` com `x` (todo) e `y` (example) ativas e `velha` em `archived/done/`,
/// mais uma nota que cita `y`; `beta` vazio. `status.json` default, sem
/// `CURRENT.md`, tudo envelhecido. O `doctor` acha aqui só um aviso,
/// `warn historico-ausente` na `velha` (arquivada sem a entrada de
/// arquivamento no `## Histórico`); nenhum teste depende disso.
fn home_base() -> tempfile::TempDir {
    let home = home_com(&[(X, X_MD), (Y, Y_MD), (VELHA, VELHA_MD), (NOTAS, NOTAS_MD)]);
    escreve_status_json(home.path(), gerador::STATUS_JSON);
    pasta(home.path(), "projects/beta/tasks");
    envelhece(home.path());
    home
}

/// `home_base` com dois achados do `doctor`: um link pendurado numa nota (sem
/// reparo) e o bloco de `y` com o link da linha 2 apontando para outro
/// projeto (`bloco-dessincronizado`, que o `--fix` repara).
fn home_com_achados() -> tempfile::TempDir {
    let home = home_base();
    arquivo(
        home.path(),
        "projects/alfa/solta.md",
        "ver [[projects/alfa/tasks/nao-existe|nao-existe]]\n",
    );
    arquivo(
        home.path(),
        Y,
        "> [!example] Ypsilon\n> [[projects/beta/tasks/y|y]]\n",
    );
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

/// Afirma uma ajuda: saída 0, stderr vazio e stdout exatamente o dado.
fn ajuda(saida: &Saida, stdout: &str, args: &[&str]) {
    assert_eq!(
        saida.codigo,
        Some(0),
        "freitask {args:?}: código; stderr: {:?}; stdout: {:?}",
        saida.stderr,
        saida.stdout
    );
    assert_eq!(saida.stderr, "", "freitask {args:?}: stderr");
    assert_eq!(saida.stdout, stdout, "freitask {args:?}: stdout");
}

/// Roda `<sub> --help` e `<sub> -h` no vault base e afirma, para os dois, a
/// saída 0, o trecho exato em stdout, stderr vazio e o vault intocado.
fn confere_trecho(sub: &str) {
    let home = home_base();
    let antes = fotografa(home.path());
    for flag in ["--help", "-h"] {
        let args = [sub, flag];
        let saida = roda_isolado(home.path(), &args);
        ajuda(&saida, trecho_de(sub), &args);
    }
    nada_mudou(&antes, &fotografa(home.path()));
    assert!(
        !home.path().join("nvim-chamado").exists(),
        "`{sub} --help` executou o nvim"
    );
}

/// Para `show`, `version` e `help`: saída 0, stderr vazio, a primeira linha
/// exata e o resto da descrição recuado em seis espaços, sem linha em branco.
fn confere_trecho_extra(sub: &str) {
    let home = home_base();
    let antes = fotografa(home.path());
    for flag in ["--help", "-h"] {
        let args = [sub, flag];
        let saida = roda_isolado(home.path(), &args);
        assert_eq!(
            saida.codigo,
            Some(0),
            "freitask {args:?}: {:?}",
            saida.stderr
        );
        assert_eq!(saida.stderr, "", "freitask {args:?}: stderr");
        let primeira = assinatura_extra(sub);
        assert!(
            saida.stdout.starts_with(primeira),
            "freitask {args:?}: a primeira linha tem de ser {primeira:?}; stdout:\n{}",
            saida.stdout
        );
        let resto = &saida.stdout[primeira.len()..];
        assert!(
            resto.ends_with('\n'),
            "freitask {args:?}: stdout termina em \\n"
        );
        let linhas: Vec<&str> = resto.trim_end_matches('\n').split('\n').collect();
        assert!(
            !resto.is_empty() && !linhas.is_empty(),
            "freitask {args:?}: falta a descrição depois da assinatura"
        );
        for l in &linhas {
            assert!(
                l.starts_with("      ") && !l.trim().is_empty(),
                "freitask {args:?}: linha de descrição fora do formato do usage: {l:?}"
            );
        }
        assert!(
            !resto.contains("\n\n"),
            "freitask {args:?}: o trecho não tem linha em branco"
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `freitask help`, `--help` no topo, sem argumentos, comando desconhecido
// ===========================================================================

#[test]
fn help_imprime_o_usage_de_hoje_byte_a_byte() {
    let home = home_base();
    let saida = roda(home.path(), &["help"]);
    ajuda(&saida, &stdout_do_help(), &["help"]);
}

#[test]
fn help_e_h_sem_subcomando_imprimem_o_usage_de_hoje_e_saem_0() {
    // Hoje `freitask --help` e `freitask -h` caem no mesmo ramo do `help`.
    let home = home_base();
    for flag in ["--help", "-h"] {
        let saida = roda(home.path(), &[flag]);
        ajuda(&saida, &stdout_do_help(), &[flag]);
    }
}

#[test]
fn sem_argumentos_fora_de_terminal_sai_2_com_o_usage_byte_a_byte() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda_isolado(home.path(), &[]);
    assert_eq!(saida.codigo, Some(2), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, stdout_do_help());
    assert_eq!(saida.stderr, "");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn tui_fora_de_terminal_sai_2_com_o_usage_byte_a_byte() {
    // §2.4 ("sem terminal para a TUI" é 2) e o próprio usage: "Sem terminal —
    // script, agente, pipe — cai neste usage com exit 2", dito do `freitask`
    // sem argumentos e do `freitask tui`.
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda_isolado(home.path(), &["tui"]);
    assert_eq!(saida.codigo, Some(2), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, stdout_do_help());
    assert_eq!(saida.stderr, "");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn comando_desconhecido_sai_2_com_o_usage_inteiro_em_stderr() {
    let home = home_base();
    let saida = roda(home.path(), &["xyz"]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(saida.stdout, "");
    assert_eq!(
        saida.stderr,
        format!("freitask: comando desconhecido: xyz\n\n{}\n", usage())
    );
}

#[test]
fn comando_desconhecido_com_help_continua_desconhecido() {
    // `--help` só é ajuda de um subcomando que existe; o oráculo diz
    // "comando desconhecido" para `xyz --help` como para `xyz`.
    let home = home_base();
    let saida = roda(home.path(), &["xyz", "--help"]);
    assert_eq!(saida.codigo, Some(2));
    assert_eq!(saida.stdout, "");
    assert_eq!(
        saida.stderr,
        format!("freitask: comando desconhecido: xyz\n\n{}\n", usage())
    );
}

// ===========================================================================
// D8: `<sub> --help` e `<sub> -h` imprimem o trecho do usage e saem 0
// ===========================================================================

#[test]
fn tui_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("tui");
}

#[test]
fn current_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("current");
}

#[test]
fn doctor_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("doctor");
}

#[test]
fn archive_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("archive");
}

#[test]
fn unarchive_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("unarchive");
}

#[test]
fn rename_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("rename");
}

#[test]
fn move_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("move");
}

#[test]
fn encerrar_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("encerrar");
}

#[test]
fn reabrir_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("reabrir");
}

#[test]
fn new_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("new");
}

#[test]
fn rebuild_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("rebuild");
}

#[test]
fn set_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("set");
}

#[test]
fn delete_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("delete");
}

#[test]
fn new_project_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("new-project");
}

#[test]
fn kebab_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("kebab");
}

#[test]
fn list_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("list");
}

#[test]
fn projects_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("projects");
}

#[test]
fn statuses_help_imprime_o_trecho_do_usage_e_sai_0() {
    confere_trecho("statuses");
}

#[test]
fn show_help_imprime_a_assinatura_e_a_descricao_e_sai_0() {
    confere_trecho_extra("show");
}

#[test]
fn version_help_imprime_a_assinatura_e_a_descricao_e_sai_0() {
    confere_trecho_extra("version");
}

#[test]
fn help_help_imprime_a_assinatura_e_a_descricao_e_sai_0() {
    confere_trecho_extra("help");
}

// ---------------------------------------------------------------------------
// `--help` não executa: nem lê, nem grava, nem espera o lock
// ---------------------------------------------------------------------------

#[test]
fn help_por_subcomando_nao_executa() {
    // §3.7 (D8): `doctor --help` num vault com achados sai 0 e não imprime
    // achado.
    let home = home_com_achados();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["doctor", "--help"]);
    ajuda(&saida, TRECHO_DOCTOR, &["doctor", "--help"]);
    assert!(!saida.stdout.contains("link-pendurado"));
    assert!(!saida.stdout.contains("achado(s)"));
    nada_mudou(&antes, &fotografa(home.path()));
    // O vault tem mesmo achados: o `doctor` sem `--help` sai 1.
    let doctor = roda(home.path(), &["doctor"]);
    assert_eq!(doctor.codigo, Some(1), "stdout: {}", doctor.stdout);
    assert!(doctor.stdout.contains("link-pendurado"));
}

#[test]
fn doctor_fix_com_help_nao_repara_nada() {
    let home = home_com_achados();
    let antes = fotografa(home.path());
    for args in [
        &["doctor", "--fix", "--help"][..],
        &["doctor", "--help", "--fix"][..],
        &["doctor", "--fix", "-h"][..],
    ] {
        let saida = roda(home.path(), args);
        ajuda(&saida, TRECHO_DOCTOR, args);
    }
    nada_mudou(&antes, &fotografa(home.path()));
    assert_eq!(
        le(home.path(), Y),
        "> [!example] Ypsilon\n> [[projects/beta/tasks/y|y]]\n"
    );
}

#[test]
fn delete_com_help_nao_apaga_a_task() {
    let home = home_base();
    let antes = fotografa(home.path());
    for args in [&["delete", "y", "--help"][..], &["delete", "y", "-h"][..]] {
        let saida = roda(home.path(), args);
        ajuda(&saida, TRECHO_DELETE, args);
    }
    nada_mudou(&antes, &fotografa(home.path()));
    assert_eq!(le(home.path(), Y), Y_MD);
}

#[test]
fn new_com_help_nao_cria_a_task_nem_o_painel() {
    let home = home_base();
    let antes = fotografa(home.path());
    let args = ["new", "beta", "nova", "Nova", "--help"];
    let saida = roda(home.path(), &args);
    ajuda(&saida, TRECHO_NEW, &args);
    nada_mudou(&antes, &fotografa(home.path()));
    assert!(!existe(home.path(), "projects/beta/tasks/nova.md"));
    assert!(!existe(home.path(), CURRENT));
}

#[test]
fn set_com_help_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    let args = ["set", "x", "--status", "example", "--help"];
    let saida = roda(home.path(), &args);
    ajuda(&saida, TRECHO_SET, &args);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn archive_com_help_nao_arquiva() {
    let home = home_base();
    let antes = fotografa(home.path());
    let args = ["archive", "x", "done", "-h"];
    let saida = roda(home.path(), &args);
    ajuda(&saida, TRECHO_ARCHIVE, &args);
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn current_com_help_nao_abre_o_nvim() {
    let home = home_base();
    for args in [&["current", "--help"][..], &["current", "-h"][..]] {
        let saida = roda_isolado(home.path(), args);
        ajuda(&saida, TRECHO_CURRENT, args);
    }
    assert!(
        !home.path().join("nvim-chamado").exists(),
        "o `current --help` executou o nvim"
    );
}

#[test]
fn help_de_subcomando_que_escreve_nao_espera_o_lock() {
    let home = home_base();
    let antes = fotografa(home.path());
    let xdg = tempfile::tempdir().expect("tempdir");
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 60);
    for (args, trecho) in [
        (&["rebuild", "--help"][..], TRECHO_REBUILD),
        (
            &["set", "x", "--status", "example", "--help"][..],
            TRECHO_SET,
        ),
        (&["new", "beta", "nova", "Nova", "-h"][..], TRECHO_NEW),
        (&["encerrar", "beta", "--help"][..], TRECHO_ENCERRAR),
        (&["doctor", "--fix", "--help"][..], TRECHO_DOCTOR),
    ] {
        let t0 = Instant::now();
        let saida = roda_com_xdg(home.path(), xdg.path(), args);
        let decorrido = t0.elapsed();
        ajuda(&saida, trecho, args);
        assert!(
            decorrido < Duration::from_secs(2),
            "freitask {args:?} esperou o lock: {decorrido:?}"
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn help_por_subcomando_num_vault_vazio_nao_cria_nada() {
    // I6: inclusive num HOME sem `.freitask/` e sem `projects/`.
    let home = home_com_vault_vazio();
    for sub in SUBCOMANDOS {
        let saida = roda_isolado(home.path(), &[sub, "--help"]);
        assert_eq!(
            saida.codigo,
            Some(0),
            "freitask {sub} --help: {:?}",
            saida.stderr
        );
        assert_eq!(saida.stderr, "", "freitask {sub} --help");
    }
    assert_eq!(entradas(&vault(home.path())), Vec::<String>::new());
}

#[test]
fn help_por_subcomando_num_home_sem_vault_nao_cria_o_vault() {
    let home = home_sem_vault();
    for sub in SUBCOMANDOS {
        let saida = roda_isolado(home.path(), &[sub, "-h"]);
        assert_eq!(
            saida.codigo,
            Some(0),
            "freitask {sub} -h: {:?}",
            saida.stderr
        );
    }
    assert!(!vault(home.path()).exists(), "ObsidianVault/ foi criado");
}

// ===========================================================================
// Flag desconhecida (§2.1): saída 2 antes de executar
// ===========================================================================

#[test]
fn flag_desconhecida_no_doctor_sai_2_sem_rodar_o_doctor() {
    // A mensagem é a do clap (decisão 19); o contrato aqui é o código, o
    // stdout vazio (nenhum achado impresso), a flag citada e nada gravado.
    let home = home_com_achados();
    let antes = fotografa(home.path());
    for args in [&["doctor", "--foo"][..], &["doctor", "--fix", "--foo"][..]] {
        let saida = roda(home.path(), args);
        assert_eq!(
            saida.codigo,
            Some(2),
            "freitask {args:?}: {:?}",
            saida.stderr
        );
        assert_eq!(saida.stdout, "", "freitask {args:?}");
        assert!(
            saida.stderr.contains("--foo"),
            "freitask {args:?}: stderr não cita a flag: {:?}",
            saida.stderr
        );
    }
    nada_mudou(&antes, &fotografa(home.path()));
}

// ===========================================================================
// `help --json` (§2.2, §2.3): a gramática
// ===========================================================================

fn gramatica() -> Value {
    let home = home_sem_vault();
    let saida = roda(home.path(), &["help", "--json"]);
    ok(&saida);
    assert!(saida.stdout.ends_with('\n'), "stdout: {:?}", saida.stdout);
    json(&saida)
}

/// O conjunto de strings de um array JSON.
fn conjunto_de(v: &Value, o_que: &str) -> BTreeSet<String> {
    v.as_array()
        .unwrap_or_else(|| panic!("{o_que} tem de ser array: {v}"))
        .iter()
        .map(|s| {
            s.as_str()
                .unwrap_or_else(|| panic!("{o_que}: item não é string: {s}"))
                .to_owned()
        })
        .collect()
}

#[test]
fn help_json_sai_0_com_json_1_e_os_subcomandos() {
    let g = gramatica();
    assert_eq!(g["json"], json!(1));
    assert!(
        g["subcommands"].is_object(),
        "subcommands: {}",
        g["subcommands"]
    );
}

#[test]
fn help_json_lista_exatamente_os_subcomandos_da_spec() {
    let g = gramatica();
    let nomes: BTreeSet<String> = g["subcommands"]
        .as_object()
        .expect("subcommands objeto")
        .keys()
        .cloned()
        .collect();
    let esperado: BTreeSet<String> = SUBCOMANDOS.iter().map(|s| s.to_string()).collect();
    assert_eq!(nomes, esperado);
}

#[test]
fn help_json_marca_quem_escreve_e_quem_so_le() {
    let g = gramatica();
    for sub in [
        "new",
        "set",
        "archive",
        "unarchive",
        "rename",
        "move",
        "encerrar",
        "reabrir",
        "delete",
        "new-project",
        "rebuild",
    ] {
        assert_eq!(g["subcommands"][sub]["writes"], json!(true), "{sub}");
    }
    for sub in [
        "show", "list", "projects", "statuses", "kebab", "version", "help",
    ] {
        assert_eq!(g["subcommands"][sub]["writes"], json!(false), "{sub}");
    }
}

#[test]
fn help_json_set_tem_ref_e_as_seis_flags_de_valor() {
    let g = gramatica();
    let set = &g["subcommands"]["set"];
    assert_eq!(set["positionals"], json!(["ref"]));
    assert_eq!(
        conjunto_de(&set["value_flags"], "set.value_flags"),
        conjunto(&[
            "status",
            "title",
            "desc",
            "volta-em",
            "prazo",
            "bloqueada-por"
        ])
    );
}

#[test]
fn help_json_so_set_e_new_tem_flag_de_valor() {
    let g = gramatica();
    for sub in SUBCOMANDOS {
        let esperado: &[&str] = match sub {
            "set" => &[
                "status",
                "title",
                "desc",
                "volta-em",
                "prazo",
                "bloqueada-por",
            ],
            "new" => &["desc"],
            _ => &[],
        };
        assert_eq!(
            conjunto_de(
                &g["subcommands"][sub]["value_flags"],
                &format!("{sub}.value_flags")
            ),
            conjunto(esperado),
            "{sub}"
        );
    }
}

#[test]
fn help_json_flags_booleanas_de_cada_subcomando() {
    // `help` (o `--help`/`-h` que todo subcomando tem, §2.1) pode ou não
    // constar; o resto é exato.
    let g = gramatica();
    for sub in SUBCOMANDOS {
        let esperado: &[&str] = match sub {
            "new" | "show" | "projects" | "statuses" | "version" | "help" => &["json"],
            "list" => &["json", "archived"],
            "doctor" => &["fix", "json", "quiet"],
            _ => &[],
        };
        let mut achado = conjunto_de(
            &g["subcommands"][sub]["bool_flags"],
            &format!("{sub}.bool_flags"),
        );
        achado.remove("help");
        assert_eq!(achado, conjunto(esperado), "{sub}");
    }
}

#[test]
fn help_json_posicionais_de_cada_subcomando() {
    // O número de posicionais é o do usage; o primeiro de quem recebe
    // `<id|caminho>` se chama `ref`, como no exemplo da spec (§2.2).
    let g = gramatica();
    for sub in SUBCOMANDOS {
        let (n, primeiro_ref) = match sub {
            "new" => (3, false),
            "archive" | "rename" | "move" => (2, true),
            "set" | "show" | "unarchive" | "delete" => (1, true),
            "encerrar" | "reabrir" | "new-project" | "kebab" => (1, false),
            _ => (0, false),
        };
        let pos = conjunto_de(&g["subcommands"][sub]["positionals"], sub);
        let lista = g["subcommands"][sub]["positionals"]
            .as_array()
            .expect("positionals array");
        assert_eq!(lista.len(), n, "{sub}: {lista:?}");
        assert_eq!(pos.len(), n, "{sub}: nomes repetidos em {lista:?}");
        if primeiro_ref {
            assert_eq!(lista[0], json!("ref"), "{sub}");
        }
    }
}

#[test]
fn help_json_nao_toca_o_vault() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["help", "--json"]);
    ok(&saida);
    json(&saida);
    nada_mudou(&antes, &fotografa(home.path()));
    let sem = home_sem_vault();
    ok(&roda(sem.path(), &["help", "--json"]));
    assert!(!vault(sem.path()).exists());
}

#[test]
fn help_json_nao_espera_o_lock() {
    let home = home_base();
    let xdg = tempfile::tempdir().expect("tempdir");
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 60);
    let t0 = Instant::now();
    let saida = roda_com_xdg(home.path(), xdg.path(), &["help", "--json"]);
    ok(&saida);
    assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    assert_eq!(json(&saida)["json"], json!(1));
}

#[test]
fn schema_help_valida_a_saida_de_help_json() {
    valida("help", &gramatica());
}

#[test]
fn schema_help_recusa_forma_errada() {
    let esquema = schema("help");
    let v = jsonschema::validator_for(&esquema).expect("schema válido");
    let certo = json!({"json": 1, "subcommands": {"set": {
        "writes": true, "positionals": ["ref"],
        "value_flags": ["status"], "bool_flags": []
    }}});
    assert!(v.is_valid(&certo));
    let mut sem_writes = certo.clone();
    sem_writes["subcommands"]["set"]
        .as_object_mut()
        .expect("objeto")
        .remove("writes");
    assert!(!v.is_valid(&sem_writes), "sem `writes` tem de falhar");
    let mut a_mais = certo.clone();
    a_mais["extra"] = json!(1);
    assert!(!v.is_valid(&a_mais), "chave a mais na raiz tem de falhar");
    let mut tipo = certo.clone();
    tipo["subcommands"]["set"]["value_flags"] = json!("status");
    assert!(!v.is_valid(&tipo), "flag fora de array tem de falhar");
    let mut sem_json = certo;
    sem_json.as_object_mut().expect("objeto").remove("json");
    assert!(!v.is_valid(&sem_json), "sem `json` tem de falhar");
}

// ===========================================================================
// `new --json` (§2.2, D10)
// ===========================================================================

#[test]
fn new_json_devolve_caminho_projeto_e_id() {
    // §3.7 (D10).
    let home = home_base();
    let saida = roda(home.path(), &["new", "beta", "nova", "Nova task", "--json"]);
    ok(&saida);
    assert!(saida.stdout.ends_with('\n'), "stdout: {:?}", saida.stdout);
    assert_eq!(
        json(&saida),
        json!({
            "path": abs(home.path(), "projects/beta/tasks/nova.md"),
            "project": "beta",
            "id": "nova",
        })
    );
}

#[test]
fn new_json_grava_a_mesma_task_e_regenera_o_painel() {
    let home = home_base();
    let antes = fotografa(home.path());
    ok(&roda(
        home.path(),
        &[
            "new",
            "beta",
            "nova",
            "Nova task",
            "--desc",
            "começando",
            "--json",
        ],
    ));
    assert_eq!(
        le(home.path(), "projects/beta/tasks/nova.md"),
        "> [!todo] Nova task\n> [[projects/beta/tasks/nova|nova]]\n> _começando_\n\n"
    );
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
    assert_eq!(
        mudancas(&antes, &fotografa(home.path())),
        conjunto(&["projects/beta/tasks/nova.md", CURRENT])
    );
}

#[test]
fn new_json_aceita_a_flag_antes_dos_posicionais() {
    let home = home_base();
    let saida = roda(
        home.path(),
        &[
            "new",
            "--json",
            "beta",
            "nova",
            "Nova task",
            "--desc=começando",
        ],
    );
    ok(&saida);
    assert_eq!(
        json(&saida),
        json!({
            "path": abs(home.path(), "projects/beta/tasks/nova.md"),
            "project": "beta",
            "id": "nova",
        })
    );
    assert_eq!(
        le(home.path(), "projects/beta/tasks/nova.md"),
        "> [!todo] Nova task\n> [[projects/beta/tasks/nova|nova]]\n> _começando_\n\n"
    );
}

#[test]
fn new_json_em_projeto_que_nao_existe_cria_e_devolve_o_projeto() {
    let home = home_base();
    let saida = roda(
        home.path(),
        &["new", "gama", "primeira", "Primeira", "--json"],
    );
    ok(&saida);
    assert_eq!(
        json(&saida),
        json!({
            "path": abs(home.path(), "projects/gama/tasks/primeira.md"),
            "project": "gama",
            "id": "primeira",
        })
    );
    assert_eq!(
        le(home.path(), "projects/gama/tasks/primeira.md"),
        "> [!todo] Primeira\n> [[projects/gama/tasks/primeira|primeira]]\n\n"
    );
}

#[test]
fn new_json_recusado_sai_1_sem_json_e_nao_grava() {
    let home = home_base();
    let antes = fotografa(home.path());
    let saida = roda(home.path(), &["new", "alfa", "x", "Outra", "--json"]);
    recusa(
        &saida,
        1,
        &format!("já existe uma task com esse id: {}", abs(home.path(), X)),
    );
    let saida = roda(home.path(), &["new", "daily", "w", "W", "--json"]);
    recusa(
        &saida,
        1,
        "\"daily\" é um diretório reservado, não um projeto",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_json_sem_titulo_sai_2_sem_json() {
    let home = home_base();
    let antes = fotografa(home.path());
    recusa(
        &roda(home.path(), &["new", "beta", "nova", "--json"]),
        2,
        "falta o título",
    );
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn new_json_espera_o_lock_e_sai_3_sem_imprimir_json() {
    let home = home_base();
    let antes = fotografa(home.path());
    let xdg = tempfile::tempdir().expect("tempdir");
    let _lock = Lock::segura(&lock_em_xdg(xdg.path()), 60);
    let t0 = Instant::now();
    let saida = roda_com_xdg(
        home.path(),
        xdg.path(),
        &["new", "beta", "nova", "Nova", "--json"],
    );
    let decorrido = t0.elapsed();
    assert_eq!(saida.codigo, Some(3), "stderr: {:?}", saida.stderr);
    assert_eq!(saida.stdout, "", "nada de JSON quando não gravou");
    assert!(
        saida.stderr.starts_with("freitask: ") && saida.stderr.ends_with('\n'),
        "stderr: {:?}",
        saida.stderr
    );
    assert_eq!(saida.stderr.matches('\n').count(), 1, "{:?}", saida.stderr);
    assert!(
        decorrido >= Duration::from_millis(4500),
        "saiu cedo demais: {decorrido:?}"
    );
    assert!(decorrido <= Duration::from_secs(15), "{decorrido:?}");
    nada_mudou(&antes, &fotografa(home.path()));
}

#[test]
fn schema_new_valida_a_saida_de_new_json() {
    let home = home_base();
    let saida = roda(home.path(), &["new", "beta", "nova", "Nova", "--json"]);
    ok(&saida);
    valida("new", &json(&saida));
}

#[test]
fn schema_new_recusa_forma_errada() {
    let esquema = schema("new");
    let v = jsonschema::validator_for(&esquema).expect("schema válido");
    let certo = json!({
        "path": "/home/u/ObsidianVault/projects/p/tasks/t.md",
        "project": "p",
        "id": "t",
    });
    assert!(v.is_valid(&certo));
    for chave in ["path", "project", "id"] {
        let mut sem = certo.clone();
        sem.as_object_mut().expect("objeto").remove(chave);
        assert!(!v.is_valid(&sem), "sem `{chave}` tem de falhar");
    }
    let mut a_mais = certo.clone();
    a_mais["extra"] = json!(1);
    assert!(!v.is_valid(&a_mais), "chave a mais tem de falhar");
    let mut relativo = certo.clone();
    relativo["path"] = json!("projects/p/tasks/t.md");
    assert!(!v.is_valid(&relativo), "caminho relativo tem de falhar");
    let mut id_numero = certo;
    id_numero["id"] = json!(1);
    assert!(!v.is_valid(&id_numero), "`id` sempre string");
}
