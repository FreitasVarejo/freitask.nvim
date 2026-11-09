//! Operações sobre as LINHAS de um arquivo markdown: blockquote, frontmatter
//! YAML, seção `##`, splice. Nada aqui sabe o que é uma task.

use std::collections::BTreeMap;

use crate::date::Date;
use crate::trim;

/// Verdadeiro se a linha começa um blockquote (`^%s*>`).
pub fn eh_quote(line: &str) -> bool {
    line.trim_start_matches(crate::eh_espaco).starts_with('>')
}

fn so_espaco(line: &str) -> bool {
    line.chars().all(crate::eh_espaco)
}

/// Remove o prefixo de blockquote (`> ` ou `>`) de uma linha (`^%s*>%s?`).
pub fn strip_quote(line: &str) -> String {
    let s = line.trim_start_matches(crate::eh_espaco);
    match s.strip_prefix('>') {
        Some(resto) => {
            let mut chars = resto.chars();
            match chars.next() {
                Some(c) if crate::eh_espaco(c) => chars.as_str().to_string(),
                _ => resto.to_string(),
            }
        }
        None => line.to_string(),
    }
}

/// Substitui `lines[s..=e]` (0-based) por `repl`, devolvendo um array novo.
pub fn splice(lines: &[String], s: usize, e: usize, repl: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len() + repl.len());
    out.extend_from_slice(&lines[..s]);
    out.extend_from_slice(repl);
    if e < lines.len() {
        out.extend_from_slice(&lines[e + 1..]);
    }
    out
}

/// Índice (0-based) da primeira linha de conteúdo depois de um frontmatter
/// YAML opcional. 0 quando não há frontmatter.
pub fn content_start(lines: &[String]) -> usize {
    if lines.first().map(String::as_str) == Some("---") {
        for (i, l) in lines.iter().enumerate().skip(1) {
            if l == "---" {
                return i + 1;
            }
        }
    }
    0
}

/// Faixa `[start, finish]` (0-based, inclusiva) do primeiro bloco `>` do
/// arquivo, pulando frontmatter e linhas em branco. `None` sem callout.
pub fn first_block_range(lines: &[String]) -> Option<(usize, usize)> {
    let mut start = None;
    for (i, l) in lines.iter().enumerate().skip(content_start(lines)) {
        if eh_quote(l) {
            start = Some(i);
            break;
        } else if !so_espaco(l) {
            return None; // conteúdo não-quote antes de qualquer bloco
        }
    }
    let start = start?;
    let mut finish = start;
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        if eh_quote(l) {
            finish = i;
        } else {
            break;
        }
    }
    Some((start, finish))
}

/// Faixa e CONTEÚDO do primeiro bloco `>`.
pub fn first_block(lines: &[String]) -> Option<(usize, usize, Vec<String>)> {
    let (s, e) = first_block_range(lines)?;
    Some((s, e, lines[s..=e].to_vec()))
}

/// Faixa do bloco `>` que contém a linha `lnum` (0-based). `None` fora de quote.
pub fn block_around(lines: &[String], lnum: usize) -> Option<(usize, usize)> {
    if !lines.get(lnum).is_some_and(|l| eh_quote(l)) {
        return None;
    }
    let mut s = lnum;
    let mut e = lnum;
    while s > 0 && eh_quote(&lines[s - 1]) {
        s -= 1;
    }
    while e + 1 < lines.len() && eh_quote(&lines[e + 1]) {
        e += 1;
    }
    Some((s, e))
}

// --- Frontmatter -----------------------------------------------------------
//
// O parser é de propósito BURRO: só pares `chave: valor` no primeiro nível. É
// todo o YAML que este vault usa; um parser completo seria uma segunda
// implementação de YAML, com a garantia de divergir da do Obsidian nos casos
// raros. Chave que ele não entende ele PRESERVA.

