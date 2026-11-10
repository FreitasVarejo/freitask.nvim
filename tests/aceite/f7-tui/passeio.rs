//! Um passeio de teclas pseudoaleatório e determinístico (spec §3.5, camada
//! 2, vista de fora): sobre vaults sintéticos do `gerador`, teclas de toda
//! espécie e a tela desenhada a cada passo num tamanho de 20×5 a 200×60. Sem
//! pânico, `Esc` volta ao modo normal em no máximo dois passos, e nada é
//! gravado fora do HOME temporário.
//!
//! O gerador de números é um LCG próprio (sem `proptest` no aceite): a mesma
//! semente dá o mesmo passeio, e a falha diz a semente, o passo e as teclas.

use std::collections::BTreeSet;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use crate::util::*;
use freitask::tui::{Pedido, Sessao};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

/// LCG de 64 bits (constantes do MMIX, Knuth).
struct Lcg(u64);

impl Lcg {
    fn novo(semente: u64) -> Lcg {
        Lcg(semente ^ 0x5DEE_CE66_D1CE_4E5B)
    }

    fn proximo(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// Um inteiro em `0..n` (n > 0).
    fn ate(&mut self, n: usize) -> usize {
        (self.proximo() % n as u64) as usize
    }

    fn escolhe<T: Copy>(&mut self, itens: &[T]) -> T {
        itens[self.ate(itens.len())]
    }
}

/// As teclas de comando (as da ajuda, e `y` das confirmações).
const COMANDOS: [char; 23] = [
    'j', 'k', 'h', 'l', 'g', 'G', 'n', 's', 't', 'i', 'r', 'a', 'A', 'S', 'd', 'N', 'c', '/', '?',
    'e', 'R', 'y', 'q',
];

/// Letras para os campos de texto (com acento, espaço e hífen).
const TEXTO: [char; 9] = ['a', 'b', 'x', ' ', 'Á', 'ç', '-', '1', 'Z'];

const ESPECIAIS: [KeyCode; 11] = [
    KeyCode::Esc,
    KeyCode::Enter,
    KeyCode::Tab,
    KeyCode::BackTab,
    KeyCode::Backspace,
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Home,
    KeyCode::End,
];

fn tecla_aleatoria(r: &mut Lcg) -> KeyEvent {
    match r.ate(100) {
        0..=54 => tecla(r.escolhe(&COMANDOS)),
        55..=74 => tecla(r.escolhe(&TEXTO)),
        75..=94 => especial(r.escolhe(&ESPECIAIS)),
        _ => ctrl(r.escolhe(&['d', 'u', 'c'])),
    }
}

/// Os nomes das entradas diretas de um diretório.
fn entradas(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| {
            e.expect("entrada")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// `Esc` duas vezes leva ao modo normal de qualquer modo: ali `?` abre a
/// ajuda, e `Esc` a fecha.
fn esc_volta_ao_normal(s: &mut Sessao, contexto: &str) {
    esc(s);
    esc(s);
    aperta(s, tecla('?'));
    let t = tela(s);
    assert!(
        t.contem("Teclas"),
        "{contexto}: depois de dois Esc, `?` tinha de abrir a ajuda:\n{}",
        t.texto()
    );
    esc(s);
    assert!(!tela(s).contem("Teclas"), "{contexto}: Esc fecha a ajuda");
}

fn passeia(semente: u64, passos: usize) {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    let fora = base.path().join("fora");
    fs::create_dir_all(&fora).expect("fora");
    fs::write(fora.join("sentinela.md"), "não me toque\n").expect("sentinela");
    gerador::sintetico(semente, &gerador::Opcoes::default())
        .gravar(&vault(&home))
        .expect("gravar o vault sintético");
    let foto_fora = fotografa_dir(&fora);
    let cwd = std::env::current_dir().expect("cwd");
    let cwd_antes = entradas(&cwd);

    let mut r = Lcg::novo(semente);
    let mut s = abre(&home);
    let mut historico: Vec<KeyEvent> = Vec::new();
    for passo in 0..passos {
        let k = tecla_aleatoria(&mut r);
        historico.push(k);
        let pedido = catch_unwind(AssertUnwindSafe(|| s.tecla(k))).unwrap_or_else(|_| {
            panic!("semente {semente}, passo {passo}: pânico na tecla {k:?}; teclas: {historico:?}")
        });
        if pedido == Pedido::Sair {
            s = abre(&home);
        }
        let (w, h) = (20 + r.ate(181) as u16, 5 + r.ate(56) as u16);
        let buf = catch_unwind(AssertUnwindSafe(|| s.tela(w, h))).unwrap_or_else(|_| {
            panic!("semente {semente}, passo {passo}: pânico ao desenhar em {w}×{h}; teclas: {historico:?}")
        });
        assert_eq!(
            (buf.area.width, buf.area.height),
            (w, h),
            "semente {semente}, passo {passo}"
        );
        if passo % 25 == 24 {
            esc_volta_ao_normal(
                &mut s,
                &format!("semente {semente}, passo {passo}, teclas {historico:?}"),
            );
        }
    }
    esc(&mut s);
    esc(&mut s);
    assert_eq!(
        s.tecla(tecla('q')),
        Pedido::Sair,
        "semente {semente}: depois de dois Esc, q sai"
    );

    assert_eq!(
        entradas(base.path()),
        ["fora", "home"].iter().map(|s| s.to_string()).collect(),
        "semente {semente}: nada nasce ao lado do HOME"
    );
    assert_eq!(
        fotografa_dir(&fora),
        foto_fora,
        "semente {semente}: nada muda fora do HOME"
    );
    assert_eq!(
        entradas(&cwd),
        cwd_antes,
        "semente {semente}: nada nasce no diretório corrente"
    );
}

#[test]
fn passeio_de_teclas_nos_vaults_sinteticos_nao_entra_em_panico_nem_sai_do_home() {
    for semente in 1..=6 {
        passeia(semente, 300);
    }
}

#[test]
fn passeio_de_teclas_no_vault_de_base_nao_entra_em_panico() {
    let home = home_base();
    let mut r = Lcg::novo(42);
    let mut s = abre(home.path());
    for passo in 0..400 {
        let k = tecla_aleatoria(&mut r);
        if s.tecla(k) == Pedido::Sair {
            s = abre(home.path());
        }
        for (w, h) in [(20, 5), (200, 60)] {
            let _ = s.tela(w, h);
        }
        if passo % 40 == 39 {
            esc_volta_ao_normal(&mut s, &format!("vault de base, passo {passo}"));
        }
    }
}
