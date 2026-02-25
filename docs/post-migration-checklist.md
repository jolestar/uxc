# Post-Migration Checklist (holon-run/uxc)

This checklist is for the repository after moving from `jolestar/uxc` to `holon-run/uxc`.

Baseline decision for Homebrew in this plan:
- Use shared tap: `holon-run/homebrew-tap`
- Formula path: `Formula/uxc.rb`

## P0 - Must Finish Before Next Release

### 1. Repository metadata and public links
- [x] Update `Cargo.toml` repository/homepage to `https://github.com/holon-run/uxc` (`Cargo.toml`).
- [x] Update README badges and links from `jolestar/uxc` to `holon-run/uxc` (`README.md`).
- [x] Update README install examples to new org paths (`README.md`).

### 2. Release and install defaults
- [x] Change installer default repo from `jolestar/uxc` to `holon-run/uxc` (`scripts/install.sh`).
- [x] Change release workflow tap repo from `jolestar/homebrew-uxc` to `holon-run/homebrew-tap` (`.github/workflows/release.yml`).
- [x] Update tap update script default tap repo to `holon-run/homebrew-tap` (`scripts/update-homebrew-formula.sh`).
- [x] Update Homebrew formula homepage template to `https://github.com/holon-run/uxc` (`scripts/update-homebrew-formula.sh`).
- [x] Switch local git remote `origin` to `git@github.com:holon-run/uxc.git`.

### 3. Release documentation
- [x] Update release guide references from `jolestar/homebrew-uxc` to `holon-run/homebrew-tap` (`docs/release.md`).
- [x] Confirm release docs use the migrated repository URLs (`docs/release.md`, `README.md`).

### 4. Secrets and permissions in new repo
- [ ] Configure `CARGO_REGISTRY_TOKEN` in `holon-run/uxc` repo secrets.
- [ ] Configure `HOMEBREW_TAP_GITHUB_TOKEN` in `holon-run/uxc` repo secrets.
- [ ] Ensure `HOMEBREW_TAP_GITHUB_TOKEN` has write access to `holon-run/homebrew-tap`.
- [ ] Verify `release.yml` has required `contents: write` permissions for release asset publishing.

### 5. Validation before tag
- [x] Run local release checks: `./scripts/release-check.sh vX.Y.Z`.
- [ ] Trigger CI on `main` and verify `CI`, `Coverage`, and `E2E Smoke Tests` are green.
- [ ] Do a release dry run by validating generated artifacts and checksum naming.

## P1 - Should Finish Soon After Release

### 1. Non-blocking hardcoded references
- [x] Update issue tooling default repo (`scripts/create_issues.py`).
- [x] Update E2E example payload repo name from `jolestar/uxc` to `holon-run/uxc` (`.github/workflows/e2e-smoke.yml`).

### 2. Crates ownership continuity
- [ ] Confirm `uxc` crate owners include maintainers under the new org workflow (`cargo owner --list uxc`).
- [ ] If needed, add maintainers so publish is not tied to one personal account.

### 3. Operational guardrails
- [ ] Confirm branch protection/rulesets on `holon-run/uxc` match previous expectations.
- [ ] Confirm release tag permission model for maintainers in the org.

## Suggested Execution Order

1. Apply all P0 file updates.
2. Configure secrets and token permissions.
3. Run local checks and CI.
4. Tag and publish.
5. Clean up P1 references in a follow-up PR.
