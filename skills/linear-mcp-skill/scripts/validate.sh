#!/bin/bash
# Validate Linear MCP Skill

set -e

echo "Validating Linear MCP Skill..."

# Check required files exist
echo "Checking required files..."
test -f SKILL.md || { echo "ERROR: SKILL.md not found"; exit 1; }
test -f agents/openai.yaml || { echo "ERROR: agents/openai.yaml not found"; exit 1; }
test -f references/usage-patterns.md || { echo "ERROR: references/usage-patterns.md not found"; exit 1; }

# Validate YAML syntax
echo "Validating YAML files..."
python3 -c "import yaml; yaml.safe_load(open('SKILL.md'))" 2>/dev/null || echo "WARNING: SKILL.md YAML may have issues"

# Check skill name
echo "Checking skill name..."
grep -q "^name: linear-mcp-skill" SKILL.md || { echo "ERROR: Invalid skill name"; exit 1; }

echo "Validation passed!"
