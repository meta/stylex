/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import path from 'node:path';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import crypto from 'node:crypto';
import { createEsbuildPlugin } from 'unplugin';

import { INDEX_CSS_RE, STYLE_CSS_RE, unpluginFactory } from './core';

const BASE32_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

function computeBase32Hash(content, length = 8) {
  const hash = crypto.createHash('sha256').update(content).digest();
  let result = '';
  let buffer = 0;
  let bits = 0;
  for (let i = 0; i < hash.length && result.length < length; i++) {
    buffer = (buffer << 8) | hash[i];
    bits += 8;
    while (bits >= 5 && result.length < length) {
      bits -= 5;
      result += BASE32_ALPHABET[(buffer >> bits) & 31];
    }
  }
  return result;
}

function parseHashFromFilename(filename) {
  const base = filename.replace(/\.css$/i, '');
  let m = /^(.*[-._])([A-Z0-9]{6,})$/i.exec(base);
  if (m) return { prefix: m[1], oldHash: m[2], suffix: '' };
  m = /^([A-Z0-9]{6,})([-._].*)$/i.exec(base);
  if (m) return { prefix: '', oldHash: m[1], suffix: m[2] };
  m = /^([A-Z0-9]{6,})$/i.exec(base);
  if (m) return { prefix: '', oldHash: m[1], suffix: '' };
  m = /^(.*[-._])([A-Z0-9]{6,})([-._].*)$/i.exec(base);
  if (m) return { prefix: m[1], oldHash: m[2], suffix: m[3] };
  return null;
}

function hasHashPattern(template) {
  return typeof template === 'string' && /\[hash(?::\d+)?\]/.test(template);
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
            let pickedKey = null;
            const meta = result && result.metafile;
            if (meta && meta.outputs) {
              const outputs = Object.keys(meta.outputs);
              const cssOutputs = outputs.filter((f) => f.endsWith('.css'));
              pickedKey =
                cssOutputs.find((f) => INDEX_CSS_RE.test(f)) ||
                cssOutputs.find((f) => STYLE_CSS_RE.test(f)) ||
                cssOutputs[0];
              if (pickedKey) {
                outfile = path.isAbsolute(pickedKey)
                  ? pickedKey
                  : path.resolve(workingDir, pickedKey);
                if (
                  !fs.existsSync(outfile) &&
                  fs.existsSync(path.resolve(outDir, pickedKey))
                ) {
                  outfile = path.resolve(outDir, pickedKey);
                }
              }
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
            let current = '';
            try {
              current = fs.readFileSync(outfile, 'utf8');
            } catch {
              return;
            }
            let nextCss = current;
            if (!current.includes(css)) {
              nextCss = current ? current + '\n' + css : css;
            }

            const usesHashedNames =
              hasHashPattern(initial.entryNames) ||
              hasHashPattern(initial.assetNames) ||
              hasHashPattern(initial.chunkNames);

            const oldBasename = path.basename(outfile);
            const parsedHash = usesHashedNames
              ? parseHashFromFilename(oldBasename)
              : null;

            let newBasename = oldBasename;
            if (parsedHash) {
              const cssForHash = nextCss
                .replace(/\/\*# sourceMappingURL=.*?\*\/[^\n]*$/m, '')
                .trim();
              const newHash = computeBase32Hash(
                cssForHash,
                parsedHash.oldHash.length,
              );
              newBasename = `${parsedHash.prefix}${newHash}${parsedHash.suffix}.css`;

              if (newBasename !== oldBasename) {
                const oldMapBasename = `${oldBasename}.map`;
                const newMapBasename = `${newBasename}.map`;
                if (nextCss.includes(oldMapBasename)) {
                  nextCss = nextCss.split(oldMapBasename).join(newMapBasename);
                }
                const oldMapPath = `${outfile}.map`;
                const newMapPath = path.join(
                  path.dirname(outfile),
                  newMapBasename,
                );
                try {
                  if (fs.existsSync(oldMapPath)) {
                    await fsp.rename(oldMapPath, newMapPath);
                  }
                } catch {}
              }
            }

            const newOutfile = path.join(path.dirname(outfile), newBasename);
            await fsp.writeFile(newOutfile, nextCss, 'utf8');
            if (newOutfile !== outfile && fs.existsSync(outfile)) {
              try {
                await fsp.unlink(outfile);
              } catch {}
            }

            if (meta && meta.outputs) {
              let newKey = pickedKey;
              if (pickedKey && newBasename !== oldBasename) {
                const dir = path.dirname(pickedKey);
                newKey =
                  dir === '.'
                    ? pickedKey.startsWith('./')
                      ? `./${newBasename}`
                      : newBasename
                    : path.join(dir, newBasename);
              }

              if (pickedKey && newKey && pickedKey !== newKey) {
                if (meta.outputs[pickedKey]) {
                  meta.outputs[newKey] = meta.outputs[pickedKey];
                  delete meta.outputs[pickedKey];
                }
                const oldMapKey = `${pickedKey}.map`;
                const newMapKey = `${newKey}.map`;
                if (meta.outputs[oldMapKey]) {
                  meta.outputs[newMapKey] = meta.outputs[oldMapKey];
                  delete meta.outputs[oldMapKey];
                }
                for (const out of Object.values(meta.outputs)) {
                  if (out.cssBundle === pickedKey) {
                    out.cssBundle = newKey;
                  }
                }
              }

              const finalKey = newKey || pickedKey;
              if (finalKey && meta.outputs[finalKey]) {
                meta.outputs[finalKey].bytes = Buffer.byteLength(
                  nextCss,
                  'utf8',
                );
              }
            }

            if (newBasename !== oldBasename) {
              const filesToCheck =
                meta && meta.outputs
                  ? Object.keys(meta.outputs).map((k) =>
                      path.isAbsolute(k) ? k : path.resolve(workingDir, k),
                    )
                  : fs.readdirSync(outDir).map((f) => path.join(outDir, f));

              for (const filePath of filesToCheck) {
                if (
                  filePath === newOutfile ||
                  filePath.endsWith('.css') ||
                  filePath.endsWith('.css.map')
                ) {
                  continue;
                }
                try {
                  if (
                    fs.existsSync(filePath) &&
                    fs.statSync(filePath).isFile()
                  ) {
                    const content = fs.readFileSync(filePath, 'utf8');
                    if (content.includes(oldBasename)) {
                      const updated = content
                        .split(oldBasename)
                        .join(newBasename);
                      fs.writeFileSync(filePath, updated, 'utf8');
                      if (meta && meta.outputs) {
                        const matchKey = Object.keys(meta.outputs).find(
                          (k) =>
                            (path.isAbsolute(k)
                              ? k
                              : path.resolve(workingDir, k)) === filePath,
                        );
                        if (matchKey && meta.outputs[matchKey]) {
                          meta.outputs[matchKey].bytes = Buffer.byteLength(
                            updated,
                            'utf8',
                          );
                        }
                      }
                    }
                  }
                } catch {}
              }
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
