-- freitask.links — reescrita dos wikilinks do vault.
--
-- Arquivar, desarquivar, mudar de projeto e renomear o id são o MESMO evento —
-- "esse arquivo mudou de endereço" — e por isso passam todos por
-- M.retarget_links. Encerrar e reabrir um projeto são o mesmo evento para uma
-- PASTA inteira, e passam por M.retarget_prefix, que usa a mesma varredura.
--
-- O Obsidian resolve `[[foo]]` pelo BASENAME, em qualquer pasta do vault. Daí
-- a assimetria que a tabela abaixo resume, e que explica por que arquivar é
-- barato do ponto de vista de referências:
--
--   operação              [[foo]]      [[projects/p/tasks/foo|foo]]   frontmatter id:
--   arquivar/desarquivar  intacto      reescrever            intacto
--   mudar de projeto      intacto      reescrever            intacto
--   encerrar/reabrir      intacto      reescrever            intacto
--   renomear id           reescrever   reescrever            reescrever

local C = require("freitask.config")
local fs = require("freitask.fs")

local M = {}

---Arquivos markdown do vault elegíveis a conter referências.
---`daily/` fica DE FORA de propósito: são snapshots de como o board
---estava naquele dia; reescrevê-los falsificaria o histórico. `CURRENT.md`
---também: é regenerado do zero, basta chamar M.rebuild_current depois.
---@return string[]
function M.ref_candidates()
  local out = {}
  for _, p in ipairs(fs.vault_notes()) do
    if not p:match("^" .. vim.pesc(C.daily) .. "/") and p ~= C.current then
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
function M.rewrite_link(inner, old, new)
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

---Reescreve um wikilink `inner` cujo alvo começa por `old_prefix` (o caminho
---vault-relative de uma pasta, com a barra final), trocando só o prefixo.
---Devolve nil quando o alvo não começa por ele. Puro.
---
---Só o começo do miolo é comparado: o prefixo não tem `|`, `#` nem `^`, então
---começar o miolo por ele é o mesmo que começar o alvo. O resto vai verbatim —
---alias, sufixo `#`/`^`, extensão de anexo e o `\|` que escapa o alias dentro
---de uma tabela. Link curto (só o nome) não começa por pasta e fica como está:
---o Obsidian o resolve pelo basename, que a mudança de pasta não toca.
---@param inner string
---@param old_prefix string ex.: `projects/p/`
---@param new_prefix string ex.: `projects/_encerrados/p/`
---@return string|nil
function M.rewrite_prefix(inner, old_prefix, new_prefix)
  local lead, rest = inner:match("^(%s*)(.*)$")
  if rest:sub(1, #old_prefix) ~= old_prefix then
    return nil
  end
  return lead .. new_prefix .. rest:sub(#old_prefix + 1)
end

---A varredura que as duas reescritas têm em comum: cada wikilink de cada nota
---elegível (M.ref_candidates) passa por `rewrite`, que devolve o miolo novo ou
---nil para manter o original. Buffers abertos COM alterações não salvas são
---pulados (e reportados) em vez de sobrescritos — mesma postura do
---rebuild_current diante de um CURRENT.md sujo.
---@param rewrite fun(inner: string): string|nil
---@param skip table<string, boolean> caminhos que não se reescrevem
---@return integer refs, integer files, string[] skipped
local function rewrite_vault(rewrite, skip)
  local refs, files, skipped = 0, 0, {}
  for _, path in ipairs(M.ref_candidates()) do
    if not skip[path] then
      local buf = fs.loaded_buf(path)
      local lines = buf and vim.api.nvim_buf_get_lines(buf, 0, -1, false)
        or fs.read_lines(path)
      local hits = 0
      for i, line in ipairs(lines) do
        local n = 0
        -- gsub com função: o retorno é usado verbatim, então um alias com `%`
        -- não vira referência de captura.
        local replaced = line:gsub("%[%[(.-)%]%]", function(inner)
          local nl = rewrite(inner)
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
            fs.reload_buf(buf)
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

---Reescreve, em todo o vault, os wikilinks que apontavam para `old_path` para
---que apontem para `new_path`. Cobre `[[alvo]]`, `[[alvo|alias]]`, embeds
---`![[...]]` e sufixos `#heading`/`^bloco`, nas formas curta e path-qualified.
---O próprio arquivo movido fica de fora: quem o move já reescreveu a linha 2.
---@param old_path string
---@param new_path string
---@return integer refs, integer files, string[] skipped
function M.retarget_links(old_path, new_path)
  local function descr(p)
    local rel = p:sub(#C.vault + 2):gsub("%.md$", "")
    return { id = vim.fn.fnamemodify(p, ":t:r"), full = rel }
  end
  local old, new = descr(old_path), descr(new_path)
  if old.full == new.full then
    return 0, 0, {}
  end
  return rewrite_vault(function(inner)
    return M.rewrite_link(inner, old, new)
  end, { [old_path] = true, [new_path] = true })
end

---Reescreve, em todo o vault, os wikilinks path-qualified que começam por
---`old_prefix` para que comecem por `new_prefix` — o retarget de uma PASTA
---inteira, que é o que encerrar e reabrir um projeto fazem. Nada fica de fora
---além de daily/ e CURRENT.md: as notas da própria pasta, já no endereço novo,
---também se reescrevem, e é assim que a linha 2 das tasks dela acompanha.
---@param old_prefix string caminho vault-relative da pasta, com a barra final
---@param new_prefix string
---@return integer refs, integer files, string[] skipped
function M.retarget_prefix(old_prefix, new_prefix)
  if old_prefix == new_prefix then
    return 0, 0, {}
  end
  return rewrite_vault(function(inner)
    return M.rewrite_prefix(inner, old_prefix, new_prefix)
  end, {})
end

return M
