//! Helper da fatia 9b (dependências, TUI). Copiado do helper da fatia 7
//! (`tests/aceite/f7-tui/util.rs`) e adaptado: sai o que só a 7 usava (o
//! vault de base dela, o painel medido, o caminho do binário), entra o vault
//! das dependências e a leitura da linha da task com a marca `⛔`, que ocupa
//! duas colunas na grade (a célula depois dela fica escondida).
//!
//! Regras de `docs/agents/aceite.md`. Os testes em processo correm em threads
//! paralelas do mesmo processo: nada aqui mexe em variável de ambiente, e o dia
//! vai explícito em `Sessao::abrir(home, HOJE)`.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use freitask::tui::{Pedido, Sessao};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use tempfile::TempDir;

/// O dia fixo dos testes desta fatia (uma terça-feira).
pub const HOJE: &str = "2026-11-10";

/// O tamanho de tela padrão dos testes (spec §3.5: 100×30 e 60×15).
pub const LARGURA: u16 = 100;
pub const ALTURA: u16 = 30;

/// Os dois tamanhos da §3.5.
pub const TAMANHOS: [(u16, u16); 2] = [(100, 30), (60, 15)];

/// A marca de bloqueada (f9b.2): o emoji, que ocupa duas colunas.
pub const MARCA: &str = "⛔";

// ---------------------------------------------------------------------------
// Arquivos do vault
// ---------------------------------------------------------------------------

/// `<HOME>/ObsidianVault`.
pub fn vault(home: &Path) -> PathBuf {
    home.join("ObsidianVault")
}

/// O caminho absoluto de `<HOME>/ObsidianVault/<rel>`.
pub fn abs(home: &Path, rel: &str) -> PathBuf {
    vault(home).join(rel)
}

/// Grava um arquivo em `<HOME>/ObsidianVault/<rel>`, criando as pastas.
pub fn arquivo(home: &Path, rel: &str, conteudo: &str) {
    let caminho = abs(home, rel);
    fs::create_dir_all(caminho.parent().expect("pai")).expect("criar pastas");
    fs::write(&caminho, conteudo).expect("gravar arquivo");
}

/// Cria a pasta `<HOME>/ObsidianVault/<rel>`.
pub fn pasta(home: &Path, rel: &str) {
    fs::create_dir_all(abs(home, rel)).expect("criar pasta");
}

/// Lê `<HOME>/ObsidianVault/<rel>` como texto.
pub fn le(home: &Path, rel: &str) -> String {
    let caminho = abs(home, rel);
    fs::read_to_string(&caminho)
        .unwrap_or_else(|e| panic!("não consegui ler {}: {e}", caminho.display()))
}

/// Um instante bem no passado (2020-09-13).
pub fn antigo() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000)
}

