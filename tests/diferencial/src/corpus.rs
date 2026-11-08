//! Os corpora (spec §3.4): (a) cópia do vault real do dono, lida de
//! `~/ObsidianVault` na hora e nunca escrita lá; (b) os vaults sintéticos do
//! gerador, com semente fixa; (c) os casos patológicos nomeados.

use std::path::{Path, PathBuf};

use crate::cenarios::Corpus;

/// Pastas que não entram na cópia do vault real: configuração do Obsidian e
/// metadados do Syncthing, que não são conteúdo e mudam por fora.
const IGNORAR: &[&str] = &[".obsidian", ".stfolder", ".stversions", ".trash"];

/// Monta `<home>/ObsidianVault` com o corpus pedido. Para `home-vazio` não
/// cria vault nenhum: o HOME fica só com o que o comando criar.
pub fn preparar(corpus: &Corpus, home: &Path) {
    let vault = home.join("ObsidianVault");
    match corpus {
        Corpus::Real => copiar_arvore(&vault_real(), &vault),
        Corpus::Sintetico(semente) => gerador::sintetico(*semente, &gerador::Opcoes::default())
            .gravar(&vault)
            .expect("gravar sintético"),
        Corpus::Minimo => gerador::minimo().gravar(&vault).expect("gravar mínimo"),
        Corpus::Patologico(nome) => {
            let (_, v) = gerador::patologicos()
                .into_iter()
                .find(|(n, _)| n == nome)
                .unwrap_or_else(|| panic!("patológico desconhecido: {nome}"));
            if v.arquivos.is_empty() && v.dirs.is_empty() {
                return; // home-vazio: nenhum vault
            }
            v.gravar(&vault).expect("gravar patológico");
        }
    }
}

/// O vault real do dono, só para leitura. `HOME` aqui é o do processo do
/// harness (o dono), nunca o dos lados A e B.
pub fn vault_real() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME");
    PathBuf::from(home).join("ObsidianVault")
}

fn copiar_arvore(de: &Path, para: &Path) {
    std::fs::create_dir_all(para).expect("criar destino");
    for entrada in std::fs::read_dir(de).unwrap_or_else(|e| panic!("ler {}: {e}", de.display())) {
        let entrada = entrada.expect("entrada");
        let nome = entrada.file_name();
        let nome_str = nome.to_string_lossy();
        if IGNORAR.contains(&nome_str.as_ref()) {
            continue;
        }
        let tipo = entrada.file_type().expect("tipo");
        let destino = para.join(&nome);
        if tipo.is_dir() {
            copiar_arvore(&entrada.path(), &destino);
        } else if tipo.is_file() {
            std::fs::copy(entrada.path(), &destino)
                .unwrap_or_else(|e| panic!("copiar {}: {e}", nome_str));
        }
        // Symlinks ficam de fora: o vault não os usa, e segui-los poderia sair dele.
    }
}
