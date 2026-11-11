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
    /// Dá a `para` as permissões de `de`, para o `rename` do temporário não
    /// trocar o modo do arquivo que substitui (o que `escrever` cria sai com
    /// o modo do umask). Se `de` não existe, não faz nada.
    fn herdar_permissoes(&self, de: &Path, para: &Path) -> io::Result<()>;
    /// `fsync` do arquivo.
    fn fsync(&self, p: &Path) -> io::Result<()>;
    fn renomear(&self, de: &Path, para: &Path) -> io::Result<()>;
    /// `rename` que nunca sobrescreve: `AlreadyExists` se `para` já existe.
    fn mover(&self, de: &Path, para: &Path) -> io::Result<()>;
    /// `rename` de uma PASTA que nunca sobrescreve nem junta: `AlreadyExists`
    /// se `para` já existe.
    fn mover_pasta(&self, de: &Path, para: &Path) -> io::Result<()>;
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

    fn herdar_permissoes(&self, de: &Path, para: &Path) -> io::Result<()> {
        match std::fs::metadata(de) {
            Ok(m) => std::fs::set_permissions(para, m.permissions()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn fsync(&self, p: &Path) -> io::Result<()> {
        std::fs::File::open(p)?.sync_all()
    }

    fn renomear(&self, de: &Path, para: &Path) -> io::Result<()> {
        std::fs::rename(de, para)
    }

    fn mover(&self, de: &Path, para: &Path) -> io::Result<()> {
        use rustix::fs::{CWD, RenameFlags, renameat_with};
        match renameat_with(CWD, de, CWD, para, RenameFlags::NOREPLACE) {
            Ok(()) => Ok(()),
            // Sistema de arquivos sem `RENAME_NOREPLACE`: o `link` também
            // recusa destino que existe, e o `unlink` tira o nome velho.
            Err(e) if e == rustix::io::Errno::INVAL || e == rustix::io::Errno::NOSYS => {
                std::fs::hard_link(de, para)?;
                std::fs::remove_file(de)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn mover_pasta(&self, de: &Path, para: &Path) -> io::Result<()> {
        use rustix::fs::{CWD, RenameFlags, renameat_with};
        match renameat_with(CWD, de, CWD, para, RenameFlags::NOREPLACE) {
            Ok(()) => Ok(()),
            Err(e) if e == rustix::io::Errno::INVAL || e == rustix::io::Errno::NOSYS => {
                mover_pasta_sem_noreplace(de, para)
            }
            Err(e) => Err(e.into()),
        }
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

/// O `mover_pasta` num sistema de arquivos sem `RENAME_NOREPLACE`: pasta não
/// tem `link`, então confere que o destino não existe e faz o `rename`. Entre
/// os dois passos outro processo pode criar o destino (o lock só vale entre
/// dois `freitask`); o `rename` de uma pasta sobre uma pasta que não está
/// vazia falha, e sobre uma vazia a substitui sem perder nada.
pub fn mover_pasta_sem_noreplace(de: &Path, para: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(para).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} já existe", para.display()),
        ));
    }
    std::fs::rename(de, para)
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

    fn herdar_permissoes(&self, de: &Path, para: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.herdar_permissoes(de, para)
    }

    fn fsync(&self, p: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.fsync(p)
    }

    fn renomear(&self, de: &Path, para: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.renomear(de, para)
    }

    fn mover(&self, de: &Path, para: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.mover(de, para)
    }

    fn mover_pasta(&self, de: &Path, para: &Path) -> io::Result<()> {
        self.conta()?;
        self.dentro.mover_pasta(de, para)
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
        let c = dir.path().join("sub/c");
        Real.escrever(&c, b"c").unwrap();
        let e = Real.mover(&b, &c).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "nunca sobrescreve");
        assert_eq!(Real.ler(&c).unwrap(), Some(b"c".to_vec()));
        Real.apagar(&c).unwrap();
        Real.mover(&b, &c).unwrap();
        assert_eq!(Real.ler(&b).unwrap(), None);
        assert_eq!(Real.ler(&c).unwrap(), Some(b"x".to_vec()));
        Real.mover(&c, &b).unwrap();
        Real.apagar(&b).unwrap();
        assert_eq!(Real.ler(&b).unwrap(), None);
        assert!(
            Real.ler(dir.path()).is_err(),
            "pasta não se lê como arquivo"
        );
        assert!(Real.fsync(&b).is_err());
        assert!(Real.fsync_dir(&dir.path().join("nao")).is_err());
    }

    fn modo(p: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p).unwrap().permissions().mode() & 0o7777
    }

    fn com_modo(p: &Path, m: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(p, b"x").unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).unwrap();
    }

    #[test]
    fn herdar_permissoes_copia_o_modo_da_origem_e_nao_faz_nada_sem_origem() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        com_modo(&a, 0o600);
        com_modo(&b, 0o644);
        Real.herdar_permissoes(&a, &b).unwrap();
        assert_eq!((modo(&a), modo(&b)), (0o600, 0o600));
        com_modo(&a, 0o644);
        Real.herdar_permissoes(&a, &b).unwrap();
        assert_eq!(modo(&b), 0o644, "nos dois sentidos");
        // Sem origem (arquivo novo): o destino fica como está.
        com_modo(&b, 0o640);
        Real.herdar_permissoes(&dir.path().join("falta"), &b)
            .unwrap();
        assert_eq!(modo(&b), 0o640);
        // Destino que falta é erro; origem ilegível (não é NotFound) também.
        assert!(
            Real.herdar_permissoes(&a, &dir.path().join("falta"))
                .is_err()
        );
        assert!(
            Real.herdar_permissoes(&a.join("dentro"), &b).is_err(),
            "ENOTDIR não é 'não existe'"
        );
        assert_eq!(modo(&b), 0o640);
    }

    #[test]
    fn falhando_herda_as_permissoes_de_verdade_quando_nao_e_o_passo() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        com_modo(&a, 0o600);
        com_modo(&b, 0o644);
        let f = Falhando::no_passo(&Real, 2);
        f.herdar_permissoes(&a, &b).unwrap();
        assert_eq!((f.passos(), modo(&b)), (1, 0o600));
        assert!(f.herdar_permissoes(&a, &b).is_err(), "o passo 2 falha");
    }

    #[test]
    fn mover_pasta_leva_o_conteudo_e_nunca_junta_nem_sobrescreve() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        Real.criar_dir(&a.join("tasks")).unwrap();
        Real.escrever(&a.join("tasks/x.md"), b"x").unwrap();
        let b = dir.path().join("enc/b");
        Real.criar_dir(&dir.path().join("enc")).unwrap();
        Real.mover_pasta(&a, &b).unwrap();
        assert_eq!(
            Real.ler(&b.join("tasks/x.md")).unwrap(),
            Some(b"x".to_vec())
        );
        assert!(!a.exists());
        for ocupado in [dir.path().join("vazia"), dir.path().join("arq")] {
            if ocupado.ends_with("vazia") {
                Real.criar_dir(&ocupado).unwrap();
            } else {
                Real.escrever(&ocupado, b"o").unwrap();
            }
            let e = Real.mover_pasta(&b, &ocupado).unwrap_err();
            assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{ocupado:?}");
            let e = mover_pasta_sem_noreplace(&b, &ocupado).unwrap_err();
            assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{ocupado:?}");
            assert!(e.to_string().ends_with("já existe"));
            assert!(b.join("tasks/x.md").is_file(), "a origem fica");
        }
        let c = dir.path().join("c");
        mover_pasta_sem_noreplace(&b, &c).unwrap();
        assert_eq!(
            Real.ler(&c.join("tasks/x.md")).unwrap(),
            Some(b"x".to_vec())
        );
        assert!(!b.exists());
        assert!(
            Real.mover_pasta(&b, &dir.path().join("d")).is_err(),
            "origem que falta"
        );
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
        f.mover(&dir.path().join("b"), &dir.path().join("m"))
            .unwrap();
        f.apagar(&dir.path().join("m")).unwrap();
        assert_eq!(f.passos(), 8);
        let f = Falhando::no_passo(&Real, 1);
        assert!(
            f.mover_pasta(&dir.path().join("c"), &dir.path().join("c2"))
                .is_err()
        );
        f.mover_pasta(&dir.path().join("c"), &dir.path().join("c2"))
            .unwrap();
        assert_eq!(f.passos(), 2);
        assert!(dir.path().join("c2").is_dir());
        for passo in 1..=8 {
            let f = Falhando::no_passo(&Real, passo);
            let p = dir.path().join(format!("p{passo}"));
            let r = [
                f.escrever(&p, b"y"),
                f.herdar_permissoes(&p, &p),
                f.fsync(&p),
                f.renomear(&p, &dir.path().join(format!("q{passo}"))),
                f.fsync_dir(dir.path()),
                f.criar_dir(&dir.path().join(format!("d{passo}"))),
                f.mover(
                    &dir.path().join(format!("q{passo}")),
                    &dir.path().join(format!("m{passo}")),
                ),
                f.apagar(&dir.path().join(format!("m{passo}"))),
            ];
            // Antes do passo, tudo dá certo; no passo, a falha é a injetada
            // (depois dele, o disco real pode falhar por conta própria).
            assert!(r[..passo - 1].iter().all(Result::is_ok), "passo {passo}");
            let erro = r[passo - 1].as_ref().unwrap_err().to_string();
            assert_eq!(erro, format!("falha injetada no passo {passo}"));
        }
    }
}
