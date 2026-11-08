//! Wikilinks: um scanner próprio de `[[...]]` que devolve, por link, os spans
//! em bytes de alvo, sufixo (`#`/`^`), separador (`|` ou `\|`) e alias, com
//! as zonas de exclusão que o `doctor` já aplicava (bloco cercado por ``` e
//! code span entre crases). A reescrita troca só o segmento do alvo (e o alias
//! quando ele era o id antigo), de trás para frente, sobre os bytes: `\|`,
//! alias, `#heading`, `^bloco`, embed e CRLF sobrevivem (spec §1.4).
//!
//! Arquivar, desarquivar, mudar de projeto e renomear são o MESMO evento
//! ("esse arquivo mudou de endereço") e passam por `rewrite_link`; encerrar e
//! reabrir um projeto são o mesmo evento para uma PASTA e passam por
//! `rewrite_prefix`. O Obsidian resolve `[[foo]]` pelo basename, daí a
//! assimetria: mover de pasta só reescreve o link path-qualified; renomear
//! reescreve os dois.

use crate::trim;

/// Um wikilink achado no texto, em offsets de bytes sobre o texto inteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// `[[` até `]]`, inclusive.
    pub full: std::ops::Range<usize>,
    /// O miolo inteiro, entre os colchetes.
    pub inner: std::ops::Range<usize>,
    /// Só o alvo (sem sufixo nem alias).
    pub target: std::ops::Range<usize>,
    /// `#...`/`^...` depois do alvo, vazio quando não há.
    pub sub: std::ops::Range<usize>,
    /// `|` ou `\|`, vazio quando não há alias.
    pub sep: std::ops::Range<usize>,
    /// O alias, vazio quando não há.
    pub alias: std::ops::Range<usize>,
    /// Linha (0-based) em que o link começa.
    pub linha: usize,
}

/// Acha os wikilinks fora de código. Linhas que começam com ``` alternam o
/// bloco cercado; dentro de uma linha, o que está entre crases é pulado.
pub fn scan(texto: &str) -> Vec<Link> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut offset = 0usize;
    for (n, linha) in texto.split_inclusive('\n').enumerate() {
        let conteudo = linha
            .strip_suffix('\n')
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .unwrap_or(linha);
        if conteudo
            .trim_start_matches(crate::eh_espaco)
            .starts_with("```")
        {
            in_fence = !in_fence;
        } else if !in_fence {
            scan_linha(conteudo, offset, n, &mut out);
        }
        offset += linha.len();
    }
    out
}

fn scan_linha(linha: &str, base: usize, n: usize, out: &mut Vec<Link>) {
    let b = linha.as_bytes();
    let mut i = 0;
    let mut in_span = false;
    while i < b.len() {
        if b[i] == b'`' {
            in_span = !in_span;
            i += 1;
            continue;
        }
        if in_span {
            i += 1;
            continue;
        }
        if b[i] == b'[' && i + 1 < b.len() && b[i + 1] == b'[' {
            let inicio = i;
            let miolo_ini = i + 2;
            // `.-` do Lua: até o primeiro `]]`.
            let Some(fim_rel) = linha[miolo_ini..].find("]]") else {
                i += 2;
                continue;
            };
            let miolo_fim = miolo_ini + fim_rel;
            let full_fim = miolo_fim + 2;
            out.push(decompor(
                linha, base, n, inicio, miolo_ini, miolo_fim, full_fim,
            ));
            i = full_fim;
            continue;
        }
        i += 1;
    }
}

