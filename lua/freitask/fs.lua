-- freitask.fs — leitura de arquivo e manipulação de buffer.
--
-- Existe para que "arquivo ausente", "recarregar buffer" e "varrer o vault"
-- signifiquem a mesma coisa em todo o módulo. Cada uma destas funções estava
-- escrita inline em quatro a seis lugares antes da divisão.

local C = require("freitask.config")

local M = {}

---Linhas de um arquivo, ou lista vazia se ele não existe. O par
---`filereadable(p) == 1 and readfile(p) or {}` aparecia em meia dúzia de
---lugares; concentrá-lo garante que "arquivo ausente" signifique a mesma coisa
---em todos eles.
---@param path string
---@return string[]
function M.read_lines(path)
  if vim.fn.filereadable(path) == 0 then
    return {}
  end
  return vim.fn.readfile(path)
end

---Recarrega um buffer a partir do disco, descartando o que estiver nele. Só é
---chamado depois de o arquivo ter sido reescrito por nós, e sempre com o
---buffer sabidamente limpo — quem pode ter alteração pendente checa `modified`
---antes e desiste.
---@param buf integer
function M.reload_buf(buf)
  vim.api.nvim_buf_call(buf, function()
    vim.cmd("silent edit!")
  end)
end

---Reaponta um buffer para o novo caminho do arquivo e o recarrega. `pcall`
---porque nvim_buf_set_name falha se já houver outro buffer com esse nome — e
---nesse caso o rename em si já aconteceu, então abortar seria pior.
---@param buf integer
---@param path string
function M.retarget_buf(buf, path)
  pcall(vim.api.nvim_buf_set_name, buf, path)
  M.reload_buf(buf)
end

---Todos os arquivos markdown do vault. Três lugares precisam desta varredura
---(referências, alvos resolvíveis e o scan de links do doctor) com filtros
---diferentes; o glob em si é o mesmo.
---@return string[]
function M.vault_notes()
  return vim.fn.glob(C.vault .. "/**/*.md", true, true)
end

---Buffer carregado para `path`, ou nil.
---@param path string
---@return integer|nil
function M.loaded_buf(path)
  local b = vim.fn.bufnr(path)
  if b ~= -1 and vim.api.nvim_buf_is_loaded(b) then
    return b
  end
  return nil
end

return M