/// Põe o mtime de todo ARQUIVO sob `<HOME>/ObsidianVault` em `antigo()`. Só
/// se usa ANTES de abrir a sessão: depois, mexer no mtime seria uma escrita
/// de fora que a sessão poderia (com razão) tomar por conflito.
pub fn envelhece(home: &Path) {
    let raiz = vault(home);
    if !raiz.exists() {
        return;
    }
    let mut pilha = vec![raiz];
    while let Some(dir) = pilha.pop() {
        for entrada in fs::read_dir(&dir).expect("read_dir") {
            let caminho = entrada.expect("entrada").path();
            let meta = fs::symlink_metadata(&caminho).expect("metadata");
            if meta.is_dir() {
                pilha.push(caminho);
            } else {
                let f = fs::OpenOptions::new()
                    .write(true)
                    .open(&caminho)
                    .expect("abrir para mudar o mtime");
                f.set_modified(antigo()).expect("set_modified");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fotografia da árvore (cópia da fatia 7)
// ---------------------------------------------------------------------------

/// Uma entrada da foto: pasta, ou arquivo com bytes e mtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entrada {
    Pasta,
    Arquivo(Vec<u8>, SystemTime),
}

/// Foto de uma árvore: caminho relativo (com `/`) → entrada.
pub type Foto = BTreeMap<String, Entrada>;

/// Fotografa a árvore sob `raiz` (vazia se não existe).
pub fn fotografa_dir(raiz: &Path) -> Foto {
    let mut foto = Foto::new();
    if !raiz.exists() {
        return foto;
    }
    let mut pilha = vec![raiz.to_path_buf()];
    while let Some(dir) = pilha.pop() {
        for entrada in fs::read_dir(&dir).expect("read_dir") {
            let caminho = entrada.expect("entrada").path();
            let meta = fs::symlink_metadata(&caminho).expect("metadata");
            let rel = caminho
                .strip_prefix(raiz)
                .expect("prefixo")
                .to_str()
                .expect("utf-8")
                .to_owned();
            if meta.is_dir() {
                foto.insert(rel, Entrada::Pasta);
                pilha.push(caminho);
            } else {
                let bytes = fs::read(&caminho).expect("ler arquivo");
                foto.insert(
                    rel,
                    Entrada::Arquivo(bytes, meta.modified().expect("mtime")),
                );
            }
        }
    }
    foto
}

/// Fotografa `<HOME>/ObsidianVault`.
pub fn fotografa(home: &Path) -> Foto {
    fotografa_dir(&vault(home))
}

/// Os caminhos que diferem entre duas fotos (bytes, mtime, apareceu, sumiu).
pub fn mudancas(antes: &Foto, depois: &Foto) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (k, v) in antes {
        match depois.get(k) {
            Some(w) if w == v => {}
            _ => {
                out.insert(k.clone());
            }
        }
    }
    for k in depois.keys() {
        if !antes.contains_key(k) {
            out.insert(k.clone());
        }
    }
    out
}

/// Um conjunto de caminhos a partir de literais.
pub fn conjunto(itens: &[&str]) -> BTreeSet<String> {
    itens.iter().map(|s| s.to_string()).collect()
}

/// Afirma que nada mudou sob o vault entre as duas fotos.
pub fn nada_mudou(antes: &Foto, depois: &Foto) {
    let m = mudancas(antes, depois);
    assert!(
        m.is_empty(),
        "nada devia ter sido gravado, mas mudou: {m:?}"
    );
}

/// Afirma o que uma gravação pelo `b` pode mudar no vault: a task (que tem de
/// mudar) e, no máximo, o `CURRENT.md` regenerado, com o mesmo conteúdo de
/// antes (a lista mora no frontmatter, que o painel nunca copiou: §10.1). É o
/// que o `set --bloqueada-por` da CLI muda (aceite da 9a,
/// `set_bloqueada_por_so_grava_a_task_e_o_painel_fica_igual`).
pub fn so_a_task_mudou(home: &Path, antes: &Foto, task: &str, painel_antes: &str) {
    let mudou = mudancas(antes, &fotografa(home));
    assert!(
        mudou.contains(task),
        "a task {task} tinha de mudar: {mudou:?}"
    );
    assert!(
        mudou.is_subset(&conjunto(&[task, CURRENT])),
        "só a task e o painel podem mudar: {mudou:?}"
    );
    assert_eq!(
        le(home, CURRENT),
        painel_antes,
        "o CURRENT.md não muda de conteúdo com a lista"
    );
}

// ---------------------------------------------------------------------------
// O vault das dependências
// ---------------------------------------------------------------------------

pub const CURRENT: &str = "CURRENT.md";

// Os ids são curtos para a linha com a marca caber inteira no painel das
// tasks em 60×15 (19 colunas por dentro: `! `, ícone, espaço, `⛔ `, id).

// `alfa`: as tasks que o `b` edita. Uma fase por task, para a ordem da lista
// ser a das fases (1, 3, 4, 5, 7), como a fatia 7 mediu.
pub const ESCREVER: &str = "projects/alfa/tasks/escrever.md";
pub const REVISAR: &str = "projects/alfa/tasks/revisar.md";
pub const MEDIR: &str = "projects/alfa/tasks/medir.md";
pub const HOMOLOGAR: &str = "projects/alfa/tasks/homologar.md";
pub const FECHAR: &str = "projects/alfa/tasks/fechar.md";
pub const VELHA: &str = "projects/alfa/tasks/archived/done/velha.md";

// `beta`: a semântica da marca.
pub const QUER_LIVRE: &str = "projects/beta/tasks/quer-livre.md";
pub const QUER_VELHA: &str = "projects/beta/tasks/quer-velha.md";
pub const QUER_SUMIDA: &str = "projects/beta/tasks/quer-sumida.md";
pub const QUER_SONHO: &str = "projects/beta/tasks/quer-sonho.md";
pub const QUER_ADIADA: &str = "projects/beta/tasks/quer-adiada.md";
pub const LIVRE: &str = "projects/beta/tasks/livre.md";
pub const SONHO: &str = "projects/beta/tasks/sonho.md";
pub const ADIADA: &str = "projects/beta/tasks/adiada.md";
pub const VELHA_BETA: &str = "projects/beta/tasks/archived/done/velha-beta.md";

// `delta`: a prioridade alta bloqueada e uma das cópias do id ambíguo `dup`;
// `gama`: a outra cópia.
pub const URGENTE: &str = "projects/delta/tasks/urgente.md";
pub const DUP_DELTA: &str = "projects/delta/tasks/dup.md";
pub const DUP_GAMA: &str = "projects/gama/tasks/dup.md";

/// `escrever`: Não iniciada, sem frontmatter, com descrição.
pub const ESCREVER_MD: &str =
    "> [!todo] Escrever a spec\n> [[projects/alfa/tasks/escrever|escrever]]\n> _em curso_\n";
/// `revisar`: Refinada, espera `medir` e `fechar`, entre chaves alheias.
pub const REVISAR_MD: &str = concat!(
    "---\n",
    "tags: [x]\n",
    "bloqueada-por: [medir, fechar]\n",
    "criado: 2026-01-01\n",
    "---\n",
    "\n",
    "> [!tip] Revisar o texto\n",
    "> [[projects/alfa/tasks/revisar|revisar]]\n",
);
/// `medir`: Em implementação, frontmatter sem a chave.
pub const MEDIR_MD: &str =
    "---\ntags: [x]\n---\n\n> [!example] Medir o tempo\n> [[projects/alfa/tasks/medir|medir]]\n";
/// `homologar`: Em homologação, espera `medir`.
pub const HOMOLOGAR_MD: &str = "---\nbloqueada-por: [medir]\n---\n\n> [!question] Homologar\n> [[projects/alfa/tasks/homologar|homologar]]\n";
/// `fechar`: Pronta, sem frontmatter.
pub const FECHAR_MD: &str = "> [!check] Fechar o ciclo\n> [[projects/alfa/tasks/fechar|fechar]]\n";
/// `velha`: arquivada em `done`, espera `medir` (ativa).
pub const VELHA_MD: &str = "---\nbloqueada-por: [medir]\n---\n\n> [!done] Velha feita\n> [[projects/alfa/tasks/archived/done/velha|velha]]\n\n## Histórico\n\n- 2026-01-05 — arquivada em `archived/done` (feito)\n";

pub const QUER_LIVRE_MD: &str = "---\nbloqueada-por: [livre]\n---\n\n> [!todo] Quer Livre\n> [[projects/beta/tasks/quer-livre|quer-livre]]\n";
pub const QUER_VELHA_MD: &str = "---\nbloqueada-por: [velha-beta]\n---\n\n> [!abstract] Quer Velha\n> [[projects/beta/tasks/quer-velha|quer-velha]]\n";
pub const QUER_SUMIDA_MD: &str = "---\nbloqueada-por: [fantasma]\n---\n\n> [!tip] Quer Sumida\n> [[projects/beta/tasks/quer-sumida|quer-sumida]]\n";
pub const QUER_SONHO_MD: &str = "---\nbloqueada-por: [sonho]\n---\n\n> [!example] Quer Sonho\n> [[projects/beta/tasks/quer-sonho|quer-sonho]]\n";
pub const QUER_ADIADA_MD: &str = "---\nbloqueada-por: [adiada]\n---\n\n> [!question] Quer Adiada\n> [[projects/beta/tasks/quer-adiada|quer-adiada]]\n";
pub const LIVRE_MD: &str = "> [!warning] Livre\n> [[projects/beta/tasks/livre|livre]]\n";
pub const SONHO_MD: &str = "> [!quote] Sonho\n> [[projects/beta/tasks/sonho|sonho]]\n";
/// `adiada`: Pronta, volta em dezembro, e ela mesma espera `livre`.
pub const ADIADA_MD: &str = "---\nvolta-em: 2026-12-01\nbloqueada-por: [livre]\n---\n\n> [!check] Adiada\n> [[projects/beta/tasks/adiada|adiada]]\n";
pub const VELHA_BETA_MD: &str = "> [!done] Velha beta\n> [[projects/beta/tasks/archived/done/velha-beta|velha-beta]]\n\n## Histórico\n\n- 2026-01-05 — arquivada em `archived/done` (feito)\n";

/// `urgente`: Em implementação, `prioridade: alta`, espera `livre` (de
/// outro projeto: o id é global, §10.1).
pub const URGENTE_MD: &str = "---\nprioridade: alta\nbloqueada-por: [livre]\n---\n\n> [!example] Urgente\n> [[projects/delta/tasks/urgente|urgente]]\n";
pub const DUP_DELTA_MD: &str = "> [!todo] Dup delta\n> [[projects/delta/tasks/dup|dup]]\n";
pub const DUP_GAMA_MD: &str = "> [!todo] Dup gama\n> [[projects/gama/tasks/dup|dup]]\n";

/// O vault das dependências, com `status.json` default, sem `CURRENT.md`,
/// tudo envelhecido. Os projetos, em ordem: `alfa`, `beta`, `delta`, `gama`.
///
/// `alfa` mostra, nesta ordem (uma fase por task): `escrever` (sem
/// frontmatter), `revisar` (espera `medir, fechar`: bloqueada), `medir`
/// (frontmatter sem a chave; esperada por `revisar`, `homologar` e a
/// arquivada `velha`), `homologar` (espera `medir`: bloqueada), `fechar` (sem
/// frontmatter); com `A`, `velha` (arquivada em `done`, espera `medir`) no
/// fim.
///
/// `beta` mostra, nesta ordem: `quer-livre` (Não iniciada, espera `livre`:
/// bloqueada), `quer-velha` (Refinando, espera a arquivada `velha-beta`:
/// não), `quer-sumida` (Refinada, espera `fantasma`, que não existe: não),
/// `quer-sonho` (Em implementação, espera `sonho`, Algum dia: bloqueada),
/// `quer-adiada` (Em homologação, espera `adiada`, que volta em 2026-12-01:
/// bloqueada), `livre` (a fase Bloqueada, sem dependência); com `S`, `adiada`
/// (Pronta, espera `livre`) e `sonho` (Algum dia) no fim, nessa ordem.
///
/// `delta` mostra `urgente` (prioridade alta, espera `livre`) antes de `dup`;
/// `gama` tem a outra `dup`.
///
/// Nenhum título traz um id como palavra em minúsculas: o item da lista se
/// acha pelo id (`item_de`).
pub fn home_deps() -> TempDir {
    let home = tempfile::tempdir().expect("tempdir");
    let h = home.path();
    arquivo(h, ".freitask/status.json", gerador::STATUS_JSON);
    for (rel, md) in [
        (ESCREVER, ESCREVER_MD),
        (REVISAR, REVISAR_MD),
        (MEDIR, MEDIR_MD),
        (HOMOLOGAR, HOMOLOGAR_MD),
        (FECHAR, FECHAR_MD),
        (VELHA, VELHA_MD),
        (QUER_LIVRE, QUER_LIVRE_MD),
        (QUER_VELHA, QUER_VELHA_MD),
        (QUER_SUMIDA, QUER_SUMIDA_MD),
        (QUER_SONHO, QUER_SONHO_MD),
        (QUER_ADIADA, QUER_ADIADA_MD),
        (LIVRE, LIVRE_MD),
        (SONHO, SONHO_MD),
        (ADIADA, ADIADA_MD),
        (VELHA_BETA, VELHA_BETA_MD),
        (URGENTE, URGENTE_MD),
        (DUP_DELTA, DUP_DELTA_MD),
        (DUP_GAMA, DUP_GAMA_MD),
    ] {
        arquivo(h, rel, md);
    }
    envelhece(h);
    home
}

/// Os ids de `alfa` na ordem da lista.
pub const ALFA: [&str; 5] = ["escrever", "revisar", "medir", "homologar", "fechar"];

/// Os ids de `beta` na ordem da lista (sem `S`).
pub const BETA: [&str; 6] = [
    "quer-livre",
    "quer-velha",
    "quer-sumida",
    "quer-sonho",
    "quer-adiada",
    "livre",
];

// ---------------------------------------------------------------------------
// A sessão e as teclas
// ---------------------------------------------------------------------------

/// Abre a TUI em processo sobre o HOME, no dia `HOJE`.
pub fn abre(home: &Path) -> Sessao {
    Sessao::abrir(home, HOJE).unwrap_or_else(|e| panic!("Sessao::abrir falhou: {e}"))
}

/// Uma tecla de caractere como a crossterm a entrega: maiúscula vem com
/// `SHIFT`, o resto sem modificador.
pub fn tecla(c: char) -> KeyEvent {
    let m = if c.is_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(c), m)
}

/// Uma tecla especial (Enter, Esc, setas…), sem modificador.
pub fn especial(k: KeyCode) -> KeyEvent {
    KeyEvent::new(k, KeyModifiers::NONE)
}

/// `Ctrl-<c>`.
pub fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// Aperta uma tecla e afirma que ela não pediu nada ao laço.
pub fn aperta(s: &mut Sessao, k: KeyEvent) {
    assert_eq!(s.tecla(k), Pedido::Nada, "a tecla {k:?} pediu algo ao laço");
}

/// Aperta cada caractere de `teclas` e afirma que nenhum pediu nada ao laço.
pub fn aperta_todas(s: &mut Sessao, teclas: &str) {
    for c in teclas.chars() {
        aperta(s, tecla(c));
    }
}

/// Digita um texto num campo (o mesmo que `aperta_todas`, com outro nome
/// para o teste ler melhor).
pub fn digita(s: &mut Sessao, texto: &str) {
    aperta_todas(s, texto);
}

pub fn enter(s: &mut Sessao) {
    aperta(s, especial(KeyCode::Enter));
}

pub fn esc(s: &mut Sessao) {
    aperta(s, especial(KeyCode::Esc));
}

/// Ao abrir, o foco está nos projetos, em `alfa` (fatia 7). Desce `projeto`
/// linhas nos projetos (0 `alfa`, 1 `beta`, 2 `delta`, 3 `gama`), passa o
/// foco para as tasks (`l`) e desce `task` linhas nelas.
pub fn vai_para(s: &mut Sessao, projeto: usize, task: usize) {
    for _ in 0..projeto {
        aperta(s, tecla('j'));
    }
    aperta(s, tecla('l'));
    for _ in 0..task {
        aperta(s, tecla('j'));
    }
}

/// O índice de `id` na lista de `alfa`.
pub fn em_alfa(id: &str) -> usize {
    ALFA.iter()
        .position(|i| *i == id)
        .unwrap_or_else(|| panic!("{id} não está em alfa"))
}

/// Foco na task `id` de `alfa`, já selecionada.
pub fn seleciona_em_alfa(s: &mut Sessao, id: &str) {
    vai_para(s, 0, em_alfa(id));
}

// ---------------------------------------------------------------------------
// A tela
// ---------------------------------------------------------------------------

/// A tela desenhada, lida como a pessoa a lê.
pub struct Tela {
    pub buf: Buffer,
}

/// Um painel achado na tela: a área de dentro da moldura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Painel {
    pub x0: u16,
    pub x1: u16,
    pub y0: u16,
    pub y1: u16,
}

