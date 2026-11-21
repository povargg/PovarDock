# Maintaining PovarDock

The `povardock` branch carries PovarDock. Releases pair `povardock-x64.exe` with
`SHA256SUMS.txt`. See [Build from source](README.md#build-from-source) for the
current package and tag commands.

PovarDock has its own roadmap and release process. Contributors can find build
and testing instructions in [CONTRIBUTING.md](CONTRIBUTING.md).

## Reviewing pull requests

- **Read the diff before you approve CI** for a first-time contributor. GitHub holds their workflows until a maintainer approves them.
- **CI must pass.** Nothing merges with a failing or missing check.
- **A build with the change must be tested** on a real desktop before merging. The author's test counts if the pull request names the build and setup. Otherwise, test it yourself, for example on Nightly.
- **Check the template is filled in:** why, how it was verified, what wasn't tested, `Fixes #`, and a changelog entry or the `skip-changelog` label.
- **Check the changelog entry's reference** before merging: the pull request number, `by @author` for contributors, and `fixes #` for the issue, as in `(#34, by @RayTrunk, fixes #33)`. Add or correct it yourself if it's missing.
- **Ask for changes rather than making them.** Ask the author to rebase or fix things. For small fixes, you may push your own commit to their branch, without rewriting theirs.

## Merging

- **Rebase and merge only**, so history stays linear and every commit message survives.
- Delete the branch after merging.
- Close the linked issue through `Fixes #` in the pull request.

## Issues

- Every bug needs steps to reproduce it and a Windows version. Label it `needs-repro` until it has them.
- Close every issue with a reason: fixed by a pull request, won't fix, or duplicate.
- Use the `enhancement` label for proposals to extend PovarDock.

### Labels

| Label | For |
| --- | --- |
| `bug` | Something broken |
| `enhancement` | A new feature or an improvement |
| `performance` | Speed, memory or CPU |
| `documentation` | Docs only |
| `windows-10` | Specific to Windows 10 |
| `needs-repro` | A bug without steps to reproduce it yet |
| `good first issue` | Small and well understood |
| `release` | A version bump; left out of release notes |
| `skip-changelog` | Nothing users notice; left out of release notes |

Release notes group pull requests by these labels ([`.github/release.yml`](.github/release.yml)).

## Releases

PovarDock follows [semantic versioning](https://semver.org) while the version is 0.x.

- **Patch** (0.5.1) for fixes only.
- **Minor** (0.6.0) for new features or changed behavior.

To release:

1. Open a release pull request that renames the `Unreleased` section of [CHANGELOG.md](CHANGELOG.md) to the version and date, adds a new empty `Unreleased` section, and bumps the version in `Cargo.toml` and `settings/Cargo.toml`. Label it `release`.
2. Merge it once CI passes.
3. Tag the tested commit on `povardock` and push the tag. Use the version you are releasing in place of `vX.Y.Z`.

   ```sh
   git tag -a vX.Y.Z -m "PovarDock vX.Y.Z"
   git push origin vX.Y.Z
   ```

4. The [release workflow](.github/workflows/release.yml) runs the tests, builds the portable executable and publishes the release with its checksum manifest. Settings uses the latest stable release to offer updates.

Never move or delete a published tag. If a release is broken, ship a patch release.

## Housekeeping

- Delete merged branches.
- Close stale pull requests after asking the author.
