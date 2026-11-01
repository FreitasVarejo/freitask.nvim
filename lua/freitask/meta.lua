-- freitask.meta — o que o freitask lê do FRONTMATTER de uma task: a PRIORIDADE
-- e a DATA DE VOLTA.
--
-- Os dois são ortogonais ao status. O status (o callout) diz em que FASE o
-- trabalho está; `prioridade: alta` diz se ele passa na frente das outras, e
-- `volta-em: AAAA-MM-DD` diz até que dia ele fica fora da tela (a task
-- ADIADA). Uma marca só para a prioridade, e sem ela a task é normal: três
-- níveis acabam com tudo em "média".
--
-- Moram no frontmatter, e não no bloco de callout, por dois motivos: são
-- metadado consultável (o Obsidian indexa propriedade, não texto de
-- blockquote) e o celular os edita. O bloco é reserializado inteiro pelo
-- doctor a partir do CAMINHO — prioridade e data de volta não são deriváveis
-- de caminho nenhum, então ficariam expostas a ser regeneradas para fora.
--
-- A prioridade é escrita à mão, de propósito: não há subcomando para gravá-la,
-- porque o freitask só cresce por dor medida. A data de volta tem o `set
-- --volta-em`, que valida o que grava. O que é automático é a vigilância: o
-- doctor acusa o valor que, sem isso, viraria "normal" (a prioridade) ou "não
-- adiada" (a data) em silêncio.

local md = require("freitask.md")

local M = {}

-- A chave do eixo. Em tabela para que doctor e leitor varram a mesma lista em
-- vez de repetir o literal.
M.KEYS = { "prioridade" }

-- O único valor que conta. Comparação EXATA: `Alta` não é alta, e é o doctor
-- que avisa, em vez de o motor adivinhar o que o dono quis dizer.
M.HIGH = "alta"

-- A chave da data de volta. Fora de `KEYS` de propósito: `write` grava o estado
-- inteiro da prioridade, e uma gravação dela não pode apagar a data (nem o
-- contrário) — cada campo tem o seu escritor.
M.RETURN_KEY = "volta-em"

---Lê a prioridade e a data de volta do frontmatter. Valor vazio conta como
---ausente. A data sai como está escrita (aparada): quem decide se é uma data é
---`parse_date`, para que o texto inválido chegue ao doctor em vez de sumir aqui.
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
  local back = fm[M.RETURN_KEY]
  if type(back) == "string" and vim.trim(back) ~= "" then
    out.volta_em = vim.trim(back)
  end
  return out
end

---Escreve a prioridade no frontmatter (e só ela; a data de volta tem o
---`write_volta_em`). Campo ausente de `meta` é REMOVIDO — a tabela é o estado
---desejado inteiro, não um patch, para que tirar a marca não dependa de
---lembrar de passar `false`.
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

--- Data de volta -------------------------------------------------------------

---Uma data `AAAA-MM-DD` que EXISTE no calendário, como o inteiro AAAAMMDD, que
---ordena como as datas ordenam. É o único jeito de comparar: o texto cru engana
---(`2026-12-1` ficaria "maior" que `2026-12-05`), e uma comparação de texto
---trataria `2026-02-30` como um dia como outro qualquer. Qualquer outra forma
---(`1/12/2026`, `2026-12-1`, um dia que não existe) devolve nil, e quem chama
---decide o que fazer com isso.
---@param s any
---@return integer|nil
function M.parse_date(s)
  if type(s) ~= "string" then
    return nil
  end
  local y, m, d = s:match("^(%d%d%d%d)%-(%d%d)%-(%d%d)$")
  if not y then
    return nil
  end
  y, m, d = tonumber(y), tonumber(m), tonumber(d)
  if m < 1 or m > 12 or d < 1 then
    return nil
  end
  local leap = (y % 4 == 0 and y % 100 ~= 0) or y % 400 == 0
  local days = { 31, leap and 29 or 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 }
  if d > days[m] then
    return nil
  end
  return y * 10000 + m * 100 + d
end

---O dia de hoje, no relógio local, como `AAAA-MM-DD`. O painel e a regra de
---quem está adiada leem o dia por aqui: a task volta à meia-noite local, e o
---cabeçalho do `CURRENT.md` tem de concordar com o filtro que o montou.
---@return string
function M.today()
  return os.date("%Y-%m-%d")
end

---Verdadeiro se a task está ADIADA em `today`: tem `volta-em` e a data é de
---um dia depois. `volta-em` é o primeiro dia visível, então no próprio dia a
---task já aparece. Data inválida NÃO adia — a task fica à vista e o doctor
---acusa: o erro tem de aparecer, não sumir com a task. Depois da data o campo
---fica no arquivo sem efeito.
---@param meta freitask.Meta|nil
---@param today string `AAAA-MM-DD`
---@return boolean
function M.is_postponed(meta, today)
  local back = meta and M.parse_date(meta.volta_em)
  local now = M.parse_date(today)
  return back ~= nil and now ~= nil and back > now
end

---Escreve a data de volta no frontmatter, ou, com `false`/nil, REMOVE a chave
---(e o bloco inteiro, se ele esvazia). Só mexe nessa chave: a prioridade e o
---que não é do freitask ficam como estão. `value` já vem validado (ver
---`task.check_volta_em`). Muta `lines` in place; quem chama grava.
---@param lines string[]
---@param value string|false|nil
---@return boolean changed
function M.write_volta_em(lines, value)
  return md.set_frontmatter(lines, { [M.RETURN_KEY] = value or false })
end

return M
