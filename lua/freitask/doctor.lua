-- freitask.doctor — verificação, reparo e migração de formato.
--
-- Existe porque o Neovim NÃO é o único escritor do vault: há o Obsidian (aqui
-- e no celular, via Syncthing) e agentes de IA com acesso a shell. Nenhum
-- deles pode ser obrigado a chamar as funções deste módulo, então a estratégia
-- não é impedir o desvio — é torná-lo BARULHENTO e, quando possível,
-- reparável. Ver docs/freitask.md, seção "freitask doctor".

local C = require("freitask.config")
local board = require("freitask.board")
local cache = require("freitask.cache")
local fs = require("freitask.fs")
local md = require("freitask.md")
local meta_ = require("freitask.meta")
local model_ = require("freitask.model")
local path_ = require("freitask.path")
local status = require("freitask.status")
local task = require("freitask.task")

local M = {}

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

---Migra todos os arquivos de task para o formato atual (`[[projects/<projeto>/tasks/<id>|id]]`
---com caminho + alias, e a linha 3 livre como "nome" em vez de status-texto).
---Cobre tanto o formato legado (`> **Título** - [[id]]` + `> Branch:`) quanto o
---formato intermediário (`> N - Título` na linha 3). Idempotente. Retorna a
---quantidade de arquivos reescritos.
---@return integer
function M.migrate_format()
  status.ensure_root()
  status.load_status()
  local n = 0

  -- Formato legado de arquivamento: `archived: true` no frontmatter. Hoje o
  -- estado é o CAMINHO, então essas tasks são movidas para archived/dropped/
  -- (o balde honesto: "parei de tocar" — o flag antigo não registrava por quê)
  -- e o flag some. Feito ANTES do loop de blocos para que elas já sejam
  -- reserializadas com o link apontando para a pasta nova.
  for _, path in ipairs(vim.fn.glob(C.projects .. "/*/tasks/*.md", true, true)) do
    local project = path_.split_task_path(path)
    if project then
      local lines = fs.read_lines(path)
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
        if task.archive_task(path, "dropped") then
          n = n + 1
        end
      end
    end
  end

  -- Normaliza o bloco de todas as tasks, ativas e arquivadas.
  local paths = vim.fn.glob(C.projects .. "/*/tasks/*.md", true, true)
  vim.list_extend(paths, vim.fn.glob(C.projects .. "/*/tasks/archived/*/*.md", true, true))
  for _, path in ipairs(paths) do
    local project, id, archived = path_.split_task_path(path)
    if project then
      local lines = fs.read_lines(path)
      local s, e, blk = md.first_block(lines)
      if s then
        local model = model_.parse_block(blk)
        if model.id == "" then
          model.id = id -- fallback: id = nome do arquivo
        end
        model.project, model.archived = project, archived
        local newblk = model_.serialize_block(model)
        if not vim.deep_equal(newblk, blk) then
          vim.fn.writefile(md.splice(lines, s, e, newblk), path)
          cache.update_cache_entry(path)
          n = n + 1
        end
      end
    end
  end
  return n
end

