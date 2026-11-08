//! freitask-core — o núcleo puro do freitask.
//!
//! Sem I/O, sem relógio, sem variável de ambiente: tudo que entra vem por
//! parâmetro e tudo que sai é valor. Operações que mudam o vault devolvem um
//! plano de efeitos sobre arquivos, que a crate `freitask-vault` executa.
//!
//! Os módulos seguem os nomes do motor Lua (`docs/freitask-internals.md`),
//! porque a responsabilidade de cada um já está justificada lá; o que muda é a
//! linguagem e a fronteira com o disco.

pub mod config;
pub mod date;
pub mod links;
pub mod md;
pub mod meta;
pub mod model;
pub mod path;
pub mod status;
pub mod task;
pub mod texto;

/// `vim.trim`: apara só o espaço em branco ASCII (`%s` do Lua), não o Unicode.
pub fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c'))
}

/// `%s` do Lua: espaço em branco ASCII.
pub fn eh_espaco(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

/// Preenche à direita por BYTES, como o `%-Ns` do `string.format` do Lua: um
/// título com acento ocupa mais bytes do que colunas, e a saída de texto de
/// hoje alinha por bytes. Reproduzido para a saída ser idêntica.
pub fn pad_bytes(s: &str, largura: usize) -> String {
    let mut out = String::from(s);
    let faltam = largura.saturating_sub(s.len());
    out.extend(std::iter::repeat_n(' ', faltam));
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn trim_apara_so_ascii() {
        assert_eq!(trim("  a b \t"), "a b");
        assert_eq!(
            trim("\u{a0}a\u{a0}"),
            "\u{a0}a\u{a0}",
            "NBSP não é %s do Lua"
        );
    }

    #[test]
    fn pad_bytes_conta_bytes_e_nao_colunas() {
        assert_eq!(pad_bytes("ab", 4), "ab  ");
        assert_eq!(pad_bytes("Não", 5), "Não ", "ã tem dois bytes");
        assert_eq!(pad_bytes("Em implementação", 12), "Em implementação");
    }
}
