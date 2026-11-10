//! Aceite da fatia 9b — Dependências, parte da TUI (spec §5, §3.5, §10.1;
//! decisões 54, 61, 62, f7.1, f9b.1 e f9b.2 de `docs/decisoes-da-reescrita.md`):
//! a tecla `b`, que edita a lista `bloqueada-por` da task selecionada num
//! prompt, e a marca `⛔ ` antes do id da task bloqueada. O prazo (tecla `p`,
//! `prazo DD/MM` na linha) é a 10b: nada aqui afirma a presença nem a ausência
//! dele, e nenhum teste aperta `p`.
//!
//! Fala com o mesmo contrato em processo da fatia 7, `freitask::tui::{Sessao,
//! Pedido}` (`Sessao::abrir(home, hoje)`, `tecla`, `tela`), e com o vault em
//! disco. Afirma o que a pessoa vê (a grade, e a cor onde ela é o
//! significado) e o que o vault guarda; nunca o estado interno.
//!
//! Fontes: a spec (§3.5, §10.1), f9b.1 (a tecla `b`: a moldura do prompt, o
//! texto preenchido, as mensagens e as cores da barra, a linha da ajuda) e
//! f9b.2 (a marca: quando e onde). O conteúdo gravado é o formato que o aceite
//! da 9a fixou para o `set --bloqueada-por` (chave nova no fim do
//! frontmatter, frontmatter criado quando falta, linha trocada no lugar, a
//! linha some com `""`). Nenhum valor esperado aqui foi gerado do binário sob
//! teste.
//!
//! Módulos: `teclas` (a tecla `b`: o prompt, o que grava, as recusas, `Esc`,
//! `C-u`, a ajuda) e `telas` (a marca em 100×30 e 60×15).
//!
//! Decisões tomadas aqui, onde a spec, f9b.1 e f9b.2 não fecham o caso
//! (repetidas no relatório do rascunho):
//! - o texto das recusas: f9b.1 manda a barra mostrar "a mensagem que a CLI
//!   imprime em stderr sem o `freitask: `", mas o aceite da 9a deixou esse
//!   texto solto (só o caminho do ciclo se fixou). Os textos daqui foram
//!   medidos uma vez na CLI da 9a já integrada (`freitask set <id>
//!   --bloqueada-por …` num HOME temporário, em 2026-11-10) e escritos à mão:
//!   `task não encontrada: <id>`, `id "<id>" é ambíguo (2 arquivos); passe o
//!   caminho`, `a task não pode ser bloqueada por si mesma: <id>` e `ciclo de
//!   dependência: a → b → a` (o caminho começa na task editada, como no
//!   exemplo de f9b.1);
//! - a marca ocupa duas colunas na grade: a célula logo depois do `⛔` é a
//!   metade escondida dela (em branco), e a linha se lê sem essa célula.
//!   "Colada antes do id" é: `⛔`, a metade escondida, um espaço e o id;
//! - "nenhuma linha passa da largura" é a moldura do painel das tasks inteira
//!   nas duas bordas, de cima a baixo, com as marcas na tela: o emoji largo
//!   não come a borda. Os ids do vault destes testes são curtos para a linha
//!   marcada caber inteira em 60×15;
//! - a ordem da linha (f9b.2): a coluna `! ` da prioridade, o ícone da fase e
//!   o `[tipo] ` da arquivada vêm antes da marca, a marca imediatamente antes
//!   do id, e a `volta` da adiada depois do id. A ordem entre o ícone e o
//!   `[tipo] ` é a da fatia 7 e não se fixa aqui;
//! - nem o preview, nem o painel dos projetos, nem o `CURRENT.md` ganham a
//!   marca (f9b.2: "nada muda no preview nem no `CURRENT.md`");
//! - "texto igual ao que veio preenchido não grava nada": o prompt fecha e
//!   nenhum arquivo muda (nem o `CURRENT.md`); a barra fica solta, como na 7.
//!   O caso inclui o prompt vazio de uma task sem a chave: `Enter` não cria
//!   frontmatter nem `bloqueada-por: []`;
//! - "aparado": espaços em volta do texto digitado não chegam à lista;
//! - a gravação pelo `b` muda a task e, no máximo, o `CURRENT.md`, que fica
//!   com o mesmo conteúdo (é o que a 9a fixou para o `set`); a mensagem de
//!   sucesso é `bloqueada-por atualizada` em verde;
//! - a moldura do prompt se confere inteira em 100×30; o campo é o texto da
//!   linha de dentro dela, aparado;
//! - fora: `b` numa task arquivada (f9b.1 não diz se o motor aceita), texto
//!   igual ao preenchido com espaço sobrando, e a dica da barra de baixo
//!   (f9b.1 diz que não muda; a 7 a confere por partes, e um teste dela aqui
//!   passaria sem implementação).

mod teclas;
mod telas;
mod util;
