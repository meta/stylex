#!/usr/bin/env node
/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('fs');
const glob = require('glob');
const path = require('path');
const yargs = require('yargs/yargs');
const { execSync, execFileSync } = require('child_process');
const { hideBin } = require('yargs/helpers');

const args = yargs(hideBin(process.argv))
  .option('commit', {
    type: 'boolean',
  })
  .option('dry-run', {
    type: 'boolean',
  })
  .option('otp', {
    type: 'string',
  })
  .option('pkg-version', {
    alias: 'v',
    type: 'string',
    demandOption: true,
  })
  .option('publish', {
    type: 'boolean',
  })
  .option('skip-install', {
    type: 'boolean',
  }).argv;

const commit = args.commit;
const dryRun = args['dry-run'];
const otp = args.otp;
const pkgVersion = args['pkg-version'];
const publish = args.publish;
const skipInstall = args['skip-install'];

const repoRoot = path.join(__dirname, '../..');

const expectedNodeMajor = fs
  .readFileSync(path.join(repoRoot, '.nvmrc'), 'utf8')
  .trim()
  .replace(/^v/, '')
  .split('.')[0];
const nodeMajor = process.versions.node.split('.')[0];
if (nodeMajor !== expectedNodeMajor) {
  console.error(
    `Releases must use Node ${expectedNodeMajor} (see .nvmrc), but this is Node ${process.versions.node}. Run \`nvm use\` and try again.`,
  );
  process.exit(1);
}

console.log(`Creating release version "${pkgVersion}"`);

const packageJsonData = require('../../package.json');

// Collect workspaces and package manifests
const workspacePaths = packageJsonData.workspaces
  .reduce((acc, w) => {
    const resolvedPaths = glob.sync(path.join(repoRoot, w));
    resolvedPaths.forEach((p) => {
      // Remove duplicates and unrelated packages
      if (acc.indexOf(p) === -1) {
        acc.push(p);
      }
    });
    return acc;
  }, [])
  .filter((p) => fs.existsSync(p));

const workspaces = workspacePaths
  .map((dir) => {
    const directory = path.resolve(dir);
    const packageJsonPath = path.join(directory, 'package.json');

    if (!fs.existsSync(packageJsonPath)) {
      console.warn(`Skipping missing package.json: ${packageJsonPath}`);
      return null;
    }

    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

    return { directory, packageJson, packageJsonPath };
  })
  .filter(Boolean);

if (workspaces.length === 0) {
  console.error('No valid packages found. Aborting.');
  process.exit(1);
}

if (publish) {
  const mismatched = workspaces.filter(
    ({ packageJson }) =>
      !packageJson.private && packageJson.version !== pkgVersion,
  );
  if (mismatched.length > 0) {
    console.error(
      `Refusing to publish ${pkgVersion} because these packages have a different version:`,
    );
    mismatched.forEach(({ packageJson }) => {
      console.error(`  ${packageJson.name}@${packageJson.version}`);
    });
    process.exit(1);
  }
}

// update each package version and its dependencies
const workspaceNames = workspaces.map(({ packageJson }) => packageJson.name);
workspaces.forEach(({ packageJson, packageJsonPath }) => {
  packageJson.version = pkgVersion;
  workspaceNames.forEach((name) => {
    if (packageJson.dependencies && packageJson.dependencies[name]) {
      packageJson.dependencies[name] = pkgVersion;
    }
    if (packageJson.devDependencies && packageJson.devDependencies[name]) {
      packageJson.devDependencies[name] = pkgVersion;
    }
    if (
      packageJson.peerDependencies &&
      packageJson.peerDependencies[name] &&
      packageJson.peerDependencies[name] !== '*'
    ) {
      packageJson.peerDependencies[name] = pkgVersion;
    }
  });

  fs.writeFileSync(
    packageJsonPath,
    JSON.stringify(packageJson, null, 2) + '\n',
  );
});

console.log('Package manifest update complete');

// Change working directory to the repo root
process.chdir(path.join(__dirname, '../..'));

// This repo uses yarn (see `packageManager` and CI). Running `npm install`
// here would generate a stray `package-lock.json`, so install with yarn.
if (!skipInstall) {
  execSync('yarn install', { stdio: 'inherit' });
}

// Commit changes
if (commit) {
  // Stage only the files this release is expected to touch: the workspace
  // manifests plus the yarn lockfile. Using an explicit pathspec instead of
  // `git add .` avoids committing unrelated or stray artifacts (e.g. a
  // `package-lock.json` if npm was ever run, or build output).
  const manifestPaths = workspaces.map(({ packageJsonPath }) =>
    path.relative(repoRoot, packageJsonPath),
  );
  // Pass args as an array (execFileSync, no shell) so paths can't be
  // interpreted by the shell.
  execFileSync('git', ['add', ...manifestPaths, 'yarn.lock'], {
    stdio: 'inherit',
  });
  // commit
  execFileSync('git', ['commit', '-m', pkgVersion, '--no-verify'], {
    stdio: 'inherit',
  });
  // tag
  // execFileSync('git', ['tag', '-f', '-a', '-m', pkgVersion, pkgVersion], { stdio: 'inherit' });
}

if (publish) {
  // Scoped packages (@stylexjs/*) default to restricted access on first
  // publish, which fails with E402 unless you have a paid plan. All stylex
  // packages are public, so always publish with `--access public`. This is a
  // no-op for packages that are already public.
  const publishArgs = ['publish', '--access', 'public'];
  if (otp != null) {
    publishArgs.push('--otp', otp);
  }
  if (dryRun) {
    publishArgs.push('--dry-run');
  }
  // publish public packages
  const failed = [];
  workspaces.forEach(({ directory, packageJson }) => {
    if (!packageJson.private) {
      const version = packageJson.version;
      const packageName = packageJson.name;
      try {
        // Check if the version has already been published
        execFileSync(
          'npm',
          ['view', '--silent', `${packageName}@${version}`, 'version'],
          { stdio: 'ignore' },
        );
        console.log(
          `Skipping ${packageName} as version ${version} has already been published`,
        );
      } catch (error) {
        // If the version has not been published, proceed with publishing.
        // Don't let a single failure (e.g. a package the current user lacks
        // publish rights for) abort the whole release — log it and continue
        // so the remaining packages still get published. Run in the package
        // directory via `cwd` (no shell) instead of `cd <dir> && ...`.
        try {
          execFileSync('npm', publishArgs, {
            cwd: directory,
            stdio: 'inherit',
          });
        } catch (publishError) {
          console.error(
            `Failed to publish ${packageName}@${version} — skipping. ${publishError.message}`,
          );
          failed.push(`${packageName}@${version}`);
        }
      }
    }
  });
  // Surface a non-zero exit if any package failed, so the release isn't
  // reported as successful when packages were silently skipped. Use
  // `exitCode` (not `exit`) so every package is still attempted first.
  if (failed.length > 0) {
    console.error(
      `\n${failed.length} package(s) failed to publish: ${failed.join(', ')}`,
    );
    process.exitCode = 1;
  }
  // push changes
  // execSync('git push --tags origin main', { stdio: 'inherit' });
}
