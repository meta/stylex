/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import path from 'node:path';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import { createEsbuildPlugin } from 'unplugin';

import { INDEX_CSS_RE, STYLE_CSS_RE, unpluginFactory } from './core';

function isInsideDir(dir, file) {
  const rel = path.relative(dir, file);
  return (
    rel !== '' &&
    rel !== '..' &&
    !rel.startsWith(`..${path.sep}`) &&
    !path.isAbsolute(rel)
  );
}

function resolveOutputFile(key, workingDir, outDir) {
  const candidates = [
    path.resolve(workingDir, key),
    path.resolve(outDir, key),
  ].filter((file) => isInsideDir(outDir, file));
  return (
    candidates.find((file) => fs.existsSync(file)) ||
    candidates[candidates.length - 1] ||
    null
  );
}

function attachEsbuildHooks(plugin) {
  return {
    ...plugin,
    esbuild: {
      name: '@stylexjs/unplugin',
      setup(build) {
        build.onEnd(async (result) => {
          try {
            const css = plugin.__stylexCollectCss?.();
            if (!css) return;
            const initial = build.initialOptions;
            const workingDir = initial.absWorkingDir || process.cwd();
            const outOption =
              initial.outdir ||
              (initial.outfile ? path.dirname(initial.outfile) : null);
            if (!outOption) return;
            const outDir = path.resolve(workingDir, outOption);
            let outfile = null;
            const meta = result && result.metafile;
            if (meta && meta.outputs) {
              const outputs = Object.keys(meta.outputs);
              const cssOutputs = outputs.filter((f) => f.endsWith('.css'));
              const pick =
                cssOutputs.find((f) => INDEX_CSS_RE.test(f)) ||
                cssOutputs.find((f) => STYLE_CSS_RE.test(f)) ||
                cssOutputs[0];
              if (pick) outfile = resolveOutputFile(pick, workingDir, outDir);
            } else {
              try {
                const files = fs
                  .readdirSync(outDir)
                  .filter((f) => f.endsWith('.css'));
                const pick =
                  files.find((f) => INDEX_CSS_RE.test(f)) ||
                  files.find((f) => STYLE_CSS_RE.test(f)) ||
                  files[0];
                if (pick) outfile = path.join(outDir, pick);
              } catch {}
            }
            if (!outfile) {
              const fallback = path.join(outDir, 'stylex.css');
              await fsp.mkdir(path.dirname(fallback), { recursive: true });
              await fsp.writeFile(fallback, css, 'utf8');
              return;
            }
            try {
              const current = fs.readFileSync(outfile, 'utf8');
              if (!current.includes(css)) {
                await fsp.writeFile(
                  outfile,
                  current ? current + '\n' + css : css,
                  'utf8',
                );
              }
            } catch (error) {
              console.warn(
                `[stylex] Could not append StyleX CSS to ${outfile}: ${error.message}`,
              );
            }
          } catch {}
        });
      },
    },
  };
}

export default createEsbuildPlugin((options, metaOptions) =>
  attachEsbuildHooks(unpluginFactory(options, metaOptions)),
);
