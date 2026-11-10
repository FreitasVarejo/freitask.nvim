//! Aceite da fatia 7 — TUI (spec §5, §1.7 e §3.5; decisões 54 e f7.1 de
//! `docs/decisoes-da-reescrita.md`).
//!
//! Fala com o contrato mínimo em processo, `freitask::tui::{Sessao, Pedido}`
//! (`crates/freitask/src/tui.rs`): `Sessao::abrir(home, hoje)`,
//! `tecla(KeyEvent) -> Pedido` e `tela(largura, altura) -> Buffer`; e com o
//! binário de verdade num pseudoterminal. Afirma o que a pessoa vê (o texto
//! da grade e, onde o estilo é o significado, o destaque da linha
//! selecionada e a cor de erro ou de ok na barra) e o que o vault guarda
//! depois da tecla; nunca o estado interno.
//!
//! Fontes: a spec (§1.7, §3.5), `docs/freitask.md` ("Uso interativo (TUI)",
//! "Volta em", "Status", "Arquivar / desarquivar / deletar", "Referências do
//! Obsidian", "`CURRENT.md`") e a TUI de hoje em `tui/src/` (`app.rs`,
//! `ui.rs`, `main.rs`, `engine.rs`), que é o oráculo da paridade: as teclas,
//! os modais, os textos da barra e da ajuda vêm de lá. O conteúdo dos
//! arquivos gravados e as mensagens de recusa do motor foram medidos no
//! oráculo Lua (`nvim -l lua/freitask/cli.lua`) num HOME temporário, no dia
//! 2026-11-10 (o `HOJE` destes testes); são os mesmos já fixados nas fatias 2
//! a 4. Nenhum valor esperado aqui foi gerado do binário sob teste.
//!
//! Módulos: `abrir` (o `rebuild` ao abrir e a falha dele), `teclas` (cada
//! tecla, o que mostra e o que grava), `telas` (as asserções semânticas da
//! tela em 100×30 e 60×15), `passeio` (teclas pseudoaleatórias sobre vaults
//! sintéticos, sem pânico) e `pty` (o binário num terminal).
//!
//! Decisões tomadas aqui, onde a spec, o doc e a TUI de hoje não fecham
//! (repetidas no relatório do rascunho):
//! - a semântica de cada tecla é a de `app.rs`: ao abrir, o foco está nos
//!   projetos e a primeira task do primeiro projeto está selecionada; `j`/`k`
//!   movem no painel com foco, sem dar a volta; trocar de projeto volta a
//!   task para o topo; `Enter` nos projetos só passa o foco; `e` abre a task
//!   selecionada de qualquer painel; teclas de task agem sobre a task
//!   selecionada qualquer que seja o foco;
//! - os textos da barra e dos modais são os de `ui.rs`/`app.rs` (a dica, "
//!   Nova task em <p>", " Título ", " Descrição do estado (vazio apaga) ", "
//!   Novo id (renomeia arquivo e links do vault) ", " Novo projeto ", " Fase
//!   de <id> ", " Arquivar <id> como ", " Confirmar ", " Teclas ", " Projetos
//!   ", " Tasks: <p> ", " Preview ", "sem task selecionada", as mensagens
//!   "criada: <id>", "título atualizado", "descrição atualizada", "renomeada:
//!   <a> → <b>", "projeto criado", "fase: <ícone> <título>", "arquivada como
//!   <tipo>: <id>", "largada: <id>", "desarquivada: <id>", "relido", "falta o
//!   título", "não pode ficar vazio", "nenhuma task selecionada", "crie um
//!   projeto antes (N)", "<id> já está arquivada (a desarquiva)"). Os
//!   modais se acham pelo título na moldura; a dica da barra se confere por
//!   partes, na ordem, para que as teclas `b` e `p` das fatias 9b e 10b
//!   possam entrar nela sem mexer aqui;
//! - recusa do motor: a barra mostra exatamente a mensagem que a CLI imprime
//!   em stderr, sem o `freitask: ` (`engine.rs` só junta linhas com " · ";
//!   aqui toda recusa tem uma linha). As de caminho longo se leem numa tela
//!   larga, para não depender do tamanho do diretório temporário;
//! - "destaque" é o fundo (ou a inversão) da primeira célula com texto da
//!   linha diferente do fundo do painel; a cor exata do destaque não se fixa.
//!   Erro sai em vermelho e ok em verde (`ui.rs`, `draw_bar`): as duas cores
//!   se fixam, porque são o significado da mensagem. A lista de tasks mostra
//!   o destaque depois de `l` (foco nelas); com o foco nos projetos, a task
//!   selecionada se lê no preview;
//! - `N` cria o projeto e não grava arquivo nenhum (o `new-project` não
//!   regenera o painel, decisão da fatia 3); `R` relê e não grava nada (hoje
//!   só relê, sem `rebuild`); `c` regenera e pede para abrir o
//!   `<vault>/CURRENT.md`; se o `rebuild` do `c` falhar, a barra mostra o erro
//!   em vermelho e nada se pede;
//! - toda tecla que grava pelo motor regenera o `CURRENT.md` (como a CLI de
//!   hoje faz em `new`, `set`, `archive`, `unarchive`, `rename`); o conjunto
//!   exato de arquivos mudados se afirma;
//! - na ajuda, a tecla seguinte só a fecha (`q` não sai); no pick, `q` e
//!   `Esc` fecham; na confirmação, tudo que não é `y` cancela; no filtro, as
//!   letras são texto (`q` e `j` entram no filtro), e `Enter` fixa o filtro no
//!   título do painel (`/<filtro>`); a barra mostra `/<filtro>` enquanto se
//!   digita; `Esc` no modo normal limpa o filtro e a mensagem;
//! - maiúscula chega com `SHIFT`, como a crossterm a entrega (`A`, `G`, `N`,
//!   `R`, `S`, `Á`); `Shift-Tab` volta ao campo anterior do modal de task nova;
//! - a data de volta da adiada e o `+algum-dia/adiadas` se leem em 160
//!   colunas: em 100, o painel do meio (35%) corta a linha da adiada já hoje;
//! - `C-d`/`C-u`: o quanto rola não se fixa; só que `C-d` mostra linhas mais
//!   abaixo e `C-u` volta ao topo;
//! - o `rebuild` que falha ao abrir é provocado por um `CURRENT.md` que é
//!   pasta; o motivo é do motor e só se afirma que existe;
//! - o passeio aleatório afirma "nada fora do HOME" por um diretório
//!   sentinela ao lado do HOME e pelo diretório corrente do teste; o lock de
//!   escrita (spec §1.5, `XDG_RUNTIME_DIR`) fica fora dessa afirmação;
//! - no PTY, o `$NVIM` é um caminho qualquer e o `nvim` falso grava os
//!   argumentos: exatamente `--server <sock> --remote-expr <expr>`, com o
//!   caminho absoluto da task dentro da expressão (a forma da expressão não
//!   se fixa); o `$EDITOR` recebe o caminho como único argumento; "o
//!   terminal volta" é sair da tela alternativa e mostrar o cursor; o modo
//!   raw se prova pelo `Ctrl-C` que sai com 0 (sem raw seria SIGINT);
//! - fora do escopo (fatias 9b e 10b): a tecla `b`, a marca de bloqueada, a
//!   tecla `p` e o prazo na tela; nada aqui afirma nem a presença nem a
//!   ausência deles.

mod abrir;
mod passeio;
mod pty;
mod teclas;
mod telas;
mod util;
