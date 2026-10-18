-- freitask.config — constantes e caminhos. Não tem lógica nem estado: é o
-- vocabulário fixo do módulo, no nível mais baixo da pilha, de onde todo o
-- resto pode importar sem risco de ciclo.
--
-- Ver docs/freitask.md para o significado de cada uma no formato das tasks.

local M = {}

M.vault = vim.fn.expand("~/ObsidianVault")

-- As tasks moram DENTRO da pasta do projeto no vault, junto da spec e das
-- decisões: `projects/<projeto>/tasks/`. Antes viviam numa árvore própria,
-- `tasks/<projeto>/`, e o efeito era que o mesmo projeto existia em dois
-- lugares — a frente de trabalho longe do porquê que a justifica.
--
-- É o que define o que é um projeto: um diretório sob `projects/` que tem um
-- `tasks/` dentro. Sem regra de nome, sem lista de exceções — a presença da
-- pasta é o registro.
M.projects = M.vault .. "/projects"

-- O painel e os snapshots sobem para a raiz do vault. O painel cruza projetos,
-- então não pertence a nenhum; os snapshots são história do painel.
M.current = M.vault .. "/CURRENT.md"
M.daily = M.vault .. "/daily"

-- Configuração, não conteúdo: fica em pasta oculta para o Obsidian não a
-- listar junto das notas.
M.status_file = M.vault .. "/.freitask/status.json"

-- Nomes que não podem virar projeto. `daily` e `templates` já não colidiriam
-- com nada (saíram da árvore de tasks), mas seguem vetados porque reintroduzir
-- qualquer um deles como projeto traria de volta a ambiguidade que a mudança
-- de layout acabou de remover.
M.RESERVED = { daily = true, templates = true }

-- Tipos de arquivamento = subdiretórios de tasks/<projeto>/archived/.
-- São três de propósito: `done` e `failed` são deriváveis do callout (ver
-- path.suggest_archive_type) e `dropped` é a única decisão que o status não
-- carrega ("não vou fazer"). Um quarto balde genérico ("misc") viraria o
-- destino de tudo que se arquiva com pressa, sem distinguir nada; e a
-- assimetria pesa — criar um diretório depois é trivial, esvaziar um cheio não.
M.ARCHIVED_TYPES = { "done", "dropped", "failed" }

M.ARCHIVED_SET = {}
for _, t in ipairs(M.ARCHIVED_TYPES) do
  M.ARCHIVED_SET[t] = true
end

-- Glosa em PT-BR de cada tipo, só para a linha de histórico no rodapé. Fica
-- fora do status.json porque não é um estado que se escolhe: é a tradução do
-- nome do diretório, que é o dado real.
M.ARCHIVED_LABELS = { done = "feito", dropped = "abandonado", failed = "falhou" }

-- Callouts que sugerem cada tipo de arquivamento. Só um DEFAULT: o
-- tipo é escolhido por quem arquiva, senão o caminho viraria uma segunda fonte
-- de verdade do status, obrigada a concordar com o callout para sempre.
--
-- FAILED_CALLOUTS está VAZIO desde que o vocabulário encolheu: nenhuma fase
-- significa "falhou" (`warning` é bloqueada, que é outra coisa — quem travou
-- não fracassou). `failed` segue existindo como destino de arquivamento, só
-- deixou de ter default: escolhe-se à mão no prompt. Preferi a tabela vazia a
-- apagá-la — no dia em que uma fase de falha entrar, o lugar de registrá-la
-- fica óbvio.
M.DONE_CALLOUTS = { done = true, check = true }
M.FAILED_CALLOUTS = {}

-- Callouts de fases que a TUI e o CURRENT.md escondem por padrão: o "Algum dia",
-- backlog que o dono aceita esquecer, mas quer poder lembrar. É regra do motor
-- e sai no `list --json` como `hidden`, porque a TUI não conhece regra de task.
-- Por callout e não por número: o número ordena o board e muda quando o
-- vocabulário cresce.
M.HIDDEN_CALLOUTS = { quote = true }

