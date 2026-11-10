//! Helper da fatia 7 (TUI). Do helper da fatia 6 só vêm o caminho do binário,
//! os arquivos do vault e a fotografia da árvore; o resto é novo: o vault de
//! base dos testes da TUI, as teclas como a crossterm as entrega, e a leitura
//! da tela (`ratatui::buffer::Buffer`) como a pessoa a vê — texto, painéis
//! achados pela moldura e pelo título, a linha em destaque, a barra e a cor
//! dela.
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

/// Raiz do repo: `FREITASK_REPO_RAIZ` no rascunho; congelado, dois níveis
/// acima do manifesto do pacote `aceite`.
pub fn raiz_do_repo() -> String {
    std::env::var("FREITASK_REPO_RAIZ")
        .unwrap_or_else(|_| format!("{}/../..", env!("CARGO_MANIFEST_DIR")))
}

/// O binário sob teste, compilado antes por `make aceite`/`rascunho.sh`.
pub fn binario() -> PathBuf {
    PathBuf::from(format!("{}/target/debug/freitask", raiz_do_repo()))
}

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

/// Verdadeiro se `<HOME>/ObsidianVault/<rel>` existe.
pub fn existe(home: &Path, rel: &str) -> bool {
    abs(home, rel).exists()
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
// Fotografia da árvore (cópia da fatia 6)
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

/// Só os ARQUIVOS que diferem entre duas fotos; pastas ficam de fora.
pub fn arquivos_mudados(antes: &Foto, depois: &Foto) -> BTreeSet<String> {
    mudancas(antes, depois)
        .into_iter()
        .filter(|k| {
            !matches!(antes.get(k), Some(Entrada::Pasta))
                && !matches!(depois.get(k), Some(Entrada::Pasta))
        })
        .collect()
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

// ---------------------------------------------------------------------------
// O vault de base
// ---------------------------------------------------------------------------

pub const ESCREVER: &str = "projects/alfa/tasks/escrever-spec.md";
pub const MEDIR: &str = "projects/alfa/tasks/medir-tempo.md";
pub const FECHAR: &str = "projects/alfa/tasks/fechar-ciclo.md";
pub const SONHO: &str = "projects/alfa/tasks/sonho-distante.md";
pub const ADIADA: &str = "projects/alfa/tasks/adiada-dezembro.md";
pub const VELHA: &str = "projects/alfa/tasks/archived/done/velha-feita.md";
pub const BETA_UM: &str = "projects/beta/tasks/beta-um.md";
pub const BETA_URGENTE: &str = "projects/beta/tasks/beta-urgente.md";
pub const NOTAS: &str = "projects/alfa/notas.md";
pub const CURRENT: &str = "CURRENT.md";

pub const ESCREVER_MD: &str = "> [!todo] Escrever a spec\n> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n> _em curso_\n> nota um\n";
pub const MEDIR_MD: &str =
    "> [!example] Medir o tempo\n> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\n";
pub const FECHAR_MD: &str =
    "> [!check] Fechar o ciclo\n> [[projects/alfa/tasks/fechar-ciclo|fechar-ciclo]]\n";
pub const SONHO_MD: &str =
    "> [!quote] Sonho distante\n> [[projects/alfa/tasks/sonho-distante|sonho-distante]]\n";
pub const ADIADA_MD: &str = "---\nvolta-em: 2026-12-01\n---\n\n> [!tip] Adiada até dezembro\n> [[projects/alfa/tasks/adiada-dezembro|adiada-dezembro]]\n";
pub const VELHA_MD: &str = "> [!done] Velha feita\n> [[projects/alfa/tasks/archived/done/velha-feita|velha-feita]]\n\n## Histórico\n\n- 2026-01-05 — arquivada em `archived/done` (feito)\n";
pub const BETA_UM_MD: &str = "> [!todo] Bê um\n> [[projects/beta/tasks/beta-um|beta-um]]\n";
pub const BETA_URGENTE_MD: &str = "---\nprioridade: alta\n---\n\n> [!example] Bê urgente\n> [[projects/beta/tasks/beta-urgente|beta-urgente]]\n";
pub const NOTAS_MD: &str =
    "# Notas\n\nver [[projects/alfa/tasks/medir-tempo|medir-tempo]] e [[medir-tempo]]\n";

/// O vault de base: `alfa` com `escrever-spec` (Não iniciada, com descrição e
/// nota), `medir-tempo` (Em implementação), `fechar-ciclo` (Pronta),
/// `sonho-distante` (Algum dia), `adiada-dezembro` (Refinada, `volta-em:
/// 2026-12-01`) e `velha-feita` arquivada em `done`, mais uma nota que cita
/// `medir-tempo`; `beta` com `beta-um` (Não iniciada) e `beta-urgente` (Em
/// implementação, `prioridade: alta`). `status.json` default, sem
/// `CURRENT.md`, tudo envelhecido.
///
/// Em `HOJE` a lista de `alfa` mostra, nesta ordem, `escrever-spec`,
/// `medir-tempo`, `fechar-ciclo`; com `S`, `escrever-spec`,
/// `adiada-dezembro`, `medir-tempo`, `fechar-ciclo`, `sonho-distante`; com
/// `A`, `velha-feita` no fim. A de `beta` mostra `beta-urgente` antes de
/// `beta-um` (ordem medida no oráculo: `freitask list --json --archived`).
pub fn home_base() -> TempDir {
    let home = tempfile::tempdir().expect("tempdir");
    let h = home.path();
    arquivo(h, ".freitask/status.json", gerador::STATUS_JSON);
    arquivo(h, ESCREVER, ESCREVER_MD);
    arquivo(h, MEDIR, MEDIR_MD);
    arquivo(h, FECHAR, FECHAR_MD);
    arquivo(h, SONHO, SONHO_MD);
    arquivo(h, ADIADA, ADIADA_MD);
    arquivo(h, VELHA, VELHA_MD);
    arquivo(h, BETA_UM, BETA_UM_MD);
    arquivo(h, BETA_URGENTE, BETA_URGENTE_MD);
    arquivo(h, NOTAS, NOTAS_MD);
    envelhece(h);
    home
}

/// O `CURRENT.md` do vault de base regenerado em `HOJE` (medido no oráculo:
/// `freitask rebuild` num HOME com o vault de base, no dia 2026-11-10).
pub const PAINEL_BASE: &str = concat!(
    "---\n",
    "date: 2026-11-10\n",
    "weekday: Tuesday\n",
    "---\n",
    "\n",
    "# Planning diário 10/11/26\n",
    "\n",
    "> Painel gerado automaticamente — 5 tasks em 2 projetos.\n",
    "\n",
    "## Notas Avulsas\n",
    "\n",
    "- \n",
    "\n",
    "## alfa\n",
    "\n",
    "> [!todo] Escrever a spec\n",
    "> [[projects/alfa/tasks/escrever-spec|escrever-spec]]\n",
    "> _em curso_\n",
    "> nota um\n",
    "\n",
    "> [!example] Medir o tempo\n",
    "> [[projects/alfa/tasks/medir-tempo|medir-tempo]]\n",
    "\n",
    "> [!check] Fechar o ciclo\n",
    "> [[projects/alfa/tasks/fechar-ciclo|fechar-ciclo]]\n",
    "\n",
    "## beta\n",
    "\n",
    "> [!example] Bê urgente\n",
    "> [[projects/beta/tasks/beta-urgente|beta-urgente]]\n",
    "\n",
    "> [!todo] Bê um\n",
    "> [[projects/beta/tasks/beta-um|beta-um]]\n",
    "\n",
);

// ---------------------------------------------------------------------------
// A sessão e as teclas
// ---------------------------------------------------------------------------

/// Abre a TUI em processo sobre o HOME, no dia `HOJE`.
pub fn abre(home: &Path) -> Sessao {
    abre_no_dia(home, HOJE)
}

/// Abre a TUI em processo sobre o HOME, no dia dado.
pub fn abre_no_dia(home: &Path, dia: &str) -> Sessao {
    Sessao::abrir(home, dia).unwrap_or_else(|e| panic!("Sessao::abrir falhou: {e}"))
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
    let m = if k == KeyCode::BackTab {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(k, m)
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

/// Apaga o campo com Backspace (mais vezes que o texto mais longo do vault).
pub fn apaga(s: &mut Sessao) {
    for _ in 0..40 {
        aperta(s, especial(KeyCode::Backspace));
    }
}

pub fn enter(s: &mut Sessao) {
    aperta(s, especial(KeyCode::Enter));
}

pub fn esc(s: &mut Sessao) {
    aperta(s, especial(KeyCode::Esc));
}

pub fn tab(s: &mut Sessao) {
    aperta(s, especial(KeyCode::Tab));
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

    fn simbolo(&self, x: u16, y: u16) -> &str {
        self.buf[(x, y)].symbol()
    }

    /// O texto da linha `y` entre as colunas `x0..x1`.
    pub fn trecho(&self, y: u16, x0: u16, x1: u16) -> String {
        (x0..x1).map(|x| self.simbolo(x, y)).collect()
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

    /// As linhas que contêm `agulha`.
    pub fn linhas_com(&self, agulha: &str) -> Vec<String> {
        self.linhas()
            .into_iter()
            .filter(|l| l.contains(agulha))
            .collect()
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
        let (tx, ty) = self
            .acha(titulo)
            .unwrap_or_else(|| panic!("não achei o painel {titulo:?} na tela:\n{}", self.texto()));
        let esq = (0..=tx)
            .rev()
            .find(|&x| CANTO_ESQ_CIMA.contains(&self.simbolo(x, ty)))
            .unwrap_or_else(|| panic!("painel {titulo:?} sem canto esquerdo:\n{}", self.texto()));
        let dir = (tx..self.largura())
            .find(|&x| CANTO_DIR_CIMA.contains(&self.simbolo(x, ty)))
            .unwrap_or_else(|| panic!("painel {titulo:?} sem canto direito:\n{}", self.texto()));
        let baixo = (ty + 1..self.altura())
            .find(|&y| CANTO_ESQ_BAIXO.contains(&self.simbolo(esq, y)))
            .unwrap_or(self.altura());
        Painel {
            x0: esq + 1,
            x1: dir,
            y0: ty + 1,
            y1: baixo,
        }
    }

    /// O texto de cada linha de dentro do painel.
    pub fn linhas_do(&self, p: Painel) -> Vec<String> {
        (p.y0..p.y1).map(|y| self.trecho(y, p.x0, p.x1)).collect()
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
    fn marca(&self, p: Painel, y: u16) -> Option<(Color, bool)> {
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
            match self.marca(p, y) {
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
            .map(|(y, _)| self.trecho(y, p.x0, p.x1).trim().to_owned())
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

    /// O título do painel das tasks: a linha de cima da moldura dele.
    pub fn titulo_das_tasks(&self) -> String {
        let p = self.tasks();
        self.trecho(p.y0 - 1, p.x0 - 1, p.x1 + 1)
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
/// TUI de hoje, `ui.rs`).
pub fn barra_de_erro(t: &Tela, msg: &str) {
    assert_eq!(t.barra(), msg, "barra; tela:\n{}", t.texto());
    assert_eq!(t.cor_da_barra(), Color::Red, "um erro sai em vermelho");
}

/// Afirma que a barra mostra `msg` como aviso de sucesso (em verde).
pub fn barra_de_ok(t: &Tela, msg: &str) {
    assert_eq!(t.barra(), msg, "barra; tela:\n{}", t.texto());
    assert_eq!(t.cor_da_barra(), Color::Green, "um ok sai em verde");
}

/// As partes da dica da barra de hoje (`ui.rs`, `draw_bar`): "? ajuda · n nova
/// · s fase · a arquivar · d largar · e abrir · / filtro · q sair".
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

/// O texto da linha de cima do painel do preview em diante, para afirmar o
/// que o preview mostra.
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
        d.contains(id),
        "a task em destaque devia ser {id:?}, é {d:?}; tela:\n{}",
        t.texto()
    );
}

/// O projeto em destaque contém `nome`.
pub fn projeto_selecionado(t: &Tela, nome: &str) {
    let d = t.destacada(t.projetos());
    assert!(
        d.contains(nome),
        "o projeto em destaque devia ser {nome:?}, é {d:?}; tela:\n{}",
        t.texto()
    );
}