/// Aspas só quando o valor não sobreviveria como escalar YAML cru.
pub fn yaml_scalar(v: &str) -> String {
    let primeiro = v.chars().next();
    let precisa = v.is_empty()
        || primeiro.is_some_and(|c| crate::eh_espaco(c) || "[]{}#&*!|>'\"%@`".contains(c))
        || v.chars().last().is_some_and(crate::eh_espaco)
        || v.contains(": ");
    if precisa {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

/// Tira as aspas de um escalar YAML, se houver.
pub fn yaml_unquote(v: &str) -> String {
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        return v[1..v.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
    }
    if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        return v[1..v.len() - 1].to_string();
    }
    v.to_string()
}

/// A chave de uma linha `chave: valor` do frontmatter (`^([%w_%-]+):`).
fn chave_da_linha(line: &str) -> Option<&str> {
    let i = line.find(':')?;
    let k = &line[..i];
    if k.is_empty()
        || !k
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return None;
    }
    Some(k)
}

/// Lê o frontmatter: mapa chave → valor e a faixa `(s, e)` (0-based, as linhas
/// dos dois `---`). Sem frontmatter, mapa vazio e `None`.
pub fn frontmatter(lines: &[String]) -> (BTreeMap<String, String>, Option<(usize, usize)>) {
    if lines.first().map(String::as_str) != Some("---") {
        return (BTreeMap::new(), None);
    }
    for (i, l) in lines.iter().enumerate().skip(1) {
        if l == "---" {
            let mut map = BTreeMap::new();
            for linha in &lines[1..i] {
                if let Some(k) = chave_da_linha(linha) {
                    let v = &linha[k.len() + 1..];
                    map.insert(k.to_string(), yaml_unquote(trim(v)));
                }
            }
            return (map, Some((0, i)));
        }
    }
    (BTreeMap::new(), None) // `---` de abertura sem fechamento: não é frontmatter
}

/// Aplica `updates` ao frontmatter: `Some(v)` grava, `None` REMOVE a chave.
/// Cria o bloco quando não existe (só se há algo a gravar) e o remove por
/// inteiro quando esvazia. Muta `lines`; devolve se mudou.
pub fn set_frontmatter(
    lines: &mut Vec<String>,
    updates: &BTreeMap<String, Option<String>>,
) -> bool {
    set_frontmatter_com(lines, updates, true)
}

/// Como `set_frontmatter`, mas com `citar = false` grava o valor cru: é como a
/// lista `bloqueada-por: [a, b]` entra sem virar a string `"[a, b]"`.
pub fn set_frontmatter_com(
    lines: &mut Vec<String>,
    updates: &BTreeMap<String, Option<String>>,
    citar: bool,
) -> bool {
    let escalar = |v: &str| if citar { yaml_scalar(v) } else { v.to_string() };
    let (map, faixa) = frontmatter(lines);
    let mut changed = false;

    let Some((s, mut e)) = faixa else {
        let add: Vec<&String> = updates
            .iter()
            .filter(|(_, v)| v.is_some())
            .map(|(k, _)| k)
            .collect();
        if add.is_empty() {
            return false;
        }
        let mut blk = vec!["---".to_string()];
        for k in add {
            if let Some(Some(v)) = updates.get(k) {
                blk.push(format!("{k}: {}", escalar(v)));
            }
        }
        blk.push("---".into());
        // Linha em branco só se a primeira linha de conteúdo já não for uma:
        // sem ela o bloco encostaria no callout e o Obsidian juntaria os dois.
        if lines.first().is_some_and(|l| !trim(l).is_empty()) {
            blk.push(String::new());
        }
        let n = blk.len();
        lines.splice(0..0, blk);
        debug_assert!(lines.len() >= n);
        return true;
    };

    // Bloco existente: reescreve no lugar, preservando ordem e chaves alheias.
    let mut i = e - 1;
    while i > s {
        if let Some(k) = chave_da_linha(&lines[i]).map(str::to_string)
            && let Some(up) = updates.get(&k)
        {
            match up {
                None => {
                    lines.remove(i);
                    e -= 1;
                    changed = true;
                }
                Some(v) => {
                    let want = format!("{k}: {}", escalar(v));
                    if lines[i] != want {
                        lines[i] = want;
                        changed = true;
                    }
                }
            }
        }
        i -= 1;
    }

    // Chaves novas entram no fim do bloco, em ordem estável (BTreeMap já ordena).
    for (k, v) in updates {
        if let Some(v) = v
            && !map.contains_key(k)
        {
            lines.insert(e, format!("{k}: {}", escalar(v)));
            e += 1;
            changed = true;
        }
    }

    // Bloco vazio é ruído: some junto com a linha vazia que o seguia.
    if e == s + 1 {
        if lines.get(e + 1).is_none_or(|l| trim(l).is_empty()) && lines.get(e + 1).is_some() {
            lines.remove(e + 1);
        }
        lines.remove(e);
        lines.remove(s);
        changed = true;
    }
    changed
}