fn decompor(
    linha: &str,
    base: usize,
    n: usize,
    inicio: usize,
    mi: usize,
    mf: usize,
    ff: usize,
) -> Link {
    let miolo = &linha[mi..mf];
    // Separador: `\|` (tabela) ou `|`, o primeiro que aparecer.
    let (sep_ini, sep_len) = match (miolo.find("\\|"), miolo.find('|')) {
        (Some(e), Some(p)) if e < p => (Some(e), 2),
        (Some(e), None) => (Some(e), 2),
        (_, Some(p)) => (Some(p), 1),
        (None, None) => (None, 0),
    };
    let corpo_fim = sep_ini.unwrap_or(miolo.len());
    let corpo = &miolo[..corpo_fim];
    // Sufixo: do primeiro `#` ou `^` até o fim do corpo.
    let sub_ini = corpo.find(['#', '^']).unwrap_or(corpo.len());
    let r = |a: usize, z: usize| base + mi + a..base + mi + z;
    Link {
        full: base + inicio..base + ff,
        inner: base + mi..base + mf,
        target: r(0, sub_ini),
        sub: r(sub_ini, corpo_fim),
        sep: match sep_ini {
            Some(s) => r(s, s + sep_len),
            None => r(corpo_fim, corpo_fim),
        },
        alias: match sep_ini {
            Some(s) => r(s + sep_len, miolo.len()),
            None => r(miolo.len(), miolo.len()),
        },
        linha: n,
    }
}

/// O alvo normalizado de um link: aparado e sem `.md`.
pub fn chave(target: &str) -> &str {
    let t = trim(target);
    t.strip_suffix(".md").unwrap_or(t)
}

/// Um endereço de task para a reescrita: o id e o caminho vault-relative sem `.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endereco {
    pub id: String,
    pub full: String,
}

impl Endereco {
    pub fn de_rel(rel: &str) -> Endereco {
        let full = rel.strip_suffix(".md").unwrap_or(rel).to_string();
        Endereco {
            id: crate::path::basename_sem_extensao(&full),
            full,
        }
    }
}

/// O que muda num link quando a task mudou de endereço: o alvo novo e, se o
/// alias ERA o id antigo, o alias novo. `None` quando o link aponta para outra
/// coisa. Puro, sobre as partes já decompostas.
pub fn rewrite_link(
    target: &str,
    alias: Option<&str>,
    old: &Endereco,
    new: &Endereco,
) -> Option<(String, Option<String>)> {
    let key = chave(target);
    let new_target = if key == old.full {
        new.full.clone()
    } else if key == old.id && old.id != new.id {
        // Forma curta: só quebra quando o BASENAME muda.
        new.id.clone()
    } else {
        return None;
    };
    let new_alias = alias.filter(|a| trim(a) == old.id).map(|_| new.id.clone());
    Some((new_target, new_alias))
}

/// O alvo novo de um link cujo alvo começa por `old_prefix` (a pasta de um
/// projeto, com barra), trocando só o prefixo. `None` quando não começa por
/// ele. Só o começo é comparado; alias, sufixo e `\|` ficam como estão.
pub fn rewrite_prefix(target: &str, old_prefix: &str, new_prefix: &str) -> Option<String> {
    let lead_len = target.len() - target.trim_start_matches(crate::eh_espaco).len();
    let (lead, rest) = target.split_at(lead_len);
    let resto = rest.strip_prefix(old_prefix)?;
    Some(format!("{lead}{new_prefix}{resto}"))
}

