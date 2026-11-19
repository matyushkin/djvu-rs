# Releasing djvu-rs

Releases are cut by **merging the release-please PR**. Nothing else is manual: the PR's
checks start on their own, and everything after the merge is automatic. No personal
access token is on the release path.

## Standard release procedure

1. **Review the release PR** — release-please keeps one open, titled
   `chore(main): release X.Y.Z`. It bumps the workspace `version` in `Cargo.toml`,
   the Python and npm package versions, `.release-please-manifest.json`, and adds the
   `CHANGELOG.md` section built from Conventional Commits since the last release.

2. **Merge.** Enable auto-merge; it merges when the required checks are green:

   ```sh
   gh pr merge <N> --squash --auto
   ```

   The PR is opened and updated with a token of the **djvu-rs release** GitHub App, so
   its `pull_request` checks start like on any other PR. A PR pushed by `GITHUB_TOKEN`
   gets its runs held for manual approval instead, and its required checks stay
   "Expected" (0.32.1–0.38.0 needed an admin bypass for that reason). Dispatching
   `ci.yml` on the release branch does not help: checks from a `workflow_dispatch` run
   are not linked to the PR (#800).

   The App lives in the repository settings as the `RELEASE_APP_CLIENT_ID` variable
   and the `RELEASE_APP_PRIVATE_KEY` secret. If the release PR stops getting checks,
   check that the App is still installed on this repository and that the key is
   valid; a new key is made on the App's settings page.

3. **The rest happens on its own.** Merging pushes a `chore(main): release X.Y.Z` commit
   to `main`, which starts `release-please.yml` again. That run:
   - creates the `vX.Y.Z` tag and the GitHub Release;
   - publishes all eight crates to crates.io in dependency order, skipping any version
     already there, then waits for each to go live;
   - starts [`.github/workflows/publish-packages.yml`](.github/workflows/publish-packages.yml)
     through `workflow_dispatch` for the Python wheels and the npm package. The tag push
     itself cannot start it, because that push is also authored by `GITHUB_TOKEN`.

4. **Check the result** — the tag, the release notes, the eight crates on crates.io, and
   the package run:

   ```sh
   gh release view vX.Y.Z
   gh run list --workflow=publish-packages.yml --limit 1
   ```

## Emergency release without release-please

This is also the path for a fix release of an older line (see below).

`publish.yml` still listens for `push: tags: ['v*']` and for a manual
`workflow_dispatch` with a `tag` input. Write the version-bump commit by hand, merge it,
then:

```sh
git tag -a vX.Y.Z <release-commit-sha> -m "Release X.Y.Z"
git push --no-verify origin vX.Y.Z
```

`--no-verify` skips the pre-push hook (a full `make check`); the commit is already on
`main` and green, so re-running it on a tag push is wasted minutes. A tag pushed by a
person — not by `GITHUB_TOKEN` — does start both publish workflows.

## Release candidates (`X.Y.Z-rc.N`)

A release candidate is a test release of a major version, for users who opt
in. Neither Cargo (`djvu-rs = "1"`), pip nor `npm install djvu-rs` selects it
unless asked by name: `djvu-rs = "=1.0.0-rc.1"`, `pip install --pre djvu-rs`,
`npm install djvu-rs@next`.

1. **Set the version.** In `release-please-config.json`, set
   `"release-as": "1.0.0-rc.1"` on the `.` package. release-please then
   titles the release PR `chore(main): release 1.0.0-rc.1`.
2. **Merge the release PR** as in the standard procedure. The publish steps
   handle the prerelease part of the version:
   - `release-please.yml` marks the GitHub Release as a pre-release, so the
     newest stable release stays "Latest";
   - `publish-packages.yml` publishes the npm package under the `next` tag,
     so `latest` does not move. PyPI and crates.io need no flag: they
     recognize the prerelease part themselves (PyPI shows `1.0.0rc1`).
3. **Set the next version at once.** `release-as` stays in force until it is
   changed, so the next release PR would offer the same version again. In
   the first PR after the release, set it to `1.0.0-rc.2` (a fix is needed)
   or `1.0.0` (the candidate is good).
4. **After the final release**, remove `release-as`. From then on,
   Conventional Commits pick the version again.

A candidate may come out before the deprecation window ends, because nobody
gets it without asking for it by name. The final release keeps to the date:
for 1.0.0, no earlier than 2026-12-30.

## Fix releases of an older line (`0.46.x`)

[`SECURITY.md`](SECURITY.md) lists which older lines get fixes. release-please
works only on `main`, so a fix release of an older line follows the
emergency procedure above, from its own branch:

1. Create the branch from the last tag of the line, once, when the first fix
   is needed: `git switch -c release/0.46 v0.46.0`.
2. As the first commit on the branch, copy the "Choose the npm tag" step
   and the `--tag "$DIST_TAG"` flag of the "Publish" step from `main`'s
   `.github/workflows/publish-packages.yml`. The tag push runs the workflow
   file of the tagged commit. Without that step, npm would move `latest` to
   the older line; with it, an older line goes under the `legacy` tag.
3. Cherry-pick the fix from `main` (`git cherry-pick -x <sha>`), or write it
   on the branch when `main` no longer has the code. Add no features.
4. Raise the patch version in the root `Cargo.toml`, in the workspace crates
   it lists, in `djvu-py/Cargo.toml` and in `.release-please-manifest.json`,
   and add a `CHANGELOG.md` section. Run `make check`.
5. Push the branch, then tag the commit and push the tag as in the emergency
   procedure. Create the GitHub Release with `gh release create v0.46.1`.
   Once 1.0.0 is out, add `--latest=false`, so the `1.x` release stays
   "Latest".

## Conventional Commits

Every commit message must start with a type prefix. release-please reads these to decide
the version bump when it prepares the release PR:

| Commit prefix | Version bump | Example |
|---------------|-------------|---------|
| `fix:` | patch | `fix: clamp overflow in IW44 normalize` |
| `perf:` | patch | `perf(iw44): SIMD YCbCr→RGB` |
| `docs:` | patch | `docs: add Rotation variants` |
| `chore:` | none | `chore: update CI cache` |
| `feat:` | minor | `feat: async render API` |
| `feat!:` or `BREAKING CHANGE:` in footer | major | `feat!: remove deprecated render_to_size` |

A major release also needs the deprecation window of
[`docs/api-compatibility.md`](docs/api-compatibility.md) §1 to have passed for
every item it removes. Do not merge the release PR of a major release before
that date.

Full spec: [conventionalcommits.org](https://www.conventionalcommits.org/en/v1.0.0/)

## Version policy

Follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

| Change | Version bump |
|--------|-------------|
| Breaking public API change | MAJOR (`feat!` / `BREAKING CHANGE`) |
| New public API, backward-compatible | MINOR (`feat`) |
| Bug fix, performance, docs, internal | PATCH (`fix`, `perf`, `docs`, `refactor`) |

A new field or variant on a type marked `#[non_exhaustive]` and an MSRV raise
are MINOR ([`docs/api-compatibility.md`](docs/api-compatibility.md) §2 and §3).

## Tokens

| Secret | Used by | Notes |
|--------|---------|-------|
| `CARGO_REGISTRY_TOKEN` | `release-please.yml` and `publish.yml` (`cargo publish`) | **Required** — crates.io cannot publish without it. Both jobs run in the `crates-io` environment, so the value they read is the **environment** secret (Settings → Environments → `crates-io`); it overrides a repository secret of the same name. The token is made at <https://crates.io/settings/tokens> with the scopes `publish-new` and `publish-update` for the crates `djvu*`. When it expires or is revoked, `cargo publish` fails with `403 Forbidden: authentication failed`, as in the 0.46.0 release: replace the environment secret and re-run the failed job. |
| `GITHUB_TOKEN` | `release-please.yml` | Built in, nothing to rotate. It opens the release PR, tags, releases, publishes to crates.io, and dispatches the other two workflows. A `RELEASE_PLEASE_TOKEN` PAT was used once, expired, and silently broke the 0.25.0 release with `Bad credentials`; it is no longer used. |

## Python wheels and npm packages

[`.github/workflows/publish-packages.yml`](.github/workflows/publish-packages.yml),
started by the release run (see step 3), builds version-matched Python wheels/sdists and the dual wasm npm
package, runs install-time smoke tests, writes `SHA256SUMS`, and attests
artifacts. Publishing to PyPI/npm is gated on repository variables
`PUBLISH_PYPI` / `PUBLISH_NPM` (set to `true`) plus the `pypi` / `npm`
environments — if a wheel, sdist, npm tarball, or smoke test fails, the
package gate fails and nothing is published. Full contract:
[`docs/packaging.md`](docs/packaging.md).

