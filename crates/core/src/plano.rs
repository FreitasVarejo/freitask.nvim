//! O plano: a lista de efeitos sobre arquivos que uma operação pede (spec
//! §1.3). O `core` monta, a crate `freitask-vault` executa, na ordem, cada
//! escrita com CAS contra os bytes que o plano diz ter lido (`antes`).

/// Um efeito sobre o vault. Caminhos relativos ao vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Efeito {
    /// Cria a pasta e as que faltarem acima dela.
    CriarDir { rel: String },
    /// Grava `depois` em `rel`. `antes` é o que a operação leu (`None`: o
    /// arquivo não existia); se o disco não tiver mais isso na hora de
    /// gravar, nada é gravado e a execução para com conflito.
    Escrever {
        rel: String,
        antes: Option<Vec<u8>>,
        depois: Vec<u8>,
    },
    /// Apaga `rel`, se ele ainda tiver os bytes `antes`.
    Apagar { rel: String, antes: Vec<u8> },
    /// Move `de` para `para` num `rename` só, se `de` ainda tiver os bytes
    /// `antes` e `para` não existir (nunca sobrescreve).
    Mover {
        de: String,
        para: String,
        antes: Vec<u8>,
    },
    /// Move a PASTA `de` para `para` num `rename` só, se `para` não existir
    /// (nunca sobrescreve nem junta duas pastas). É o `encerrar` e o
    /// `reabrir`: o projeto inteiro muda de endereço de uma vez.
    MoverPasta { de: String, para: String },
}

pub type Plano = Vec<Efeito>;
