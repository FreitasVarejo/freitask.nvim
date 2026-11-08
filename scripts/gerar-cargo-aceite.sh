#!/usr/bin/env bash
# Regenera tests/aceite/Cargo.toml a partir das pastas de fatia presentes: um
# alvo `[[test]]` por tests/aceite/<fatia>/main.rs, mais `dores.rs` e os
# schemas quando existirem. Chamado só por scripts/congelar.sh.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

out=tests/aceite/Cargo.toml
{
  cat <<'EOF'
# GERADO por scripts/gerar-cargo-aceite.sh a cada `make congelar`. Não edite.
[package]
name = "aceite"
description = "Testes de aceitação congelados do freitask (spec §3.3 e §4)"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
autotests = false
autobins = false
autoexamples = false
autobenches = false

[lib]
path = "lib.rs"

[dependencies]

[dev-dependencies]
assert_cmd = { workspace = true }
predicates = { workspace = true }
tempfile = { workspace = true }
serde_json = { workspace = true }
jsonschema = { workspace = true }
sha2 = { workspace = true }
toml = { workspace = true }
gerador = { workspace = true }

[lints.clippy]
unwrap_used = "allow"
expect_used = "allow"
EOF
  for main in tests/aceite/*/main.rs; do
    [[ -f "$main" ]] || continue
    fatia=$(basename "$(dirname "$main")")
    [[ "$fatia" == "schema" || "$fatia" == "target" ]] && continue
    printf '\n[[test]]\nname = "%s"\npath = "%s/main.rs"\n' "$fatia" "$fatia"
  done
  if [[ -f tests/aceite/dores.rs ]]; then
    printf '\n[[test]]\nname = "dores"\npath = "dores.rs"\n'
  fi
} >"$out"

# O pacote precisa de um lib.rs; é um arquivo vazio e fixo, parte do hash.
[[ -f tests/aceite/lib.rs ]] || printf '//! Pacote dos testes de aceitação congelados; não tem código de biblioteca.\n' >tests/aceite/lib.rs
echo "gerado: $out"