-- Metadados de status padrão, espelhados no status.json no primeiro uso e
-- usados como fallback quando o arquivo está ausente ou corrompido.
--
-- São as fases de execução mais `done`, e não os 27 callouts que o
-- render-markdown.nvim suporta. O vocabulário largo não estava sendo escolhido
-- — 36 tasks ativas, todas em `todo` — e status que ninguém escolhe não informa
-- nada: a diferenciação real terminava na frase em itálico, que é texto livre e
-- não se consulta. Cada fase existe porque alguém precisa distingui-la de outra
-- para não atropelá-la.
--
-- O número ordena o board, então a ordem numérica É a narrativa: não iniciada →
-- refinando → refinada → implementando → homologando → bloqueada → pronta →
-- algum dia. `done` fecha a lista porque é o que `archive` grava, não uma fase
-- que se escolhe no form; por isso "Algum dia" é a última fase que aparece.
--
-- Os callouts de `tip` (Refinada) e `quote` (Algum dia) saem de famílias que o
-- Obsidian do celular não confunde com as outras: ele renderiza igual os tipos
-- da mesma família (`check`/`done`/`success`, `question`/`help`/`faq`,
-- `warning`/`caution`/`attention`, `abstract`/`summary`/`tldr`).
--
-- Os nomes de callout honram o vocabulário ANTIGO deste plugin, preservado em
-- LEGACY_STATUS_TITLES: `example` já era "In Progress", `question` já era
-- "Review" e `warning` já era "Blocked". Inventar três nomes novos tornaria as
-- linhas legadas do vault ilegíveis sem ganho nenhum.
--
-- Sair do status.json NÃO tira um callout do Obsidian: `note`, `info` e os
-- demais seguem renderizando no corpo das notas. Este arquivo governa só como o
-- freitask lê o PRIMEIRO bloco de uma task.
M.DEFAULT_STATUS_JSON = [[{
  "1": { "title": "Não iniciada", "callout": "todo", "icon": "󰗡", "hl_group": "DiagnosticInfo" },
  "2": { "title": "Refinando", "callout": "abstract", "icon": "󰨸", "hl_group": "DiagnosticInfo" },
  "3": { "title": "Refinada", "callout": "tip", "icon": "󰌶", "hl_group": "DiagnosticHint" },
  "4": { "title": "Em implementação", "callout": "example", "icon": "󰉹", "hl_group": "DiagnosticHint" },
  "5": { "title": "Em homologação", "callout": "question", "icon": "󰘥", "hl_group": "DiagnosticWarn" },
  "6": { "title": "Bloqueada", "callout": "warning", "icon": "󰀪", "hl_group": "DiagnosticWarn" },
  "7": { "title": "Pronta", "callout": "check", "icon": "󰄬", "hl_group": "DiagnosticOk" },
  "8": { "title": "Algum dia", "callout": "quote", "icon": "󱆨", "hl_group": "Comment" },
  "9": { "title": "Arquivada", "callout": "done", "icon": "󰄬", "hl_group": "DiagnosticOk" }
}]]

-- Status sentinela (0) para callouts cujo tipo não existe no status.json.
-- Deliberadamente FORA do status.json: aquele arquivo lista estados válidos, e
-- 0 não é um estado que se escolhe — é o que sobra quando o tipo não casa.
-- Um callout do Obsidian que não é fase (um `[!note]` ou `[!quote]` no corpo)
-- não cai aqui: o status vem só do PRIMEIRO bloco, e o resto do arquivo o
-- freitask nem olha.
M.STATUS_INVALID = { title = "Sem status", callout = "invalid", icon = "󰘸", hl_group = "DiagnosticError" }

-- Títulos do status.json ANTIGO (pré-callouts), que já não existem no atual.
-- Valor = o CALLOUT equivalente hoje, para o caso raro de a linha 1 não ter
-- callout. Em callout e não em número: o número depende do status.json de cada
-- vault, e o callout é o que o arquivo guarda. Usado só por is_status_text,
-- para reconhecer e descartar as linhas "> N - Título" espalhadas pelo vault
-- durante a migração.
M.LEGACY_STATUS_TITLES = {
  ["Backlog"] = "todo",
  ["In Progress"] = "example",
  ["Blocked"] = "warning",
  ["Review"] = "question",
}

return M
