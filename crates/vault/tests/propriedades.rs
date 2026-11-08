//! Propriedades da gravação (spec §3.2): I8 (falha no meio) e I9 (CAS).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::Path;

use freitask_core::plano::Efeito;
use freitask_vault::ErroEscrita;
use freitask_vault::disco::{Falhando, Real};
use freitask_vault::escrita::{aplicar, gravar};
use proptest::prelude::*;

/// Os arquivos (relativo → bytes) de uma árvore, sem pastas.
fn arvore(raiz: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut pilha = vec![raiz.to_path_buf()];
    while let Some(dir) = pilha.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                pilha.push(p);
            } else {
                let rel = p.strip_prefix(raiz).unwrap().to_string_lossy().into_owned();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    out
}

fn bytes() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        Just(b"> [!todo] X\r\n> [[x]]\r\n".to_vec()),
        Just(b"\xEF\xBB\xBFsem fim".to_vec()),
        proptest::collection::vec(any::<u8>(), 0..40),
    ]
}

proptest! {
    /// I8: um plano de escritas sobre arquivos que existem ou não, com uma
    /// falha injetada num passo qualquer. Depois, cada arquivo tem o
    /// conteúdo de antes ou o de depois (nunca outro, nunca pela metade);
    /// os efeitos antes do que falhou aconteceram, os de depois não; e não
    /// sobra temporário.
    #[test]
    fn i8_falha_em_qualquer_passo_deixa_cada_arquivo_antigo_ou_novo(
        arquivos in proptest::collection::vec((any::<bool>(), bytes(), bytes()), 1..6),
        passo in 1usize..30,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let mut plano = vec![Efeito::CriarDir { rel: "p/tasks".into() }];
        std::fs::create_dir_all(dir.path().join("p/tasks")).unwrap();
        let mut esperado_antes = BTreeMap::new();
        let mut esperado_depois = BTreeMap::new();
        for (i, (existe, antes, depois)) in arquivos.iter().enumerate() {
            let rel = format!("p/tasks/t{i}.md");
            let antes = existe.then(|| antes.clone());
            if let Some(a) = &antes {
                std::fs::write(dir.path().join(&rel), a).unwrap();
                esperado_antes.insert(rel.clone(), a.clone());
            }
            esperado_depois.insert(rel.clone(), depois.clone());
            plano.push(Efeito::Escrever { rel, antes, depois: depois.clone() });
        }
        let f = Falhando::no_passo(&Real, passo);
        let r = aplicar(&f, dir.path(), &plano);
        let agora = arvore(dir.path());
        for (rel, b) in &agora {
            prop_assert!(!rel.contains(".freitask."), "temporário sobrou: {rel}");
            let velho = esperado_antes.get(rel);
            let novo = esperado_depois.get(rel);
            prop_assert!(Some(b) == velho || Some(b) == novo, "{rel} ficou com outro conteúdo");
        }
        if r.is_ok() {
            prop_assert_eq!(agora, esperado_depois);
        } else {
            // Os efeitos são feitos em ordem: depois do primeiro arquivo que
            // ficou antigo (e que mudaria), nenhum ficou novo.
            let mut viu_antigo = false;
            for i in 0..arquivos.len() {
                let rel = format!("p/tasks/t{i}.md");
                let novo = esperado_depois.get(&rel);
                let velho = esperado_antes.get(&rel);
                if novo == velho {
                    continue;
                }
                let ficou_novo = agora.get(&rel) == novo;
                prop_assert!(!(viu_antigo && ficou_novo), "{rel} gravado depois de uma falha");
                viu_antigo |= !ficou_novo;
            }
        }
    }

    /// I9: se o destino mudou desde a leitura, sai conflito e nada é
    /// gravado; se não mudou, grava (ou não precisa).
    #[test]
    fn i9_cas_so_grava_sobre_o_que_foi_lido(
        lido in proptest::option::of(bytes()),
        no_disco in proptest::option::of(bytes()),
        depois in bytes(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        if let Some(b) = &no_disco {
            std::fs::write(&destino, b).unwrap();
        }
        let r = gravar(&Real, &destino, lido.as_deref(), &depois);
        let agora = std::fs::read(&destino).ok();
        if lido.as_deref() == Some(depois.as_slice()) {
            prop_assert!(!r.unwrap());
            prop_assert_eq!(agora, no_disco);
        } else if lido == no_disco {
            prop_assert!(r.unwrap());
            prop_assert_eq!(agora, Some(depois));
        } else {
            prop_assert!(matches!(r, Err(ErroEscrita::Conflito(_))));
            prop_assert_eq!(agora, no_disco);
        }
        prop_assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), usize::from(destino.exists()));
    }
}
