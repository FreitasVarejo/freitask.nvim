//! O binário de verdade num pseudoterminal (spec §3.5, camada 4, e decisão
//! f7.1): o que só o laço prova — desenha os três painéis, sai com 0 e devolve
//! o terminal (fora da tela alternativa, cursor visível), está em modo raw
//! (`Ctrl-C` é tecla, não sinal), abre a task no Neovim pai por `$NVIM` e,
//! fora dele, roda o `$EDITOR` e volta com o painel regenerado
//! (`tui/src/main.rs`, `open`).
//!
//! `portable-pty` dá o terminal e `vt100` o lê. Espera-se por texto na tela
//! ou pelo fim do processo com prazo, sondando; nunca `sleep` como
//! sincronização. Ambiente: `env_clear()` e só `HOME`, `PATH`, `LANG`,
//! `LC_ALL`, `FREITASK_TODAY` (mais `NVIM`/`EDITOR` onde o teste os põe).

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};

use crate::util::*;

const LINHAS: u16 = 30;
const COLUNAS: u16 = 100;
const PRAZO: Duration = Duration::from_secs(20);

/// Escrever um executável e dar `exec` nele com outra thread fazendo `fork`
/// no meio da escrita dá `ETXTBSY` (o filho herda o descritor aberto até o
/// seu `exec`). Quem escreve script e quem lança processo tomam esta trava
/// (a técnica de `crates/freitask/tests/current.rs`).
static TRAVA: Mutex<()> = Mutex::new(());

