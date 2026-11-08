//! Gravação atômica (spec §1.5): temporário oculto na mesma pasta, `fsync` do
//! arquivo, `rename`, `fsync` do diretório. O Syncthing detecta mudança por
//! mtime/tamanho e só vê o arquivo final; um leitor nunca encontra o destino
//! pela metade. O CAS (conferir o hash antes do `rename`) entra na fatia 3.

use std::io::Write;
use std::path::Path;

/// Grava `bytes` em `destino` de uma vez só.
pub fn escrever_atomico(destino: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let pai = destino.parent().unwrap_or(Path::new("."));
    let nome = destino
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = pai.join(format!(".freitask.{nome}.{}.tmp", std::process::id()));
    let resultado = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, destino)?;
        if let Ok(dir) = std::fs::File::open(pai) {
            // Falha no fsync do diretório não desfaz o rename: só perde a
            // garantia de durabilidade imediata, e o arquivo já está lá.
            let _ = dir.sync_all();
        }
        Ok(())
    })();
    if resultado.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    resultado
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn grava_e_nao_deixa_temporario() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("x.md");
        escrever_atomico(&destino, b"a\r\nb").unwrap();
        assert_eq!(std::fs::read(&destino).unwrap(), b"a\r\nb");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn falha_na_pasta_que_nao_existe_e_erro_sem_lixo() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("nao/existe/x.md");
        assert!(escrever_atomico(&destino, b"a").is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
