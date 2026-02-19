#!/usr/bin/env python3
"""
Script to create GitHub issues for UXC project.

Usage:
    1. Set GH_TOKEN environment variable with your GitHub token
    2. Run: python scripts/create_issues.py
"""

import os
import subprocess
import json

# GitHub repository
REPO = "jolestar/uxc"

# Issues to create
ISSUES = [
    # Infrastructure
    {
        "title": "Setup CI/CD pipeline (GitHub Actions)",
        "body": "## Task\n\n- [ ] Create `.github/workflows/ci.yml`\n- [ ] Add rustfmt check\n- [ ] Add clippy linter\n- [ ] Add unit tests\n- [ ] Add security audit\n- [ ] Test on multiple Rust versions\n\n## Acceptance\n\n- All checks pass on PR\n- Tests run automatically on push\n- Failed builds block merging",
        "labels": ["priority: high", "type: infrastructure", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement OpenAPI schema parser",
        "body": "## Task\n\n- [ ] Parse OpenAPI 3.0 and 3.1 specs\n- [ ] Extract paths, operations, parameters\n- [ ] Support common OpenAPI formats\n- [ ] Handle authentication schemes\n- [ ] Validate schema structure\n\n## Acceptance\n\n- Can parse standard OpenAPI specs\n- Extract all operations\n- Handle errors gracefully",
        "labels": ["priority: critical", "type: feature", "component: openapi", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement protocol detection",
        "body": "## Task\n\n- [ ] Probe OpenAPI endpoints\n- [ ] Attempt gRPC reflection\n- [ ] Try MCP discovery\n- [ ] Attempt GraphQL introspection\n- [ ] Return protocol type or error\n\n## Acceptance\n\n- Auto-detects all 4 protocols\n- Fast detection (< 2s per endpoint)\n- Clear error on unknown protocol",
        "labels": ["priority: critical", "type: feature", "component: cli", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Add OpenAPI operation listing",
        "body": "## Task\n\n- [ ] List all operations from schema\n- [ ] Show operation name and method\n- [ ] Display parameters\n- [ ] Support verbose output\n\n## Acceptance\n\n```bash\nuxc https://api.example.com list\n```\n\nShows all available operations",
        "labels": ["priority: high", "type: feature", "component: openapi", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement `uxc <url> list` command",
        "body": "## Task\n\n- [ ] Parse list command\n- [ ] Detect protocol\n- [ ] Fetch schema\n- [ ] Display operations\n- [ ] Handle errors\n\n## Acceptance\n\n```bash\nuxc https://api.example.com list\n```\n\nLists all available operations",
        "labels": ["priority: high", "type: feature", "component: cli", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement OpenAPI operation execution",
        "body": "## Task\n\n- [ ] Parse operation name\n- [ ] Extract parameters from schema\n- [ ] Make HTTP request\n- [ ] Handle response\n- [ ] Return JSON result\n\n## Acceptance\n\n```bash\nuxc https://api.example.com GET /users id=42\n```\n\nExecutes operation and returns result",
        "labels": ["priority: critical", "type: feature", "component: openapi", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement `uxc <url> <operation> [args]` execution",
        "body": "## Task\n\n- [ ] Parse execution command\n- [ ] Extract operation and args\n- [ ] Call appropriate adapter\n- [ ] Format output\n- [ ] Handle errors\n\n## Acceptance\n\n```bash\nuxc https://api.example.com user.get id=42\n```\n\nExecutes operation",
        "labels": ["priority: critical", "type: feature", "component: cli", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement JSON output envelope",
        "body": "## Task\n\n- [ ] Define output schema\n- [ ] Add ok field\n- [ ] Add protocol/endpoint/operation\n- [ ] Add result data\n- [ ] Add metadata (duration)\n- [ ] Handle error envelope\n\n## Acceptance\n\n```json\n{\n  \"ok\": true,\n  \"protocol\": \"openapi\",\n  \"endpoint\": \"https://api.example.com\",\n  \"operation\": \"GET /users\",\n  \"result\": {...},\n  \"meta\": {\"duration_ms\": 128}\n}\n```",
        "labels": ["priority: critical", "type: feature", "component: cli", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Write getting started guide",
        "body": "## Task\n\n- [ ] Installation instructions\n- [ ] Basic usage examples\n- [ ] OpenAPI example\n- [ ] MCP example\n- [ ] Common troubleshooting\n\n## Sections\n\n1. Installation\n2. First call\n3. Protocol detection\n4. Operation discovery\n5. Execution\n6. Error handling",
        "labels": ["priority: medium", "type: docs", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
    {
        "title": "Implement MCP stdio client",
        "body": "## Task\n\n- [ ] Spawn MCP server process\n- [ ] Initialize MCP session\n- [ ] Handle JSON-RPC messages\n- [ ] Support tool calls\n- [ ] Support resources\n- [ ] Support prompts\n\n## Acceptance\n\n- Can connect to MCP servers\n- List tools\n- Execute tools",
        "labels": ["priority: high", "type: feature", "component: mcp", "phase: 1"],
        "milestone": "Milestone 1: MVP"
    },
]

def create_issue(issue):
    """Create a GitHub issue using gh CLI"""
    title = issue["title"]
    body = issue["body"]
    labels = ",".join(issue["labels"])

    cmd = [
        "gh", "issue", "create",
        "--repo", REPO,
        "--title", title,
        "--body", body,
        "--label", labels
    ]

    if "milestone" in issue:
        cmd.extend(["--milestone", issue["milestone"]])

    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode == 0:
        print(f"✅ Created: {title}")
        return True
    else:
        print(f"❌ Failed: {title}")
        print(result.stderr)
        return False

def main():
    # Check if gh is authenticated
    result = subprocess.run(
        ["gh", "auth", "status"],
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        print("❌ GitHub CLI not authenticated. Run: gh auth login")
        return 1

    print(f"Creating {len(ISSUES)} issues for {REPO}...\n")

    # Create milestones first
    milestones = ["Milestone 1: MVP", "Milestone 2: Multi-Protocol", "Milestone 3: Production Ready"]
    for ms in milestones:
        subprocess.run([
            "gh", "api",
            "repos/{repo}/milestones".format(repo=REPO),
            "--method", "POST",
            "-f", f"title={ms}"
        ], capture_output=True)

    # Create issues
    success = 0
    for issue in ISSUES:
        if create_issue(issue):
            success += 1

    print(f"\n✨ Created {success}/{len(ISSUES)} issues")
    return 0

if __name__ == "__main__":
    exit(main())
