# AGENTS.md — freitask.nvim

O freitask: um binário Rust (workspace na raiz, `crates/`) que é a CLI e a TUI. Até o
cutover, `lua/` e `tui/` ficam como oráculo (`docs/freitask-internals.md`, "Até o
cutover").

- Antes de mexer no código: `docs/freitask-internals.md` — mapa dos módulos, regras
  de dependência e os portões antes de dar por pronto.
- Formato das tasks e o contrato com agentes: `docs/freitask.md`.
- O porquê e o backlog moram no vault: `docs/agents/domain.md`.
- O vault tem contrato próprio: `~/ObsidianVault/AGENTS.md`.
- Reescrita em Rust em curso na branch `reescrever-o-freitask` (spec em
  `~/ObsidianVault/projects/workflow-ia/reescrita-do-freitask-spec.md`): os alvos
  são os do `Makefile` (`make help`). Teste de `tests/aceite/` e snapshot não se
  afrouxam nem se apagam; teste que parecer errado é registrado em
  `docs/decisoes-da-reescrita.md` e reavaliado por outro agente antes de mudar
  (spec, §4).
- Complexidade nova entra só por dor medida: a que já aconteceu em dado do vault ou
  já custou trabalho, não a que o desenho prevê. Ideia sem dor medida vira task em
  "Algum dia".