/// Atualiza a chave `key`, se (e só se) ela já existir. Não cria frontmatter.
pub fn update_frontmatter_key(lines: &mut [String], key: &str, value: &str) -> bool {
    if lines.first().map(String::as_str) != Some("---") {
        return false;
    }
    for l in lines.iter_mut().skip(1) {
        if l == "---" {
            return false;
        }
        if l.starts_with(key) && l[key.len()..].starts_with(':') {
            let new = format!("{key}: {value}");
            if *l == new {
                return false;
            }
            *l = new;
            return true;
        }
    }
    false
}

// --- Histórico -------------------------------------------------------------

fn eh_titulo_historico(l: &str) -> bool {
    // `^##%s+Histórico%s*$`
    let Some(resto) = l.strip_prefix("##") else {
        return false;
    };
    let sem = resto.trim_start_matches(crate::eh_espaco);
    if sem.len() == resto.len() {
        return false;
    }
    let Some(fim) = sem.strip_prefix("Histórico") else {
        return false;
    };
    so_espaco(fim)
}

fn eh_secao(l: &str) -> bool {
    // `^##%s`
    l.strip_prefix("##")
        .is_some_and(|r| r.chars().next().is_some_and(crate::eh_espaco))
}

/// Acrescenta `entry` ao rodapé `## Histórico`, criando a seção se não existe.
/// Entra no FIM da seção (não do arquivo): uma `## seção` escrita depois do
/// histórico continua abaixo.
fn append_entry(lines: &mut Vec<String>, entry: String) {
    // Última ocorrência: se houver mais de um `## Histórico`, a de baixo é a
    // que o olho lê como rodapé.
    let hstart = lines.iter().rposition(|l| eh_titulo_historico(l));
    let Some(hstart) = hstart else {
        while lines.last().is_some_and(|l| trim(l).is_empty()) {
            lines.pop();
        }
        lines.extend([
            String::new(),
            "## Histórico".to_string(),
            String::new(),
            entry,
        ]);
        return;
    };
    let mut hend = lines.len() - 1;
    if let Some(i) = lines
        .iter()
        .enumerate()
        .skip(hstart + 1)
        .find(|(_, l)| eh_secao(l))
        .map(|(i, _)| i)
    {
        hend = i - 1;
    }
    while hend > hstart && trim(&lines[hend]).is_empty() {
        hend -= 1;
    }
    lines.insert(hend + 1, entry);
}

/// A entrada de (des)arquivamento. É um LOG: nunca sobrescreve.
pub fn append_history(lines: &mut Vec<String>, dest: Option<&str>, today: Date) {
    let entry = match dest {
        Some(d) => format!(
            "- {today} — arquivada em `archived/{d}` ({})",
            crate::config::archived_label(d)
        ),
        None => format!("- {today} — desarquivada, de volta ao board"),
    };
    append_entry(lines, entry);
}

/// A entrada de mudança de projeto.
pub fn append_history_moved(lines: &mut Vec<String>, from: &str, to: &str, today: Date) {
    append_entry(lines, format!("- {today} — movida de `{from}` para `{to}`"));
}

/// A entrada de rename: o único registro de que `from` um dia foi o id.
pub fn append_history_renamed(lines: &mut Vec<String>, from: &str, to: &str, today: Date) {
    append_entry(
        lines,
        format!("- {today} — renomeada de `{from}` para `{to}`"),
    );
}

/// A marca da linha que o `doctor --fix` reconstrói: inventar a data real
/// seria pior que não ter a linha.
pub const MARCA_RECONSTRUIDA: &str = "[reconstruído pelo doctor; data real desconhecida]";