/// Aplica `f` a cada link fora de código e devolve o texto novo e o número de
/// links trocados, ou `None` se nada mudou. `f` recebe (alvo, alias) e devolve
/// o alvo novo e, opcionalmente, o alias novo. A substituição é de trás para
/// frente, só nos spans, sobre a string: o resto do arquivo não é tocado.
pub fn rewrite_text(
    texto: &str,
    f: impl Fn(&str, Option<&str>) -> Option<(String, Option<String>)>,
) -> Option<(String, usize)> {
    let links = scan(texto);
    let mut trocas: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for l in &links {
        let alias = if l.sep.is_empty() {
            None
        } else {
            Some(&texto[l.alias.clone()])
        };
        if let Some((novo_alvo, novo_alias)) = f(&texto[l.target.clone()], alias) {
            trocas.push((l.target.clone(), novo_alvo));
            if let Some(a) = novo_alias
                && !l.sep.is_empty()
            {
                trocas.push((l.alias.clone(), a));
            }
        }
    }
    if trocas.is_empty() {
        return None;
    }
    let n = links
        .iter()
        .filter(|l| trocas.iter().any(|(r, _)| *r == l.target))
        .count();
    let mut out = texto.to_string();
    trocas.sort_by_key(|t| std::cmp::Reverse(t.0.start));
    for (r, novo) in trocas {
        out.replace_range(r, &novo);
    }
    Some((out, n))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn end(id: &str, full: &str) -> Endereco {
        Endereco {
            id: id.into(),
            full: full.into(),
        }
    }

    /// O miolo reescrito, como o `rewrite_link(inner)` do Lua devolvia.
    fn rl(inner: &str, old: &Endereco, new: &Endereco) -> Option<String> {
        let texto = format!("[[{inner}]]");
        rewrite_text(&texto, |t, a| rewrite_link(t, a, old, new))
            .map(|(s, _)| s[2..s.len() - 2].to_string())
    }

    fn rp(inner: &str, o: &str, n: &str) -> Option<String> {
        let texto = format!("[[{inner}]]");
        rewrite_text(&texto, |t, _| rewrite_prefix(t, o, n).map(|x| (x, None)))
            .map(|(s, _)| s[2..s.len() - 2].to_string())
    }

    #[test]
    fn ao_mover_reescreve_so_o_link_path_qualified() {
        let (o, n) = (
            end("x", "projects/p/tasks/x"),
            end("x", "projects/p/tasks/archived/done/x"),
        );
        assert_eq!(
            rl("projects/p/tasks/x|x", &o, &n).as_deref(),
            Some("projects/p/tasks/archived/done/x|x")
        );
        assert_eq!(
            rl("x", &o, &n),
            None,
            "o Obsidian resolve o link curto por basename"
        );
    }

    #[test]
    fn ao_renomear_reescreve_as_duas_formas() {
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        assert_eq!(rl("a", &o, &n).as_deref(), Some("b"));
        assert_eq!(
            rl("projects/p/tasks/a|a", &o, &n).as_deref(),
            Some("projects/p/tasks/b|b")
        );
    }

    #[test]
    fn preserva_um_alias_que_nao_era_o_id() {
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        assert_eq!(
            rl("projects/p/tasks/a|Meu Título", &o, &n).as_deref(),
            Some("projects/p/tasks/b|Meu Título")
        );
    }

    #[test]
    fn preserva_sufixo_de_heading_e_de_bloco() {
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        assert_eq!(
            rl("projects/p/tasks/a#Seção", &o, &n).as_deref(),
            Some("projects/p/tasks/b#Seção")
        );
        assert_eq!(
            rl("projects/p/tasks/a^bloco|a", &o, &n).as_deref(),
            Some("projects/p/tasks/b^bloco|b")
        );
    }

    #[test]
    fn tolera_o_sufixo_md_no_alvo() {
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        assert_eq!(rl("a.md", &o, &n).as_deref(), Some("b"));
    }

    #[test]
    fn devolve_nil_para_link_que_aponta_para_outra_coisa() {
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        assert_eq!(rl("outra-nota", &o, &n), None);
        assert_eq!(rl("projects/outro/tasks/a|a", &o, &n), None);
    }

    #[test]
    fn alias_escapado_de_tabela_acompanha_o_move_e_o_rename() {
        // D1: `\|` separa o alias dentro de tabela; o Lua cortava no `|` e deixava o `\` no alvo.
        let (o, n) = (
            end("x", "projects/p/tasks/x"),
            end("x", "projects/p/tasks/archived/done/x"),
        );
        assert_eq!(
            rl("projects/p/tasks/x\\|x", &o, &n).as_deref(),
            Some("projects/p/tasks/archived/done/x\\|x")
        );
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        assert_eq!(
            rl("projects/p/tasks/a\\|a", &o, &n).as_deref(),
            Some("projects/p/tasks/b\\|b")
        );
        assert_eq!(
            rl("projects/p/tasks/a\\|rótulo", &o, &n).as_deref(),
            Some("projects/p/tasks/b\\|rótulo")
        );
    }

    #[test]
    fn rewrite_prefix_troca_so_o_prefixo_em_qualquer_profundidade() {
        let (o, n) = ("projects/p/", "projects/_encerrados/p/");
        assert_eq!(
            rp("projects/p/tasks/x|x", o, n).as_deref(),
            Some("projects/_encerrados/p/tasks/x|x")
        );
        assert_eq!(
            rp("projects/p/tasks/archived/done/x|x", o, n).as_deref(),
            Some("projects/_encerrados/p/tasks/archived/done/x|x")
        );
        assert_eq!(
            rp("projects/p/p", o, n).as_deref(),
            Some("projects/_encerrados/p/p"),
            "o MOC também"
        );
    }

    #[test]
    fn rewrite_prefix_preserva_alias_sufixo_extensao_e_pipe_escapado() {
        let (o, n) = ("projects/p/", "projects/_encerrados/p/");
        assert_eq!(
            rp("projects/p/p#Escopo|o projeto", o, n).as_deref(),
            Some("projects/_encerrados/p/p#Escopo|o projeto")
        );
        assert_eq!(
            rp("projects/p/tasks/x^bloco", o, n).as_deref(),
            Some("projects/_encerrados/p/tasks/x^bloco")
        );
        assert_eq!(
            rp("projects/p/anexos/a/f.pdf\\|f.pdf", o, n).as_deref(),
            Some("projects/_encerrados/p/anexos/a/f.pdf\\|f.pdf")
        );
        assert_eq!(
            rp(" projects/p/x", o, n).as_deref(),
            Some(" projects/_encerrados/p/x")
        );
    }

    #[test]
    fn rewrite_prefix_deixa_o_curto_o_vizinho_parecido_e_a_pasta_sem_barra() {
        let (o, n) = ("projects/p/", "projects/_encerrados/p/");
        assert_eq!(rp("x", o, n), None);
        assert_eq!(rp("projects/p-outro/tasks/x|x", o, n), None);
        assert_eq!(rp("projects/p", o, n), None);
        assert_eq!(rp("outra/projects/p/x", o, n), None);
    }

    #[test]
    fn rewrite_prefix_ida_e_volta_devolve_o_original() {
        let (o, n) = ("projects/p/", "projects/_encerrados/p/");
        let inner = "projects/p/tasks/archived/done/x#Para retomar|x";
        assert_eq!(rp(&rp(inner, o, n).unwrap(), n, o).as_deref(), Some(inner));
    }

    #[test]
    fn scan_pula_bloco_cercado_e_code_span_e_preserva_offsets_com_crlf() {
        let texto = "a [[x]] `[[y]]` b\r\n```\n[[z]]\n```\r\n[[w|W]] ![[e#h]]\n";
        let links = scan(texto);
        let alvos: Vec<&str> = links.iter().map(|l| &texto[l.target.clone()]).collect();
        assert_eq!(alvos, vec!["x", "w", "e"]);
        assert_eq!(&texto[links[1].alias.clone()], "W");
        assert_eq!(&texto[links[2].sub.clone()], "#h");
        assert_eq!(links[2].linha, 4);
    }

    #[test]
    fn rewrite_text_troca_so_os_spans_e_conta() {
        let (o, n) = (
            end("a", "projects/p/tasks/a"),
            end("b", "projects/p/tasks/b"),
        );
        let texto = "| t | [[projects/p/tasks/a\\|a]] |\r\nver [[a]] e `[[a]]`\n```\n[[a]]\n```\n";
        let (novo, qtd) = rewrite_text(texto, |t, al| rewrite_link(t, al, &o, &n)).unwrap();
        assert_eq!(
            novo,
            "| t | [[projects/p/tasks/b\\|b]] |\r\nver [[b]] e `[[a]]`\n```\n[[a]]\n```\n"
        );
        assert_eq!(qtd, 2);
        assert_eq!(
            rewrite_text("nada [[outro]]", |t, al| rewrite_link(t, al, &o, &n)),
            None
        );
    }

    #[test]
    fn link_sem_fechamento_e_ignorado() {
        assert!(scan("[[aberto").is_empty());
        assert_eq!(scan("[[a]] [[b").len(), 1);
    }
}
