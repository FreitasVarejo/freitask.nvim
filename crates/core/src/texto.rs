//! Um arquivo de texto como linhas, sem perder os bytes que o comando não
//! devia tocar: BOM, CRLF por linha e a ausência de `\n` final sobrevivem ao
//! round-trip (spec §1.5, D4). `vim.fn.readfile`/`writefile` normalizavam
//! tudo para LF com `\n` final; este tipo existe para não normalizar.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
}

/// Linhas mais o que as separa. `eols[i]` é o fim da linha `i`; a última linha
/// pode não ter fim (`final_eol == false`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    pub bom: bool,
    pub lines: Vec<String>,
    eols: Vec<Eol>,
    pub final_eol: bool,
}

impl Doc {
    /// Decodifica bytes UTF-8. Erro quando não é UTF-8: um arquivo assim não é
    /// nota do vault, e reescrevê-lo "por cima" é que corromperia.
    pub fn parse(bytes: &[u8]) -> Result<Doc, std::str::Utf8Error> {
        let (bom, corpo) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
            Some(resto) => (true, resto),
            None => (false, bytes),
        };
        let texto = std::str::from_utf8(corpo)?;
        let mut lines = Vec::new();
        let mut eols = Vec::new();
        let mut resto = texto;
        let mut final_eol = true;
        loop {
            match resto.find('\n') {
                Some(i) => {
                    let linha = &resto[..i];
                    if let Some(sem_cr) = linha.strip_suffix('\r') {
                        lines.push(sem_cr.to_string());
                        eols.push(Eol::CrLf);
                    } else {
                        lines.push(linha.to_string());
                        eols.push(Eol::Lf);
                    }
                    resto = &resto[i + 1..];
                    if resto.is_empty() {
                        break;
                    }
                }
                None => {
                    // Última linha sem `\n`. Arquivo vazio não tem linha nenhuma.
                    if !resto.is_empty() {
                        lines.push(resto.to_string());
                        final_eol = false;
                    } else if lines.is_empty() {
                        final_eol = false;
                    }
                    break;
                }
            }
        }
        Ok(Doc {
            bom,
            lines,
            eols,
            final_eol,
        })
    }

    /// O fim de linha que as linhas novas herdam: o da maioria, LF em empate
    /// ou sem linhas.
    pub fn eol_dominante(&self) -> Eol {
        let crlf = self.eols.iter().filter(|e| **e == Eol::CrLf).count();
        if crlf > self.eols.len() - crlf {
            Eol::CrLf
        } else {
            Eol::Lf
        }
    }

    /// Reconstrói os bytes. Com as linhas intocadas, é byte a byte o original.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        let dominante = self.eol_dominante();
        let n = self.lines.len();
        for (i, l) in self.lines.iter().enumerate() {
            out.extend_from_slice(l.as_bytes());
            if i + 1 < n || self.final_eol {
                match self.eols.get(i).copied().unwrap_or(dominante) {
                    Eol::Lf => out.push(b'\n'),
                    Eol::CrLf => out.extend_from_slice(b"\r\n"),
                }
            }
        }
        out
    }

    /// Troca as linhas por `novas`, mantendo BOM, o fim de cada linha que
    /// não mudou e a presença do `\n` final. É o que toda operação de linhas
    /// usa para gravar: o conteúdo muda, a forma do arquivo não.
    ///
    /// As linhas iguais do começo e do fim (prefixo e sufixo comuns) guardam o
    /// fim de linha que tinham; as do meio, que são as novas ou trocadas,
    /// herdam o dominante. Assim um arquivo com fins misturados só muda nas
    /// linhas que o comando tocou (I2).
    pub fn com_linhas(&self, novas: Vec<String>) -> Doc {
        let dominante = self.eol_dominante();
        let fim_de = |i: usize| self.eols.get(i).copied().unwrap_or(dominante);
        let (n, m) = (self.lines.len(), novas.len());
        let prefixo = self
            .lines
            .iter()
            .zip(&novas)
            .take_while(|(a, b)| a == b)
            .count();
        let cabe = n.min(m) - prefixo;
        let sufixo = (0..cabe)
            .take_while(|j| self.lines[n - 1 - j] == novas[m - 1 - j])
            .count();
        let eols = (0..m)
            .map(|i| {
                if i < prefixo {
                    fim_de(i)
                } else if i >= m - sufixo {
                    fim_de(i + n - m)
                } else {
                    dominante
                }
            })
            .collect();
        Doc {
            bom: self.bom,
            lines: novas,
            eols,
            final_eol: self.final_eol,
        }
    }

    /// Um documento novo, LF, com `\n` final.
    pub fn novo(lines: Vec<String>) -> Doc {
        let n = lines.len();
        Doc {
            bom: false,
            lines,
            eols: vec![Eol::Lf; n],
            final_eol: true,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn rt(bytes: &[u8]) {
        assert_eq!(
            Doc::parse(bytes).unwrap().to_bytes(),
            bytes,
            "{:?}",
            String::from_utf8_lossy(bytes)
        );
    }

    #[test]
    fn round_trip_preserva_lf_crlf_bom_e_sem_fim() {
        rt(b"a\nb\n");
        rt(b"a\r\nb\r\n");
        rt(b"a\nb");
        rt(b"a\r\nb");
        rt(b"\xEF\xBB\xBFa\nb\n");
        rt(b"a\r\nb\nc\r\n");
        rt(b"");
        rt(b"\n");
        rt(b"\n\n");
        rt(b"a");
    }

    #[test]
    fn linhas_sao_sem_o_fim() {
        let d = Doc::parse(b"a\r\nb\n").unwrap();
        assert_eq!(d.lines, vec!["a", "b"]);
        assert!(d.final_eol);
        let d = Doc::parse(b"a\nb").unwrap();
        assert_eq!(d.lines, vec!["a", "b"]);
        assert!(!d.final_eol);
        assert!(Doc::parse(b"").unwrap().lines.is_empty());
    }

    #[test]
    fn linhas_novas_herdam_o_fim_dominante() {
        let d = Doc::parse(b"a\r\nb\r\n").unwrap();
        let e = d.com_linhas(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(e.to_bytes(), b"a\r\nb\r\nc\r\n");
        let d = Doc::parse(b"\xEF\xBB\xBFa\n").unwrap();
        let e = d.com_linhas(vec!["x".into()]);
        assert_eq!(e.to_bytes(), b"\xEF\xBB\xBFx\n");
    }

    #[test]
    fn sem_fim_final_continua_sem_fim() {
        let d = Doc::parse(b"a\nb").unwrap();
        let e = d.com_linhas(vec!["a".into(), "c".into()]);
        assert_eq!(e.to_bytes(), b"a\nc");
    }

    #[test]
    fn linha_intocada_guarda_o_proprio_fim_em_arquivo_misto() {
        let d = Doc::parse(b"a\r\nb\nc\r\nd\n").unwrap();
        // Uma linha nova no meio: `a` e `b` (prefixo) e `c`, `d` (sufixo)
        // guardam os fins; a nova herda o dominante (empate: LF).
        let e = d.com_linhas(vec![
            "a".into(),
            "b".into(),
            "x".into(),
            "c".into(),
            "d".into(),
        ]);
        assert_eq!(e.to_bytes(), b"a\r\nb\nx\nc\r\nd\n");
        // Uma linha a menos no meio.
        let e = d.com_linhas(vec!["a".into(), "c".into(), "d".into()]);
        assert_eq!(e.to_bytes(), b"a\r\nc\r\nd\n");
        // Linha trocada no lugar herda o dominante, não o fim da antiga.
        let d = Doc::parse(b"a\r\nb\r\nc\n").unwrap();
        let e = d.com_linhas(vec!["a".into(), "B".into(), "c".into()]);
        assert_eq!(e.to_bytes(), b"a\r\nB\r\nc\n");
        // Sem mudança, os mesmos bytes.
        let d = Doc::parse(b"a\nb\r\nc").unwrap();
        assert_eq!(d.com_linhas(d.lines.clone()).to_bytes(), b"a\nb\r\nc");
    }

    #[test]
    fn linha_acrescentada_depois_da_ultima_sem_fim_herda_o_dominante() {
        let d = Doc::parse(b"a\r\nb").unwrap();
        let e = d.com_linhas(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(e.to_bytes(), b"a\r\nb\r\nc");
        let e = d.com_linhas(vec!["z".into(), "a".into(), "b".into()]);
        assert_eq!(e.to_bytes(), b"z\r\na\r\nb");
    }

    #[test]
    fn nao_utf8_e_erro() {
        assert!(Doc::parse(&[0xFF, 0xFE, b'a']).is_err());
    }
}