/// Grava um script executável (sob a trava, por quem o chama).
fn script(caminho: &Path, corpo: &str) {
    fs::write(caminho, corpo).expect("gravar script");
    fs::set_permissions(caminho, fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// A TUI rodando num pseudoterminal.
struct Tui {
    tela: Arc<Mutex<vt100::Parser>>,
    fim_da_saida: Arc<AtomicBool>,
    filho: Box<dyn Child + Send + Sync>,
    escritor: Box<dyn Write + Send>,
    _mestre: Box<dyn MasterPty + Send>,
}

impl Tui {
    /// Lança `freitask <args>` no terminal. `antes` roda sob a trava, antes
    /// do `spawn`, para gravar os scripts falsos.
    fn abre(
        home: &Path,
        args: &[&str],
        path: &str,
        extra: &[(&str, String)],
        antes: impl FnOnce(),
    ) -> Tui {
        let par = native_pty_system()
            .openpty(PtySize {
                rows: LINHAS,
                cols: COLUNAS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");
        let mut cmd = CommandBuilder::new(binario());
        cmd.args(args);
        cmd.env_clear();
        cmd.env("HOME", home);
        cmd.env("PATH", path);
        cmd.env("LANG", "C.UTF-8");
        cmd.env("LC_ALL", "C.UTF-8");
        cmd.env("FREITASK_TODAY", HOJE);
        for (k, v) in extra {
            cmd.env(k, v);
        }
        cmd.cwd(home);
        let filho = {
            let _trava = TRAVA.lock().unwrap_or_else(|e| e.into_inner());
            antes();
            par.slave
                .spawn_command(cmd)
                .expect("lançar o freitask no pty")
        };
        drop(par.slave);
        let mut leitor = par.master.try_clone_reader().expect("leitor do pty");
        let escritor = par.master.take_writer().expect("escritor do pty");
        let tela = Arc::new(Mutex::new(vt100::Parser::new(LINHAS, COLUNAS, 0)));
        let fim_da_saida = Arc::new(AtomicBool::new(false));
        {
            let tela = Arc::clone(&tela);
            let fim = Arc::clone(&fim_da_saida);
            std::thread::spawn(move || {
                let mut b = [0u8; 8192];
                loop {
                    match leitor.read(&mut b) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => tela
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .process(&b[..n]),
                    }
                }
                fim.store(true, Ordering::SeqCst);
            });
        }
        Tui {
            tela,
            fim_da_saida,
            filho,
            escritor,
            _mestre: par.master,
        }
    }

    fn conteudo(&self) -> String {
        self.tela
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .screen()
            .contents()
    }

    fn tela_alternativa(&self) -> bool {
        self.tela
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .screen()
            .alternate_screen()
    }

    fn cursor_escondido(&self) -> bool {
        self.tela
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .screen()
            .hide_cursor()
    }

    /// Espera até a tela satisfazer `cond`; devolve a tela.
    fn espera_que(&mut self, o_que: &str, cond: impl Fn(&str) -> bool) -> String {
        let limite = Instant::now() + PRAZO;
        loop {
            let c = self.conteudo();
            if cond(&c) {
                return c;
            }
            if let Ok(Some(st)) = self.filho.try_wait() {
                panic!("o freitask saiu ({st:?}) antes de {o_que}; tela:\n{c}");
            }
            if Instant::now() >= limite {
                let _ = self.filho.kill();
                panic!("a tela não chegou a {o_que} em {PRAZO:?}; tela:\n{c}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Espera até a tela conter todos os textos; devolve a tela.
    fn espera(&mut self, textos: &[&str]) -> String {
        self.espera_que(&format!("mostrar {textos:?}"), |c| {
            textos.iter().all(|t| c.contains(t))
        })
    }

    fn envia(&mut self, bytes: &[u8]) {
        self.escritor.write_all(bytes).expect("escrever no pty");
        self.escritor.flush().expect("flush do pty");
    }

    /// Espera o processo sair e a saída dele acabar de chegar.
    fn espera_sair(&mut self) -> ExitStatus {
        let limite = Instant::now() + PRAZO;
        let st = loop {
            if let Some(st) = self.filho.try_wait().expect("try_wait") {
                break st;
            }
            if Instant::now() >= limite {
                let _ = self.filho.kill();
                panic!(
                    "o freitask não saiu em {PRAZO:?}; tela:\n{}",
                    self.conteudo()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let limite = Instant::now() + PRAZO;
        while !self.fim_da_saida.load(Ordering::SeqCst) {
            assert!(
                Instant::now() < limite,
                "a saída do pty não terminou depois do fim do processo"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        st
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if let Ok(None) = self.filho.try_wait() {
            let _ = self.filho.kill();
            let _ = self.filho.wait();
        }
    }
}

/// Espera (sondando, com prazo) o arquivo do vault conter `trecho`.
fn espera_arquivo(home: &Path, rel: &str, trecho: &str) {
    let limite = Instant::now() + PRAZO;
    loop {
        let c = fs::read_to_string(abs(home, rel)).unwrap_or_default();
        if c.contains(trecho) {
            return;
        }
        assert!(
            Instant::now() < limite,
            "{rel} não chegou a conter {trecho:?}:\n{c}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn path_do_sistema() -> String {
    std::env::var("PATH").unwrap_or_default()
}

/// Afirma saída 0 e o terminal devolvido.
fn saiu_com_0_e_devolveu_o_terminal(t: &mut Tui) {
    let st = t.espera_sair();
    assert!(st.success(), "o freitask tinha de sair com 0: {st:?}");
    assert_eq!(st.exit_code(), 0);
    assert!(
        !t.tela_alternativa(),
        "o terminal ficou na tela alternativa"
    );
    assert!(!t.cursor_escondido(), "o cursor ficou escondido");
}

#[test]
fn pty_sem_argumentos_regenera_o_painel_desenha_os_tres_paineis_e_q_sai_com_0() {
    let home = home_base();
    let mut t = Tui::abre(home.path(), &[], &path_do_sistema(), &[], || {});
    let tela = t.espera(&["Projetos", "Tasks: alfa", "Preview", "escrever-spec"]);
    assert!(tela.contains("Escrever a spec"), "{tela}");
    assert!(t.tela_alternativa(), "a TUI desenha na tela alternativa");
    assert_eq!(
        le(home.path(), CURRENT),
        PAINEL_BASE,
        "o rebuild roda ao abrir"
    );
    t.envia(b"q");
    saiu_com_0_e_devolveu_o_terminal(&mut t);
}

#[test]
fn pty_tui_esta_em_modo_raw_e_ctrl_c_sai_com_0() {
    // Em modo raw `Ctrl-C` chega como tecla; fora dele seria SIGINT e o
    // processo morreria por sinal, sem código 0.
    let home = home_base();
    let mut t = Tui::abre(home.path(), &["tui"], &path_do_sistema(), &[], || {});
    t.espera(&["Projetos", "Tasks: alfa", "Preview"]);
    t.envia(b"\x03");
    saiu_com_0_e_devolveu_o_terminal(&mut t);
}

#[test]
fn pty_vault_sintetico_desenha_e_sai() {
    let home = tempfile::tempdir().expect("tempdir");
    gerador::sintetico(3, &gerador::Opcoes::default())
        .gravar(&vault(home.path()))
        .expect("gravar o vault sintético");
    let mut t = Tui::abre(home.path(), &[], &path_do_sistema(), &[], || {});
    t.espera(&["Projetos", "Tasks", "Preview"]);
    t.envia(b"?");
    t.espera(&["Teclas"]);
    // Na ajuda, qualquer tecla só a fecha (um `Esc` solto seguido de `q`
    // poderia chegar junto e virar Alt-q).
    t.envia(b"x");
    t.espera_que("fechar a ajuda", |c| !c.contains("Teclas"));
    t.envia(b"q");
    saiu_com_0_e_devolveu_o_terminal(&mut t);
}

#[test]
fn pty_dentro_do_neovim_enter_abre_a_task_no_pai_por_nvim_server_e_sai_com_0() {
    // Com `$NVIM` (o socket do Neovim pai), Enter roda `nvim --server <sock>
    // --remote-expr <expr>` com o caminho da task na expressão, e a TUI sai
    // (`tui/src/main.rs`, `open`). O `nvim` falso grava os argumentos
    // separados por NUL.
    let home = home_base();
    let bin = tempfile::tempdir().expect("tempdir");
    let args = home.path().join("nvim-args");
    let sock = home.path().join("nvim.sock");
    let path = format!("{}:{}", bin.path().display(), path_do_sistema());
    let nvim = bin.path().join("nvim");
    let corpo = format!(
        "#!/bin/sh\nprintf '%s\\0' \"$@\" > '{}'\nexit 0\n",
        args.display()
    );
    let mut t = Tui::abre(
        home.path(),
        &[],
        &path,
        &[("NVIM", sock.display().to_string())],
        || script(&nvim, &corpo),
    );
    t.espera(&["Projetos", "Tasks: alfa", "medir-tempo"]);
    t.envia(b"lj\r");
    saiu_com_0_e_devolveu_o_terminal(&mut t);

    let bruto = fs::read(&args).expect("o nvim falso não foi chamado");
    let texto = String::from_utf8(bruto).expect("utf-8");
    let partes: Vec<&str> = texto.trim_end_matches('\0').split('\0').collect();
    assert_eq!(partes.len(), 4, "argumentos do nvim: {partes:?}");
    assert_eq!(partes[0], "--server");
    assert_eq!(partes[1], sock.display().to_string());
    assert_eq!(partes[2], "--remote-expr");
    let caminho = abs(home.path(), MEDIR).display().to_string();
    assert!(
        partes[3].contains(&caminho),
        "a expressão tem de levar o caminho da task {caminho:?}: {:?}",
        partes[3]
    );
    assert_eq!(le(home.path(), MEDIR), MEDIR_MD, "abrir não grava a task");
}

#[test]
fn pty_fora_do_neovim_enter_roda_o_editor_e_volta_com_a_edicao_e_o_painel_regenerado() {
    // Sem `$NVIM`, Enter roda o `$EDITOR` com o caminho da task, em primeiro
    // plano; na volta a TUI regenera o `CURRENT.md` e relê o vault.
    let home = home_base();
    let bin = tempfile::tempdir().expect("tempdir");
    let editor = bin.path().join("editor-falso");
    let args = home.path().join("editor-args");
    let corpo = format!(
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$@\" > '{}'\n",
            "printf '> [!example] Medir o tempo editado\\n> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\\n' > \"$1\"\n",
            "exit 0\n",
        ),
        args.display()
    );
    let mut t = Tui::abre(
        home.path(),
        &[],
        &path_do_sistema(),
        &[("EDITOR", editor.display().to_string())],
        || script(&editor, &corpo),
    );
    t.espera(&["Projetos", "Tasks: alfa", "medir-tempo"]);
    t.envia(b"lj\r");
    t.espera(&["Medir o tempo editado", "Projetos", "Preview"]);
    assert_eq!(
        fs::read_to_string(&args).expect("o editor falso não foi chamado"),
        format!("{}\n", abs(home.path(), MEDIR).display())
    );
    espera_arquivo(
        home.path(),
        CURRENT,
        "> [!example] Medir o tempo editado\n> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\n",
    );
    t.envia(b"q");
    saiu_com_0_e_devolveu_o_terminal(&mut t);
}
