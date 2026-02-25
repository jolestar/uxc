#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SKILL_DIR="${ROOT_DIR}/skills/context7"
SKILL_FILE="${SKILL_DIR}/SKILL.md"
OPENAI_FILE="${SKILL_DIR}/agents/openai.yaml"

fail() {
  printf '[validate] error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

# Check dependencies
need_cmd rg

required_files=(
  "${SKILL_FILE}"
  "${OPENAI_FILE}"
  "${SKILL_DIR}/references/usage-patterns.md"
)

for file in "${required_files[@]}"; do
  if [[ ! -f "${file}" ]]; then
    echo "missing required file: ${file}"
    exit 1
  fi
done

# Validate SKILL frontmatter minimum fields.
# Require the first line to be '---' and a subsequent closing '---'.
if ! head -n 1 "${SKILL_FILE}" | rg -q '^---$'; then
  echo "SKILL.md must include YAML frontmatter"
  exit 1
fi

if ! tail -n +2 "${SKILL_FILE}" | rg -q '^---$'; then
  echo "SKILL.md must include YAML frontmatter"
  exit 1
fi

if ! rg -q '^name:\s*context7\s*$' "${SKILL_FILE}"; then
  echo "SKILL.md frontmatter must define: name: context7"
  exit 1
fi

if ! rg -q '^description:\s*.+' "${SKILL_FILE}"; then
  echo "SKILL.md frontmatter must define a description"
  exit 1
fi

# Validate required invocation contract appears in SKILL text.
if ! rg -q 'mcp.context7.com/mcp' "${SKILL_FILE}"; then
  echo "SKILL.md must document MCP endpoint"
  exit 1
fi

if ! rg -q 'resolve-library-id' "${SKILL_FILE}"; then
  echo "SKILL.md must document resolve-library-id tool"
  exit 1
fi

if ! rg -q 'query-docs' "${SKILL_FILE}"; then
  echo "SKILL.md must document query-docs tool"
  exit 1
fi

# Validate references linked from SKILL body.
if ! rg -q 'references/usage-patterns.md' "${SKILL_FILE}"; then
  echo "SKILL.md must reference usage-patterns.md"
  exit 1
fi

# Validate openai.yaml minimum fields.
if ! rg -q '^\s*display_name:\s*"Context7"\s*$' "${OPENAI_FILE}"; then
  echo "agents/openai.yaml must define interface.display_name"
  exit 1
fi

if ! rg -q '^\s*short_description:\s*".+"\s*$' "${OPENAI_FILE}"; then
  echo "agents/openai.yaml must define interface.short_description"
  exit 1
fi

if ! rg -q '^\s*default_prompt:\s*".*\$context7.*"\s*$' "${OPENAI_FILE}"; then
  echo 'agents/openai.yaml default_prompt must mention $context7'
  exit 1
fi

echo "skills/context7 validation passed"
