# Versioning Setup Guide

## What This Is

Drop-in instructions for Claude Code (or any AI agent) to set up automatic versioning in a project repo. Uses Semantic Versioning (SemVer), Conventional Commits, and Release Please.

---

## Quick Reference

### Version Format

`MAJOR.MINOR.PATCH` — e.g. `1.4.2`

| Bump  | When                          | Example commit            |
|-------|-------------------------------|---------------------------|
| MAJOR | Breaking change               | `feat!: redesign auth`    |
| MINOR | New feature, no breakage      | `feat: add export button` |
| PATCH | Bug fix, no new features      | `fix: correct date parse` |
| None  | Docs, chores, tests, refactor | `chore: update deps`      |

Each bump resets numbers to its right: `1.4.2` → `2.0.0` or `1.4.2` → `1.5.0`.

### Commit Prefixes

```
feat:      New feature (MINOR)
fix:       Bug fix (PATCH)
feat!:     Breaking feature (MAJOR)
fix!:      Breaking fix (MAJOR)
docs:      Documentation only
chore:     Maintenance, deps
refactor:  Code restructure, no behaviour change
perf:      Performance improvement
test:      Adding or fixing tests
ci:        CI/CD config changes
style:     Formatting, no logic change
```

Add `!` after prefix for breaking changes. Add scope in brackets if useful: `feat(api): add rate limiting`.

---

## Setup Steps

### 1. Create Release Please Config

Create `.release-please-config.json` in the repo root:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "rust",
  "include-component-in-tag": false,
  "packages": {
    ".": {
      "changelog-path": "CHANGELOG.md",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true
    }
  }
}
```

**For Svelte frontend in the same repo**, change `packages` to:

```json
"packages": {
  ".": {
    "release-type": "rust",
    "changelog-path": "CHANGELOG.md"
  },
  "frontend": {
    "release-type": "node",
    "changelog-path": "frontend/CHANGELOG.md"
  }
}
```

Set `"include-component-in-tag": true` when using multiple packages.

### 2. Create Release Please Manifest

Create `.release-please-manifest.json` in the repo root:

```json
{
  ".": "0.1.0"
}
```

This tracks the current version. Release Please updates it automatically. Set the starting version to match your `Cargo.toml` or `package.json`.

### 3. Create the GitHub Action

Create `.github/workflows/release-please.yml`:

```yaml
name: Release Please

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### 4. Create Commit Lint Config

Create `commitlint.config.js` in the repo root:

```js
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      [
        'feat', 'fix', 'docs', 'chore',
        'refactor', 'perf', 'test', 'ci', 'style'
      ]
    ],
    'subject-empty': [2, 'never'],
    'type-empty': [2, 'never']
  }
};
```

### 5. Create the Commit Lint GitHub Action

Create `.github/workflows/commitlint.yml`:

```yaml
name: Commit Lint

on:
  pull_request:
    branches:
      - main

jobs:
  commitlint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - run: npm install @commitlint/cli @commitlint/config-conventional

      - run: npx commitlint --from ${{ github.event.pull_request.base.sha }} --to ${{ github.event.pull_request.head.sha }}
```

### 6. Set GitHub Repo Settings

In the repo settings on GitHub:

- **Settings → General → Pull Requests**: tick "Allow squash merging", untick the others. Set default commit message to "Pull request title".
- **Settings → Branches → Branch protection rule** for `main`:
  - Tick "Require a pull request before merging"
  - Tick "Require status checks to pass before merging"
  - Add `commitlint` as a required status check

This means only the PR title needs to follow the commit format. Squash merge keeps the history clean.

---

## How It Works End-to-End

1. Dev creates a branch, makes commits (any format is fine).
2. Dev opens a PR. The PR title MUST follow conventional format: `feat: add user export`.
3. Commitlint action checks the PR title. Blocks merge if wrong.
4. PR merges to `main` via squash merge. The squashed commit uses the PR title.
5. Release Please action runs. It reads all new commits since last release.
6. It opens (or updates) a Release PR with bumped version and updated CHANGELOG.
7. When the Release PR merges, it creates a GitHub Release and git tag.

No humans needed after step 2.

---

## File Checklist

When setting up a new repo, check these files exist:

- [ ] `.release-please-config.json`
- [ ] `.release-please-manifest.json`
- [ ] `.github/workflows/release-please.yml`
- [ ] `commitlint.config.js`
- [ ] `.github/workflows/commitlint.yml`
- [ ] `CHANGELOG.md` (can be empty, Release Please will populate it)
- [ ] GitHub repo settings: squash merge only, branch protection on `main`

---

## Agent Instructions

**If this file is present in a project**, the versioning setup is documented. Check each item in the File Checklist above. Create any missing files using the configs shown. Do not overwrite files that already exist unless the user asks.

**If this file is NOT present in a project**, copy it to the repo root and then create all files in the checklist.

**Commit message for setup**: `ci: add versioning automation with Release Please and commitlint`

**PR title for setup**: `ci: add versioning automation`
