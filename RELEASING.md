# Releasing djvu-rs

One tag → two things happen automatically:
- crates.io publish (`cargo publish`)
- GitHub Release created with notes from `CHANGELOG.md`

## Checklist

```
[ ] 1. Update CHANGELOG.md
       - Replace `## [Unreleased]` with `## [X.Y.Z] — YYYY-MM-DD`
       - Add a new empty `## [Unreleased]` section above it
       - Update the comparison links at the bottom

[ ] 2. Bump version in Cargo.toml
       version = "X.Y.Z"

[ ] 3. Commit
       git add Cargo.toml CHANGELOG.md
       git commit -m "chore: release vX.Y.Z"

[ ] 4. Tag and push
       git tag vX.Y.Z
       git push origin master --tags

[ ] 5. CI does the rest
       - Verifies Cargo.toml version matches the tag
       - Runs tests
       - cargo publish → crates.io
       - gh release create → GitHub Release (notes from CHANGELOG)
```

## Version policy

Follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

| Change | Version bump |
|--------|-------------|
| Breaking API change | MAJOR (X) |
| New public API, no breakage | MINOR (Y) |
| Bug fix, docs, perf, internal refactor | PATCH (Z) |

While version is `0.x`, minor bumps may include breaking changes.
