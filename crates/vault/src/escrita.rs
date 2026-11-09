//! Gravação atômica com CAS (spec §1.5). Toda escrita no vault passa por
//! `gravar`:
//!
//! 1. o chamador leu `B0` (`antes`; `None` se o arquivo não existia);
//! 2. se `B1 == B0`, nada se grava e o mtime fica;
//! 3. escreve `.freitask.<nome>.<pid>.tmp` na mesma pasta e faz `fsync`;
//! 4. relê o destino; se o sha256 não é mais o de `B0`, apaga o temporário e
//!    devolve conflito (saída 3): nada foi gravado;
//! 5. `rename` e `fsync` do diretório.
//!
//! O Syncthing detecta mudança por mtime/tamanho e só vê o arquivo final; um
//! leitor nunca encontra o destino pela metade.

use std::path::{Path, PathBuf};

use freitask_core::plano::Efeito;
use sha2::{Digest, Sha256};

use crate::disco::Disco;

/// Por que uma gravação não aconteceu.
#[derive(Debug)]
pub enum ErroEscrita {
    /// O arquivo mudou entre a leitura e a gravação (CAS). Nada foi gravado;
    /// repetir o comando é seguro.
    Conflito(PathBuf),
    /// O disco recusou um passo.
    Io(PathBuf, std::io::Error),
}

impl ErroEscrita {
    /// O código de saída da CLI: 3 para conflito (spec §2.4), 1 para o resto.
    pub fn codigo(&self) -> u8 {
        match self {
            ErroEscrita::Conflito(_) => 3,
            ErroEscrita::Io(..) => 1,
        }
    }
}

impl std::fmt::Display for ErroEscrita {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroEscrita::Conflito(p) => write!(
                f,
                "{} mudou durante a gravação; nada foi gravado, repita o comando",
                p.display()
            ),
            ErroEscrita::Io(p, e) => write!(f, "não consegui gravar {}: {e}", p.display()),
        }
    }
}

fn hash(b: Option<&[u8]>) -> Option<[u8; 32]> {
    b.map(|b| Sha256::digest(b).into())
}

/// O temporário de `destino`: oculto, na mesma pasta (o `rename` não cruza
/// sistema de arquivos), com o pid para dois processos não colidirem.
pub fn temporario(destino: &Path) -> PathBuf {
    let pai = destino.parent().unwrap_or(Path::new("."));
    let nome = destino
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    pai.join(format!(".freitask.{nome}.{}.tmp", std::process::id()))
}

/// Grava `depois` em `destino` com CAS contra `antes`. Devolve se gravou.
pub fn gravar(
    disco: &dyn Disco,
    destino: &Path,
    antes: Option<&[u8]>,
    depois: &[u8],
) -> Result<bool, ErroEscrita> {
    if antes == Some(depois) {
        return Ok(false);
    }
    let io = |e| ErroEscrita::Io(destino.to_path_buf(), e);
    let tmp = temporario(destino);
    let desfaz = |e: ErroEscrita| {
        // O temporário pode nem ter nascido; o erro que importa é o original.
        let _ = disco.apagar(&tmp);
        e
    };
    disco.escrever(&tmp, depois).map_err(|e| desfaz(io(e)))?;
    disco.fsync(&tmp).map_err(|e| desfaz(io(e)))?;
    let atual = disco.ler(destino).map_err(|e| desfaz(io(e)))?;
    if hash(atual.as_deref()) != hash(antes) {
        return Err(desfaz(ErroEscrita::Conflito(destino.to_path_buf())));
    }
    disco.renomear(&tmp, destino).map_err(|e| desfaz(io(e)))?;
    sincroniza_pai(disco, destino);
    Ok(true)
}

/// `fsync` do diretório depois de `rename` ou `remove`. Uma falha aqui não
/// desfaz nada: o vault já está no estado novo, só sem a garantia de
/// durabilidade imediata.
fn sincroniza_pai(disco: &dyn Disco, p: &Path) {
    if let Some(pai) = p.parent() {
        let _ = disco.fsync_dir(pai);
    }
}

