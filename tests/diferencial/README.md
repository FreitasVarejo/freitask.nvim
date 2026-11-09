# Diferencial contra o oráculo Lua

`make diff` roda `cargo run -p diferencial -- --novo target/debug/freitask`;
`make diff-oraculo` roda o Lua dos dois lados (prova do harness). Flags:
`--corpus real|sintetico|patologico|todos`, `--cenarios <regex>` (sobre o nome
`<corpus>::<cenario>`), `--parcial` (não falha por entrada não exercitada),
`--manter` (deixa as cópias em disco), `--relatorio <arquivo>`.

O que ele faz, por cenário: copia o corpus para `A/ObsidianVault` e
`B/ObsidianVault` (vault real sem `.obsidian/`, `.stfolder/`, `.stversions/`,
`.trash/`), roda cada passo nos dois lados com `HOME` trocado e o mesmo
ambiente (`LANG=LC_ALL=C.UTF-8`, `TZ` herdado, `FREITASK_TODAY=<hoje>`, cwd no
HOME), compara stdout, stderr e código, e no fim o manifesto
`caminho → sha256` da árvore (diretórios como `dir`). Aborta se o dia mudou.

Normalização: o prefixo do HOME vira `<HOME>`; stdout que os dois lados
decodificam como JSON é comparado como valor, com ponteiro por diferença; os
achados do `doctor` (em `json:/findings` e no texto) se comparam como
multiconjunto, porque o Lua os emite na ordem de `pairs()`: cada achado que só
um lado tem vira `achado só em A: <level> <kind> <path> …` ou `achado só em B:
…`, e o resto se compara como antes.

## `tests/aceite/divergencias.toml`

Toda diferença precisa casar uma entrada. Formato:

```toml
[[divergencia]]
id = "D1"
motivo = "por que o novo diverge"
teste = "link_em_tabela_acompanha_o_archive_o_move_e_o_rename"   # em tests/aceite/dores.rs
cenario = "regex sobre `<corpus>::<cenario>`"
onde = "regex sobre `codigo` | `stdout` | `stderr` | `json:<ponteiro>` | `arquivo:<rel>`"
detalhe = "regex opcional sobre o detalhe (ex.: `só em B`, `A=1 B=0`)"
observavel = true   # false para o que o harness não enxerga (ordem de chaves, mtime)

[[divergencia.tambem]]   # opcional: outra combinação de cenario, onde e detalhe
cenario = "…"
onde = "…"
detalhe = "…"
```

A primeira entrada que casa (cenário, onde e, se houver, detalhe, na cláusula
principal ou numa `tambem`) cobre a diferença. A `tambem` existe porque as três
regex são independentes: uma causa com formas diferentes em cenários
diferentes, numa cláusula só, levaria cada forma para todos os cenários. Na rodada completa (`--corpus todos`, sem `--cenarios`, sem
`--parcial`), entrada que não cobriu nada falha: a lista não acumula entrada
morta. Os nomes dos cenários estão em `src/cenarios.rs` (`comuns` e
`especificos`); os ponteiros JSON seguem o RFC 6901 (`/0/notes`,
`/findings/2/kind`).
