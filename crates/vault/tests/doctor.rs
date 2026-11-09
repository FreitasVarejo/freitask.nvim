//! I3 (spec §3.2): `doctor --fix` é idempotente, e depois dele o `doctor` só
//! acusa o que não se repara sem decisão humana. Sobre vaults do gerador
//! gravados num diretório temporário, com o diagnóstico do `core` e o
//! executor de verdade (`Vault::aplicar`), como na CLI; o painel fica de fora
//! (é a I4).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use freitask_core::date::Date;
use freitask_core::doctor::{self, Diagnostico};
use freitask_vault::Vault;
use proptest::prelude::*;

/// Os achados que o `--fix` nunca repara: o `doctor.lua` também não os
/// repara, e cada um pede uma escolha (qual cópia vale, qual status, que
/// alvo o link devia ter).
const SEM_REPARO: &[&str] = &[
    "sync-conflict",
    "link-pendurado",
    "sem-callout",
    "fora-do-padrao",
    "id-duplicado",
    "status-0",
    "prioridade-desconhecida",
    "volta-em-invalido",
];

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

fn hoje() -> Date {
    Date::parse("2026-11-09").unwrap()
}

fn vault_do_gerador(semente: u64) -> (tempfile::TempDir, Vault) {
    let dir = tempfile::tempdir().unwrap();
    let raiz = dir.path().join("ObsidianVault");
    gerador::sintetico(semente, &gerador::Opcoes::default())
        .gravar(&raiz)
        .unwrap();
    (dir, Vault::em(raiz))
}

/// Um temporário órfão que o gerador não cria: o `--fix` o apaga.
fn com_temporario(v: &Vault, semente: u64) {
    if semente.is_multiple_of(3) {
        std::fs::write(
            v.raiz().join("projects").join(".freitask.x.md.4242.tmp"),
            b"meio escrito",
        )
        .unwrap();
    }
}

fn diagnostico(v: &Vault, fix: bool) -> Diagnostico {
    let (vocab, _) = v.vocab();
    doctor::diagnosticar(&vocab, &v.fotografia_do_doctor(), fix, hoje())
}

fn fix(v: &Vault) -> Diagnostico {
    let d = diagnostico(v, true);
    v.aplicar(&d.plano).unwrap();
    d
}

fn achados(d: &Diagnostico) -> BTreeSet<(String, String, String)> {
    d.achados
        .iter()
        .map(|a| (a.kind.to_string(), a.path.clone(), a.msg.clone()))
        .collect()
}

/// Sem isto a propriedade passaria num gerador que não produz nada para
/// reparar.
#[test]
fn o_fix_repara_algo_nos_vaults_do_gerador() {
    let mut reparados = BTreeSet::new();
    for semente in 0..30 {
        let (_dir, v) = vault_do_gerador(semente);
        com_temporario(&v, semente);
        let d = fix(&v);
        for a in d.achados.iter().filter(|a| a.fixed) {
            reparados.insert(a.kind);
        }
    }
    for kind in [
        "bloco-dessincronizado",
        "callout-vs-pasta",
        "temporario-orfao",
    ] {
        assert!(
            reparados.contains(kind),
            "nenhum {kind} reparado: {reparados:?}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn i3_fix_idempotente(semente in 0u64..1_000) {
        let (_dir, v) = vault_do_gerador(semente);
        com_temporario(&v, semente);
        let antes = diagnostico(&v, false);
        fix(&v);
        let uma = arvore(v.raiz());

        // fix(fix(v)) == fix(v): o segundo não tem o que fazer.
        let segundo = fix(&v);
        prop_assert!(segundo.plano.is_empty(), "plano no segundo fix: {:?}", segundo.plano);
        prop_assert!(segundo.achados.iter().all(|a| !a.fixed));
        prop_assert_eq!(&arvore(v.raiz()), &uma);

        // Depois do fix só resta o que não tem reparo, e o fix não cria
        // achado novo. Pode tirar um: o `link-pendurado` da linha do link no
        // bloco some quando o bloco é reserializado com o endereço certo.
        let depois = diagnostico(&v, false);
        for a in &depois.achados {
            prop_assert!(SEM_REPARO.contains(&a.kind), "{} sobrou depois do fix: {}", a.kind, a.path);
        }
        let sem_reparo = |d: &Diagnostico| -> BTreeSet<_> {
            achados(d).into_iter().filter(|(k, _, _)| SEM_REPARO.contains(&k.as_str())).collect()
        };
        let (depois, antes) = (sem_reparo(&depois), sem_reparo(&antes));
        prop_assert!(depois.is_subset(&antes), "achado novo depois do fix: {:?}", depois.difference(&antes).collect::<Vec<_>>());
    }
}