/// Apaga `destino` se ele ainda tem os bytes `antes`.
pub fn apagar(disco: &dyn Disco, destino: &Path, antes: &[u8]) -> Result<(), ErroEscrita> {
    let io = |e| ErroEscrita::Io(destino.to_path_buf(), e);
    let atual = disco.ler(destino).map_err(io)?;
    if hash(atual.as_deref()) != hash(Some(antes)) {
        return Err(ErroEscrita::Conflito(destino.to_path_buf()));
    }
    disco.apagar(destino).map_err(io)?;
    sincroniza_pai(disco, destino);
    Ok(())
}

/// Move `de` para `para` se `de` ainda tem os bytes `antes` e `para` não
/// existe. Um `rename` só: o arquivo nunca fica com os dois nomes nem com
/// nenhum. Exceção: num sistema de arquivos sem `RENAME_NOREPLACE`,
/// `Disco::mover` cai para `link` + `unlink`, e uma queda entre os dois deixa
/// os dois nomes (o `doctor` acusa o id duplicado).
pub fn mover(disco: &dyn Disco, de: &Path, para: &Path, antes: &[u8]) -> Result<(), ErroEscrita> {
    let atual = disco
        .ler(de)
        .map_err(|e| ErroEscrita::Io(de.to_path_buf(), e))?;
    if hash(atual.as_deref()) != hash(Some(antes)) {
        return Err(ErroEscrita::Conflito(de.to_path_buf()));
    }
    match disco.mover(de, para) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ErroEscrita::Conflito(para.to_path_buf()));
        }
        Err(e) => return Err(ErroEscrita::Io(para.to_path_buf(), e)),
    }
    sincroniza_pai(disco, para);
    if de.parent() != para.parent() {
        sincroniza_pai(disco, de);
    }
    Ok(())
}

/// Move a pasta `de` para `para` se `para` não existe. Um `rename` só: a
/// pasta nunca fica com os dois endereços nem com nenhum, e nunca se junta a
/// outra.
pub fn mover_pasta(disco: &dyn Disco, de: &Path, para: &Path) -> Result<(), ErroEscrita> {
    match disco.mover_pasta(de, para) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ErroEscrita::Conflito(para.to_path_buf()));
        }
        Err(e) => return Err(ErroEscrita::Io(de.to_path_buf(), e)),
    }
    sincroniza_pai(disco, para);
    if de.parent() != para.parent() {
        sincroniza_pai(disco, de);
    }
    Ok(())
}

