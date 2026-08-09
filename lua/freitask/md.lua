-- freitask.md — operações sobre as LINHAS de um arquivo markdown.
--
-- Nada aqui sabe o que é uma task: só o que é um blockquote, um frontmatter
-- YAML e uma seção `##`. É a camada que o parser do bloco (freitask.model) usa
-- para achar o que parsear, e que as operações de arquivo usam para reescrever
-- no lugar certo sem tocar no resto do arquivo.

local C = require("freitask.config")

local M = {}

---Remove o prefixo de blockquote ("> " / ">") de uma linha.
---@param line string
---@return string
function M.strip_quote(line)
  return (line:gsub("^%s*>%s?", ""))
end

---Substitui lines[s..e] por `repl`, retornando um novo array.
---@param lines string[]
---@param s integer
---@param e integer
---@param repl string[]
---@return string[]
function M.splice(lines, s, e, repl)
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
function M.content_start(lines)
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
function M.first_block_range(lines)
  local start
  for i = M.content_start(lines), #lines do
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

---Faixa e CONTEÚDO do primeiro bloco `>`. O par first_block_range + laço de
---cópia `for i = s, e do blk[#blk+1] = lines[i] end` aparecia em sete lugares;
---aqui ele existe uma vez.
---@param lines string[]
---@return integer|nil start, integer|nil finish, string[]|nil block
function M.first_block(lines)
  local s, e = M.first_block_range(lines)
  if not s then
    return nil
  end
  return s, e, vim.list_slice(lines, s, e)
end

---Faixa do bloco `>` que contém a linha `lnum` (1-indexed). nil se `lnum` não
---estiver sobre uma linha de blockquote.
---@param lines string[]
---@param lnum integer
---@return integer|nil, integer|nil
function M.block_around(lines, lnum)
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

---Atualiza a chave `key` do frontmatter YAML, se (e só se) ela já existir.
---Deliberadamente não cria frontmatter: quem o injeta é o obsidian.nvim, e
---inventá-lo aqui acrescentaria ruído a arquivos que não o têm.
---@param lines string[]
---@param key string
---@param value string
---@return boolean changed
function M.update_frontmatter_key(lines, key, value)
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
function M.append_history(lines, dest)
  local entry
  if dest then
    entry = string.format(
      "- %s — arquivada em `archived/%s` (%s)",
      os.date("%Y-%m-%d"),
      dest,
      C.ARCHIVED_LABELS[dest] or dest
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

return M
