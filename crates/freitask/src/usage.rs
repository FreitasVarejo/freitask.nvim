//! O texto de uso, byte a byte o de `lua/freitask/cli.lua` (spec §2.1): é o
//! que `freitask help` imprime e o que o diferencial compara. `show`,
//! `version` e `help --json` não entram nele de propósito: acrescentar linhas
//! quebraria a paridade do `help`; eles estão em `docs/freitask.md`, em
//! `freitask <sub> --help` e na gramática de `help --json`.

pub const USAGE: &str = "freitask — gestão de tasks do vault do Obsidian

  freitask            (sem argumentos, num terminal)
  freitask tui
      Abre a TUI (freitask-tui). Sem terminal — script, agente, pipe — cai
      neste usage com exit 2. Tratado pelo shim em bash, antes de chegar aqui.

  freitask current
      Abre o CURRENT.md no Neovim interativo (também tratado pelo shim).

  freitask doctor [--fix] [--json] [--quiet]
      Verifica a integridade do vault. Read-only por padrão; --fix repara o
      que é derivável do caminho (nunca inventa datas nem resolve conflitos).

  freitask archive   <id|caminho> <done|dropped|failed>
  freitask unarchive <id|caminho>
  freitask rename    <id|caminho> <novo-id>
      Movem/renomeiam mantendo os invariantes: wikilink da linha 2, `id:` do
      frontmatter, referências do vault inteiro e o log em `## Histórico`.
      `archive <id> done` também grava o callout `done`, e desarquivar uma
      task `done` a devolve como `check` (Pronta).

  freitask move <id|caminho> <projeto>
      Leva a task para projects/<projeto>/tasks/ — ou para o mesmo
      archived/<tipo>/ de lá, se estava arquivada: o tipo e a fase não mudam.
      Reescreve o wikilink da linha 2 e as referências path-qualificadas do
      vault, registra a mudança no `## Histórico` e deixa como estão os links
      curtos (só o id), que seguem valendo. Recusa projeto que não existe (crie
      com `freitask new-project`), projeto encerrado (reabra com `freitask
      reabrir`), projeto reservado, task que já está nele e id já ocupado lá
      (inclusive arquivado).

  freitask encerrar <projeto>
  freitask reabrir  <projeto>
      Encerrar leva a pasta projects/<projeto>/ inteira para
      projects/_encerrados/<projeto>/, que fica fora do painel, do list, do
      projects e do doctor; reabrir a traz de volta. Os dois reescrevem os
      wikilinks path-qualificados do vault que apontam para dentro dela (menos
      daily/ e CURRENT.md), deixam como estão os links curtos e não escrevem no
      `## Histórico` das tasks. Encerrar recusa projeto com task ativa em
      qualquer fase, inclusive Algum dia, e as lista; recusa também projeto
      que não existe, nome reservado (daily, templates, _encerrados) e nome já
      encerrado. Reabrir recusa o que não está encerrado e o nome que já
      existe em projects/. Um nome encerrado não volta por outra porta: new,
      new-project e move o recusam e apontam o reabrir.

  freitask new <projeto> <id> \"<título>\" [--desc \"<estado>\"]
      Cria uma task em projects/<projeto>/tasks/<id>.md e regenera o painel.
      O <id> é o nome do arquivo E o da branch git: precisa vir em kebab-case,
      e não é corrigido em silêncio. Recusa projeto reservado e id já existente
      (inclusive arquivado). Projeto que ainda não existe é criado; o
      encerrado não: reabra-o com `freitask reabrir`.

  freitask rebuild
      Regenera o CURRENT.md sem tocar em task nenhuma. É o que falta depois que
      um agente sem shell (o claude.ai pelo MCP) criou o arquivo na mão.

  freitask set <id|caminho> [--status <fase>] [--title \"<título>\"] [--desc \"<estado>\"]
                            [--volta-em AAAA-MM-DD]
      Troca a fase (o tipo do callout), o título e/ou a descrição em itálico,
      reserializando o bloco pelo mesmo código de sempre. `--desc \"\"` apaga a
      descrição. Recusa `done` (quem grava `done` é `archive <id> done`).
      `--volta-em` adia a task até o dia dado, que é o primeiro em que ela volta
      a aparecer (grava `volta-em:` no frontmatter; a fase não muda); `\"\"`
      limpa. Recusa o que não é uma data AAAA-MM-DD de um dia que existe, e não
      escreve nada, mesmo que outra flag fosse válida.

  freitask delete <id|caminho>
      Apaga a task, sem perguntar — quem confirma é quem chama. Referências a
      ela ficam penduradas de propósito: o `doctor` as aponta.

  freitask new-project <nome>
      Cria projects/<nome-em-kebab>/tasks/. Projeto já existente ou encerrado
      é erro.

  freitask kebab \"<texto>\"
      Imprime o id que o motor derivaria do texto (acentos dobrados,
      kebab-case). É como a TUI sugere o id de uma task nova sem ter uma
      segunda cópia da regra.

  freitask list [--json] [--archived]
      Lista as tasks, as de prioridade alta primeiro. O JSON traz, por task:
      project, id, path, archived, status (título da fase), status_num,
      callout, title, desc, suggested_archive, hidden (o que o painel e a TUI
      escondem por padrão: a fase Algum dia ou a task adiada, com `volta-em`
      num dia que vem; só task ativa), prioridade (\"alta\" ou null) e volta_em
      (o AAAA-MM-DD em que a adiada volta, ou null para quem não está adiada).
      A task adiada segue na lista.

  freitask projects [--json]
      Lista os projetos (inclusive os sem task ativa; os encerrados não) e
      quantas tasks ativas cada um tem, as que o painel mostra (as escondidas
      não contam).

  freitask statuses [--json]
      O vocabulário de fases do .freitask/status.json, em ordem, com ícone,
      grupo de highlight e hidden (a fase que o painel e a TUI escondem por
      padrão); o 0 é o sentinela \"Sem status\".

NUNCA mova nem renomeie um arquivo de task com `mv`: as referências do vault
ficam penduradas e o histórico não é escrito. E prefira `new` a escrever o
markdown na mão: o bloco é serializado pelo mesmo código de todo o resto.
";

/// Texto dos subcomandos que o usage de hoje não lista, para `<sub> --help`.
const EXTRAS: &[(&str, &str)] = &[
    (
        "show",
        "  freitask show <id|caminho> [--json]
      Mostra o bloco da task e, embaixo, `bloqueada por: a, b` e `bloqueia: N
      (x, y)` quando houver dependências, e `prazo: AAAA-MM-DD (em N dias)` ou
      `(vencido há N dias)` quando houver prazo. Com ciclo de dependências,
      imprime `CICLO: a → b → a` em stderr e sai 1. `--json` devolve um objeto
      com os campos de uma linha do `list --json` mais `ciclo` (array ou null).
",
    ),
    (
        "version",
        "  freitask version [--json]
      Imprime `freitask <versão>`; com --json, `{\"freitask\": \"<versão>\", \"json\": 1}`.
",
    ),
    (
        "help",
        "  freitask help [--json]
      Este usage. Com --json, a gramática dos subcomandos (quais escrevem,
      posicionais, flags com e sem valor), gerada das definições da CLI.
",
    ),
];

/// O trecho do usage de um subcomando: do seu `  freitask <sub>` (e dos
/// irmãos listados no mesmo bloco) até a linha em branco seguinte.
pub fn trecho(sub: &str) -> String {
    if let Some((_, t)) = EXTRAS.iter().find(|(s, _)| *s == sub) {
        return (*t).to_string();
    }
    let linhas: Vec<&str> = USAGE.lines().collect();
    let prefixo = format!("  freitask {sub}");
    let casa = |l: &str| l == prefixo || l.starts_with(&format!("{prefixo} "));
    let Some(pos) = linhas.iter().position(|l| casa(l)) else {
        return USAGE.to_string();
    };
    // Volta até o começo do bloco (as linhas `  freitask …` coladas acima).
    let mut ini = pos;
    while ini > 0 && linhas[ini - 1].starts_with("  freitask ") {
        ini -= 1;
    }
    let mut fim = pos;
    while fim + 1 < linhas.len() && !linhas[fim + 1].is_empty() {
        fim += 1;
    }
    let mut out = linhas[ini..=fim].join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn trecho_pega_o_bloco_do_subcomando_com_os_irmaos() {
        let t = trecho("unarchive");
        assert!(t.starts_with("  freitask archive   <id|caminho>"));
        assert!(t.contains("  freitask rename"));
        assert!(t.ends_with("(Pronta).\n"));
        assert!(!t.contains("freitask move"));
    }

    #[test]
    fn trecho_de_set_inclui_a_segunda_linha_de_flags() {
        let t = trecho("set");
        assert!(t.contains("[--volta-em AAAA-MM-DD]"));
        assert!(t.contains("escreve nada, mesmo que outra flag fosse válida.\n"));
    }

    #[test]
    fn trecho_dos_extras_e_do_desconhecido() {
        assert!(trecho("version").contains("freitask version"));
        assert_eq!(trecho("nada"), USAGE);
    }

    #[test]
    fn usage_termina_com_quebra_de_linha_e_nao_lista_os_comandos_novos() {
        assert!(USAGE.ends_with("todo o resto.\n"));
        assert!(!USAGE.contains("freitask show"));
    }
}