/// A entrada de arquivamento reconstruída pelo `doctor --fix`, com o dia de
/// hoje e a marca NA PRÓPRIA LINHA, no fim da seção `## Histórico`. O Lua
/// colava a marca na última linha do arquivo, que não é a entrada quando há
/// outra seção depois do histórico (decisão 39 do log da reescrita).
pub fn append_history_reconstruida(lines: &mut Vec<String>, dest: &str, today: Date) {
    let entry = format!(
        "- {today} — arquivada em `archived/{dest}` ({}) {MARCA_RECONSTRUIDA}",
        crate::config::archived_label(dest)
    );
    append_entry(lines, entry);
}

/// Verdadeiro se há uma linha de arquivamento no log
/// (`^%- %d%d%d%d%-%d%d%-%d%d — arquivada em`). Só dígitos, como o padrão do
/// Lua: `2026-13-45` conta, porque o que o `doctor` cobra é a linha existir,
/// não a data ser válida.
pub fn tem_entrada_de_arquivamento(lines: &[String]) -> bool {
    lines.iter().any(|l| {
        let b = l.as_bytes();
        let digitos = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
        b.len() >= 12
            && b.starts_with(b"- ")
            && digitos(2..6)
            && b[6] == b'-'
            && digitos(7..9)
            && b[9] == b'-'
            && digitos(10..12)
            && b[12..].starts_with(" — arquivada em".as_bytes())
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    fn hoje() -> Date {
        Date::parse("2026-11-08").unwrap()
    }

    #[test]
    fn strip_quote_tira_o_prefixo_e_um_espaco() {
        assert_eq!(strip_quote("> a"), "a");
        assert_eq!(strip_quote(">a"), "a");
        assert_eq!(strip_quote(">"), "");
        assert_eq!(strip_quote("  >  a"), " a");
        assert_eq!(strip_quote("sem"), "sem");
    }

    #[test]
    fn first_block_range_acha_o_bloco_no_topo() {
        assert_eq!(
            first_block_range(&v(&["> [!todo] T", "> [[x]]", "", "corpo"])),
            Some((0, 1))
        );
    }

    #[test]
    fn first_block_range_pula_frontmatter_e_linhas_em_branco() {
        assert_eq!(
            first_block_range(&v(&[
                "---",
                "id: x",
                "---",
                "",
                "> [!todo] T",
                "> [[x]]",
                "",
                "corpo"
            ])),
            Some((4, 5))
        );
    }

    #[test]
    fn first_block_range_e_nil_com_conteudo_antes_do_bloco() {
        assert_eq!(
            first_block_range(&v(&["# Título", "", "> [!todo] T"])),
            None
        );
    }

    #[test]
    fn first_block_range_e_nil_sem_bloco() {
        assert_eq!(first_block_range(&v(&["corpo", "mais corpo"])), None);
        assert_eq!(first_block_range(&[]), None);
    }

    #[test]
    fn block_around_expande_para_as_bordas() {
        assert_eq!(
            block_around(&v(&["texto", "> a", "> b", "> c", "texto"]), 2),
            Some((1, 3))
        );
    }

    #[test]
    fn block_around_e_nil_fora_de_blockquote() {
        assert_eq!(block_around(&v(&["texto", "> a"]), 0), None);
    }

    #[test]
    fn splice_substitui_a_faixa_preservando_o_entorno() {
        assert_eq!(
            splice(&v(&["a", "b", "c", "d"]), 1, 2, &v(&["X", "Y"])),
            v(&["a", "X", "Y", "d"])
        );
    }

    #[test]
    fn splice_aceita_substituicao_vazia() {
        assert_eq!(splice(&v(&["a", "b", "c", "d"]), 1, 2, &[]), v(&["a", "d"]));
    }

    #[test]
    fn append_history_cria_a_secao_quando_nao_existe() {
        let mut l = v(&["# Task", "", "corpo", "", ""]);
        append_history(&mut l, Some("done"), hoje());
        assert_eq!(
            l,
            v(&[
                "# Task",
                "",
                "corpo",
                "",
                "## Histórico",
                "",
                "- 2026-11-08 — arquivada em `archived/done` (feito)"
            ])
        );
    }

    #[test]
    fn append_history_acrescenta_ao_fim_da_secao_nao_do_arquivo() {
        let mut l = v(&[
            "## Histórico",
            "",
            "- 2026-01-01 — arquivada em `archived/done` (feito)",
            "",
            "## Outra seção",
            "conteúdo",
        ]);
        append_history(&mut l, None, hoje());
        assert_eq!(l[3], "- 2026-11-08 — desarquivada, de volta ao board");
        assert_eq!(l[4], "");
        assert_eq!(l[5], "## Outra seção");
    }

    #[test]
    fn append_history_e_um_log() {
        let mut l = v(&[
            "## Histórico",
            "",
            "- 2026-01-01 — arquivada em `archived/done` (feito)",
        ]);
        append_history(&mut l, Some("dropped"), hoje());
        assert_eq!(l.len(), 4);
        assert_eq!(
            l[3],
            "- 2026-11-08 — arquivada em `archived/dropped` (abandonado)"
        );
    }

    #[test]
    fn append_history_moved_cria_a_secao() {
        let mut l = v(&["# Task", "", "corpo", ""]);
        append_history_moved(&mut l, "projeto-a", "projeto-b", hoje());
        assert_eq!(
            l,
            v(&[
                "# Task",
                "",
                "corpo",
                "",
                "## Histórico",
                "",
                "- 2026-11-08 — movida de `projeto-a` para `projeto-b`"
            ])
        );
    }

    #[test]
    fn append_history_moved_vai_depois_do_arquivamento() {
        let mut l = v(&[
            "## Histórico",
            "",
            "- 2026-01-01 — arquivada em `archived/done` (feito)",
            "",
            "## Outra seção",
            "conteúdo",
        ]);
        append_history_moved(&mut l, "projeto-a", "projeto-b", hoje());
        assert_eq!(l[2], "- 2026-01-01 — arquivada em `archived/done` (feito)");
        assert_eq!(
            l[3],
            "- 2026-11-08 — movida de `projeto-a` para `projeto-b`"
        );
        assert_eq!(l[4], "");
        assert_eq!(l[5], "## Outra seção");
    }

    #[test]
    fn append_history_moved_duas_vezes_deixa_duas_entradas() {
        let mut l = v(&["corpo"]);
        append_history_moved(&mut l, "a", "b", hoje());
        append_history_moved(&mut l, "b", "a", hoje());
        assert_eq!(l.len(), 6);
        assert_eq!(l[4], "- 2026-11-08 — movida de `a` para `b`");
        assert_eq!(l[5], "- 2026-11-08 — movida de `b` para `a`");
    }

    #[test]
    fn append_history_renamed_cria_a_secao() {
        let mut l = v(&["# Task", "", "corpo", ""]);
        append_history_renamed(&mut l, "foo", "bar", hoje());
        assert_eq!(
            l,
            v(&[
                "# Task",
                "",
                "corpo",
                "",
                "## Histórico",
                "",
                "- 2026-11-08 — renomeada de `foo` para `bar`"
            ])
        );
    }

    #[test]
    fn append_history_renamed_vai_depois_do_arquivamento() {
        let mut l = v(&[
            "## Histórico",
            "",
            "- 2026-01-01 — arquivada em `archived/done` (feito)",
            "",
            "## Outra seção",
            "conteúdo",
        ]);
        append_history_renamed(&mut l, "foo", "bar", hoje());
        assert_eq!(l[3], "- 2026-11-08 — renomeada de `foo` para `bar`");
        assert_eq!(l[4], "");
        assert_eq!(l[5], "## Outra seção");
    }

    #[test]
    fn append_history_renamed_duas_vezes_guarda_os_dois_nomes() {
        let mut l = v(&["corpo"]);
        append_history_renamed(&mut l, "a", "b", hoje());
        append_history_renamed(&mut l, "b", "c", hoje());
        assert_eq!(l.len(), 6);
        assert_eq!(l[4], "- 2026-11-08 — renomeada de `a` para `b`");
        assert_eq!(l[5], "- 2026-11-08 — renomeada de `b` para `c`");
    }

    #[test]
    fn append_history_usa_a_ultima_secao_historico() {
        let mut l = v(&[
            "## Histórico",
            "",
            "- velho",
            "",
            "## Histórico",
            "",
            "- novo",
        ]);
        append_history(&mut l, None, hoje());
        assert_eq!(l[7], "- 2026-11-08 — desarquivada, de volta ao board");
    }

    #[test]
    fn update_frontmatter_key_atualiza_a_chave_existente() {
        let mut l = v(&["---", "id: velho", "tags: []", "---", "corpo"]);
        assert!(update_frontmatter_key(&mut l, "id", "novo"));
        assert_eq!(l[1], "id: novo");
    }

    #[test]
    fn update_frontmatter_key_nao_cria_frontmatter_nem_chave() {
        let mut sem = v(&["corpo"]);
        assert!(!update_frontmatter_key(&mut sem, "id", "novo"));
        assert_eq!(sem, v(&["corpo"]));
        let mut sem_chave = v(&["---", "tags: []", "---"]);
        assert!(!update_frontmatter_key(&mut sem_chave, "id", "novo"));
        assert_eq!(sem_chave, v(&["---", "tags: []", "---"]));
    }

    #[test]
    fn update_frontmatter_key_nao_reporta_mudanca_quando_ja_esta_certo() {
        assert!(!update_frontmatter_key(
            &mut v(&["---", "id: x", "---"]),
            "id",
            "x"
        ));
    }

    #[test]
    fn frontmatter_le_pares_e_devolve_a_faixa() {
        let (map, faixa) = frontmatter(&v(&[
            "---",
            "dono: fedora/nvim",
            "dominio: schema",
            "---",
            "",
            "> [!todo] x",
        ]));
        assert_eq!(map["dono"], "fedora/nvim");
        assert_eq!(map["dominio"], "schema");
        assert_eq!(faixa, Some((0, 3)));
    }

    #[test]
    fn frontmatter_e_vazio_sem_bloco_e_sem_fechamento() {
        assert!(frontmatter(&v(&["> [!todo] x"])).0.is_empty());
        assert!(frontmatter(&v(&["---", "dono: x"])).0.is_empty());
    }

    #[test]
    fn frontmatter_tira_as_aspas_do_escalar() {
        let (map, _) = frontmatter(&v(&["---", "dominio: \"a: b\"", "---"]));
        assert_eq!(map["dominio"], "a: b");
    }

    fn up(pares: &[(&str, Option<&str>)]) -> BTreeMap<String, Option<String>> {
        pares
            .iter()
            .map(|(k, v)| (k.to_string(), v.map(str::to_string)))
            .collect()
    }

    #[test]
    fn set_frontmatter_cria_o_bloco_com_linha_em_branco_antes_do_conteudo() {
        let mut l = v(&["> [!todo] x"]);
        assert!(set_frontmatter(
            &mut l,
            &up(&[("dono", Some("fedora/nvim"))])
        ));
        assert_eq!(
            l,
            v(&["---", "dono: fedora/nvim", "---", "", "> [!todo] x"])
        );
    }

    #[test]
    fn set_frontmatter_nao_cria_bloco_so_com_remocoes() {
        let mut l = v(&["> [!todo] x"]);
        assert!(!set_frontmatter(&mut l, &up(&[("dono", None)])));
        assert_eq!(l, v(&["> [!todo] x"]));
    }

    #[test]
    fn set_frontmatter_preserva_chaves_alheias_e_a_ordem() {
        let mut l = v(&["---", "id: x", "dono: a", "tags: [p]", "---", ""]);
        assert!(set_frontmatter(&mut l, &up(&[("dono", Some("b"))])));
        assert_eq!(l, v(&["---", "id: x", "dono: b", "tags: [p]", "---", ""]));
    }

    #[test]
    fn set_frontmatter_acrescenta_chave_nova_no_fim() {
        let mut l = v(&["---", "id: x", "---"]);
        assert!(set_frontmatter(&mut l, &up(&[("dominio", Some("schema"))])));
        assert_eq!(l, v(&["---", "id: x", "dominio: schema", "---"]));
    }

    #[test]
    fn set_frontmatter_remove_e_apaga_o_bloco_quando_esvazia() {
        let mut l = v(&["---", "dono: a", "---", "", "> [!todo] x"]);
        assert!(set_frontmatter(&mut l, &up(&[("dono", None)])));
        assert_eq!(l, v(&["> [!todo] x"]));
    }

    #[test]
    fn set_frontmatter_poe_aspas_so_quando_preciso() {
        let mut l = v(&["---", "id: x", "---"]);
        set_frontmatter(
            &mut l,
            &up(&[
                ("a", Some("a: b")),
                ("b", Some("simples com espaço")),
                ("c", Some("[nao-lista]")),
            ]),
        );
        assert_eq!(l[2], "a: \"a: b\"");
        assert_eq!(l[3], "b: simples com espaço");
        assert_eq!(l[4], "c: \"[nao-lista]\"");
    }

    #[test]
    fn set_frontmatter_sobrevive_ao_round_trip() {
        let mut l = v(&["> [!todo] x"]);
        set_frontmatter(
            &mut l,
            &up(&[
                ("dominio", Some("a: b")),
                ("dono", Some("fedora/claude-code")),
            ]),
        );
        let (map, _) = frontmatter(&l);
        assert_eq!(map["dominio"], "a: b");
        assert_eq!(map["dono"], "fedora/claude-code");
    }

    #[test]
    fn set_frontmatter_sem_mudanca_devolve_falso() {
        let mut l = v(&["---", "dono: a", "---"]);
        assert!(!set_frontmatter(&mut l, &up(&[("dono", Some("a"))])));
    }

    #[test]
    fn yaml_scalar_e_unquote() {
        assert_eq!(yaml_scalar(""), "\"\"");
        assert_eq!(yaml_scalar("x "), "\"x \"");
        assert_eq!(yaml_scalar("#x"), "\"#x\"");
        assert_eq!(yaml_scalar("a\"b"), "a\"b", "aspa no meio não pede aspas");
        assert_eq!(
            yaml_scalar("\"a"),
            "\"\\\"a\"",
            "aspa no começo pede, e é escapada"
        );
        assert_eq!(yaml_unquote("'a'"), "a");
        assert_eq!(yaml_unquote("\"a\\\\b\""), "a\\b");
        assert_eq!(yaml_unquote("cru"), "cru");
    }

    #[test]
    fn tem_entrada_de_arquivamento_reconhece_a_linha() {
        assert!(tem_entrada_de_arquivamento(&v(&[
            "- 2026-01-01 — arquivada em `archived/done` (feito)"
        ])));
        assert!(!tem_entrada_de_arquivamento(&v(&[
            "- 2026-01-01 — desarquivada, de volta ao board"
        ])));
        assert!(!tem_entrada_de_arquivamento(&v(&["arquivada em"])));
        assert!(
            !tem_entrada_de_arquivamento(&v(&["- lavar o cão", "- ação — arquivada em x"])),
            "multibyte não derruba"
        );
        assert!(
            tem_entrada_de_arquivamento(&v(&["- 2026-13-45 — arquivada em x"])),
            "só dígitos, como o Lua: a data não precisa existir"
        );
        for quase in [
            "- 2026-01-01 — desarquivada",
            "- 2026/01-01 — arquivada em x",
            "- 2026-01/01 — arquivada em x",
            "- 20a6-01-01 — arquivada em x",
            "- 2026-0a-01 — arquivada em x",
            "- 2026-01-0a — arquivada em x",
            "* 2026-01-01 — arquivada em x",
            "- 2026-01-01 - arquivada em x",
            "- 2026-01-01",
            "- 2026-01-0",
        ] {
            assert!(!tem_entrada_de_arquivamento(&v(&[quase])), "{quase}");
        }
    }

    #[test]
    fn historico_reconstruido_leva_a_marca_na_propria_linha() {
        let mut l = v(&[
            "corpo",
            "",
            "## Histórico",
            "",
            "- a",
            "",
            "## Depois",
            "",
            "x",
        ]);
        append_history_reconstruida(&mut l, "failed", hoje());
        assert_eq!(
            l,
            v(&[
                "corpo",
                "",
                "## Histórico",
                "",
                "- a",
                "- 2026-11-08 — arquivada em `archived/failed` (falhou) [reconstruído pelo doctor; data real desconhecida]",
                "",
                "## Depois",
                "",
                "x"
            ])
        );
    }
}