const CANTO_ESQ_CIMA: [&str; 4] = ["╭", "┌", "╔", "┏"];
const CANTO_DIR_CIMA: [&str; 4] = ["╮", "┐", "╗", "┓"];
const CANTO_ESQ_BAIXO: [&str; 4] = ["╰", "└", "╚", "┗"];

/// A tela de `s` em `LARGURA` × `ALTURA`.
pub fn tela(s: &Sessao) -> Tela {
    tela_de(s, LARGURA, ALTURA)
}

/// A tela de `s` em `largura` × `altura`; afirma que o buffer tem o tamanho
/// pedido.
pub fn tela_de(s: &Sessao, largura: u16, altura: u16) -> Tela {
    let buf = s.tela(largura, altura);
    assert_eq!(
        (buf.area.width, buf.area.height),
        (largura, altura),
        "a tela tem de ter o tamanho pedido"
    );
    Tela { buf }
}

impl Tela {
    pub fn largura(&self) -> u16 {
        self.buf.area.width
    }

    pub fn altura(&self) -> u16 {
        self.buf.area.height
    }

    pub fn simbolo(&self, x: u16, y: u16) -> &str {
        self.buf[(x, y)].symbol()
    }

    /// O texto da linha `y` entre as colunas `x0..x1`, célula a célula (a
    /// célula escondida por um caractere largo entra como está, um espaço).
    pub fn trecho(&self, y: u16, x0: u16, x1: u16) -> String {
        (x0..x1).map(|x| self.simbolo(x, y)).collect()
    }