/// Executa um plano do `core`, na ordem, com os caminhos relativos a `raiz`.
/// Para no primeiro erro: o que já foi feito fica (sem journal, spec §1.5), e
/// o estado intermediário é o que o `doctor` acusa e o `--fix` repara.
pub fn aplicar(disco: &dyn Disco, raiz: &Path, plano: &[Efeito]) -> Result<(), ErroEscrita> {
    for efeito in plano {
        match efeito {
            Efeito::CriarDir { rel } => {
                let p = raiz.join(rel);
                disco.criar_dir(&p).map_err(|e| ErroEscrita::Io(p, e))?;
            }
            Efeito::Escrever { rel, antes, depois } => {
                gravar(disco, &raiz.join(rel), antes.as_deref(), depois)?;
            }
            Efeito::Apagar { rel, antes } => apagar(disco, &raiz.join(rel), antes)?,
            Efeito::Mover { de, para, antes } => {
                mover(disco, &raiz.join(de), &raiz.join(para), antes)?;
            }
            Efeito::MoverPasta { de, para } => {
                mover_pasta(disco, &raiz.join(de), &raiz.join(para))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::disco::{Falhando, Real};

    fn ls(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn grava_arquivo_novo_e_nao_deixa_temporario() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        assert!(gravar(&Real, &destino, None, b"a\r\nb").unwrap());
        assert_eq!(std::fs::read(&destino).unwrap(), b"a\r\nb");
        assert_eq!(ls(dir.path()), ["x.md"]);
    }

    #[test]
    fn sem_mudanca_nao_grava_nem_toca_o_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        std::fs::write(&destino, b"igual").unwrap();
        let velho =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
        std::fs::File::options()
            .write(true)
            .open(&destino)
            .unwrap()
            .set_modified(velho)
            .unwrap();
        assert!(!gravar(&Real, &destino, Some(b"igual"), b"igual").unwrap());
        assert_eq!(
            std::fs::metadata(&destino).unwrap().modified().unwrap(),
            velho
        );
    }

    /// I9: o arquivo mudou entre a leitura e a gravação → conflito, nada
    /// gravado, nenhum temporário.
    #[test]
    fn cas_recusa_quando_o_arquivo_mudou_desde_a_leitura() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        std::fs::write(&destino, b"de outro escritor").unwrap();
        let e = gravar(&Real, &destino, Some(b"o que li"), b"novo").unwrap_err();
        assert!(matches!(&e, ErroEscrita::Conflito(p) if p == &destino));
        assert_eq!(e.codigo(), 3);
        assert_eq!(std::fs::read(&destino).unwrap(), b"de outro escritor");
        assert_eq!(ls(dir.path()), ["x.md"]);
        // Criar por cima de um arquivo que apareceu também é conflito.
        let e = gravar(&Real, &destino, None, b"novo").unwrap_err();
        assert!(matches!(e, ErroEscrita::Conflito(_)));
        // E sumir depois da leitura também.
        let sumiu = dir.path().join("sumiu.md");
        assert!(matches!(
            gravar(&Real, &sumiu, Some(b"x"), b"y").unwrap_err(),
            ErroEscrita::Conflito(_)
        ));
        assert_eq!(ls(dir.path()), ["x.md"]);
    }

    #[test]
    fn apagar_com_cas() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        std::fs::write(&destino, b"b").unwrap();
        assert!(matches!(
            apagar(&Real, &destino, b"outro").unwrap_err(),
            ErroEscrita::Conflito(_)
        ));
        assert!(destino.exists());
        apagar(&Real, &destino, b"b").unwrap();
        assert!(!destino.exists());
        assert!(matches!(
            apagar(&Real, &destino, b"b").unwrap_err(),
            ErroEscrita::Conflito(_)
        ));
    }

    /// I8: com a falha em cada passo de uma gravação, o arquivo fica com o
    /// conteúdo antigo ou com o novo, nunca pela metade, e nenhum temporário
    /// sobra (exceto quando a falha é no próprio `apagar` do temporário, que
    /// só acontece depois de uma falha anterior, e não há segunda falha).
    #[test]
    fn falha_em_cada_passo_deixa_o_antigo_ou_o_novo() {
        // Passos de uma gravação: escrever tmp (1), fsync (2), rename (3),
        // fsync dir (4). O 5 não existe: é a gravação sem falha nenhuma.
        for passo in 1..=5 {
            let dir = tempfile::tempdir().unwrap();
            let destino = dir.path().join("x.md");
            std::fs::write(&destino, b"antigo").unwrap();
            let f = Falhando::no_passo(&Real, passo);
            let r = gravar(&f, &destino, Some(b"antigo"), b"novo");
            let agora = std::fs::read(&destino).unwrap();
            match passo {
                1..=3 => {
                    assert!(matches!(r, Err(ErroEscrita::Io(..))), "passo {passo}");
                    assert_eq!(agora, b"antigo", "passo {passo}");
                }
                // 4: a falha no fsync do diretório chega depois do rename, o
                // arquivo já é o novo e a gravação conta como feita. 5: sem
                // falha.
                _ => {
                    assert!(r.unwrap(), "passo {passo}");
                    assert_eq!(agora, b"novo", "passo {passo}");
                }
            }
            assert_eq!(
                ls(dir.path()),
                ["x.md"],
                "temporário sobrou no passo {passo}"
            );
        }
    }

    #[test]
    fn falha_ao_apagar_deixa_o_arquivo() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        std::fs::write(&destino, b"b").unwrap();
        let f = Falhando::no_passo(&Real, 1);
        assert!(matches!(
            apagar(&f, &destino, b"b").unwrap_err(),
            ErroEscrita::Io(..)
        ));
        assert!(destino.exists());
        // Falha no fsync do diretório depois do remove: apagado mesmo assim.
        let f = Falhando::no_passo(&Real, 2);
        apagar(&f, &destino, b"b").unwrap();
        assert!(!destino.exists());
    }

    /// I8 entre arquivos: um plano com pasta e arquivo para no passo que
    /// falha; o que veio antes fica, o que vem depois não acontece.
    #[test]
    fn plano_para_no_primeiro_erro() {
        let plano = vec![
            Efeito::CriarDir {
                rel: "p/tasks".into(),
            },
            Efeito::Escrever {
                rel: "p/tasks/x.md".into(),
                antes: None,
                depois: b"x".to_vec(),
            },
            Efeito::Escrever {
                rel: "p/tasks/y.md".into(),
                antes: None,
                depois: b"y".to_vec(),
            },
        ];
        // Sem falha: tudo.
        let dir = tempfile::tempdir().unwrap();
        aplicar(&Real, dir.path(), &plano).unwrap();
        assert_eq!(ls(&dir.path().join("p/tasks")), ["x.md", "y.md"]);
        // Falha na pasta: nada.
        let dir = tempfile::tempdir().unwrap();
        let e = aplicar(&Falhando::no_passo(&Real, 1), dir.path(), &plano).unwrap_err();
        assert!(matches!(&e, ErroEscrita::Io(p, _) if p.ends_with("p/tasks")));
        assert_eq!(e.codigo(), 1);
        assert!(!dir.path().join("p").exists());
        // Falha no rename do primeiro arquivo (passo 4): só a pasta.
        let dir = tempfile::tempdir().unwrap();
        aplicar(&Falhando::no_passo(&Real, 4), dir.path(), &plano).unwrap_err();
        assert!(ls(&dir.path().join("p/tasks")).is_empty());
        // Falha no rename do segundo (passo 4 + 4 = 8): só o primeiro.
        let dir = tempfile::tempdir().unwrap();
        aplicar(&Falhando::no_passo(&Real, 8), dir.path(), &plano).unwrap_err();
        assert_eq!(ls(&dir.path().join("p/tasks")), ["x.md"]);
        // Apagar no plano.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("z.md"), b"z").unwrap();
        aplicar(
            &Real,
            dir.path(),
            &[Efeito::Apagar {
                rel: "z.md".into(),
                antes: b"z".to_vec(),
            }],
        )
        .unwrap();
        assert!(ls(dir.path()).is_empty());
    }

    /// `59875bf`: o rename de uma task é um `rename` só, e o conteúdo novo vem
    /// depois. Com uma falha injetada em cada passo, a task existe com UM
    /// nome só, e uma falha depois do `rename` deixa o conteúdo antigo com o
    /// nome novo — nunca os dois nomes, nunca nenhum.
    #[test]
    fn rename_e_atomico() {
        let plano = vec![
            Efeito::CriarDir {
                rel: "p/tasks/archived/done".into(),
            },
            Efeito::Mover {
                de: "p/tasks/x.md".into(),
                para: "p/tasks/archived/done/y.md".into(),
                antes: b"velho".to_vec(),
            },
            Efeito::Escrever {
                rel: "p/tasks/archived/done/y.md".into(),
                antes: Some(b"velho".to_vec()),
                depois: b"novo".to_vec(),
            },
        ];
        let mut viu_velho_no_novo = false;
        for passo in 1..=12 {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join("p/tasks")).unwrap();
            std::fs::write(dir.path().join("p/tasks/x.md"), b"velho").unwrap();
            let f = Falhando::no_passo(&Real, passo);
            let r = aplicar(&f, dir.path(), &plano);
            let x = std::fs::read(dir.path().join("p/tasks/x.md")).ok();
            let y = std::fs::read(dir.path().join("p/tasks/archived/done/y.md")).ok();
            match (&x, &y) {
                (Some(v), None) => assert_eq!(v, b"velho", "passo {passo}"),
                (None, Some(v)) => {
                    assert!(v == b"velho" || v == b"novo", "passo {passo}");
                    viu_velho_no_novo |= v == b"velho";
                    if r.is_ok() {
                        assert_eq!(v, b"novo", "passo {passo}");
                    }
                }
                _ => panic!("passo {passo}: x={x:?} y={y:?}"),
            }
            let done = dir.path().join("p/tasks/archived/done");
            let tmps = if done.is_dir() { ls(&done) } else { Vec::new() };
            assert!(tmps.iter().all(|n| !n.contains(".freitask.")), "{tmps:?}");
        }
        assert!(viu_velho_no_novo, "alguma falha cai depois do rename");
    }

    #[test]
    fn mover_pasta_recusa_destino_ocupado_e_sincroniza_as_duas_maes() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("projects/a");
        std::fs::create_dir_all(a.join("tasks")).unwrap();
        std::fs::write(a.join("tasks/x.md"), b"x").unwrap();
        let enc = dir.path().join("projects/_encerrados");
        std::fs::create_dir_all(enc.join("a")).unwrap();
        let e = mover_pasta(&Real, &a, &enc.join("a")).unwrap_err();
        assert!(matches!(&e, ErroEscrita::Conflito(p) if p == &enc.join("a")));
        assert_eq!(e.codigo(), 3);
        std::fs::remove_dir(enc.join("a")).unwrap();
        // Mães diferentes: o rename e o fsync de cada mãe.
        let f = Falhando::no_passo(&Real, 99);
        mover_pasta(&f, &a, &enc.join("a")).unwrap();
        assert_eq!(f.passos(), 3);
        assert_eq!(std::fs::read(enc.join("a/tasks/x.md")).unwrap(), b"x");
        // Mesma mãe: um fsync só.
        let f = Falhando::no_passo(&Real, 99);
        mover_pasta(&f, &enc.join("a"), &enc.join("b")).unwrap();
        assert_eq!(f.passos(), 2);
        // Falha no rename: nada muda, e é erro de I/O da origem.
        let f = Falhando::no_passo(&Real, 1);
        let e = mover_pasta(&f, &enc.join("b"), &a).unwrap_err();
        assert!(matches!(&e, ErroEscrita::Io(p, _) if p == &enc.join("b")));
        assert!(enc.join("b/tasks/x.md").is_file());
        // Falha no fsync depois do rename: movida mesmo assim.
        let f = Falhando::no_passo(&Real, 2);
        mover_pasta(&f, &enc.join("b"), &a).unwrap();
        assert!(a.join("tasks/x.md").is_file());
        aplicar(
            &Real,
            dir.path(),
            &[Efeito::MoverPasta {
                de: "projects/a".into(),
                para: "projects/_encerrados/a".into(),
            }],
        )
        .unwrap();
        assert!(enc.join("a/tasks/x.md").is_file());
    }

    #[test]
    fn mover_recusa_origem_mudada_e_destino_ocupado() {
        let dir = tempfile::tempdir().unwrap();
        let de = dir.path().join("x.md");
        let para = dir.path().join("y.md");
        std::fs::write(&de, b"outro").unwrap();
        let e = mover(&Real, &de, &para, b"lido").unwrap_err();
        assert!(matches!(&e, ErroEscrita::Conflito(p) if p == &de));
        std::fs::write(&para, b"ocupado").unwrap();
        let e = mover(&Real, &de, &para, b"outro").unwrap_err();
        assert!(matches!(&e, ErroEscrita::Conflito(p) if p == &para));
        assert_eq!(e.codigo(), 3);
        assert_eq!(std::fs::read(&de).unwrap(), b"outro");
        assert_eq!(std::fs::read(&para).unwrap(), b"ocupado");
        std::fs::remove_file(&para).unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        mover(&Real, &de, &sub.join("z.md"), b"outro").unwrap();
        assert_eq!(ls(dir.path()), ["sub"]);
        assert_eq!(std::fs::read(sub.join("z.md")).unwrap(), b"outro");
        let e = mover(&Real, &de, &para, b"outro").unwrap_err();
        assert!(
            matches!(&e, ErroEscrita::Conflito(p) if p == &de),
            "sumiu: conflito"
        );
        let e = mover(
            &Real,
            &sub.join("z.md"),
            &dir.path().join("nao/w.md"),
            b"outro",
        )
        .unwrap_err();
        assert!(matches!(&e, ErroEscrita::Io(..)));
        assert_eq!(e.codigo(), 1);
    }

    #[test]
    fn mensagens_dizem_o_caminho() {
        let c = ErroEscrita::Conflito(PathBuf::from("/v/x.md"));
        assert_eq!(
            c.to_string(),
            "/v/x.md mudou durante a gravação; nada foi gravado, repita o comando"
        );
        let i = ErroEscrita::Io(PathBuf::from("/v/x.md"), std::io::Error::other("cheio"));
        assert_eq!(i.to_string(), "não consegui gravar /v/x.md: cheio");
    }

    #[test]
    fn temporario_e_oculto_e_na_mesma_pasta() {
        let t = temporario(Path::new("/v/p/x.md"));
        assert_eq!(
            t,
            PathBuf::from(format!("/v/p/.freitask.x.md.{}.tmp", std::process::id()))
        );
    }
}
