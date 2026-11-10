//! Aceite da fatia 10b — Prazo, parte da TUI (spec §5, §10.2 e §3.5;
//! decisões 54, f7.1 e f10b.1 a f10b.4 de `docs/decisoes-da-reescrita.md`):
//! a tecla `p`, o `prazo DD/MM` na linha da task, I11 na TUI e a linha da
//! ajuda.
//!
//! Fala com o contrato em processo da fatia 7, `freitask::tui::{Sessao,
//! Pedido}`: `Sessao::abrir(home, hoje)`, `tecla(KeyEvent) -> Pedido` e
//! `tela(largura, altura) -> Buffer`, e nada mais (decisão 54: o rascunho
//! compila contra o código de hoje). Afirma o que a pessoa vê (o texto da
//! grade e, onde o estilo é o significado, a cor do prazo e da barra) e o que
//! o vault guarda depois da tecla; nunca o estado interno.
//!
//! Fontes: a spec (§3.5, §10.2), as decisões f10b.1 a f10b.4 (o título do
//! prompt, as mensagens da barra, a posição e as cores do prazo, a linha da
//! ajuda), o aceite congelado da fatia 7 (o estilo, os helpers copiados em
//! `util.rs`, o vault de base e o painel dele, medido no oráculo) e o aceite
//! congelado da 10a (os bytes que o `set --prazo` grava no frontmatter e a
//! mensagem de data inválida). Nenhum valor esperado aqui foi gerado do
//! binário sob teste. O dia fixo é `HOJE` = 2026-11-10, como na fatia 7.
//!
//! Módulos: `teclas` (a tecla `p`), `telas` (o prazo na linha, a ajuda, o
//! terminal estreito) e `invariante` (I11 na TUI).
//!
//! Decisões tomadas aqui, onde a spec e as f10b.N não fecham (repetidas no
//! relatório do rascunho):
//! - "amarelo" e "vermelho" são `Color::Yellow` e `Color::Red`, como a fatia
//!   7 fixa `Red`/`Green` na barra; afirma-se o fg de toda célula não branca
//!   de `prazo DD/MM`. Só se leem linhas fora do destaque: no vault com
//!   prazos, a selecionada ao abrir é `livre` (prioridade alta, sem prazo);
//! - a linha se lê em 160 colunas (como a `volta` na fatia 7) e a ordem é
//!   id, `volta AAAA-MM-DD` (se houver), `prazo DD/MM`, título; o que vem
//!   antes do id (ícone, `!`, `[tipo]`) e o espaçamento não se fixam;
//! - "a arquivada, o prazo inválido e o vazio não mostram" se afirma junto
//!   com uma linha da mesma tela que tem de mostrar: sozinho, o teste
//!   passaria numa TUI sem prazo nenhum. `prazo:` vazio conta como sem prazo,
//!   como no JSON da 10a;
//! - o dia que conta é o da sessão (`Sessao::abrir(home, dia)`): o mesmo vault
//!   aberto em outro dia muda a cor;
//! - gravar um prazo muda a task e, no máximo, o `CURRENT.md`, que fica byte a
//!   byte o mesmo (o prazo não entra no painel; a 10a aceita os dois em
//!   `set_prazo_so_grava_a_task_e_o_painel`). Que o `p` regenera o painel,
//!   como toda tecla que grava na fatia 7, se prova por uma edição de fora
//!   depois de abrir: aí o conjunto exato é a task e o `CURRENT.md`;
//! - "igual ao que está escrito" inclui o prazo inválido confirmado sem
//!   mudança: não grava, e o prompt fecha (como `t`, `i`, `r` na fatia 7). A
//!   barra nesse caso não se fixa;
//! - o texto aparado: espaços em volta de uma data válida não a recusam e não
//!   vão para o arquivo;
//! - no prompt, as letras são texto: `quando` é digitado (o `q` não sai) e
//!   recusado como data;
//! - a barra da recusa se lê numa tela larga, para a mensagem caber inteira;
//! - a linha da ajuda é a que, aparada, começa por `p` e um espaço e traz
//!   "definir / limpar o prazo"; vem depois da linha do `i` (f10b.4), sem
//!   fixar que é a seguinte (a 9b acrescenta `b` em paralelo). O resto da
//!   ajuda, a quantidade de linhas e a dica da barra não se fixam (a dica se
//!   confere só por partes, como na fatia 7, ao voltar do `Esc`);
//! - I11 na TUI: um vault escrito à mão, com e sem `prazo:`, comparado nos
//!   modos normal, `S`, `A` e `S`+`A`, por projeto (a ordem dos ids) e no
//!   painel de projetos (os itens inteiros, com a contagem). O conjunto de
//!   ids de cada modo também se confere contra a spec, para a comparação não
//!   ser vazia. A ordem em si não se fixa: é a da fatia 7.
//!
//! Fica de fora: a tecla `b`, a marca `⛔` e `bloqueada-por` (fatia 9b; nada
//! aqui afirma presença nem ausência deles); `p` numa task arquivada (f10b.1
//! não fixa); o preview (f10b.2: não ganha linha, o frontmatter já está no
//! conteúdo); os snapshots (do implementador); tudo da CLI que a 10a fixou.

mod invariante;
mod teclas;
mod telas;
mod util;