    /// O texto da linha `y` entre `x0..x1` como se lê: a célula logo depois
    /// da marca `⛔` (que ocupa duas colunas) é a metade escondida dela e não
    /// entra. Pânico se essa célula tiver texto: a marca teria sido
    /// sobrescrita.
    pub fn lido(&self, y: u16, x0: u16, x1: u16) -> String {
        let mut out = String::new();
        let mut x = x0;
        while x < x1 {
            let s = self.simbolo(x, y);
            out.push_str(s);
            if s == MARCA && x + 1 < x1 {
                let escondida = self.simbolo(x + 1, y);
                assert!(
                    escondida.trim().is_empty(),
                    "a célula depois da marca tinha de ser a metade escondida dela, tem {escondida:?}:\n{}",
                    self.texto()
                );
                x += 2;
            } else {
                x += 1;
            }
        }
        out
    }

    /// O texto de cada linha da tela.
    pub fn linhas(&self) -> Vec<String> {
        (0..self.altura())
            .map(|y| self.trecho(y, 0, self.largura()))
            .collect()
    }

    /// A tela inteira, uma linha por linha, para as mensagens de falha.
    pub fn texto(&self) -> String {
        self.linhas().join("\n")
    }

    pub fn contem(&self, agulha: &str) -> bool {
        self.linhas().iter().any(|l| l.contains(agulha))
    }

