# Tools

Tools used to manage monorepo tasks like pre-commit hooks and npm releases.

## How to create a release

The release script (run from the monorepo root) must be used to create and
publish releases. All releases must follow semver versioning standards, e.g.,
`0.20.1`.

Checkout a release branch from latest `main`:

```shell
git checkout main
git pull origin main
git checkout -b release/<version>
```

You can perform a dry run by only specifying the new version for a release:

```shell
npm run release -- --pkg-version <version>
```

Modify the CHANGELOG.md with new changes. Then commit the release:

```shell
npm run release -- --pkg-version <version> --commit
```

Push the branch up for review:

```shell
git push origin release/<version>
```

Once the branch has been merged, update `main` and push the release tag.

```shell
git checkout main
git pull origin main
git tag -am <version> "<version>"
git push origin <version>
```

The [publish workflow](../.github/workflows/publish.yml) builds the tagged
commit, publishes every package to npm and creates the GitHub release. It can
also be run manually from the Actions tab by picking the release tag in "Use
workflow from". Leave "dry run" checked to try it without publishing.

If the workflow can't be used, publish from your machine with Node 22:

```shell
npm run release -- --pkg-version <version> --publish --otp 123456
```

### One-time setup

Each npm package needs a
[trusted publisher](https://docs.npmjs.com/trusted-publishers) for repository
`meta/stylex`, workflow `publish.yml` and environment `npm-publish`. The `fb`
npm user should also be invited to each package.
