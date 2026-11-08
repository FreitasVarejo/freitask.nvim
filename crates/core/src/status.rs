//! O vocabulário de fases, lido de `.freitask/status.json`. O status de uma
//! task NÃO mora aqui: ele é derivado do tipo do callout, e este módulo só
//! sabe traduzir um no outro.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config;

/// Metadados de uma fase, como vêm do `status.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusMeta {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub callout: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub hl_group: String,
}

/// O sentinela 0: callout cujo tipo não existe no vocabulário. Fora do
/// `status.json` de propósito: 0 não é um estado que se escolhe.
pub fn status_invalid() -> StatusMeta {
    StatusMeta {
        title: "Sem status".into(),
        callout: "invalid".into(),
        icon: "󰘸".into(),
        hl_group: "DiagnosticError".into(),
    }
}

/// O vocabulário: número → fase. Lido a cada execução.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocab {
    fases: BTreeMap<u32, StatusMeta>,
    invalid: StatusMeta,
}

impl Default for Vocab {
    fn default() -> Self {
        Vocab::from_json(config::DEFAULT_STATUS_JSON).unwrap_or_else(|_| Vocab {
            fases: BTreeMap::new(),
            invalid: status_invalid(),
        })
    }
}

impl Vocab {
    /// Decodifica um `status.json`. Erro quando não é JSON ou não é um objeto;
    /// quem chama cai no default e avisa, como `status.lua:62`.
    pub fn from_json(texto: &str) -> Result<Vocab, String> {
        let mapa: BTreeMap<String, StatusMeta> =
            serde_json::from_str(texto).map_err(|e| e.to_string())?;
        let mut fases = BTreeMap::new();
        for (k, v) in mapa {
            // Chave que não é número não é fase (`tonumber(k)` daria nil).
            if let Ok(n) = k.trim().parse::<u32>() {
                fases.insert(n, v);
            }
        }
        Ok(Vocab {
            fases,
            invalid: status_invalid(),
        })
    }

    /// Metadados da fase `num`, com o sentinela para 0 e para número que não
    /// existe.
    pub fn meta(&self, num: u32) -> &StatusMeta {
        if num == 0 {
            return &self.invalid;
        }
        self.fases.get(&num).unwrap_or(&self.invalid)
    }

    /// Número da fase pelo callout, ou `None`.
    pub fn by_callout(&self, callout: &str) -> Option<u32> {
        // Em empate (dois números com o mesmo callout) vale o maior, como o
        // `pairs()` do Lua tende a deixar; o vocabulário real não repete.
        self.fases
            .iter()
            .filter(|(_, m)| m.callout == callout)
            .map(|(n, _)| *n)
            .next_back()
    }

    /// Verdadeiro se a fase fica fora do painel e da TUI por padrão. Pelo
    /// callout, não pelo número, que muda quando o vocabulário cresce.
    pub fn is_hidden(&self, num: u32) -> bool {
        config::HIDDEN_CALLOUTS.contains(&self.meta(num).callout.as_str())
    }

    /// Os números das fases, em ordem.
    pub fn numeros(&self) -> impl Iterator<Item = u32> + '_ {
        self.fases.keys().copied()
    }

    /// Título pelo texto: `is_status_text` precisa achar a fase cujo título
    /// casa uma linha legada.
    pub fn by_title(&self, title: &str) -> Option<u32> {
        self.fases
            .iter()
            .filter(|(_, m)| m.title == title)
            .map(|(n, _)| *n)
            .next_back()
    }

    /// As linhas de `statuses`, com o sentinela 0 na frente.
    pub fn linhas(&self) -> Vec<(u32, StatusMeta, bool)> {
        let mut out = vec![(0, self.invalid.clone(), false)];
        for (n, m) in &self.fases {
            out.push((*n, m.clone(), self.is_hidden(*n)));
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn status_meta_resolve_um_status_valido() {
        let v = Vocab::default();
        assert_eq!(v.meta(1).callout, "todo");
        assert_eq!(v.meta(1).title, "Não iniciada");
    }

    #[test]
    fn status_meta_cai_no_sentinela_para_0_e_desconhecido() {
        let v = Vocab::default();
        assert_eq!(v.meta(0).callout, "invalid");
        assert_eq!(v.meta(999).callout, "invalid");
        assert_eq!(v.meta(0).title, "Sem status");
    }

    #[test]
    fn is_hidden_esconde_so_a_fase_quote() {
        let v = Vocab::default();
        assert!(v.is_hidden(v.by_callout("quote").unwrap()));
        for c in [
            "todo", "abstract", "tip", "example", "question", "warning", "check",
        ] {
            assert!(!v.is_hidden(v.by_callout(c).unwrap()), "{c} aparece");
        }
    }

    #[test]
    fn o_sentinela_nao_e_escondido() {
        assert!(!Vocab::default().is_hidden(0));
    }

    #[test]
    fn vocabulario_tem_as_nove_fases_na_ordem_com_callouts_distintos() {
        let v = Vocab::default();
        let ordem = [
            "todo", "abstract", "tip", "example", "question", "warning", "check", "quote", "done",
        ];
        let mut vistos = std::collections::BTreeSet::new();
        for (i, c) in ordem.iter().enumerate() {
            assert_eq!(v.meta(i as u32 + 1).callout, *c);
            assert!(vistos.insert(*c), "{c} repetido");
        }
        assert_eq!(v.numeros().count(), ordem.len());
    }

    #[test]
    fn algum_dia_vem_depois_de_pronta_e_antes_de_arquivada() {
        let v = Vocab::default();
        assert!(v.by_callout("check") < v.by_callout("quote"));
        assert!(v.by_callout("quote") < v.by_callout("done"));
    }

    #[test]
    fn json_invalido_e_erro_e_chave_nao_numerica_e_ignorada() {
        assert!(Vocab::from_json("não é json").is_err());
        assert!(Vocab::from_json("[1,2]").is_err());
        let v = Vocab::from_json(
            r#"{"1": {"title": "A", "callout": "todo"}, "x": {"title": "B", "callout": "quote"}}"#,
        )
        .unwrap();
        assert_eq!(v.numeros().collect::<Vec<_>>(), vec![1]);
        assert_eq!(v.by_callout("quote"), None);
        assert_eq!(v.meta(1).icon, "", "campo ausente vira vazio");
    }

    #[test]
    fn linhas_comecam_pelo_sentinela() {
        let l = Vocab::default().linhas();
        assert_eq!(l[0].0, 0);
        assert_eq!(l[0].1.callout, "invalid");
        assert!(!l[0].2);
        assert_eq!(l.len(), 10);
        assert!(
            l.iter()
                .filter(|(_, _, h)| *h)
                .map(|(_, m, _)| m.callout.as_str())
                .eq(["quote"])
        );
    }
}
