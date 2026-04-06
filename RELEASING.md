# Releasing djvu-rs

Releases are automated via [release-please](https://github.com/googleapis/release-please).
You do **not** need to manually edit `CHANGELOG.md`, bump `Cargo.toml`, or create tags.

## How it works

1. **Merge PRs to `master`** — use [Conventional Commits](#conventional-commits) in every commit
   message so release-please can determine the correct version bump.

2. **release-please opens a Release PR automatically** — after each push to `master` it creates
   (or updates) a PR titled `chore(main): release X.Y.Z` containing:
   - `Cargo.toml` version bump
   - `CHANGELOG.md` update (new section with all changes since last release)

3. **Merge the Release PR when ready** — this is the only manual step. release-please then:
   - Creates the `vX.Y.Z` git tag
   - Creates a GitHub Release with the changelog notes

4. **CI publishes to crates.io** — `.github/workflows/publish.yml` triggers on the new tag,
   runs tests, and runs `cargo publish`.

## Conventional Commits

Every commit message must start with a type prefix. release-please reads these to decide
the version bump:

| Commit prefix | Version bump | Example |
|---------------|-------------|---------|
| `fix:` | patch | `fix: clamp overflow in IW44 normalize` |
| `perf:` | patch | `perf(iw44): SIMD YCbCr→RGB` |
| `docs:` | patch | `docs: add Rotation variants` |
| `chore:` | none | `chore: update CI cache` |
| `feat:` | minor | `feat: async render API` |
| `feat!:` or `BREAKING CHANGE:` in footer | major | `feat!: remove deprecated render_to_size` |

**While version is `0.x`:** `feat!` bumps minor (not major) — configured via
`bump-minor-pre-major: true` in `release-please-config.json`.

Full spec: [conventionalcommits.org](https://www.conventionalcommits.org/en/v1.0.0/)

## Version policy

Follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

| Change | Version bump |
|--------|-------------|
| Breaking public API change | MAJOR (`feat!` / `BREAKING CHANGE`) |
| New public API, backward-compatible | MINOR (`feat`) |
| Bug fix, performance, docs, internal | PATCH (`fix`, `perf`, `docs`, `refactor`) |

While version is `0.x`, minor bumps may include breaking changes per SemVer §4.

## Emergency / manual release

If you need to release outside the normal flow:

```sh
# 1. Edit Cargo.toml and CHANGELOG.md manually
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release vX.Y.Z"

# 2. Tag and push
git tag vX.Y.Z
git push origin master --tags

# 3. Create the GitHub Release manually (publish.yml does not do this for manual tags)
gh release create vX.Y.Z --title "vX.Y.Z" --notes-file /tmp/release-notes.md
```

CI will pick up the tag and publish to crates.io. Note that `publish.yml` skips
the publish step automatically if the version already exists on crates.io, so
pushing a tag for a version you already published manually is safe.

> **Important:** manual tags do not create a GitHub Release automatically.
> Always run `gh release create` after a manual tag, otherwise the GitHub
> Releases page will be out of sync with crates.io.
