-- freitask.meta — a PRIORIDADE de uma task, lida do frontmatter.
--
-- É ortogonal ao status. O status (o callout) diz em que FASE o trabalho está;
-- `prioridade: alta` diz se ele passa na frente das outras. Uma marca só, e sem
-- ela a task é normal: três níveis acabam com tudo em "média".
--
-- Mora no frontmatter, e não no bloco de callout, por dois motivos: é metadado
-- consultável (o Obsidian indexa propriedade, não texto de blockquote) e o
-- celular o edita. O bloco é reserializado inteiro pelo doctor a partir do
-- CAMINHO — prioridade não é derivável de caminho nenhum, então ficaria exposta
-- a ser regenerada para fora.
--
-- A escrita é à mão, de propósito: não há subcomando para gravá-la, porque o
-- freitask só cresce por dor medida. O que é automático é a vigilância: o
-- doctor acusa valor diferente de `alta`, que de outro modo viraria "normal"
-- em silêncio.

local md = require("freitask.md")

local M = {}

-- A chave do eixo. Em tabela para que doctor e leitor varram a mesma lista em
-- vez de repetir o literal.
M.KEYS = { "prioridade" }

-- O único valor que conta. Comparação EXATA: `Alta` não é alta, e é o doctor
-- que avisa, em vez de o motor adivinhar o que o dono quis dizer.
M.HIGH = "alta"

---Lê a prioridade do frontmatter. Valor vazio conta como ausente.
---@param lines string[]
---@return freitask.Meta
function M.read(lines)
  local fm = md.frontmatter(lines)
  local out = {}
  for _, k in ipairs(M.KEYS) do
    local v = fm[k]
    if type(v) == "string" and vim.trim(v) ~= "" then
      out[k] = vim.trim(v)
    end
  end
  return out
end

---Escreve a prioridade no frontmatter. Campo ausente de `meta` é REMOVIDO — a
---tabela é o estado desejado inteiro, não um patch, para que tirar a marca não
---dependa de lembrar de passar `false`.
---@param lines string[]
---@param meta freitask.Meta
---@return boolean changed
function M.write(lines, meta)
  local updates = {}
  for _, k in ipairs(M.KEYS) do
    updates[k] = meta[k] or false
  end
  return md.set_frontmatter(lines, updates)
end

---Verdadeiro se a task é de prioridade alta.
---@param meta freitask.Meta|nil
---@return boolean
function M.is_high(meta)
  return meta ~= nil and meta.prioridade == M.HIGH
end

return M
