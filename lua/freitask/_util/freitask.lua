-- util.freitask — núcleo do freitask baseado em Obsidian.
--
-- Opera exclusivamente sob ~/ObsidianVault/tasks/. Cada projeto é um
-- subdiretório (id kebab-case) e cada task é um arquivo markdown cujo bloco
-- inicial (`>` blockquote) codifica status/título/id/nome, e da linha 4 em
-- diante linhas arbitrárias (impedimentos, notas — texto livre). O id da task
-- (= nome do arquivo) é também o nome da branch associada; por isso não há
-- linha `Branch:` separada — o wikilink na linha 2 representa ambos. Formato
-- atual do bloco:
--   > [!todo] Título da task                          ← callout dá ícone/cor do status + título
--   > [[tasks/projeto/id-da-task|id-da-task]]          ← link do obsidian (caminho + alias == id == branch)
--   > _Em análise do Fábio_                            ← descrição OPCIONAL do estado (itálico)
--   > qualquer outra linha                             ← notas/impedimentos (texto livre)
-- O status é derivado EXCLUSIVAMENTE do TIPO do callout da linha 1 (mapeado
-- via status.json, que cobre todos os callouts do render-markdown.nvim).
--
-- A descrição do estado é identificada pelo MARCADOR de itálico, não pela
-- posição: uma linha inteiramente `_..._` (antes de qualquer nota) é a
-- descrição; qualquer outra linha é nota. Isso evita a ambiguidade posicional
-- de "sem descrição + uma nota" vs. "com descrição + sem nota", e é por isso
-- que a linha é simplesmente omitida quando não há descrição (nunca gravamos
-- um `>` vazio no meio do bloco).
--
-- Um callout cujo tipo não existe no status.json vira `status_num = 0`
-- (STATUS_INVALID) e o tipo digitado é preservado verbatim no arquivo, para
-- que a task continue editável pelo <leader>oe e o erro apareça no form em vez
-- de sumir. Status 0 ordena no topo do projeto, gritando para ser consertado.
-- O form é ESTRITO (recusa tipo desconhecido); só o parser é tolerante.
--
-- O parser também aceita os formatos legados (título em negrito, `[[id]]` sem
-- caminho, status-texto `N - Título` na linha 3, e/ou linha `> Branch:`) para
-- migração; ver M.migrate_format. O antigo "nome livre" da linha 3 vira nota,
-- pois sua semântica era "nome", não "estado" — tratá-lo como descrição de
-- estado inventaria informação.
--
-- Uma task pode ser arquivada (movida para tasks/<projeto>/archived/<tipo>/,
-- com tipo ∈ done|dropped|failed), desarquivada (de volta para
-- tasks/<projeto>/) ou deletada de fato (arquivo removido do disco). O CAMINHO
-- é a única fonte de verdade do arquivamento — não existe flag no frontmatter —
-- assim como o callout é a única fonte do status. Como archived/ acrescenta
-- dois níveis, o glob do cache (tasks/*/*.md) já as ignora de graça.
--
-- Arquivar, desarquivar e renomear o id são o MESMO evento ("o arquivo mudou de
-- endereço") e por isso compartilham M.retarget_links, que reescreve os
-- wikilinks do vault inteiro. Como o Obsidian resolve `[[foo]]` por basename em
-- qualquer pasta, mudar de pasta só quebra os links path-qualified; renomear o
-- id quebra também os curtos (e o `id:` do frontmatter).
--
-- Este módulo é `require`-ável (`require("util.freitask")`) para que tanto o
-- plugin do picker (plugins/freitask.lua) quanto os keymaps buffer-local
-- de edição por cursor possam consumi-lo. Toca apenas arquivos + Snacks +
-- vim.ui, portanto NÃO usa a API do obsidian.nvim (configurado à parte).

local M = {}

local vault = vim.fn.expand("~/ObsidianVault")
local root = vault .. "/tasks"

-- Diretórios sob tasks/ que NÃO são projetos.
local RESERVED = { daily = true, templates = true }

-- Tipos de arquivamento = subdiretórios de tasks/<projeto>/archived/.
-- São três de propósito: `done` e `failed` são deriváveis do callout (ver
-- M.suggest_archive_type) e `dropped` é a única decisão que o status não
-- carrega ("não vou fazer"). Um quarto balde genérico ("misc") viraria o
-- destino de tudo que se arquiva com pressa, sem distinguir nada; e a
-- assimetria pesa — criar um diretório depois é trivial, esvaziar um cheio não.
M.ARCHIVED_TYPES = { "done", "dropped", "failed" }

local ARCHIVED_SET = {}
for _, t in ipairs(M.ARCHIVED_TYPES) do
  ARCHIVED_SET[t] = true
end

-- Glosa em PT-BR de cada tipo, só para a linha de histórico no rodapé. Fica
-- fora do status.json porque não é um estado que se escolhe: é a tradução do
-- nome do diretório, que é o dado real.
local ARCHIVED_LABELS = { done = "feito", dropped = "abandonado", failed = "falhou" }

-- Callouts que sugerem cada tipo no prompt de arquivamento. Só um DEFAULT: o
-- tipo é escolhido por quem arquiva, senão o caminho viraria uma segunda fonte
-- de verdade do status, obrigada a concordar com o callout para sempre.
local DONE_CALLOUTS = { done = true, success = true, check = true, tip = true, hint = true }
local FAILED_CALLOUTS = { failure = true, fail = true, error = true, danger = true, missing = true, bug = true }

-- Metadados de status padrão, espelhados em tasks/status.json no primeiro uso e
-- usados como fallback quando o arquivo está ausente ou corrompido. Cobre todos
-- os tipos de callout suportados pelo render-markdown.nvim (ver
-- https://github.com/MeanderingProgrammer/render-markdown.nvim/wiki/Callouts),
-- ordenados como uma narrativa de workflow: todo → notas/info → em progresso →
-- aguardando/atenção → bloqueado/problema → concluído → referência.
local DEFAULT_STATUS_JSON = [[{
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
local STATUS_INVALID = { title = "Sem status", callout = "invalid", icon = "󰘸", hl_group = "DiagnosticError" }

-- Títulos do status.json ANTIGO (pré-callouts), que já não existem no atual.
-- Valor = status equivalente hoje, para o caso raro de a linha 1 não ter
-- callout. Usado só por is_status_text, para reconhecer e descartar as linhas
-- "> N - Título" espalhadas pelo vault durante a migração.
local LEGACY_STATUS_TITLES = {
  ["Backlog"] = 1, -- todo
  ["In Progress"] = 7, -- example
  ["Blocked"] = 13, -- warning
  ["Review"] = 9, -- question
}

---Metadados do status `num`, com fallback para o sentinela de inválido.
---@param num integer|nil
---@return table
function M.status_meta(num)
  if not num or num == 0 then
    return STATUS_INVALID
  end
  return (M.status and M.status[tostring(num)]) or STATUS_INVALID
end

-- Dobra de acentos multibyte-safe para ids kebab-case (títulos PT-BR).
local ACCENTS = {
  ["á"] = "a",
  ["à"] = "a",
  ["â"] = "a",
  ["ã"] = "a",
  ["ä"] = "a",
  ["é"] = "e",
  ["ê"] = "e",
  ["è"] = "e",
  ["ë"] = "e",
  ["í"] = "i",
  ["ì"] = "i",
  ["î"] = "i",
  ["ï"] = "i",
  ["ó"] = "o",
  ["ô"] = "o",
  ["õ"] = "o",
  ["ò"] = "o",
  ["ö"] = "o",
  ["ú"] = "u",
  ["ù"] = "u",
  ["û"] = "u",
  ["ü"] = "u",
  ["ç"] = "c",
  ["ñ"] = "n",
}

---Converte uma string arbitrária num identificador kebab-case.
---@param s string
---@return string
local function kebab(s)
  -- vim.fn.tolower e não s:lower(): o lower() do Lua só mexe em bytes ASCII,
  -- então "Á" não virava "á", não casava a tabela de dobra abaixo e acabava
  -- descartado por [^%w%s%-] — "Ávida" saía como "vida". Passou a importar
  -- agora que a criação deriva o id do título.
  s = vim.fn.tolower(s)
  for from, to in pairs(ACCENTS) do
    s = s:gsub(from, to)
  end
  s = s:gsub("[^%w%s%-]", "") -- descarta o que não for word/space/dash
  s = s:gsub("[%s_]+", "-") -- espaços e underscores viram dash
  s = s:gsub("%-+", "-") -- colapsa dashes repetidos
  s = s:gsub("^%-+", ""):gsub("%-+$", "") -- apara dashes nas pontas
  return s
end

--- Caminhos de task ----------------------------------------------------------

---Decompõe o caminho de um arquivo de task. Aceita as duas formas:
---  tasks/<projeto>/<id>.md                    → project, id, nil
---  tasks/<projeto>/archived/<tipo>/<id>.md    → project, id, tipo
---Qualquer outra coisa sob tasks/ (projeto reservado, subdiretório
---desconhecido, tipo de archived inválido, arquivo de conflito do Syncthing)
---devolve nil — é o guarda único que impede um arquivo fora de padrão de
---entrar no cache ou no board.
---@param path string
---@return string|nil project, string|nil task_id, string|nil archived
local function split_task_path(path)
  local project, rest = path:match(".*/tasks/([^/]+)/(.+)%.md$")
  if not project or RESERVED[project] then
    return nil
  end
  -- `foo.sync-conflict-20260809-123456-ABCDEFG.md`: o Syncthing cria esse
  -- arquivo ao lado do original quando dois dispositivos editam a mesma task.
  -- Sem este guarda ele passaria por uma task legítima de id
  -- "foo.sync-conflict-…" e apareceria no board como um clone fantasma da task
  -- real — com o agravante de o <C-r> poder arquivar a cópia errada. Ele NÃO é
  -- escondido: M.doctor o reporta, porque o conflito é informação que você
  -- precisa resolver, não lixo a varrer para baixo do tapete.
  if rest:match("%.sync%-conflict%-") then
    return nil
  end
  local tipo, id = rest:match("^archived/([^/]+)/([^/]+)$")
  if tipo then
    if not ARCHIVED_SET[tipo] then
      return nil
    end
    return project, id, tipo
  end
  if rest:find("/") then
    return nil -- subdiretório que não é archived/<tipo>: não é uma task
  end
  return project, rest, nil
end
M.split_task_path = split_task_path

---Caminho absoluto do arquivo de uma task.
---@param project string
---@param id string
---@param archived string|nil tipo de arquivamento, ou nil para task ativa
---@return string
local function task_file(project, id, archived)
  if archived then
    return string.format("%s/%s/archived/%s/%s.md", root, project, archived, id)
  end
  return string.format("%s/%s/%s.md", root, project, id)
end

---Diretório vault-relative de uma task, usado como alvo do wikilink da linha 2.
---@param project string
---@param archived string|nil
---@return string
local function vault_dir(project, archived)
  if archived then
    return string.format("tasks/%s/archived/%s", project, archived)
  end
  return "tasks/" .. project
end

---Verdadeiro se a task está arquivada. O CAMINHO é a fonte de verdade (não há
---mais flag `archived: true` no frontmatter — ver M.migrate_format).
---@param path string
---@return boolean
function M.is_archived(path)
  local _, _, archived = split_task_path(path)
  return archived ~= nil
end

---Tipo de arquivamento sugerido para um status — só o DEFAULT do prompt.
---@param status_num integer|nil
---@return string
function M.suggest_archive_type(status_num)
  local callout = M.status_meta(status_num).callout
  if DONE_CALLOUTS[callout] then
    return "done"
  elseif FAILED_CALLOUTS[callout] then
    return "failed"
  end
  return "dropped"
end

--- State ---------------------------------------------------------------------

M.status = nil ---@type table<string, table>|nil
M.cache = nil ---@type table[]|nil

---Garante que a raiz de tasks e o status.json existem.
function M.ensure_root()
  if vim.fn.isdirectory(root) == 0 then
    vim.fn.mkdir(root, "p")
  end
  local sj = root .. "/status.json"
  if vim.fn.filereadable(sj) == 0 then
    vim.fn.writefile(vim.split(DEFAULT_STATUS_JSON, "\n"), sj)
  end
end

---Carrega metadados de status de tasks/status.json (cai no default).
function M.load_status()
  local sj = root .. "/status.json"
  if vim.fn.filereadable(sj) == 1 then
    local ok, decoded = pcall(function()
      return vim.json.decode(table.concat(vim.fn.readfile(sj), "\n"))
    end)
    if ok and type(decoded) == "table" then
      M.status = decoded
      return
    end
    vim.notify("freitask: não consegui parsear status.json, usando defaults", vim.log.levels.WARN)
  end
  M.status = vim.json.decode(DEFAULT_STATUS_JSON)
end

---Lê só a primeira linha de um arquivo de task e extrai o número do status.
---@param path string
---@return integer|nil
function M.parse_status_num(path)
  local blk = M.read_callout(path)
  if #blk == 0 then
    return nil
  end
  if not M.status then
    M.load_status()
  end
  return M.parse_block(blk).status_num
end

--- Modelo de task (parse/serialize do bloco de callout) ----------------------

---Remove o prefixo de blockquote ("> " / ">") de uma linha.
---@param line string
---@return string
local function strip_quote(line)
  return (line:gsub("^%s*>%s?", ""))
end

---Mapa tipo-de-callout → número de status (ex.: todo→1), a partir do status.json.
---@return table<string, integer>
local function status_by_callout()
  local rev = {}
  if M.status then
    for k, v in pairs(M.status) do
      if v.callout then
        rev[v.callout] = tonumber(k)
      end
    end
  end
  return rev
end

---Verdadeiro se `content` é um "texto de status" legado (ex.: "1 - Backlog")
---— usado p/ não confundir a linha de status com uma nota/nome. O título é
---comparado contra QUALQUER entrada de status.json (não só a de número
---correspondente), pois renumerar/reordenar status.json não deve fazer uma
---linha legada como "3 - Blocked" ser lida como nome livre.
---@param content string
---@return boolean, integer|nil
local function is_status_text(content)
  local snum, stitle = content:match("^(%d+)%s*%-%s*(.+)$")
  if not snum or not M.status then
    return false, nil
  end
  stitle = vim.trim(stitle)
  for k, v in pairs(M.status) do
    if v.title == stitle then
      -- usa a chave ATUAL do título casado (não o dígito legado da linha),
      -- para que uma renumeração de status.json não produza um status errado.
      return true, tonumber(k)
    end
  end
  -- Vocabulário legado que NÃO existe mais no status.json (ele foi trocado
  -- pelos nomes de callout do render-markdown). Sem esta tabela, linhas como
  -- "1 - Backlog" deixam de casar e a migração as promoveria a nota
  -- permanente em toda task do vault. Mantida separada e fechada: casar
  -- qualquer "N - Texto" comeria notas legítimas como "3 - comprar leite".
  local legacy = LEGACY_STATUS_TITLES[stitle]
  if legacy then
    return true, legacy
  end
  return false, nil
end

---Parseia o bloco `>` de uma task num modelo estruturado. O status vem do TIPO
---do callout na linha 1; título da linha 1 (novo) ou de uma linha `**...**`
---(legado); id do primeiro `[[...]]`. Linhas de status-texto, id-só e `Branch:`
---legada são consumidas; o resto vira `extras` (texto livre).
---@param block string[] linhas cruas do bloco (com `>`)
---@return table model { status_num, raw_callout, title, id, desc, extras[] }
function M.parse_block(block)
  if not M.status then
    M.load_status()
  end
  local rev = status_by_callout()
  local model = { status_num = 1, raw_callout = "", title = "", id = "", desc = "", extras = {} }

  -- Linha 1: sempre "> [!callout] <resto>". O callout dá o status. O match é
  -- PERMISSIVO (`[^%]]*` e não `%w+`) porque um tipo inválido pode conter
  -- acento ou hífen; com `%w+` o match falhava por completo, o prefixo não era
  -- removido e "[!questão] Título" virava o título inteiro — corrupção.
  local first = block[1] and strip_quote(block[1]) or ""
  local callout = first:match("^%[!([^%]]*)%]")
  if callout then
    model.raw_callout = callout
    -- tipo desconhecido → 0, preservando o texto digitado para o form mostrar.
    model.status_num = rev[callout] or 0
  end
  local rest = callout and first:gsub("^%[![^%]]*%]%s*", "") or first
  if rest ~= "" and not (is_status_text(rest)) then
    -- resto da linha 1 é o título (novo formato); no legado seria status-texto.
    model.title = vim.trim(rest:match("%*%*(.-)%*%*") or rest)
  end

  -- Linhas 2..n
  local seen_note = false
  for i = 2, #block do
    local content = strip_quote(block[i])
    if model.id == "" then
      local lid = content:match("%[%[(.-)%]%]")
      if lid then
        local target = vim.trim((lid:gsub("|.*$", "")))
        model.id = vim.fn.fnamemodify(target, ":t:r")
      end
    end
    local is_only_link = content:match("^%s*%[%[.-%]%]%s*$") ~= nil
    local statusy, snum = is_status_text(content)
    local is_branch = content:match("^[Bb]ranch:") ~= nil
    local bold = content:match("%*%*(.-)%*%*")
    -- Descrição do estado: linha inteiramente em itálico. Só vale ANTES de
    -- qualquer nota, senão uma nota em itálico lá embaixo seria promovida a
    -- descrição do estado.
    local italic = content:match("^_(.-)_$")
    if is_only_link or statusy or is_branch then
      if statusy and not callout then
        model.status_num = snum -- fallback quando a linha 1 não tinha callout
      end
    elseif bold and model.title == "" then
      model.title = vim.trim(bold)
    elseif italic and italic ~= "" and model.desc == "" and not seen_note then
      model.desc = vim.trim(italic)
    else
      -- Todo o resto é nota — inclusive o antigo "nome livre" da linha 3, que
      -- é rebaixado a nota na migração (ver comentário do topo do módulo).
      model.extras[#model.extras + 1] = content
      if vim.trim(content) ~= "" then
        seen_note = true
      end
    end
  end
  return model
end

---Reconstrói o bloco `>` a partir do modelo (ordem canônica).
---@param model table
---@return string[]
function M.serialize_block(model)
  -- Status válido → normaliza para o callout canônico do status.json. Status 0
  -- → devolve o tipo digitado verbatim, para o erro sobreviver ao round-trip e
  -- aparecer no form (gravar um blockquote sem `[!tipo]` deixaria a task
  -- ilegível para resolve_under_cursor, isto é: inconsertável pelo plugin).
  local callout
  if model.status_num == 0 then
    callout = vim.trim(model.raw_callout or "")
    if callout == "" then
      callout = "invalid" -- não existe no status.json, logo reparseia como 0
    end
  else
    callout = M.status_meta(model.status_num).callout or "note"
  end
  -- Link com caminho relativo ao vault + id como alias evita ambiguidade entre
  -- projetos diferentes que tenham tasks com o mesmo id; sem `model.project`
  -- (ex.: contexto de teste) cai no formato legado `[[id]]`. `model.archived`
  -- move o alvo para archived/<tipo>/, senão o link de uma task arquivada
  -- apontaria para um caminho que não existe mais.
  local link = model.project
      and string.format("%s/%s|%s", vault_dir(model.project, model.archived), model.id or "", model.id or "")
    or (model.id or "")
  local out = {
    string.format("> [!%s] %s", callout, model.title or ""),
    string.format("> [[%s]]", link),
  }
  -- Omitida quando vazia: um `>` vazio no meio do bloco faria a primeira nota
  -- ser promovida a descrição no re-parse.
  local desc = vim.trim(model.desc or "")
  if desc ~= "" then
    out[#out + 1] = "> _" .. desc .. "_"
  end
  local extras = vim.deepcopy(model.extras or {})
  while #extras > 0 and vim.trim(extras[#extras]) == "" do
    table.remove(extras)
  end
  for _, e in ipairs(extras) do
    out[#out + 1] = (vim.trim(e) == "") and ">" or ("> " .. e)
  end
  return out
end

--- Localização de blocos -----------------------------------------------------

---Substitui lines[s..e] por `repl`, retornando um novo array.
---@param lines string[]
---@param s integer
---@param e integer
---@param repl string[]
---@return string[]
local function splice(lines, s, e, repl)
  local out = {}
  for i = 1, s - 1 do
    out[#out + 1] = lines[i]
  end
  for _, r in ipairs(repl) do
    out[#out + 1] = r
  end
  for i = e + 1, #lines do
    out[#out + 1] = lines[i]
  end
  return out
end

---Índice (1-based) da primeira linha de conteúdo após um frontmatter YAML
---opcional (`---` … `---`). Retorna 1 quando não há frontmatter.
---@param lines string[]
---@return integer
local function content_start(lines)
  if lines[1] == "---" then
    for i = 2, #lines do
      if lines[i] == "---" then
        return i + 1
      end
    end
  end
  return 1
end

---Faixa [start, finish] do primeiro bloco `>` de um arquivo, pulando um
---frontmatter YAML e linhas em branco iniciais. nil se não houver callout.
---@param lines string[]
---@return integer|nil, integer|nil
local function first_block_range(lines)
  local start
  for i = content_start(lines), #lines do
    local l = lines[i]
    if l:match("^%s*>") then
      start = i
      break
    elseif not l:match("^%s*$") then
      return nil -- conteúdo não-quote antes de qualquer bloco
    end
  end
  if not start then
    return nil
  end
  local finish = start
  for i = start + 1, #lines do
    if lines[i]:match("^%s*>") then
      finish = i
    else
      break
    end
  end
  return start, finish
end

---Faixa do bloco `>` que contém a linha `lnum` (1-indexed). nil se `lnum` não
---estiver sobre uma linha de blockquote.
---@param lines string[]
---@param lnum integer
---@return integer|nil, integer|nil
local function block_around(lines, lnum)
  if not lines[lnum] or not lines[lnum]:match("^%s*>") then
    return nil
  end
  local s, e = lnum, lnum
  while s > 1 and lines[s - 1]:match("^%s*>") do
    s = s - 1
  end
  while e < #lines and lines[e + 1]:match("^%s*>") do
    e = e + 1
  end
  return s, e
end

---Buffer carregado para `path`, ou nil.
---@param path string
---@return integer|nil
local function loaded_buf(path)
  local b = vim.fn.bufnr(path)
  if b ~= -1 and vim.api.nvim_buf_is_loaded(b) then
    return b
  end
  return nil
end

--- Cache ---------------------------------------------------------------------

---Lê o arquivo de uma task UMA vez e extrai tudo que o cache precisa.
---Substitui o par is_archived + read_callout, que abria o mesmo arquivo duas
---vezes por task — e o rebuild_current abria uma terceira para reimprimir o
---bloco. Guardando o bloco na entry, o regen não toca mais no disco.
---Não checa mais arquivamento: isso agora é o CAMINHO, resolvido antes de
---chegar aqui por split_task_path.
---@param path string
---@return { block: string[], status_num: integer }|nil
local function scan_task(path)
  if vim.fn.filereadable(path) == 0 then
    return nil
  end
  local lines = vim.fn.readfile(path)

  local block = {}
  local s, e = first_block_range(lines)
  if s then
    for i = s, e do
      block[#block + 1] = lines[i]
    end
  end

  if not M.status then
    M.load_status()
  end
  -- Arquivo de task sem callout é um arquivo quebrado: 0 o deixa no topo do
  -- projeto em vez de escondê-lo no meio do Backlog.
  local status_num = (#block > 0) and M.parse_block(block).status_num or 0
  return { block = block, status_num = status_num }
end

---Insere ou atualiza uma task no cache em memória. Tasks arquivadas nunca
---entram no cache — logo, nunca aparecem no CURRENT.md.
---@param path string
function M.update_cache_entry(path)
  local project, task_id, archived = split_task_path(path)
  if not project or archived then
    return
  end
  M.cache = M.cache or {}
  local scan = scan_task(path)
  if not scan then
    M.remove_cache_entry(path)
    return
  end
  for _, e in ipairs(M.cache) do
    if e.path == path then
      e.status_num = scan.status_num
      e.block = scan.block
      return
    end
  end
  M.cache[#M.cache + 1] = {
    project = project,
    task_id = task_id,
    status_num = scan.status_num,
    block = scan.block,
    path = path,
  }
end

---Remove uma task do cache (após deleção).
---@param path string
function M.remove_cache_entry(path)
  if not M.cache then
    return
  end
  for i, e in ipairs(M.cache) do
    if e.path == path then
      table.remove(M.cache, i)
      return
    end
  end
end

---Rescan completo de tasks/*/*.md para o cache.
function M.build_cache()
  M.cache = {}
  for _, path in ipairs(vim.fn.glob(root .. "/*/*.md", true, true)) do
    M.update_cache_entry(path)
  end
end

---Tasks cacheadas de um projeto, ordenadas por status e depois id.
---@param project string
---@return table[]
function M.entries_for(project)
  local list = {}
  for _, e in ipairs(M.cache or {}) do
    if e.project == project then
      list[#list + 1] = e
    end
  end
  table.sort(list, function(a, b)
    if a.status_num == b.status_num then
      return a.task_id < b.task_id
    end
    return a.status_num < b.status_num
  end)
  return list
end

---Lista de ids de projeto (subdiretórios da raiz, exceto reservados).
---@return string[]
function M.list_projects()
  local out = {}
  for _, path in ipairs(vim.fn.glob(root .. "/*", true, true)) do
    if vim.fn.isdirectory(path) == 1 then
      local name = vim.fn.fnamemodify(path, ":t")
      if not RESERVED[name] then
        out[#out + 1] = name
      end
    end
  end
  table.sort(out)
  return out
end

--- Referências do vault ------------------------------------------------------
--
-- Arquivar, desarquivar e renomear o id são o mesmo evento — "esse arquivo
-- mudou de endereço" — e por isso passam todos por M.retarget_links.
--
-- O Obsidian resolve `[[foo]]` pelo BASENAME, em qualquer pasta do vault. Daí
-- a assimetria que a tabela abaixo resume, e que explica por que arquivar é
-- barato do ponto de vista de referências:
--
--   operação              [[foo]]      [[tasks/p/foo|foo]]   frontmatter id:
--   arquivar/desarquivar  intacto      reescrever            intacto
--   renomear id           reescrever   reescrever            reescrever

---Arquivos markdown do vault elegíveis a conter referências.
---`tasks/daily/` fica DE FORA de propósito: são snapshots de como o board
---estava naquele dia; reescrevê-los falsificaria o histórico. `CURRENT.md`
---também: é regenerado do zero, basta chamar M.rebuild_current depois.
---@return string[]
local function ref_candidates()
  local out = {}
  for _, p in ipairs(vim.fn.glob(vault .. "/**/*.md", true, true)) do
    if not p:match("^" .. vim.pesc(root) .. "/daily/") and p ~= root .. "/CURRENT.md" then
      out[#out + 1] = p
    end
  end
  return out
end

---Reescreve um wikilink `inner` (o miolo de `[[...]]`) quando ele aponta para
---a task que mudou de endereço. Devolve nil quando não há o que mudar.
---Preserva sufixo de heading/bloco (`#Seção`, `^bloco`) e o alias — exceto
---quando o alias ERA o id antigo, caso em que ele acompanha o rename.
---@param inner string
---@param old table { id: string, full: string }
---@param new table { id: string, full: string }
---@return string|nil
local function rewrite_link(inner, old, new)
  local body, alias = inner:match("^(.-)|(.*)$")
  if not body then
    body = inner
  end
  local target, sub = body:match("^([^#^]*)([#^].*)$")
  if not target then
    target, sub = body, ""
  end
  local key = vim.trim(target):gsub("%.md$", "")

  local new_target
  if key == old.full then
    new_target = new.full
  elseif key == old.id and old.id ~= new.id then
    -- Forma curta: só quebra quando o BASENAME muda; mudança de pasta não
    -- afeta a resolução do Obsidian, então não tocamos no link.
    new_target = new.id
  else
    return nil
  end

  local new_alias = alias
  if alias and vim.trim(alias) == old.id then
    new_alias = new.id
  end
  return new_target .. sub .. (new_alias and ("|" .. new_alias) or "")
end

---Reescreve, em todo o vault, os wikilinks que apontavam para `old_path` para
---que apontem para `new_path`. Cobre `[[alvo]]`, `[[alvo|alias]]`, embeds
---`![[...]]` e sufixos `#heading`/`^bloco`, nas formas curta e path-qualified.
---Buffers abertos COM alterações não salvas são pulados (e reportados) em vez
---de sobrescritos — mesma postura do rebuild_current diante de um CURRENT.md
---sujo.
---@param old_path string
---@param new_path string
---@return integer refs, integer files, string[] skipped
function M.retarget_links(old_path, new_path)
  local function descr(p)
    local rel = p:sub(#vault + 2):gsub("%.md$", "")
    return { id = vim.fn.fnamemodify(p, ":t:r"), full = rel }
  end
  local old, new = descr(old_path), descr(new_path)
  if old.full == new.full then
    return 0, 0, {}
  end

  local refs, files, skipped = 0, 0, {}
  for _, path in ipairs(ref_candidates()) do
    if path ~= old_path and path ~= new_path then
      local buf = loaded_buf(path)
      local lines = buf and vim.api.nvim_buf_get_lines(buf, 0, -1, false)
        or (vim.fn.filereadable(path) == 1 and vim.fn.readfile(path) or {})
      local hits = 0
      for i, line in ipairs(lines) do
        local n = 0
        -- gsub com função: o retorno é usado verbatim, então um alias com `%`
        -- não vira referência de captura.
        local replaced = line:gsub("%[%[(.-)%]%]", function(inner)
          local nl = rewrite_link(inner, old, new)
          if not nl then
            return nil -- mantém o original
          end
          n = n + 1
          return "[[" .. nl .. "]]"
        end)
        if n > 0 then
          lines[i] = replaced
          hits = hits + n
        end
      end
      if hits > 0 then
        if buf and vim.bo[buf].modified then
          skipped[#skipped + 1] = path
        else
          vim.fn.writefile(lines, path)
          if buf then
            vim.api.nvim_buf_call(buf, function()
              vim.cmd("silent edit!")
            end)
          end
          refs, files = refs + hits, files + 1
        end
      end
    end
  end

  if refs > 0 then
    vim.notify(string.format("freitask: %d referência(s) atualizada(s) em %d arquivo(s)", refs, files))
  end
  if #skipped > 0 then
    vim.notify(
      "freitask: referências NÃO atualizadas (buffer com alterações não salvas):\n  " .. table.concat(skipped, "\n  "),
      vim.log.levels.WARN
    )
  end
  return refs, files, skipped
end

--- File operations -----------------------------------------------------------

---Template markdown para uma nova task, a partir do modelo vindo do form.
---@param model table { status_num, title, id, desc, extras[], project }
---@return string[]
function M.template(model)
  if not M.status then
    M.load_status()
  end
  local project = model.project
  local out = vim.deepcopy(M.serialize_block(model))
  out[#out + 1] = ""
  out[#out + 1] = "## Notas Soltas"
  out[#out + 1] = "- "
  out[#out + 1] = ""
  out[#out + 1] = "### [" .. project .. "]"
  out[#out + 1] = "- [ ] "
  return out
end

---Reescreve o bloco de callout de um arquivo no disco (substituindo o primeiro
---bloco `>`, ou prependando se não houver).
---@param path string
---@param new_block string[]
local function file_replace_callout(path, new_block)
  local lines = vim.fn.filereadable(path) == 1 and vim.fn.readfile(path) or {}
  local s, e = first_block_range(lines)
  local out
  if s then
    out = splice(lines, s, e, new_block)
  else
    out = vim.deepcopy(new_block)
    out[#out + 1] = ""
    for _, l in ipairs(lines) do
      out[#out + 1] = l
    end
  end
  vim.fn.writefile(out, path)
end

---Atualiza a chave `key` do frontmatter YAML, se (e só se) ela já existir.
---Deliberadamente não cria frontmatter: quem o injeta é o obsidian.nvim, e
---inventá-lo aqui acrescentaria ruído a arquivos que não o têm.
---@param lines string[]
---@param key string
---@param value string
---@return boolean changed
local function update_frontmatter_key(lines, key, value)
  if lines[1] ~= "---" then
    return false
  end
  for i = 2, #lines do
    if lines[i] == "---" then
      return false
    end
    if lines[i]:match("^" .. key .. ":") then
      local new = key .. ": " .. value
      if lines[i] == new then
        return false
      end
      lines[i] = new
      return true
    end
  end
  return false
end

---Acrescenta uma entrada ao rodapé `## Histórico` do arquivo (criando a seção
---se não existir). É um LOG, não uma linha única: arquivar → desarquivar →
---rearquivar é um ciclo normal, e sobrescrever apagaria justamente a
---informação interessante (quantas vezes, e quando você mudou de ideia).
---Muta `lines` in place; quem chama grava.
---@param lines string[]
---@param dest string|nil tipo de archived, ou nil para desarquivar
local function append_history(lines, dest)
  local entry
  if dest then
    entry = string.format(
      "- %s — arquivada em `archived/%s` (%s)",
      os.date("%Y-%m-%d"),
      dest,
      ARCHIVED_LABELS[dest] or dest
    )
  else
    entry = string.format("- %s — desarquivada, de volta ao board", os.date("%Y-%m-%d"))
  end

  -- Última ocorrência: se houver mais de um `## Histórico` (edição manual),
  -- a de baixo é a que o olho lê como rodapé.
  local hstart
  for i = #lines, 1, -1 do
    if lines[i]:match("^##%s+Histórico%s*$") then
      hstart = i
      break
    end
  end

  if not hstart then
    while #lines > 0 and vim.trim(lines[#lines]) == "" do
      table.remove(lines)
    end
    vim.list_extend(lines, { "", "## Histórico", "", entry })
    return
  end

  -- Insere no FIM da seção (não no fim do arquivo): se o usuário escreveu
  -- outra `## seção` depois do histórico, a entrada nova continua no lugar
  -- certo. As linhas em branco finais da seção são puladas.
  local hend = #lines
  for i = hstart + 1, #lines do
    if lines[i]:match("^##%s") then
      hend = i - 1
      break
    end
  end
  while hend > hstart and vim.trim(lines[hend]) == "" do
    hend = hend - 1
  end
  table.insert(lines, hend + 1, entry)
end

---Move o arquivo de uma task entre ativo e arquivado — o motor por trás de
---M.archive_task e M.unarchive_task, que são só nomes para os dois sentidos.
---Reescreve o bloco (o wikilink da linha 2 precisa acompanhar a pasta),
---migra as referências do vault e acerta o cache. NÃO chama rebuild_current:
---quem chama decide quando regenerar (o picker faz isso uma vez só).
---@param path string
---@param dest string|nil tipo de archived, ou nil para desarquivar
---@return string|nil new_path
local function move_task(path, dest)
  local project, id, cur = split_task_path(path)
  if not project then
    vim.notify("freitask: não é um arquivo de task: " .. path, vim.log.levels.WARN)
    return nil
  end
  if dest and not ARCHIVED_SET[dest] then
    vim.notify("freitask: tipo de arquivamento inválido: " .. tostring(dest), vim.log.levels.WARN)
    return nil
  end
  if dest == cur then
    return path -- já está onde deveria
  end

  local buf = loaded_buf(path)
  if buf and vim.bo[buf].modified then
    vim.notify("freitask: salve o arquivo da task antes de (des)arquivar", vim.log.levels.WARN)
    return nil
  end

  local new_path = task_file(project, id, dest)
  if vim.fn.filereadable(new_path) == 1 then
    vim.notify("freitask: já existe um arquivo em " .. new_path, vim.log.levels.WARN)
    return nil
  end
  local dir = vim.fn.fnamemodify(new_path, ":h")
  if vim.fn.isdirectory(dir) == 0 then
    vim.fn.mkdir(dir, "p")
  end
  if vim.fn.rename(path, new_path) ~= 0 then
    vim.notify("freitask: não consegui mover " .. id, vim.log.levels.ERROR)
    return nil
  end

  -- O link da linha 2 é path-qualified, então precisa seguir a pasta nova; e o
  -- rodapé ganha a entrada de histórico. Um write só para as duas coisas.
  local lines = vim.fn.readfile(new_path)
  local s, e = first_block_range(lines)
  if s then
    local block = {}
    for i = s, e do
      block[#block + 1] = lines[i]
    end
    local model = M.parse_block(block)
    if model.id == "" then
      model.id = id
    end
    model.project, model.archived = project, dest
    lines = splice(lines, s, e, M.serialize_block(model))
  end
  append_history(lines, dest)
  vim.fn.writefile(lines, new_path)

  if buf then
    pcall(vim.api.nvim_buf_set_name, buf, new_path)
    vim.api.nvim_buf_call(buf, function()
      vim.cmd("silent edit!")
    end)
  end

  M.remove_cache_entry(path)
  if not dest then
    M.update_cache_entry(new_path)
  end
  M.retarget_links(path, new_path)
  return new_path
end

---Arquiva uma task: move para tasks/<projeto>/archived/<tipo>/, tirando-a do
---cache e do CURRENT.md sem apagar nada.
---@param path string
---@param tipo string um de M.ARCHIVED_TYPES
---@return string|nil new_path
function M.archive_task(path, tipo)
  return move_task(path, tipo)
end

---Desarquiva uma task: move de volta para tasks/<projeto>/, devolvendo-a ao
---cache — e portanto ao CURRENT.md na próxima regeneração.
---@param path string
---@return string|nil new_path
function M.unarchive_task(path)
  return move_task(path, nil)
end

---Lista as tasks arquivadas de um projeto, lidas do disco sob demanda. NÃO
---entram em M.cache de propósito: assim o board, o regen e o resto do módulo
---seguem enxergando só as tasks ativas, sem um filtro novo em cada leitor.
---@param project string
---@return table[]
function M.archived_entries_for(project)
  local out = {}
  for _, path in ipairs(vim.fn.glob(root .. "/" .. project .. "/archived/*/*.md", true, true)) do
    local p, id, tipo = split_task_path(path)
    if p then
      local scan = scan_task(path)
      out[#out + 1] = {
        project = p,
        task_id = id,
        archived = tipo,
        status_num = scan and scan.status_num or 0,
        block = scan and scan.block or {},
        path = path,
      }
    end
  end
  table.sort(out, function(a, b)
    if a.archived == b.archived then
      return a.task_id < b.task_id
    end
    return a.archived < b.archived
  end)
  return out
end

--- CURRENT.md dashboard ------------------------------------------------------

---Lê o bloco de callout inicial (linhas `>` contíguas) de um arquivo de task.
---@param path string
---@return string[]
function M.read_callout(path)
  if vim.fn.filereadable(path) == 0 then
    return {}
  end
  local lines = vim.fn.readfile(path)
  local s, e = first_block_range(lines)
  if not s then
    return {}
  end
  local out = {}
  for i = s, e do
    out[#out + 1] = lines[i]
  end
  return out
end

---Extrai o corpo autoral de `## Notas Avulsas` para preservá-lo entre regens.
---@param path string
---@return string[]
function M.extract_loose_notes(path)
  if vim.fn.filereadable(path) == 0 then
    return {}
  end
  local out, capturing = {}, false
  for _, l in ipairs(vim.fn.readfile(path)) do
    if l:match("^##%s+Notas Avulsas") then
      capturing = true
    elseif capturing and l:match("^##%s+") then
      break -- próxima seção
    elseif capturing then
      out[#out + 1] = l
    end
  end
  while #out > 0 and out[1]:match("^%s*$") do
    table.remove(out, 1)
  end
  while #out > 0 and out[#out]:match("^%s*$") do
    table.remove(out)
  end
  if #out == 1 and out[1]:match("^%-%s*$") then
    out = {} -- descarta o placeholder solitário
  end
  return out
end

---Data do frontmatter de CURRENT.md (YYYY-MM-DD) ou nil.
---@param path string
---@return string|nil
local function frontmatter_date(path)
  if vim.fn.filereadable(path) == 0 then
    return nil
  end
  local lines = vim.fn.readfile(path)
  if lines[1] ~= "---" then
    return nil
  end
  for i = 2, #lines do
    if lines[i] == "---" then
      break
    end
    local d = lines[i]:match("^date:%s*(%d%d%d%d%-%d%d%-%d%d)")
    if d then
      return d
    end
  end
  return nil
end

---Regenera tasks/CURRENT.md como o "board de hoje": frontmatter date/weekday,
---título de planning diário, `## Notas Avulsas` preservadas e uma seção
---`## <project>` por projeto. Antes de sobrescrever, se o board vigente for de
---um dia anterior, arquiva-o em tasks/daily/<data>.md. Recusa sobrescrever um
---CURRENT.md com edições não salvas.
---@param opts? { quiet?: boolean } quiet suprime o aviso de "regen pulado"
function M.rebuild_current(opts)
  opts = opts or {}
  M.ensure_root()
  M.load_status()
  M.build_cache()
  local path = root .. "/CURRENT.md"
  local buf = vim.fn.bufnr(path)
  if buf ~= -1 and vim.api.nvim_buf_is_loaded(buf) and vim.bo[buf].modified then
    if not opts.quiet then
      vim.notify("freitask: CURRENT.md tem alterações não salvas; regen pulado", vim.log.levels.WARN)
    end
    return
  end

  local today = os.date("%Y-%m-%d")
  local prev = frontmatter_date(path)
  if prev and prev < today then
    local daily_dir = root .. "/daily"
    if vim.fn.isdirectory(daily_dir) == 0 then
      vim.fn.mkdir(daily_dir, "p")
    end
    local archive = daily_dir .. "/" .. prev .. ".md"
    if vim.fn.filereadable(archive) == 0 then
      vim.fn.writefile(vim.fn.readfile(path), archive)
    end
  end

  local notes = M.extract_loose_notes(path)
  local projects = M.list_projects()
  local total = #(M.cache or {})

  local lines = {
    "---",
    "date: " .. today,
    "weekday: " .. os.date("%A"),
    "---",
    "",
    "# Planning diário " .. os.date("%d/%m/%y"),
    "",
    string.format("> Painel gerado automaticamente — %d tasks em %d projetos.", total, #projects),
    "> Abra o gestor com `<leader>ob` · regenere com `<C-a>` · edite o callout sob o cursor com `<leader>oe`.",
    "",
    "## Notas Avulsas",
    "",
  }
  if #notes == 0 then
    lines[#lines + 1] = "- "
  else
    for _, l in ipairs(notes) do
      lines[#lines + 1] = l
    end
  end
  lines[#lines + 1] = ""

  for _, project in ipairs(projects) do
    lines[#lines + 1] = "## " .. project
    lines[#lines + 1] = ""
    local entries = M.entries_for(project)
    if #entries == 0 then
      lines[#lines + 1] = "_(sem tasks)_"
      lines[#lines + 1] = ""
    else
      for _, e in ipairs(entries) do
        -- bloco vem do cache (scan_task já o leu); sem I/O aqui.
        for _, cl in ipairs(e.block or {}) do
          lines[#lines + 1] = cl
        end
        lines[#lines + 1] = ""
      end
    end
  end

  vim.fn.writefile(lines, path)
  if buf ~= -1 and vim.api.nvim_buf_is_loaded(buf) then
    vim.api.nvim_buf_call(buf, function()
      vim.cmd("silent edit!")
    end)
  end
end

---Migra todos os arquivos de task para o formato atual (`[[tasks/<projeto>/<id>|id]]`
---com caminho + alias, e a linha 3 livre como "nome" em vez de status-texto).
---Cobre tanto o formato legado (`> **Título** - [[id]]` + `> Branch:`) quanto o
---formato intermediário (`> N - Título` na linha 3). Idempotente. Retorna a
---quantidade de arquivos reescritos.
---@return integer
function M.migrate_format()
  M.ensure_root()
  M.load_status()
  local n = 0

  -- Formato legado de arquivamento: `archived: true` no frontmatter. Hoje o
  -- estado é o CAMINHO, então essas tasks são movidas para archived/dropped/
  -- (o balde honesto: "parei de tocar" — o flag antigo não registrava por quê)
  -- e o flag some. Feito ANTES do loop de blocos para que elas já sejam
  -- reserializadas com o link apontando para a pasta nova.
  for _, path in ipairs(vim.fn.glob(root .. "/*/*.md", true, true)) do
    local project = split_task_path(path)
    if project then
      local lines = vim.fn.readfile(path)
      local flagged
      if lines[1] == "---" then
        for i = 2, #lines do
          if lines[i] == "---" then
            break
          end
          if lines[i]:match("^archived:%s*true%s*$") then
            flagged = i
            break
          end
        end
      end
      if flagged then
        table.remove(lines, flagged)
        vim.fn.writefile(lines, path)
        if move_task(path, "dropped") then
          n = n + 1
        end
      end
    end
  end

  -- Normaliza o bloco de todas as tasks, ativas e arquivadas.
  local paths = vim.fn.glob(root .. "/*/*.md", true, true)
  vim.list_extend(paths, vim.fn.glob(root .. "/*/archived/*/*.md", true, true))
  for _, path in ipairs(paths) do
    local project, id, archived = split_task_path(path)
    if project then
      local lines = vim.fn.readfile(path)
      local s, e = first_block_range(lines)
      if s then
        local blk = {}
        for i = s, e do
          blk[#blk + 1] = lines[i]
        end
        local model = M.parse_block(blk)
        if model.id == "" then
          model.id = id -- fallback: id = nome do arquivo
        end
        model.project, model.archived = project, archived
        local newblk = M.serialize_block(model)
        if not vim.deep_equal(newblk, blk) then
          vim.fn.writefile(splice(lines, s, e, newblk), path)
          M.update_cache_entry(path)
          n = n + 1
        end
      end
    end
  end
  return n
end

--- Doctor: verificação e reparo ----------------------------------------------
--
-- Existe porque o Neovim NÃO é o único escritor do vault: há o Obsidian (aqui e
-- no celular, via Syncthing) e agentes de IA com acesso a shell. Nenhum deles
-- pode ser obrigado a chamar as funções deste módulo, então a estratégia não é
-- impedir o desvio — é torná-lo BARULHENTO e, quando possível, reparável.
--
-- A boa notícia é que a maior parte dos invariantes é DERIVÁVEL do caminho do
-- arquivo (o wikilink da linha 2, o `id:` do frontmatter, o estado de
-- arquivamento), e portanto não precisa ser obedecida — precisa ser
-- regenerada. Só duas coisas se perdem de verdade:
--   1. backlinks externos após um RENAME (nada registra que `foo` se chamava
--      `bar`; a informação some junto com o link);
--   2. as entradas de histórico (são fatos sobre o passado).
-- Note que ARQUIVAR não está nessa lista: o move preserva o basename, o
-- Obsidian resolve `[[foo]]` por basename e o link da linha 2 é derivável — um
-- agente que só faz `mv` para archived/ causa dano inteiramente reparável.

---Todos os arquivos de task do vault (ativos + arquivados), já decompostos.
---@return { path: string, project: string, id: string, archived: string|nil }[]
local function all_tasks()
  local out = {}
  local globs = { root .. "/*/*.md", root .. "/*/archived/*/*.md" }
  for _, g in ipairs(globs) do
    for _, path in ipairs(vim.fn.glob(g, true, true)) do
      local project, id, archived = split_task_path(path)
      if project then
        out[#out + 1] = { path = path, project = project, id = id, archived = archived }
      end
    end
  end
  return out
end

---Resolve um argumento de linha de comando — caminho ou id — no arquivo da
---task. Existe para a CLI: obrigar um agente a saber se a task está em
---`archived/` para poder citá-la seria justamente o tipo de regra que ele vai
---errar.
---@param ref string caminho (absoluto/relativo) ou id da task
---@return string|nil path, string|nil err
function M.find_task(ref)
  if vim.fn.filereadable(ref) == 1 then
    return vim.fn.fnamemodify(ref, ":p")
  end
  local hits = {}
  for _, t in ipairs(all_tasks()) do
    if t.id == ref then
      hits[#hits + 1] = t.path
    end
  end
  if #hits == 1 then
    return hits[1]
  elseif #hits == 0 then
    return nil, "task não encontrada: " .. ref
  end
  return nil, string.format("id %q é ambíguo (%d arquivos); passe o caminho", ref, #hits)
end

---Conjunto de alvos de wikilink que resolvem para algum arquivo do vault, nas
---duas formas que o Obsidian aceita (basename e caminho vault-relative).
---@return table<string, boolean>
local function resolvable_targets()
  local set = {}
  for _, p in ipairs(vim.fn.glob(vault .. "/**/*.md", true, true)) do
    set[vim.fn.fnamemodify(p, ":t:r")] = true
    set[(p:sub(#vault + 2):gsub("%.md$", ""))] = true
  end
  return set
end

---Diagnostica o vault. READ-ONLY por padrão; `opts.fix` repara o que é
---derivável. Devolve uma lista de achados, cada um com `fixed` indicando se
---foi reparado nesta passagem.
---@param opts? { fix?: boolean }
---@return { level: "error"|"warn", kind: string, path: string, msg: string, fixed: boolean }[]
function M.doctor(opts)
  opts = opts or {}
  M.ensure_root()
  M.load_status()
  local found = {}
  local function add(level, kind, path, msg, fixed)
    found[#found + 1] = {
      level = level,
      kind = kind,
      path = path:sub(#vault + 2),
      msg = msg,
      fixed = fixed or false,
    }
  end

  -- 1. Conflitos do Syncthing. Reportados e NUNCA reparados: escolher qual
  -- cópia vale é decisão editorial, e apagar a errada em silêncio perderia
  -- trabalho feito no outro dispositivo.
  for _, p in ipairs(vim.fn.glob(root .. "/**/*.sync-conflict-*.md", true, true)) do
    add("error", "sync-conflict", p, "conflito do Syncthing — compare com o original e resolva à mão")
  end

  -- 2. Arquivos sob tasks/<projeto>/ que a ferramenta não enxerga. São
  -- invisíveis ao board e ao picker, então sumiriam sem barulho nenhum.
  -- `daily/` e `templates/` são reservados e ficam fora por definição.
  for _, p in ipairs(vim.fn.glob(root .. "/*/**/*.md", true, true)) do
    local top = p:match(".*/tasks/([^/]+)/")
    if not RESERVED[top or ""] and not split_task_path(p) and not p:match("%.sync%-conflict%-") then
      add("warn", "fora-do-padrao", p, "não casa tasks/<projeto>/[archived/<tipo>/]<id>.md — invisível ao freitask")
    end
  end

  -- 3. Ids duplicados entre projetos: tornam `[[id]]` ambíguo, e é o link curto
  -- que o Obsidian resolve por basename.
  local by_id = {}
  for _, t in ipairs(all_tasks()) do
    by_id[t.id] = by_id[t.id] or {}
    table.insert(by_id[t.id], t.path)
  end
  for id, paths in pairs(by_id) do
    if #paths > 1 then
      add("warn", "id-duplicado", paths[1], string.format("id %q existe em %d arquivos — `[[%s]]` fica ambíguo", id, #paths, id))
    end
  end

  -- 4. Invariantes deriváveis do caminho: bloco e frontmatter. Tudo aqui é
  -- reparável sem adivinhação, porque o caminho é a fonte de verdade.
  for _, t in ipairs(all_tasks()) do
    local lines = vim.fn.readfile(t.path)
    local s, e = first_block_range(lines)
    if not s then
      add("error", "sem-callout", t.path, "arquivo de task sem bloco de callout")
    else
      local blk = {}
      for i = s, e do
        blk[#blk + 1] = lines[i]
      end
      local model = M.parse_block(blk)
      local dirty = false

      if model.status_num == 0 then
        add("warn", "status-0", t.path, string.format("callout %q não existe em status.json", model.raw_callout))
      end

      -- O NOME DO ARQUIVO vence sempre, e não só quando o link está vazio: se
      -- alguém renomeou com `mv`, o bloco ainda carrega o id antigo, e
      -- respeitá-lo faria o --fix deixar o arquivo eternamente dessincronizado
      -- do próprio caminho — justo o que este check existe para pegar.
      model.id = t.id
      model.project, model.archived = t.project, t.archived
      local want = M.serialize_block(model)
      if not vim.deep_equal(want, blk) then
        if opts.fix then
          lines = splice(lines, s, e, want)
          dirty = true
        end
        add("warn", "bloco-dessincronizado", t.path, "bloco não corresponde ao caminho (link/id)", opts.fix)
      end

      -- Trabalha numa cópia para que o diagnóstico continue read-only quando
      -- não há --fix (update_frontmatter_key muta o array que recebe).
      local probe = vim.deepcopy(lines)
      if update_frontmatter_key(probe, "id", t.id) then
        if opts.fix then
          lines = probe
          dirty = true
        end
        add("warn", "frontmatter-id", t.path, "`id:` do frontmatter ≠ nome do arquivo", opts.fix)
      end

      -- 5. Arquivada sem registro de arquivamento. O reparo escreve uma linha
      -- EXPLICITAMENTE marcada como reconstruída: inventar a data real seria
      -- pior que não ter a linha.
      if t.archived then
        local logged = false
        for _, l in ipairs(lines) do
          if l:match("^%- %d%d%d%d%-%d%d%-%d%d — arquivada em") then
            logged = true
            break
          end
        end
        if not logged then
          if opts.fix then
            append_history(lines, t.archived)
            lines[#lines] = lines[#lines] .. " [reconstruído pelo doctor; data real desconhecida]"
            dirty = true
          end
          add("warn", "historico-ausente", t.path, "está em archived/ sem entrada de histórico", opts.fix)
        end
      end

      if dirty then
        vim.fn.writefile(lines, t.path)
      end
    end
  end

  -- 6. Wikilinks path-qualified pendurados: a impressão digital de um rename
  -- feito sem M.retarget_links. NÃO é reparável — nada no vault registra que
  -- `foo` um dia se chamou `bar` —, e é justamente por isso que precisa ser
  -- reportado.
  --
  -- Só links `tasks/...` com caminho entram aqui. Um `[[nota-futura]]` curto
  -- que não resolve é comportamento NORMAL do Obsidian (o link vira um
  -- placeholder que cria a nota ao ser clicado), e cobrá-lo encheria o doctor
  -- de alarme falso — um checker em que não se confia é um checker que não se
  -- lê. Já um link com caminho para tasks/ foi gerado por este módulo e
  -- portanto tem obrigação de resolver.
  local resolvable = resolvable_targets()
  for _, p in ipairs(vim.fn.glob(vault .. "/**/*.md", true, true)) do
    if not p:match("^" .. vim.pesc(root) .. "/daily/") then
      local in_fence = false
      for i, line in ipairs(vim.fn.readfile(p)) do
        if line:match("^%s*```") then
          in_fence = not in_fence
        elseif not in_fence then
          -- O Obsidian não renderiza wikilink dentro de código — nem em bloco
          -- cercado, nem entre crases. Sem espelhar essa regra, qualquer
          -- documentação que CITE um link (este vault tem um AGENTS.md cheio
          -- deles) viraria falso positivo, e um checker com alarme falso é um
          -- checker que se aprende a ignorar.
          local scan = line:gsub("`[^`]*`", "")
          for inner in scan:gmatch("%[%[(.-)%]%]") do
            local target = inner:gsub("|.*$", ""):gsub("[#^].*$", "")
            target = vim.trim(target):gsub("%.md$", "")
            if target:match("^tasks/") and not resolvable[target] then
              add("error", "link-pendurado", p, string.format("linha %d: `[[%s]]` não resolve", i, target))
            end
          end
        end
      end
    end
  end

  if opts.fix then
    M.build_cache()
    M.rebuild_current({ quiet = true })
  end
  return found
end

--- Edição por contexto do cursor + form -------------------------------------

---Resolve a task cujo callout está sob o cursor no buffer atual.
---@return table|nil ctx { source, buf, block_start, block_end, project, task_path, model }
function M.resolve_under_cursor()
  M.ensure_root()
  M.load_status()
  local buf = vim.api.nvim_get_current_buf()
  local path = vim.api.nvim_buf_get_name(buf)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local lnum = vim.api.nvim_win_get_cursor(0)[1]

  local s, e = block_around(lines, lnum)
  if not s then
    vim.notify("freitask: cursor não está sobre um callout", vim.log.levels.WARN)
    return nil
  end
  -- permissivo como o parser: uma task com tipo inválido (status 0) precisa
  -- continuar abrindo no form, senão o estado quebrado seria inconsertável.
  if not lines[s]:match(">%s*%[![^%]]*%]") then
    vim.notify("freitask: bloco sob o cursor não é um callout de task", vim.log.levels.WARN)
    return nil
  end

  local block = {}
  for i = s, e do
    block[#block + 1] = lines[i]
  end
  local model = M.parse_block(block)
  if model.id == "" then
    vim.notify("freitask: callout sem [[id]]", vim.log.levels.WARN)
    return nil
  end

  local is_current = path:match("/CURRENT%.md$") ~= nil
  local project, task_path, archived
  if is_current then
    for i = s, 1, -1 do
      local p = lines[i]:match("^##%s+(.+)$")
      if p then
        project = vim.trim(p)
        break
      end
    end
    if not project then
      vim.notify("freitask: não achei o projeto (## ...) acima do callout", vim.log.levels.WARN)
      return nil
    end
    -- Task arquivada nunca aparece no CURRENT.md, logo archived é sempre nil aqui.
    task_path = task_file(project, model.id, nil)
  else
    -- split_task_path devolve (project, id, archived); o id vem do próprio
    -- caminho e aqui já temos o do bloco, então só o 1º e o 3º interessam.
    local parts = { split_task_path(path) }
    project, archived = parts[1], parts[3]
    if not project then
      vim.notify("freitask: buffer não é um arquivo de task", vim.log.levels.WARN)
      return nil
    end
    task_path = path
  end

  return {
    source = is_current and "current" or "task",
    buf = buf,
    block_start = s,
    block_end = e,
    project = project,
    archived = archived,
    task_path = task_path,
    model = model,
  }
end

--- Form posicional ------------------------------------------------------------
--
-- O buffer não tem rótulos — é para uso rápido, quatro campos por posição:
--   1  título
--   2  id / branch
--   3  <tipo-de-callout> [descrição opcional do estado]
--   4+ notas (texto livre)
-- Os rótulos aparecem como virtual text à direita (ver render_form_labels), o
-- que preserva a densidade do buffer sem custar a legibilidade: sem eles, uma
-- linha apagada por acidente desloca todos os campos em silêncio — e como
-- apply_edit trata mudança de id como rename, o deslocamento chega a mexer em
-- arquivo. Por isso o confirm também VALIDA antes de aplicar.

local FORM_LABELS = { "título", "id / branch", "tipo de status + descrição" }
local form_ns = vim.api.nvim_create_namespace("obsidian_task_form")

---Desenha os rótulos das 3 primeiras linhas como virtual text.
---@param buf integer
local function render_form_labels(buf)
  if not vim.api.nvim_buf_is_valid(buf) then
    return
  end
  vim.api.nvim_buf_clear_namespace(buf, form_ns, 0, -1)
  local n = vim.api.nvim_buf_line_count(buf)
  for i, label in ipairs(FORM_LABELS) do
    if i <= n then
      vim.api.nvim_buf_set_extmark(buf, form_ns, i - 1, 0, {
        virt_text = { { "  " .. label, "Comment" } },
        virt_text_pos = "eol",
      })
    end
  end
end

---Completion de tipos de callout, para a linha 3 do form. Sem isso o formato
---posicional exigiria decorar os 27 tipos.
---@param findstart integer
---@param base string
function M.callout_omnifunc(findstart, base)
  if findstart == 1 then
    local line = vim.api.nvim_get_current_line()
    local col = vim.api.nvim_win_get_cursor(0)[2]
    local start = col
    while start > 0 and line:sub(start, start):match("[%w_]") do
      start = start - 1
    end
    return start
  end
  if not M.status then
    M.load_status()
  end
  local out = {}
  local keys = vim.tbl_keys(M.status or {})
  table.sort(keys, function(a, b)
    return tonumber(a) < tonumber(b)
  end)
  for _, k in ipairs(keys) do
    local st = M.status[k]
    if st.callout and vim.startswith(st.callout, base) then
      out[#out + 1] = { word = st.callout, menu = k .. " · " .. (st.title or "") }
    end
  end
  return out
end

---Reconstrói um modelo a partir das linhas do form posicional.
---Estrito de propósito: o parser de arquivo é tolerante (tipo desconhecido →
---status 0), mas aqui recusamos, senão um typo viraria uma task inválida em
---silêncio — que é justamente o bug que o status 0 existe para tornar visível.
---@param bl string[]
---@param base table modelo original (fallback de project/raw_callout)
---@return table|nil model, string|nil err
local function parse_form(bl, base)
  if #bl < 3 then
    return nil, "form precisa de ao menos 3 linhas (título, id, status)"
  end
  if not M.status then
    M.load_status()
  end

  local title = vim.trim(bl[1])
  local id = kebab(vim.trim(bl[2]))
  if id == "" then
    id = kebab(title) -- id em branco deriva do título (usado na criação)
  end
  if title == "" and id == "" then
    return nil, "título e id vazios"
  end
  if id == "" then
    return nil, "id inválido"
  end

  local tag, desc = vim.trim(bl[3]):match("^(%S+)%s*(.*)$")
  if not tag then
    return nil, "linha 3 precisa começar com um tipo de callout (ex.: todo)"
  end
  local rev = status_by_callout()
  local status_num = rev[tag]
  if not status_num then
    return nil, string.format("tipo de callout desconhecido: %q (use <C-s> ou <C-x><C-o>)", tag)
  end

  local extras = {}
  for i = 4, #bl do
    extras[#extras + 1] = bl[i]
  end
  while #extras > 0 and vim.trim(extras[#extras]) == "" do
    table.remove(extras)
  end

  return {
    status_num = status_num,
    raw_callout = tag,
    title = title,
    id = id,
    desc = vim.trim(desc or ""),
    extras = extras,
    project = base and base.project,
  }
end

---Abre o form posicional flutuante. Usado tanto pela edição quanto pela
---criação — um único formato, um único parser.
---@param model table modelo inicial (na criação, campos vazios + status default)
---@param opts { title: string }
---@param on_confirm fun(new_model: table)
function M.edit_task_form(model, opts, on_confirm)
  M.load_status()
  -- Status 0 devolve o tipo digitado, para o erro ficar visível e editável.
  local tag = (model.status_num == 0) and vim.trim(model.raw_callout or "")
    or (M.status_meta(model.status_num).callout or "todo")
  local status_line = tag
  if vim.trim(model.desc or "") ~= "" then
    status_line = tag .. " " .. vim.trim(model.desc)
  end

  local lines = {
    model.title or "",
    model.id or "",
    status_line,
  }
  for _, e in ipairs(model.extras or {}) do
    lines[#lines + 1] = e
  end

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].filetype = "markdown"
  vim.bo[buf].omnifunc = "v:lua.require'util.freitask'.callout_omnifunc"

  local width = 66
  local height = math.max(#lines + 2, 8)
  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = math.floor((vim.o.lines - height) / 2),
    col = math.floor((vim.o.columns - width) / 2),
    style = "minimal",
    border = "rounded",
    title = " " .. (opts.title or "Task") .. "  ⏎ salva · q cancela · C-s status ",
    title_pos = "center",
  })

  render_form_labels(buf)
  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    buffer = buf,
    callback = function()
      render_form_labels(buf)
    end,
  })

  local function close()
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
  end

  local function confirm()
    local bl = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
    local nm, err = parse_form(bl, model)
    if not nm then
      -- não fecha: o buffer fica aberto com o texto para corrigir.
      vim.notify("freitask: " .. err, vim.log.levels.WARN)
      return
    end
    close()
    on_confirm(nm)
  end

  local function pick_status()
    local keys = vim.tbl_keys(M.status or {})
    table.sort(keys, function(a, b)
      return tonumber(a) < tonumber(b)
    end)
    local choices = {}
    for _, n in ipairs(keys) do
      choices[#choices + 1] = { callout = M.status[n].callout, label = n .. " - " .. (M.status[n].title or "") }
    end
    vim.ui.select(choices, {
      prompt = "Status:",
      format_item = function(c)
        return c.label
      end,
    }, function(c)
      if not c or not vim.api.nvim_buf_is_valid(buf) then
        return
      end
      -- troca só o tipo, preservando a descrição do estado que já estiver lá.
      local cur = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1] or ""
      local rest = vim.trim(cur):match("^%S+%s+(.*)$") or ""
      local new = (rest ~= "") and (c.callout .. " " .. rest) or c.callout
      vim.api.nvim_buf_set_lines(buf, 2, 3, false, { new })
      render_form_labels(buf)
    end)
  end

  vim.keymap.set("n", "<CR>", confirm, { buffer = buf, nowait = true, desc = "Salvar task" })
  vim.keymap.set("n", "q", close, { buffer = buf, nowait = true, desc = "Cancelar" })
  vim.keymap.set("n", "<Esc>", close, { buffer = buf, nowait = true, desc = "Cancelar" })
  vim.keymap.set({ "n", "i" }, "<C-s>", pick_status, { buffer = buf, desc = "Escolher status" })
end

---Aplica um modelo editado: persiste no arquivo-fonte (com rename via
---delete+recreate quando o id muda) e reflete na origem (buffer da task ou
---bloco em CURRENT.md).
---@param ctx table contexto de resolve_under_cursor / edit_task_file
---@param nm table novo modelo vindo do form
function M.apply_edit(ctx, nm)
  M.load_status()
  if nm.id == "" then
    vim.notify("freitask: id inválido", vim.log.levels.WARN)
    return
  end

  local old_id = ctx.model.id
  local renaming = nm.id ~= old_id
  -- id == nome da branch: renomear o id renomeia a branch (via delete+recreate).

  local project = ctx.project
  -- ctx.archived acompanha o rename: sem ele, renomear o id de uma task
  -- arquivada a ressuscitaria em tasks/<projeto>/ — e no board — em silêncio.
  local old_path = (ctx.source == "task") and ctx.task_path or task_file(project, old_id, ctx.archived)
  local new_path = renaming and task_file(project, nm.id, ctx.archived) or old_path
  if renaming and vim.fn.filereadable(new_path) == 1 then
    vim.notify("freitask: já existe task com id " .. nm.id, vim.log.levels.WARN)
    return
  end

  nm.project, nm.archived = project, ctx.archived
  local new_block = M.serialize_block(nm)
  local tbuf = loaded_buf(old_path)

  if renaming then
    if tbuf and vim.bo[tbuf].modified then
      vim.notify("freitask: salve o arquivo da task antes de renomear o id", vim.log.levels.WARN)
      return
    end
    -- rename de verdade, em vez de escrever-o-novo + apagar-o-velho: se a
    -- escrita falhar no meio, o arquivo antigo não chega a ser destruído.
    if vim.fn.filereadable(old_path) == 1 and vim.fn.rename(old_path, new_path) ~= 0 then
      vim.notify("freitask: não consegui renomear para " .. nm.id, vim.log.levels.ERROR)
      return
    end
    local src = tbuf and vim.api.nvim_buf_get_lines(tbuf, 0, -1, false)
      or (vim.fn.filereadable(new_path) == 1 and vim.fn.readfile(new_path) or {})
    local s, e = first_block_range(src)
    local out = s and splice(src, s, e, new_block) or vim.deepcopy(new_block)
    -- `id:` do frontmatter (injetado pelo obsidian.nvim) espelha o nome do
    -- arquivo; sem isto ele ficaria apontando para o id antigo depois do rename.
    update_frontmatter_key(out, "id", nm.id)
    vim.fn.writefile(out, new_path)
    if tbuf then
      pcall(vim.api.nvim_buf_set_name, tbuf, new_path)
      vim.api.nvim_buf_call(tbuf, function()
        vim.cmd("silent edit!")
      end)
    end
    M.remove_cache_entry(old_path)
    M.update_cache_entry(new_path)
    -- Renomear muda o basename, então quebra TAMBÉM os links curtos `[[id]]`
    -- espalhados pelo vault — não só os path-qualified.
    M.retarget_links(old_path, new_path)
  else
    if tbuf then
      local s, e = first_block_range(vim.api.nvim_buf_get_lines(tbuf, 0, -1, false))
      if s then
        vim.api.nvim_buf_set_lines(tbuf, s - 1, e, false, new_block)
      else
        local prepend = vim.deepcopy(new_block)
        prepend[#prepend + 1] = ""
        vim.api.nvim_buf_set_lines(tbuf, 0, 0, false, prepend)
      end
      vim.api.nvim_buf_call(tbuf, function()
        vim.cmd("silent keepjumps write")
      end)
    else
      file_replace_callout(old_path, new_block)
    end
    M.update_cache_entry(old_path)
  end

  -- Reflete in place no CURRENT.md quando a edição começou lá.
  if ctx.source == "current" and vim.api.nvim_buf_is_valid(ctx.buf) then
    vim.api.nvim_buf_set_lines(ctx.buf, ctx.block_start - 1, ctx.block_end, false, new_block)
    vim.api.nvim_buf_call(ctx.buf, function()
      vim.cmd("silent keepjumps write")
    end)
  end

  -- Editar uma task arquivada NÃO a desarquiva, nem mesmo quando o novo callout
  -- é de task ativa: mover arquivo como efeito colateral de um save é o tipo de
  -- mágica que come dado. O aviso existe para que o silêncio não seja lido como
  -- "voltou pro board".
  if ctx.archived then
    vim.notify(
      string.format("freitask: %s atualizada, mas segue em archived/%s (<leader>oa para desarquivar)", nm.id, ctx.archived),
      vim.log.levels.INFO
    )
  else
    vim.notify("freitask: task atualizada (" .. nm.id .. ")", vim.log.levels.INFO)
  end
end

---Ponto de entrada do keymap: edita o callout sob o cursor.
function M.edit_under_cursor()
  local ctx = M.resolve_under_cursor()
  if not ctx then
    return
  end
  local title = ctx.archived and ("Editar task (archived/" .. ctx.archived .. ")") or "Editar task"
  M.edit_task_form(ctx.model, { title = title }, function(nm)
    M.apply_edit(ctx, nm)
  end)
end

---Abre o form para um arquivo de task específico (usado pelo picker).
---@param path string
function M.edit_task_file(path)
  M.ensure_root()
  M.load_status()
  local lines = vim.fn.filereadable(path) == 1 and vim.fn.readfile(path) or {}
  local s, e = first_block_range(lines)
  if not s then
    vim.notify("freitask: arquivo de task sem callout", vim.log.levels.WARN)
    return
  end
  local block = {}
  for i = s, e do
    block[#block + 1] = lines[i]
  end
  local model = M.parse_block(block)
  local project, _, archived = split_task_path(path)
  if not project then
    vim.notify("freitask: não é um arquivo de task: " .. path, vim.log.levels.WARN)
    return
  end
  local ctx = {
    source = "task",
    buf = vim.fn.bufnr(path),
    block_start = s,
    block_end = e,
    project = project,
    archived = archived,
    task_path = path,
    model = model,
  }
  local title = archived and ("Editar task (archived/" .. archived .. ")") or "Editar task"
  M.edit_task_form(model, { title = title }, function(nm)
    M.apply_edit(ctx, nm)
  end)
end

---Abre o form vazio para criar uma task no projeto — mesmo buffer e mesmo
---parser da edição, então só existe um formato para aprender. O id em branco é
---derivado do título por parse_form.
---@param project string
function M.create_task_form(project)
  M.ensure_root()
  M.load_status()
  local seed = { status_num = 1, title = "", id = "", desc = "", extras = {}, project = project }
  M.edit_task_form(seed, { title = "Nova task em " .. project }, function(nm)
    local path = root .. "/" .. project .. "/" .. nm.id .. ".md"
    if vim.fn.filereadable(path) == 1 then
      vim.notify("freitask: task já existe: " .. nm.id, vim.log.levels.WARN)
      return
    end
    nm.project = project
    local out = M.template(nm)
    vim.fn.writefile(out, path)
    M.update_cache_entry(path)
    M.rebuild_current({ quiet = true })
    vim.cmd.edit(vim.fn.fnameescape(path))
  end)
end

--- Pickers -------------------------------------------------------------------

---Level 2: tasks de um único projeto.
---@param project string
function M.open_tasks(project)
  M.ensure_root()
  M.load_status()
  -- Arquivadas ficam ocultas por default e são lidas do disco só quando este
  -- toggle liga (M.archived_entries_for) — nunca entram em M.cache.
  local show_archived = false
  Snacks.picker.pick({
    source = "obsidian_tasks",
    title = "Tasks: "
      .. project
      .. "   ⏎ open · C-t new · C-x del · C-r archive · C-u archived · C-e edit · C-o back · ? help",
    show_empty = true,
    finder = function()
      local items = {}
      for _, e in ipairs(M.entries_for(project)) do
        items[#items + 1] = { text = e.task_id, file = e.path, entry = e }
      end
      if show_archived then
        for _, e in ipairs(M.archived_entries_for(project)) do
          items[#items + 1] = { text = e.archived .. " " .. e.task_id, file = e.path, entry = e }
        end
      end
      return items
    end,
    format = function(item)
      local e = item.entry
      local st = M.status_meta(e.status_num)
      local hl = st.hl_group or "Normal"
      local out = {}
      if e.archived then
        -- Prefixo em Comment: a task arquivada aparece visivelmente rebaixada,
        -- para não competir com o board ativo na leitura da lista.
        out[#out + 1] = { string.format("[%s] ", e.archived), "Comment" }
      end
      vim.list_extend(out, {
        { (st.icon or "") .. " ", hl },
        { st.title or ("Status " .. tostring(e.status_num)), hl },
        { " - ", "SnacksPickerDelim" },
        { e.task_id, e.archived and "Comment" or "SnacksPickerLabel" },
      })
      return out
    end,
    confirm = function(picker, item)
      if not item then
        return
      end
      picker:close()
      vim.schedule(function()
        vim.cmd.edit(vim.fn.fnameescape(item.file))
      end)
    end,
    actions = {
      -- <C-t>: cria uma nova task no form posicional (mesmo buffer da edição).
      new_task = function(picker)
        picker:close()
        vim.schedule(function()
          M.create_task_form(project)
        end)
      end,
      -- <C-x>: deleta a task selecionada após confirmação.
      delete_task = function(picker, item)
        item = item or picker:current()
        if not item then
          return
        end
        local choice = vim.fn.confirm("Delete task '" .. item.entry.task_id .. "'?", "&Yes\n&No", 2)
        if choice ~= 1 then
          return
        end
        vim.fn.delete(item.file)
        M.remove_cache_entry(item.file)
        M.rebuild_current({ quiet = true })
        picker:find()
      end,
      -- <C-r>: arquiva a task (perguntando o tipo) ou, se ela já estiver
      -- arquivada, desarquiva — a mesma tecla nos dois sentidos.
      -- vim.fn.confirm e não vim.ui.select: é bloqueante e não abre uma janela
      -- que dispute foco com a do picker.
      archive_task = function(picker, item)
        item = item or picker:current()
        if not item then
          return
        end
        local e = item.entry
        if e.archived then
          if vim.fn.confirm("Desarquivar '" .. e.task_id .. "'?", "&Sim\n&Não", 2) ~= 1 then
            return
          end
          M.unarchive_task(item.file)
        else
          -- Default derivado do callout; as iniciais de atalho são distintas
          -- (o/p/f) porque "done" e "dropped" colidiriam em "d".
          local suggested = M.suggest_archive_type(e.status_num)
          local labels, default = { "d&one", "dro&pped", "&failed" }, 1
          for i, t in ipairs(M.ARCHIVED_TYPES) do
            if t == suggested then
              default = i
            end
          end
          local choice =
            vim.fn.confirm("Arquivar '" .. e.task_id .. "' como:", table.concat(labels, "\n") .. "\n&cancelar", default)
          if choice < 1 or choice > #M.ARCHIVED_TYPES then
            return
          end
          M.archive_task(item.file, M.ARCHIVED_TYPES[choice])
        end
        M.rebuild_current({ quiet = true })
        picker:find()
      end,
      -- <C-u>: mostra/esconde as tasks arquivadas do projeto na mesma lista.
      toggle_archived = function(picker)
        show_archived = not show_archived
        picker:find()
      end,
      -- <C-e>: edita a task selecionada no form multi-campo.
      edit_task = function(picker, item)
        item = item or picker:current()
        if not item then
          return
        end
        local file = item.file
        picker:close()
        vim.schedule(function()
          M.edit_task_file(file)
        end)
      end,
      -- <C-o>: volta para a lista de projetos (Level 1).
      back_to_projects = function(picker)
        picker:close()
        vim.schedule(function()
          M.open_projects()
        end)
      end,
    },
    win = {
      input = {
        keys = {
          ["<c-t>"] = { "new_task", mode = { "i", "n" }, desc = "New task" },
          ["<c-x>"] = { "delete_task", mode = { "i", "n" }, desc = "Delete task" },
          ["<c-r>"] = { "archive_task", mode = { "i", "n" }, desc = "Archive/unarchive task" },
          ["<c-u>"] = { "toggle_archived", mode = { "i", "n" }, desc = "Toggle archived" },
          ["<c-e>"] = { "edit_task", mode = { "i", "n" }, desc = "Edit task" },
          ["<c-o>"] = { "back_to_projects", mode = { "i", "n" }, desc = "Back to projects" },
        },
      },
    },
  })
end

---Level 1: projetos.
function M.open_projects()
  M.ensure_root()
  M.load_status()
  M.build_cache()
  Snacks.picker.pick({
    source = "obsidian_projects",
    title = "Task Projects   ⏎ open · C-c new · C-a CURRENT.md · ? help",
    show_empty = true,
    preview = "none",
    layout = { preview = false },
    finder = function()
      local items = {}
      for _, project in ipairs(M.list_projects()) do
        items[#items + 1] = { text = project, project = project }
      end
      return items
    end,
    format = function(item)
      return {
        { "  ", "SnacksPickerDirectory" },
        { item.project, "SnacksPickerDirectory" },
      }
    end,
    confirm = function(picker, item)
      if not item then
        return
      end
      picker:close()
      vim.schedule(function()
        M.open_tasks(item.project)
      end)
    end,
    actions = {
      -- <C-c>: cria um novo diretório de projeto.
      new_project = function(picker)
        vim.ui.input({ prompt = "New project name: " }, function(input)
          if not input or input == "" then
            return
          end
          local id = kebab(input)
          if id == "" then
            vim.notify("freitask: nome de projeto inválido", vim.log.levels.WARN)
            return
          end
          vim.fn.mkdir(root .. "/" .. id, "p")
          picker:find()
        end)
      end,
      -- <C-a>: regenera e abre o dashboard CURRENT.md.
      open_current = function(picker)
        picker:close()
        vim.schedule(function()
          M.rebuild_current()
          vim.cmd.edit(vim.fn.fnameescape(root .. "/CURRENT.md"))
        end)
      end,
    },
    win = {
      input = {
        keys = {
          ["<c-c>"] = { "new_project", mode = { "i", "n" }, desc = "New project" },
          ["<c-a>"] = { "open_current", mode = { "i", "n" }, desc = "Open CURRENT.md" },
        },
      },
    },
  })
end

---Arquiva/desarquiva a task do buffer atual — a contraparte do <C-r> do picker
---para quando você já está dentro do arquivo. Não é campo do form de propósito:
---mover arquivo é uma ação com confirmação, e uma quarta linha posicional
---quebraria o contrato "linha 4+ = notas" (uma linha apagada por acidente
---passaria a mover arquivo de lugar).
function M.toggle_archive_current_file()
  local path = vim.api.nvim_buf_get_name(0)
  local project, id, archived = split_task_path(path)
  if not project then
    vim.notify("freitask: buffer não é um arquivo de task", vim.log.levels.WARN)
    return
  end
  local new_path
  if archived then
    if vim.fn.confirm("Desarquivar '" .. id .. "'?", "&Sim\n&Não", 2) ~= 1 then
      return
    end
    new_path = M.unarchive_task(path)
  else
    local suggested = M.suggest_archive_type(M.parse_status_num(path))
    local default = 1
    for i, t in ipairs(M.ARCHIVED_TYPES) do
      if t == suggested then
        default = i
      end
    end
    local choice = vim.fn.confirm("Arquivar '" .. id .. "' como:", "d&one\ndro&pped\n&failed\n&cancelar", default)
    if choice < 1 or choice > #M.ARCHIVED_TYPES then
      return
    end
    new_path = M.archive_task(path, M.ARCHIVED_TYPES[choice])
  end
  if new_path then
    M.rebuild_current({ quiet = true })
  end
end

--- Autocmds ------------------------------------------------------------------

---Mantém o cache fresco e instala o keymap buffer-local de edição por cursor.
function M.setup_autocmd()
  local grp = vim.api.nvim_create_augroup("freitask", { clear = true })

  -- Ao salvar uma task, regenera o CURRENT.md (silenciosamente) para que novas
  -- tasks/mudanças de status apareçam sozinhas no painel.
  vim.api.nvim_create_autocmd("BufWritePost", {
    group = grp,
    pattern = root .. "/*/*.md",
    callback = function(args)
      -- `*` casa `/` em pattern de autocmd, então isto também dispara para
      -- arquivos em archived/; split_task_path devolve o tipo e nós pulamos —
      -- salvar uma task arquivada não deve mexer no board.
      local project, _, archived = split_task_path(args.match)
      if not project or archived then
        return
      end
      vim.schedule(function()
        M.rebuild_current({ quiet = true })
      end)
    end,
  })

  -- <leader>oe buffer-local: em CURRENT.md e em qualquer arquivo de task
  -- (inclusive arquivadas — `*` casa `/`). <leader>oa só faz sentido num
  -- arquivo de task, então é registrado condicionalmente.
  vim.api.nvim_create_autocmd("BufEnter", {
    group = grp,
    pattern = { root .. "/*.md", root .. "/*/*.md" },
    callback = function(args)
      vim.keymap.set("n", "<leader>oe", M.edit_under_cursor, {
        buffer = args.buf,
        desc = "Editar callout sob cursor",
      })
      if split_task_path(vim.api.nvim_buf_get_name(args.buf)) then
        vim.keymap.set("n", "<leader>oa", M.toggle_archive_current_file, {
          buffer = args.buf,
          desc = "Arquivar/desarquivar task",
        })
      end
    end,
  })
end

--- Superfície de teste -------------------------------------------------------
--
-- As funções puras deste módulo (parser, serializer, derivação de id, reescrita
-- de link) são locais de propósito: não fazem parte da API. Mas são justamente
-- as que precisam de teste, porque são as que erram em silêncio — um link
-- reescrito errado só aparece semanas depois, quando você clica nele.
--
-- Esta tabela é TEMPORÁRIA: quando o módulo for dividido (ver
-- docs/freitask-internals.md), cada uma dessas funções vira função pública do
-- seu módulo (`path.kebab`, `md.first_block_range`, `links.rewrite_link`) e
-- este bloco some junto com o arquivo monolítico.
M.__test = {
  DEFAULT_STATUS_JSON = DEFAULT_STATUS_JSON,
  kebab = kebab,
  strip_quote = strip_quote,
  is_status_text = is_status_text,
  content_start = content_start,
  first_block_range = first_block_range,
  block_around = block_around,
  splice = splice,
  rewrite_link = rewrite_link,
  append_history = append_history,
  update_frontmatter_key = update_frontmatter_key,
  vault_dir = vault_dir,
}

return M
