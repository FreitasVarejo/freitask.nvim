-- freitask.meta — o eixo de EXECUÇÃO de uma task: quem está mexendo, desde
-- quando, sobre qual domínio.
--
-- É ortogonal ao status. O status (o callout) diz em que FASE o trabalho está;
-- estes três campos dizem QUEM o ocupa. Os dois eixos existem separados porque
-- respondem a perguntas diferentes: "isto já foi implementado?" é fase, "posso
-- pegar isto agora?" é posse. Juntá-los num vocabulário só daria estados como
-- "em implementação pelo claude-code", que multiplicam sem fim.
--
-- Mora no frontmatter, e não no bloco de callout, por dois motivos: é metadado
-- consultável (o Obsidian indexa propriedade, não texto de blockquote), e o
-- bloco é reserializado inteiro pelo doctor a partir do CAMINHO — posse não é
-- derivável de caminho nenhum, então ficaria exposta a ser regenerada para
-- fora.
--
-- Todos os campos são OPCIONAIS. Ausência de `dono` = task livre. Este é o
-- default deliberado: o custo de uma task sem dono é alguém perguntar, e o de
-- uma task com dono errado é alguém não mexer no que devia.

local md = require("freitask.md")

local M = {}

-- As três chaves do eixo. Em tabela para que doctor, CLI e form varram a mesma
-- lista em vez de repetir os literais — acrescentar um campo é editar aqui.
M.KEYS = { "dono", "desde", "dominio" }

-- Uma garra mais velha que isto é suspeita: o agente que a cravou provavelmente
-- morreu sem soltar. 24h e não 1h porque fechar o notebook à noite e voltar no
-- dia seguinte é trabalho normal, não abandono; e porque o custo de um aviso
-- falso é alto — um doctor em que não se confia é um doctor que não se lê.
M.STALE_SECONDS = 24 * 60 * 60

--- Tempo ----------------------------------------------------------------------

---Dias desde a época civil (algoritmo de Howard Hinnant). Puro, e é o que
---permite converter um ISO-8601 COM offset sem passar por os.time, que
---interpreta a tabela no fuso local e erraria toda vez que o carimbo viesse de
---outra máquina — exatamente o caso de uso (pi01 e notebook escrevem no mesmo
---vault).
---@param y integer
---@param m integer
---@param d integer
---@return integer
local function days_from_civil(y, m, d)
  y = (m <= 2) and (y - 1) or y
  local era = math.floor(y / 400)
  local yoe = y - era * 400
  local mp = (m + 9) % 12
  local doy = math.floor((153 * mp + 2) / 5) + d - 1
  local doe = yoe * 365 + math.floor(yoe / 4) - math.floor(yoe / 100) + doy
  return era * 146097 + doe - 719468
end

---Carimbo ISO-8601 local com offset, ex.: "2026-09-22T14:30:00-03:00".
---@return string
function M.now()
  local stamp = os.date("%Y-%m-%dT%H:%M:%S%z")
  -- %z sai como "-0300"; a ISO aceita, mas com dois-pontos é o que o Obsidian
  -- e o olho humano leem melhor.
  return (stamp:gsub("([+-]%d%d)(%d%d)$", "%1:%2"))
end

---Converte um carimbo ISO-8601 em epoch UTC. nil se não parsear — e nil é
---tratado como "não sei", nunca como "antigo": um carimbo ilegível não deve
---fazer o doctor acusar abandono.
---@param s string|nil
---@return integer|nil
function M.epoch(s)
  if type(s) ~= "string" then
    return nil
  end
  local y, mo, d, h, mi, sec = s:match("^(%d%d%d%d)%-(%d%d)%-(%d%d)T(%d%d):(%d%d):(%d%d)")
  if not y then
    return nil
  end
  local base = days_from_civil(tonumber(y), tonumber(mo), tonumber(d)) * 86400
    + tonumber(h) * 3600
    + tonumber(mi) * 60
    + tonumber(sec)
  local sign, oh, om = s:match("([+-])(%d%d):?(%d%d)$")
  if sign then
    local off = (tonumber(oh) * 3600 + tonumber(om) * 60) * (sign == "-" and -1 or 1)
    return base - off
  end
  if s:sub(-1) == "Z" then
    return base
  end
  return base -- sem offset: assume UTC, que é o menos surpreendente
end

--- Leitura e escrita ----------------------------------------------------------

---Lê o eixo de execução do frontmatter.
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

---Escreve o eixo no frontmatter. Campo ausente de `meta` é REMOVIDO — a tabela
---é o estado desejado inteiro, não um patch, para que soltar uma garra não
---dependa de lembrar de passar `false` em cada chave.
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

---Verdadeiro se a task está reivindicada por alguém.
---@param meta freitask.Meta
---@return boolean
function M.claimed(meta)
  return meta.dono ~= nil
end

---Verdadeiro se a garra existe mas está velha demais para valer. Carimbo
---ausente ou ilegível conta como fantasma: uma garra sem `desde` nunca expira
---sozinha, e uma garra que nunca expira é pior que nenhuma.
---@param meta freitask.Meta
---@param now? integer epoch; default os.time()
---@return boolean
function M.stale(meta, now)
  if not meta.dono then
    return false
  end
  local since = M.epoch(meta.desde)
  if not since then
    return true
  end
  return (now or os.time()) - since > M.STALE_SECONDS
end

---Identidade deste agente: "<maquina>/<ferramenta>". A máquina vem do hostname
---porque é o que distingue notebook, pi01 e WSL do trabalho, que compartilham o
---vault; a ferramenta vem de quem chamou, porque num mesmo host convivem Claude
---Code, Cursor e o Neovim do dono.
---@param tool? string default "nvim"
---@return string
function M.whoami(tool)
  local host = vim.trim(vim.fn.hostname() or "")
  if host == "" then
    host = "desconhecido"
  end
  return host .. "/" .. (tool or vim.env.FREITASK_AGENT or "nvim")
end

return M
