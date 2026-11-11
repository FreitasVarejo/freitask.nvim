//! freitask-core — o núcleo puro do freitask.
//!
//! Sem I/O, sem relógio, sem variável de ambiente: tudo que entra vem por
//! parâmetro e tudo que sai é valor. Operações que mudam o vault devolvem um
//! plano de efeitos sobre arquivos, que a crate `freitask-vault` executa.
//!
//! Os nomes dos módulos vêm do motor Lua que este núcleo substitui; o mapa de
//! cada um — o que é e o que não é responsabilidade dele — está em
//! `docs/freitask-internals.md`.

pub mod board;
pub mod config;
pub mod date;
pub mod deps;
pub mod deslocar;
pub mod doctor;
pub mod index;
pub mod links;
pub mod md;
pub mod meta;
pub mod model;
pub mod operacao;
pub mod path;
pub mod plano;
pub mod projeto;
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

/// O `%q` do `string.format` do LuaJIT: entre aspas, com `"`, `\\` e `\n`
/// precedidos de barra, e os outros caracteres de controle em `\ddd` (com
/// três dígitos quando o seguinte é dígito). As mensagens do `doctor` citam o
/// valor assim, e a saída de texto é comparada byte a byte com o oráculo.
pub fn lua_q(s: &str) -> String {
    let mut out = String::from("\"");
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' | '\\' | '\n' => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 32 || c as u32 == 127 => {
                let n = c as u32;
                let segue_digito = it.peek().is_some_and(char::is_ascii_digit);
                if segue_digito {
                    out.push_str(&format!("\\{n:03}"));
                } else {
                    out.push_str(&format!("\\{n}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn lua_q_escapa_como_o_luajit() {
        assert_eq!(lua_q("todo"), "\"todo\"");
        assert_eq!(lua_q("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(lua_q("a\nb"), "\"a\\\nb\"");
        assert_eq!(lua_q("a\rb"), "\"a\\13b\"");
        assert_eq!(lua_q("a\r1"), "\"a\\0131\"");
        assert_eq!(lua_q("\t"), "\"\\9\"");
        assert_eq!(lua_q("\u{1f}\u{7f}"), "\"\\31\\127\"");
        assert_eq!(lua_q(" ~é"), "\" ~é\"", "espaço, til e não-ASCII passam");
    }

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
