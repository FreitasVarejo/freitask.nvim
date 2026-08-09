-- freitask.config — constantes e caminhos. Não tem lógica nem estado: é o
-- vocabulário fixo do módulo, no nível mais baixo da pilha, de onde todo o
-- resto pode importar sem risco de ciclo.
--
-- Ver docs/freitask.md para o significado de cada uma no formato das tasks.

local M = {}

M.vault = vim.fn.expand("~/ObsidianVault")
M.root = M.vault .. "/tasks"

-- Diretórios sob tasks/ que NÃO são projetos.
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

-- Rótulos do vim.fn.confirm, com `&` marcando a tecla de atalho. As iniciais
-- são distintas de propósito (o/p/f): "done" e "dropped" colidiriam em "d".
-- Fica ao lado de ARCHIVED_TYPES para que acrescentar um tipo sem lhe dar um
-- rótulo apareça na hora, e não como um item mudo no prompt.
M.ARCHIVED_PROMPT = { done = "d&one", dropped = "dro&pped", failed = "&failed" }

-- Callouts que sugerem cada tipo no prompt de arquivamento. Só um DEFAULT: o
-- tipo é escolhido por quem arquiva, senão o caminho viraria uma segunda fonte
-- de verdade do status, obrigada a concordar com o callout para sempre.
M.DONE_CALLOUTS = { done = true, success = true, check = true, tip = true, hint = true }
M.FAILED_CALLOUTS = { failure = true, fail = true, error = true, danger = true, missing = true, bug = true }

-- Metadados de status padrão, espelhados em tasks/status.json no primeiro uso e
-- usados como fallback quando o arquivo está ausente ou corrompido. Cobre todos
-- os tipos de callout suportados pelo render-markdown.nvim (ver
-- https://github.com/MeanderingProgrammer/render-markdown.nvim/wiki/Callouts),
-- ordenados como uma narrativa de workflow: todo → notas/info → em progresso →
-- aguardando/atenção → bloqueado/problema → concluído → referência.
M.DEFAULT_STATUS_JSON = [[{
  "1": { "title": "Todo", "callout": "todo", "icon": "󰗡", "hl_group": "DiagnosticInfo" },
  "2": { "title": "Note", "callout": "note", "icon": "󰋽", "hl_group": "DiagnosticInfo" },
  "3": { "title": "Info", "callout": "info", "icon": "󰋽", "hl_group": "DiagnosticInfo" },
  "4": { "title": "Abstract", "callout": "abstract", "icon": "󰨸", "hl_group": "DiagnosticInfo" },
  "5": { "title": "Summary", "callout": "summary", "icon": "󰨸", "hl_group": "DiagnosticInfo" },
  "6": { "title": "TL;DR", "callout": "tldr", "icon": "󰨸", "hl_group": "DiagnosticInfo" },
  "7": { "title": "Example", "callout": "example", "icon": "󰉹", "hl_group": "DiagnosticHint" },
  "8": { "title": "Important", "callout": "important", "icon": "󰅾", "hl_group": "DiagnosticHint" },
  "9": { "title": "Question", "callout": "question", "icon": "󰘥", "hl_group": "DiagnosticWarn" },
  "10": { "title": "Help", "callout": "help", "icon": "󰘥", "hl_group": "DiagnosticWarn" },
  "11": { "title": "FAQ", "callout": "faq", "icon": "󰘥", "hl_group": "DiagnosticWarn" },
  "12": { "title": "Attention", "callout": "attention", "icon": "󰀪", "hl_group": "DiagnosticWarn" },
  "13": { "title": "Warning", "callout": "warning", "icon": "󰀪", "hl_group": "DiagnosticWarn" },
  "14": { "title": "Caution", "callout": "caution", "icon": "󰳦", "hl_group": "DiagnosticWarn" },
  "15": { "title": "Bug", "callout": "bug", "icon": "󰨰", "hl_group": "DiagnosticError" },
  "16": { "title": "Failure", "callout": "failure", "icon": "󰅖", "hl_group": "DiagnosticError" },
  "17": { "title": "Fail", "callout": "fail", "icon": "󰅖", "hl_group": "DiagnosticError" },
  "18": { "title": "Missing", "callout": "missing", "icon": "󰅖", "hl_group": "DiagnosticError" },
  "19": { "title": "Danger", "callout": "danger", "icon": "󱐌", "hl_group": "DiagnosticError" },
  "20": { "title": "Error", "callout": "error", "icon": "󱐌", "hl_group": "DiagnosticError" },
  "21": { "title": "Tip", "callout": "tip", "icon": "󰌶", "hl_group": "DiagnosticOk" },
  "22": { "title": "Hint", "callout": "hint", "icon": "󰌶", "hl_group": "DiagnosticOk" },
  "23": { "title": "Success", "callout": "success", "icon": "󰄬", "hl_group": "DiagnosticOk" },
  "24": { "title": "Check", "callout": "check", "icon": "󰄬", "hl_group": "DiagnosticOk" },
  "25": { "title": "Done", "callout": "done", "icon": "󰄬", "hl_group": "DiagnosticOk" },
  "26": { "title": "Quote", "callout": "quote", "icon": "󱆨", "hl_group": "Comment" },
  "27": { "title": "Cite", "callout": "cite", "icon": "󱆨", "hl_group": "Comment" }
}]]

-- Status sentinela (0) para callouts cujo tipo não existe no status.json.
-- Deliberadamente FORA do status.json: aquele arquivo lista estados válidos, e
-- 0 não é um estado que se escolhe — é o que sobra quando o tipo não casa.
-- Note que quote/cite (26/27) NÃO caem aqui: são estados válidos e deliberados
-- para material de referência, e devem continuar ordenando por último.
M.STATUS_INVALID = { title = "Sem status", callout = "invalid", icon = "󰘸", hl_group = "DiagnosticError" }

-- Títulos do status.json ANTIGO (pré-callouts), que já não existem no atual.
-- Valor = status equivalente hoje, para o caso raro de a linha 1 não ter
-- callout. Usado só por is_status_text, para reconhecer e descartar as linhas
-- "> N - Título" espalhadas pelo vault durante a migração.
M.LEGACY_STATUS_TITLES = {
  ["Backlog"] = 1, -- todo
  ["In Progress"] = 7, -- example
  ["Blocked"] = 13, -- warning
  ["Review"] = 9, -- question
}

return M