    /// Onde `agulha` começa na tela (coluna, linha), procurando célula a
    /// célula (cada célula tem um símbolo).
    pub fn acha(&self, agulha: &str) -> Option<(u16, u16)> {
        let alvo: Vec<char> = agulha.chars().collect();
        for y in 0..self.altura() {
            for x in 0..self.largura() {
                let mut ok = true;
                let mut i = 0;
                let mut cx = x;
                while i < alvo.len() {
                    if cx >= self.largura() {
                        ok = false;
                        break;
                    }
                    let sim: Vec<char> = self.simbolo(cx, y).chars().collect();
                    if sim.is_empty()
                        || alvo.len() < i + sim.len()
                        || alvo[i..i + sim.len()] != sim[..]
                    {
                        ok = false;
                        break;
                    }
                    i += sim.len();
                    cx += 1;
                }
                if ok {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// O painel cujo título (na moldura de cima) contém `titulo`: a moldura
    /// se acha andando do título para os cantos.
    pub fn painel(&self, titulo: &str) -> Painel {
        self.talvez_painel(titulo)
            .unwrap_or_else(|| panic!("não achei o painel {titulo:?} na tela:\n{}", self.texto()))
    }

    /// O painel cujo título contém `titulo`, se houver.
    pub fn talvez_painel(&self, titulo: &str) -> Option<Painel> {
        let (tx, ty) = self.acha(titulo)?;
        let esq = (0..=tx)
            .rev()
            .find(|&x| CANTO_ESQ_CIMA.contains(&self.simbolo(x, ty)))?;
        let dir = (tx..self.largura()).find(|&x| CANTO_DIR_CIMA.contains(&self.simbolo(x, ty)))?;
        let baixo = (ty + 1..self.altura())
            .find(|&y| CANTO_ESQ_BAIXO.contains(&self.simbolo(esq, y)))
            .unwrap_or(self.altura());
        Some(Painel {
            x0: esq + 1,
            x1: dir,
            y0: ty + 1,
            y1: baixo,
        })
    }

    /// O título de um painel: a linha de cima da moldura dele, de canto a
    /// canto.
    pub fn titulo_do(&self, p: Painel) -> String {
        self.trecho(p.y0 - 1, p.x0 - 1, p.x1 + 1)
    }

    /// O texto de cada linha de dentro do painel, como se lê.
    pub fn linhas_do(&self, p: Painel) -> Vec<String> {
        (p.y0..p.y1).map(|y| self.lido(y, p.x0, p.x1)).collect()
    }

    /// O texto de dentro do painel, aparado, sem as linhas em branco.
    pub fn itens_do(&self, p: Painel) -> Vec<String> {
        self.linhas_do(p)
            .into_iter()
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// O painel inteiro como um texto só (para procurar algo dentro dele).
    pub fn texto_do(&self, p: Painel) -> String {
        self.linhas_do(p).join("\n")
    }

    /// O "estilo de destaque" de uma linha do painel: fundo e inversão da
    /// primeira célula com texto. `None` se a linha está em branco.
    fn marca_de_destaque(&self, p: Painel, y: u16) -> Option<(Color, bool)> {
        (p.x0..p.x1)
            .find(|&x| !self.simbolo(x, y).trim().is_empty())
            .map(|x| {
                let c = &self.buf[(x, y)];
                (c.bg, c.modifier.contains(Modifier::REVERSED))
            })
    }

    /// As linhas do painel desenhadas em destaque: as que têm fundo (ou
    /// inversão) diferente do resto do painel. O "resto" é o fundo de uma
    /// linha em branco do painel; sem linha em branco, o estilo mais comum
    /// entre as linhas.
    pub fn destacadas(&self, p: Painel) -> Vec<String> {
        let mut fundo: Option<(Color, bool)> = None;
        let mut marcas: Vec<(u16, (Color, bool))> = Vec::new();
        for y in p.y0..p.y1 {
            match self.marca_de_destaque(p, y) {
                Some(m) => marcas.push((y, m)),
                None => {
                    if fundo.is_none() {
                        let c = &self.buf[(p.x0, y)];
                        fundo = Some((c.bg, c.modifier.contains(Modifier::REVERSED)));
                    }
                }
            }
        }
        let fundo = fundo.unwrap_or_else(|| {
            let mut conta: Vec<((Color, bool), usize)> = Vec::new();
            for (_, m) in &marcas {
                match conta.iter_mut().find(|(k, _)| k == m) {
                    Some((_, n)) => *n += 1,
                    None => conta.push((*m, 1)),
                }
            }
            conta
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(k, _)| k)
                .unwrap_or((Color::Reset, false))
        });
        marcas
            .into_iter()
            .filter(|(_, m)| *m != fundo)
            .map(|(y, _)| self.lido(y, p.x0, p.x1).trim().to_owned())
            .collect()
    }

    /// A única linha em destaque do painel (pânico se não houver exatamente
    /// uma).
    pub fn destacada(&self, p: Painel) -> String {
        let d = self.destacadas(p);
        assert_eq!(
            d.len(),
            1,
            "o painel tinha de ter exatamente uma linha em destaque, tem {d:?}; tela:\n{}",
            self.texto()
        );
        d.into_iter().next().expect("uma")
    }

    /// O painel das tasks (título `Tasks…`).
    pub fn tasks(&self) -> Painel {
        self.painel("Tasks")
    }

    /// O painel dos projetos.
    pub fn projetos(&self) -> Painel {
        self.painel("Projetos")
    }

    /// O painel do preview.
    pub fn preview(&self) -> Painel {
        self.painel("Preview")
    }

    /// A barra de baixo (a última linha), aparada.
    pub fn barra(&self) -> String {
        self.trecho(self.altura() - 1, 0, self.largura())
            .trim()
            .to_owned()
    }

    /// A cor do texto da barra: a da primeira célula com texto.
    pub fn cor_da_barra(&self) -> Color {
        let y = self.altura() - 1;
        let x = (0..self.largura())
            .find(|&x| !self.simbolo(x, y).trim().is_empty())
            .unwrap_or_else(|| panic!("a barra está vazia:\n{}", self.texto()));
        self.buf[(x, y)].fg
    }
}

/// Afirma que a barra mostra `msg` como erro (em vermelho, a cor do erro na
/// TUI, `ui.rs`).
pub fn barra_de_erro(t: &Tela, msg: &str) {
    assert_eq!(t.barra(), msg, "barra; tela:\n{}", t.texto());
    assert_eq!(t.cor_da_barra(), Color::Red, "um erro sai em vermelho");
}

/// Afirma que a barra mostra `msg` como aviso de sucesso (em verde).
pub fn barra_de_ok(t: &Tela, msg: &str) {
    assert_eq!(t.barra(), msg, "barra; tela:\n{}", t.texto());
    assert_eq!(t.cor_da_barra(), Color::Green, "um ok sai em verde");
}

/// As partes da dica da barra (`ui.rs`, `draw_bar`), conferidas por partes e
/// na ordem, como na fatia 7.
pub const DICA: [&str; 8] = [
    "? ajuda",
    "n nova",
    "s fase",
    "a arquivar",
    "d largar",
    "e abrir",
    "/ filtro",
    "q sair",
];

/// Afirma que a barra mostra a dica, com as partes nesta ordem.
pub fn barra_com_a_dica(t: &Tela) {
    let b = t.barra();
    let mut desde = 0;
    for parte in DICA {
        let i = b[desde..]
            .find(parte)
            .unwrap_or_else(|| panic!("a dica da barra não traz {parte:?} na ordem: {b:?}"));
        desde += i + parte.len();
    }
}

/// O texto do painel do preview.
pub fn preview(t: &Tela) -> String {
    t.texto_do(t.preview())
}

/// Os itens (linhas com texto) do painel das tasks.
pub fn itens_das_tasks(t: &Tela) -> Vec<String> {
    t.itens_do(t.tasks())
}

/// Afirma a ordem dos ids na lista de tasks (cada item contém o seu id, e
/// são só estes).
pub fn ids_na_lista(t: &Tela, ids: &[&str]) {
    let itens = itens_das_tasks(t);
    assert_eq!(
        itens.len(),
        ids.len(),
        "itens da lista: {itens:#?}; esperava {ids:?}"
    );
    for (item, id) in itens.iter().zip(ids) {
        assert!(
            item.contains(id),
            "esperava {id:?} nesta posição, veio {item:?}; lista: {itens:#?}"
        );
    }
}

/// A task em destaque na lista contém `id`.
pub fn selecionada(t: &Tela, id: &str) {
    let d = t.destacada(t.tasks());
    assert!(
        tem_o_id(&d, id),
        "a task em destaque devia ser {id:?}, é {d:?}; tela:\n{}",
        t.texto()
    );
}

/// Verdadeiro se o item da lista traz `id` como palavra inteira (o que vem
/// antes e depois não é letra, dígito nem `-`): `livre` não casa em
/// `espera-livre`.
pub fn tem_o_id(item: &str, id: &str) -> bool {
    posicao_do_id(item, id).is_some()
}

/// A posição (em bytes) do id como palavra inteira no item.
pub fn posicao_do_id(item: &str, id: &str) -> Option<usize> {
    let de_id = |c: char| c.is_alphanumeric() || c == '-';
    let mut desde = 0;
    while let Some(i) = item[desde..].find(id) {
        let i = desde + i;
        let antes = item[..i].chars().next_back();
        let depois = item[i + id.len()..].chars().next();
        if !antes.is_some_and(de_id) && !depois.is_some_and(de_id) {
            return Some(i);
        }
        desde = i + id.len();
    }
    None
}

/// O item da lista de tasks que traz `id` (tem de haver exatamente um).
pub fn item_de(t: &Tela, id: &str) -> String {
    let itens: Vec<String> = itens_das_tasks(t)
        .into_iter()
        .filter(|l| tem_o_id(l, id))
        .collect();
    assert_eq!(
        itens.len(),
        1,
        "esperava um item com {id:?} na lista, há {itens:?}; tela:\n{}",
        t.texto()
    );
    itens.into_iter().next().expect("um")
}

/// Afirma que a linha da task `id` traz a marca `⛔ ` imediatamente antes do
/// id (f9b.2), uma vez só.
pub fn com_a_marca(t: &Tela, id: &str) {
    let item = item_de(t, id);
    let i = posicao_do_id(&item, id).expect("o id");
    assert!(
        item[..i].ends_with(&format!("{MARCA} ")),
        "a linha de {id:?} tinha de ter `{MARCA} ` colada antes do id: {item:?}; tela:\n{}",
        t.texto()
    );
    assert_eq!(
        item.matches(MARCA).count(),
        1,
        "uma marca só na linha de {id:?}: {item:?}"
    );
}

/// Afirma que a linha da task `id` não traz a marca.
pub fn sem_a_marca(t: &Tela, id: &str) {
    let item = item_de(t, id);
    assert!(
        !item.contains(MARCA),
        "a linha de {id:?} não podia ter a marca: {item:?}; tela:\n{}",
        t.texto()
    );
}

/// Afirma que a moldura do painel está inteira em todas as linhas: a coluna
/// da esquerda e a da direita têm o mesmo símbolo de borda de cima a baixo
/// (nenhuma linha passou da largura do painel).
pub fn moldura_inteira(t: &Tela, p: Painel) {
    let borda_esq = t.simbolo(p.x0 - 1, p.y0).to_owned();
    let borda_dir = t.simbolo(p.x1, p.y0).to_owned();
    assert!(
        !borda_esq.trim().is_empty() && !borda_dir.trim().is_empty(),
        "o painel tem moldura dos dois lados:\n{}",
        t.texto()
    );
    for y in p.y0..p.y1 {
        assert_eq!(
            t.simbolo(p.x0 - 1, y),
            borda_esq,
            "a borda esquerda quebrou na linha {y}:\n{}",
            t.texto()
        );
        assert_eq!(
            t.simbolo(p.x1, y),
            borda_dir,
            "a borda direita quebrou na linha {y}:\n{}",
            t.texto()
        );
    }
}

// ---------------------------------------------------------------------------
// O prompt do `b` (f9b.1)
// ---------------------------------------------------------------------------

/// O título da moldura do prompt, com os espaços das pontas (f9b.1).
pub const TITULO_DO_PROMPT: &str = " Bloqueada por (ids separados por vírgula; vazio limpa) ";

/// O pedaço do título pelo qual o prompt se acha na tela.
pub const PROMPT: &str = "Bloqueada por";

/// O texto do campo do prompt, aparado (pânico se o prompt não está aberto).
pub fn campo(t: &Tela) -> String {
    let p = t.painel(PROMPT);
    t.itens_do(p).join("\n")
}

/// Afirma que o prompt do `b` está aberto, com a moldura de f9b.1, e
/// devolve o texto do campo.
pub fn prompt_aberto(t: &Tela) -> String {
    let p = t.painel(PROMPT);
    let titulo = t.titulo_do(p);
    assert!(
        titulo.contains(TITULO_DO_PROMPT),
        "a moldura do prompt tinha de trazer {TITULO_DO_PROMPT:?}, traz {titulo:?}; tela:\n{}",
        t.texto()
    );
    campo(t)
}

/// Afirma que o prompt do `b` não está na tela.
pub fn prompt_fechado(t: &Tela) {
    assert!(
        t.talvez_painel(PROMPT).is_none(),
        "o prompt do `b` tinha de ter fechado:\n{}",
        t.texto()
    );
}

/// O que o `set --bloqueada-por` grava (formato medido na 9a): numa task sem
/// frontmatter, o frontmatter só com a lista, a linha em branco e o arquivo.
pub fn com_frontmatter_novo(lista: &str, md: &str) -> String {
    format!("---\nbloqueada-por: [{lista}]\n---\n\n{md}")
}
