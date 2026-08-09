-- freitask.model — parser e serializer do bloco de callout de uma task.
--
-- O núcleo, e a única parte 100% pura do freitask: recebe linhas, devolve um
-- freitask.Model, e vice-versa. Não lê disco, não notifica, não conhece
-- buffer. É também a parte com mais teste (tests/freitask_spec.lua), porque é
-- a que erra em silêncio.
--
-- O parser é TOLERANTE (aceita os formatos legados, e um callout desconhecido
-- vira status 0 em vez de erro); quem é estrito é o form. Ver docs/freitask.md.

local C = require("freitask.config")
local md = require("freitask.md")
local path = require("freitask.path")
local status = require("freitask.status")

local M = {}

---Verdadeiro se `content` é um "texto de status" legado (ex.: "1 - Backlog")
---— usado p/ não confundir a linha de status com uma nota/nome. O título é
---comparado contra QUALQUER entrada de status.json (não só a de número
---correspondente), pois renumerar/reordenar status.json não deve fazer uma
---linha legada como "3 - Blocked" ser lida como nome livre.
---@param content string
---@return boolean, integer|nil
function M.is_status_text(content)
  local snum, stitle = content:match("^(%d+)%s*%-%s*(.+)$")
  if not snum or not status.status then
    return false, nil
  end
  stitle = vim.trim(stitle)
  for k, v in pairs(status.status) do
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
  local legacy = C.LEGACY_STATUS_TITLES[stitle]
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
---@return freitask.Model
function M.parse_block(block)
  status.ensure()
  local rev = status.by_callout()
  local model = { status_num = 1, raw_callout = "", title = "", id = "", desc = "", extras = {} }

  -- Linha 1: sempre "> [!callout] <resto>". O callout dá o status. O match é
  -- PERMISSIVO (`[^%]]*` e não `%w+`) porque um tipo inválido pode conter
  -- acento ou hífen; com `%w+` o match falhava por completo, o prefixo não era
  -- removido e "[!questão] Título" virava o título inteiro — corrupção.
  local first = block[1] and md.strip_quote(block[1]) or ""
  local callout = first:match("^%[!([^%]]*)%]")
  if callout then
    model.raw_callout = callout
    -- tipo desconhecido → 0, preservando o texto digitado para o form mostrar.
    model.status_num = rev[callout] or 0
  end
  local rest = callout and first:gsub("^%[![^%]]*%]%s*", "") or first
  if rest ~= "" and not (M.is_status_text(rest)) then
    -- resto da linha 1 é o título (novo formato); no legado seria status-texto.
    model.title = vim.trim(rest:match("%*%*(.-)%*%*") or rest)
  end

  -- Linhas 2..n
  local seen_note = false
  for i = 2, #block do
    local content = md.strip_quote(block[i])
    if model.id == "" then
      local lid = content:match("%[%[(.-)%]%]")
      if lid then
        local target = vim.trim((lid:gsub("|.*$", "")))
        model.id = vim.fn.fnamemodify(target, ":t:r")
      end
    end
    local is_only_link = content:match("^%s*%[%[.-%]%]%s*$") ~= nil
    local statusy, snum = M.is_status_text(content)
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
---@param model freitask.Model
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
    callout = status.status_meta(model.status_num).callout or "note"
  end
  -- Link com caminho relativo ao vault + id como alias evita ambiguidade entre
  -- projetos diferentes que tenham tasks com o mesmo id; sem `model.project`
  -- (ex.: contexto de teste) cai no formato legado `[[id]]`. `model.archived`
  -- move o alvo para archived/<tipo>/, senão o link de uma task arquivada
  -- apontaria para um caminho que não existe mais.
  local link = model.project
      and string.format("%s/%s|%s", path.vault_dir(model.project, model.archived), model.id or "", model.id or "")
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

return M