---Conjunto de alvos de wikilink que resolvem para algum arquivo do vault, nas
---duas formas que o Obsidian aceita (basename e caminho vault-relative).
---@return table<string, boolean>
---@param notes string[] a varredura do vault, para não repeti-la
local function resolvable_targets(notes)
  local set = {}
  for _, p in ipairs(notes) do
    set[vim.fn.fnamemodify(p, ":t:r")] = true
    set[(p:sub(#C.vault + 2):gsub("%.md$", ""))] = true
  end
  return set
end

---Diagnostica o vault. READ-ONLY por padrão; `opts.fix` repara o que é
---derivável. Devolve uma lista de achados, cada um com `fixed` indicando se
---foi reparado nesta passagem.
---@param opts? { fix?: boolean }
---@return freitask.Finding[]
function M.doctor(opts)
  opts = opts or {}
  status.ensure_root()
  status.load_status()
  local found = {}
  local function add(level, kind, path, msg, fixed)
    found[#found + 1] = {
      level = level,
      kind = kind,
      path = path:sub(#C.vault + 2),
      msg = msg,
      fixed = fixed or false,
    }
  end

  -- 1. Conflitos do Syncthing. Reportados e NUNCA reparados: escolher qual
  -- cópia vale é decisão editorial, e apagar a errada em silêncio perderia
  -- trabalho feito no outro dispositivo.
  for _, p in ipairs(vim.fn.glob(C.projects .. "/*/tasks/**/*.sync-conflict-*.md", true, true)) do
    add("error", "sync-conflict", p, "conflito do Syncthing — compare com o original e resolva à mão")
  end

  -- 2. Arquivos sob projects/<projeto>/tasks/ que a ferramenta não enxerga. São
  -- invisíveis ao board e ao picker, então sumiriam sem barulho nenhum. O resto
  -- de `projects/<projeto>/` (spec, decisoes/, dominio/…) fica fora por
  -- construção: o glob só entra em `tasks/`.
  for _, p in ipairs(vim.fn.glob(C.projects .. "/*/tasks/**/*.md", true, true)) do
    if not path_.split_task_path(p) and not p:match("%.sync%-conflict%-") then
      add(
        "warn",
        "fora-do-padrao",
        p,
        "não casa projects/<projeto>/tasks/[archived/<tipo>/]<id>.md — invisível ao freitask"
      )
    end
  end

  -- 3. Ids duplicados entre projetos: tornam `[[id]]` ambíguo, e é o link curto
  -- que o Obsidian resolve por basename.
  local by_id = {}
  for _, t in ipairs(cache.all_tasks()) do
    by_id[t.id] = by_id[t.id] or {}
    table.insert(by_id[t.id], t.path)
  end
  for id, paths in pairs(by_id) do
    if #paths > 1 then
      add(
        "warn",
        "id-duplicado",
        paths[1],
        string.format("id %q existe em %d arquivos — `[[%s]]` fica ambíguo", id, #paths, id)
      )
    end
  end

  -- 4. Invariantes deriváveis do caminho: bloco e frontmatter. Tudo aqui é
  -- reparável sem adivinhação, porque o caminho é a fonte de verdade.
  for _, t in ipairs(cache.all_tasks()) do
    local lines = fs.read_lines(t.path)
    local s, e, blk = md.first_block(lines)
    if not s then
      add("error", "sem-callout", t.path, "arquivo de task sem bloco de callout")
    else
      local model = model_.parse_block(blk)
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
      local want = model_.serialize_block(model)
      local reserialize = false
      if not vim.deep_equal(want, blk) then
        reserialize = opts.fix
        add("warn", "bloco-dessincronizado", t.path, "bloco não corresponde ao caminho (link/id)", opts.fix)
      end

      -- Callout que contradiz a pasta de arquivamento. Só vale para `done`:
      -- o status `done` significa literalmente
      -- "Arquivada", e um arquivo em archived/done/ dizendo "Não iniciada" é
      -- contradição, não nuance. Em `dropped` e `failed` a fase em que a task
      -- estava quando foi largada é INFORMAÇÃO ("estava bloqueada quando
      -- desisti") e segue preservada de propósito — os dois eixos só se amarram
      -- na ponta em que um deles deixou de ter outra leitura possível.
      -- Avaliado DEPOIS do check acima para que as duas causas não se confundam
      -- no relatório.
      local done_num = status.by_callout()["done"]
      if t.archived == "done" and done_num and model.status_num ~= done_num then
        if opts.fix then
          model.status_num = done_num
          want = model_.serialize_block(model)
          reserialize = true
        end
        local raw = (model.raw_callout ~= "" and model.raw_callout) or "?"
        add("warn", "callout-vs-pasta", t.path, string.format("está em archived/done/ com callout %q", raw), opts.fix)
      end

      if reserialize then
        lines = md.splice(lines, s, e, want)
        dirty = true
      end

      -- Trabalha numa cópia para que o diagnóstico continue read-only quando
      -- não há --fix (update_frontmatter_key muta o array que recebe).
      local probe = vim.deepcopy(lines)
      if md.update_frontmatter_key(probe, "id", t.id) then
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
            md.append_history(lines, t.archived)
            lines[#lines] = lines[#lines] .. " [reconstruído pelo doctor; data real desconhecida]"
            dirty = true
          end
          add("warn", "historico-ausente", t.path, "está em archived/ sem entrada de histórico", opts.fix)
        end
      end

      -- 6. Prioridade: só `alta` conta. Outro valor (`Alta`, `urgente`) viraria
      -- "normal" em silêncio, o mesmo furo que o status 0 fecha para o callout.
      -- Sem --fix: não se adivinha o que o dono quis dizer. Em arquivada não há
      -- o que priorizar.
      if not t.archived then
        local prioridade = meta_.read(lines).prioridade
        if prioridade and prioridade ~= meta_.HIGH then
          add(
            "warn",
            "prioridade-desconhecida",
            t.path,
            string.format("prioridade %q não é `%s` — a task conta como normal", prioridade, meta_.HIGH)
          )
        end
      end

      if dirty then
        vim.fn.writefile(lines, t.path)
      end
    end
  end

  -- 7. Wikilinks path-qualified pendurados: a impressão digital de um rename
  -- feito sem M.retarget_links. NÃO é reparável — nada no vault registra que
  -- `foo` um dia se chamou `bar` —, e é justamente por isso que precisa ser
  -- reportado.
  --
  -- Só links `projects/<p>/tasks/...` com caminho entram aqui. Um `[[nota-futura]]` curto
  -- que não resolve é comportamento NORMAL do Obsidian (o link vira um
  -- placeholder que cria a nota ao ser clicado), e cobrá-lo encheria o doctor
  -- de alarme falso — um checker em que não se confia é um checker que não se
  -- lê. Já um link com caminho para uma pasta de tasks foi gerado por este módulo e
  -- portanto tem obrigação de resolver.
  -- Uma varredura só do vault serve às duas coisas: montar o conjunto de
  -- alvos resolvíveis e procurar os links pendurados. Antes eram dois globs
  -- completos por execução do doctor, sobre o vault inteiro.
  local notes = fs.vault_notes()
  local resolvable = resolvable_targets(notes)
  for _, p in ipairs(notes) do
    if not p:match("^" .. vim.pesc(C.daily) .. "/") then
      local in_fence = false
      for i, line in ipairs(fs.read_lines(p)) do
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
            if target:match("^projects/[^/]+/tasks/") and not resolvable[target] then
              add("error", "link-pendurado", p, string.format("linha %d: `[[%s]]` não resolve", i, target))
            end
          end
        end
      end
    end
  end

  if opts.fix then
    cache.build_cache()
    board.rebuild_current({ quiet = true })
  end
  return found
end

return M
