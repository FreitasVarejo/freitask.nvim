//! O lock entre dois `freitask` na mesma máquina (spec §1.5): os hooks do
//! Claude Code chamam a CLI a cada comando de shell, e duas escritas
//! intercaladas perderiam uma delas. Só os comandos que escrevem o tomam;
//! leitura nunca espera. Mora fora do vault, para o Syncthing não o ver.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustix::fs::{FlockOperation, flock};

/// Quanto um comando que escreve espera pelo lock antes de sair 3.
pub const ESPERA: Duration = Duration::from_secs(5);

/// Onde o lock mora: `$XDG_RUNTIME_DIR/freitask.lock` ou, sem a variável,
/// `~/.local/state/freitask/lock`. `None` sem nenhum dos dois.
pub fn caminho(xdg_runtime_dir: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    if let Some(x) = xdg_runtime_dir.filter(|x| !x.is_empty()) {
        return Some(Path::new(x).join("freitask.lock"));
    }
    home.filter(|h| !h.is_empty())
        .map(|h| Path::new(h).join(".local/state/freitask/lock"))
}

/// O lock tomado. Solta ao sair de escopo (o kernel solta o `flock` quando o
/// descritor fecha, inclusive se o processo morrer).
#[derive(Debug)]
pub struct Trava {
    _arquivo: std::fs::File,
}

#[derive(Debug)]
pub enum ErroTrava {
    /// Outro processo segurou o lock por mais que a espera.
    Ocupada(PathBuf),
    Io(PathBuf, std::io::Error),
}

impl ErroTrava {
    /// Ocupada é a saída 3 (repetir é seguro); erro de I/O é 1.
    pub fn codigo(&self) -> u8 {
        match self {
            ErroTrava::Ocupada(_) => 3,
            ErroTrava::Io(..) => 1,
        }
    }
}

impl std::fmt::Display for ErroTrava {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroTrava::Ocupada(p) => write!(
                f,
                "outro freitask está gravando (lock {} ocupado há mais de {} s); nada foi gravado, repita o comando",
                p.display(),
                ESPERA.as_secs()
            ),
            ErroTrava::Io(p, e) => write!(f, "não consegui abrir o lock {}: {e}", p.display()),
        }
    }
}

/// Toma o lock exclusivo em `caminho`, esperando até `espera`.
pub fn tomar(caminho: &Path, espera: Duration) -> Result<Trava, ErroTrava> {
    let io = |e| ErroTrava::Io(caminho.to_path_buf(), e);
    if let Some(pai) = caminho.parent() {
        std::fs::create_dir_all(pai).map_err(io)?;
    }
    let arquivo = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(caminho)
        .map_err(io)?;
    let inicio = Instant::now();
    loop {
        match flock(&arquivo, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => return Ok(Trava { _arquivo: arquivo }),
            Err(e) if e == rustix::io::Errno::WOULDBLOCK || e == rustix::io::Errno::INTR => {
                if inicio.elapsed() >= espera {
                    return Err(ErroTrava::Ocupada(caminho.to_path_buf()));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(io(e.into())),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn caminho_prefere_xdg_e_cai_no_local_state() {
        assert_eq!(
            caminho(Some("/run/user/1"), Some("/h")),
            Some(PathBuf::from("/run/user/1/freitask.lock"))
        );
        assert_eq!(
            caminho(Some(""), Some("/h")),
            Some(PathBuf::from("/h/.local/state/freitask/lock"))
        );
        assert_eq!(
            caminho(None, Some("/h")),
            Some(PathBuf::from("/h/.local/state/freitask/lock"))
        );
        assert_eq!(caminho(None, None), None);
        assert_eq!(caminho(None, Some("")), None);
    }

    #[test]
    fn segundo_tomador_espera_e_desiste_e_o_primeiro_solta_ao_sair() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub/lock");
        let primeiro = tomar(&p, ESPERA).unwrap();
        let inicio = Instant::now();
        let e = tomar(&p, Duration::from_millis(200)).unwrap_err();
        assert!(inicio.elapsed() >= Duration::from_millis(200));
        assert!(matches!(&e, ErroTrava::Ocupada(q) if q == &p));
        assert_eq!(e.codigo(), 3);
        assert!(e.to_string().contains("ocupado há mais de 5 s"));
        drop(primeiro);
        tomar(&p, Duration::from_millis(200)).unwrap();
    }

    #[test]
    fn erro_de_io_e_saida_1() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("arquivo"), b"").unwrap();
        // O pai do lock é um arquivo: não dá para criar a pasta.
        let e = tomar(&dir.path().join("arquivo/lock"), ESPERA).unwrap_err();
        assert!(matches!(e, ErroTrava::Io(..)));
        assert_eq!(e.codigo(), 1);
        assert!(e.to_string().starts_with("não consegui abrir o lock "));
    }
}
