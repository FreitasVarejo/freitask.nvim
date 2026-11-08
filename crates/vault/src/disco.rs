//! O sistema de arquivos atrás de um trait, para a gravação poder ser testada
//! com uma falha injetada em cada passo (spec §3.2, I8): escrever o
//! temporário, `fsync`, `rename`, `fsync` do diretório, apagar, criar pasta.

use std::cell::Cell;
use std::io::{self, Write};
use std::path::Path;

/// Os passos de I/O que a gravação usa. Ler não muda nada; o resto conta como
/// passo para a injeção de falha.
pub trait Disco {
    /// Os bytes do arquivo, ou `None` se ele não existe.
    fn ler(&self, p: &Path) -> io::Result<Option<Vec<u8>>>;
    /// Cria (ou trunca) `p` com `bytes`.
    fn escrever(&self, p: &Path, bytes: &[u8]) -> io::Result<()>;
    /// `fsync` do arquivo.
    fn fsync(&self, p: &Path) -> io::Result<()>;
    fn renomear(&self, de: &Path, para: &Path) -> io::Result<()>;
    /// `fsync` do diretório, para o `rename` sobreviver a uma queda de luz.
    fn fsync_dir(&self, p: &Path) -> io::Result<()>;
    fn apagar(&self, p: &Path) -> io::Result<()>;
    /// Cria a pasta e as que faltarem acima.
    fn criar_dir(&self, p: &Path) -> io::Result<()>;
}

/// O disco de verdade.
#[derive(Debug, Clone, Copy, Default)]
pub struct Real;

impl Disco for Real {
    fn ler(&self, p: &Path) -> io::Result<Option<Vec<u8>>> {
        match std::fs::read(p) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn escrever(&self, p: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut f = std::fs::File::create(p)?;
        f.write_all(bytes)
    }

    fn fsync(&self, p: &Path) -> io::Result<()> {
        std::fs::File::open(p)?.sync_all()
    }

    fn renomear(&self, de: &Path, para: &Path) -> io::Result<()> {
        std::fs::rename(de, para)
    }

    fn fsync_dir(&self, p: &Path) -> io::Result<()> {
        std::fs::File::open(p)?.sync_all()
    }

    fn apagar(&self, p: &Path) -> io::Result<()> {
        std::fs::remove_file(p)
    }

    fn criar_dir(&self, p: &Path) -> io::Result<()> {
        std::fs::create_dir_all(p)
    }
}

/// Um disco que delega a outro e falha UMA vez, no passo de número `passo`
/// (contando de 1, só os passos que mudam algo). É o ponto de falha de I8.
pub struct Falhando<'a> {
    dentro: &'a dyn Disco,
    passo: usize,
    contados: Cell<usize>,
}

impl<'a> Falhando<'a> {
    pub fn no_passo(dentro: &'a dyn Disco, passo: usize) -> Falhando<'a> {
        Falhando {
            dentro,
            passo,
            contados: Cell::new(0),
        }
    }

    /// Quantos passos que mudam algo já foram pedidos.
    pub fn passos(&self) -> usize {
        self.contados.get()
    }

    fn conta(&self) -> io::Result<()> {
        let n = self.contados.get() + 1;
        self.contados.set(n);
        if n == self.passo {
            return Err(io::Error::other(format!("falha injetada no passo {n}")));
        }
        Ok(())
    }
}

impl Disco for Falhando<'_> {
    fn ler(&self, p: &Path) -> io::Result<Option<Vec<u8>>> {
        self.dentro.ler(p)
    }

    fn escrever(&self, p: &Path, bytes: &[u8]) -> io::Result<()> {
        self.conta()?;
        self.dentro.escrever(p, bytes)
    }

    fn fsync(&self, p: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.fsync(p)
    }

    fn renomear(&self, de: &Path, para: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.renomear(de, para)
    }

    fn fsync_dir(&self, p: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.fsync_dir(p)
    }

    fn apagar(&self, p: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.apagar(p)
    }

    fn criar_dir(&self, p: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.criar_dir(p)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn real_le_none_quando_falta_e_faz_cada_passo() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("sub/a");
        assert_eq!(Real.ler(&a).unwrap(), None);
        Real.criar_dir(&dir.path().join("sub")).unwrap();
        Real.escrever(&a, b"x").unwrap();
        Real.fsync(&a).unwrap();
        assert_eq!(Real.ler(&a).unwrap(), Some(b"x".to_vec()));
        let b = dir.path().join("sub/b");
        Real.renomear(&a, &b).unwrap();
        Real.fsync_dir(&dir.path().join("sub")).unwrap();
        assert_eq!(Real.ler(&a).unwrap(), None);
        Real.apagar(&b).unwrap();
        assert_eq!(Real.ler(&b).unwrap(), None);
        assert!(
            Real.ler(dir.path()).is_err(),
            "pasta não se lê como arquivo"
        );
        assert!(Real.fsync(&b).is_err());
        assert!(Real.fsync_dir(&dir.path().join("nao")).is_err());
    }

    #[test]
    fn falhando_falha_so_no_passo_pedido_e_conta_os_que_mudam() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let f = Falhando::no_passo(&Real, 2);
        f.escrever(&a, b"x").unwrap();
        f.ler(&a).unwrap();
        assert_eq!(f.passos(), 1, "ler não conta");
        assert!(f.fsync(&a).is_err());
        f.fsync(&a).unwrap();
        f.renomear(&a, &dir.path().join("b")).unwrap();
        f.fsync_dir(dir.path()).unwrap();
        f.criar_dir(&dir.path().join("c")).unwrap();
        f.apagar(&dir.path().join("b")).unwrap();
        assert_eq!(f.passos(), 7);
        for passo in 1..=6 {
            let f = Falhando::no_passo(&Real, passo);
            let p = dir.path().join(format!("p{passo}"));
            let r = [
                f.escrever(&p, b"y"),
                f.fsync(&p),
                f.renomear(&p, &dir.path().join(format!("q{passo}"))),
                f.fsync_dir(dir.path()),
                f.criar_dir(&dir.path().join(format!("d{passo}"))),
                f.apagar(&dir.path().join(format!("q{passo}"))),
            ];
            // Antes do passo, tudo dá certo; no passo, a falha é a injetada
            // (depois dele, o disco real pode falhar por conta própria).
            assert!(r[..passo - 1].iter().all(Result::is_ok), "passo {passo}");
            let erro = r[passo - 1].as_ref().unwrap_err().to_string();
            assert_eq!(erro, format!("falha injetada no passo {passo}"));
        }
    }
}
