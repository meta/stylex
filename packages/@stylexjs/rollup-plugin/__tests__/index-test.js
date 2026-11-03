/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import path from 'path';
import rollup from 'rollup';
import { babel } from '@rollup/plugin-babel';
import commonjs from '@rollup/plugin-commonjs';
import { nodeResolve } from '@rollup/plugin-node-resolve';
import stylexPlugin from '../src/index';

describe('rollup-plugin-stylex', () => {
  async function runStylex(options) {
    // Configure a rollup bundle
    const bundle = await rollup.rollup({
      // Remove stylex runtime from bundle
      external: [
        'stylex',
        '@stylexjs/stylex',
        '@stylexjs/stylex/lib/stylex-inject',
      ],
      input: path.resolve(__dirname, '__fixtures__/index.js'),
      plugins: [
        nodeResolve(),
        commonjs(),
        babel({
          babelHelpers: 'bundled',
          configFile: path.resolve(__dirname, '__fixtures__/.babelrc.json'),
          exclude: [/npmStyles\.js/],
        }),
        stylexPlugin({
          useCSSLayers: true,
          ...options,
          lightningcssOptions: { minify: false },
        }),
      ],
    });

    // Generate output specific code in-memory
    // You can call this function multiple times on the same bundle object
    const { output } = await bundle.generate({
      file: path.resolve(__dirname, '/__builds__/bundle.js'),
    });

    let css, js;

    for (const chunkOrAsset of output) {
      if (chunkOrAsset.fileName === 'stylex.css') {
        css = chunkOrAsset.source;
      } else if (chunkOrAsset.fileName === 'bundle.js') {
        js = chunkOrAsset.code;
      }
    }

    return { css, js, output };
  }

  it('still transforms built-in imports when a custom importSource is set', async () => {
    const { css } = await runStylex({
      fileName: 'stylex.css',
      importSources: ['custom-stylex'],
    });
    expect(css).toContain('display: flex;');
  });

  it('transforms a module that imports from a custom importSource', async () => {
    const bundle = await rollup.rollup({
      external: ['custom-stylex', '@stylexjs/stylex/lib/stylex-inject'],
      input: path.resolve(__dirname, '__fixtures__/customSource.js'),
      plugins: [
        babel({
          babelHelpers: 'bundled',
          configFile: path.resolve(__dirname, '__fixtures__/.babelrc.json'),
        }),
        stylexPlugin({
          fileName: 'stylex.css',
          importSources: ['custom-stylex'],
          lightningcssOptions: { minify: false },
        }),
      ],
    });
    const { output } = await bundle.generate({
      file: path.resolve(__dirname, '/__builds__/bundle.js'),
    });
    const css = output.find((o) => o.fileName === 'stylex.css')?.source;
    expect(css).toContain('color: red;');
  });

  it('extracts CSS and removes stylex.inject calls', async () => {
    const { css, js } = await runStylex({ fileName: 'stylex.css' });

    expect(css).toMatchInlineSnapshot(`
      "@layer priority1;

      @layer priority2 {
        .xeuoslp {
          animation-name: xgnty7z-B;
        }

        .xu4yf9m {
          border-start-start-radius: 7.5px;
        }

        .x1lliihq {
          display: block;
        }

        .x78zum5 {
          display: flex;
        }

        .xt0psk2 {
          display: inline;
        }

        .x1hm9lzh {
          margin-inline-start: 10px;
        }

        .x1gykpug:hover {
          background-color: red;
        }
      }

      @layer priority3 {
        .x1egiwwb {
          height: 500px;
        }

        .xlrshdv {
          margin-top: 99px;
        }

        .xh8yej3 {
          width: 100%;
        }

        .x3hqpx7 {
          width: 50%;
        }
      }

      @keyframes xgnty7z-B {
        0% {
          opacity: .25;
        }

        100% {
          opacity: 1;
        }
      }
      "
    `);

    expect(js).toMatchInlineSnapshot(`
      "import * as stylex from 'stylex';

      /**
       * Copyright (c) Meta Platforms, Inc. and affiliates.
       *
       * This source code is licensed under the MIT license found in the
       * LICENSE file in the root directory of this source tree.
       */

      var styles$2 = {
        bar: {
          k1xSpc: "x1lliihq",
          kzqmXN: "xh8yej3",
          $$css: true
        }
      };

      /**
       * Copyright (c) Meta Platforms, Inc. and affiliates.
       *
       * This source code is licensed under the MIT license found in the
       * LICENSE file in the root directory of this source tree.
       */

      const styles$1 = {
        baz: {
          k1xSpc: "xt0psk2",
          kZKoxP: "x1egiwwb",
          kzqmXN: "x3hqpx7",
          $$css: true
        }
      };

      /**
       * Copyright (c) Meta Platforms, Inc. and affiliates.
       *
       * This source code is licensed under the MIT license found in the
       * LICENSE file in the root directory of this source tree.
       */

      var styles = {
        foo: {
          kKVMdj: "xeuoslp",
          kWkggS: "x1gykpug",
          krdFHd: "xu4yf9m",
          k1xSpc: "x78zum5",
          kZKoxP: "x1egiwwb",
          keTefX: "x1hm9lzh",
          keoZOQ: "xlrshdv",
          $$css: true
        }
      };
      function App() {
        return stylex.props(styles$2.bar, styles.foo, styles$1.baz);
      }

      export { App as default };
      "
    `);
  });

  describe('runtimeInjection:true', () => {
    it('preserves stylex.inject calls and does not extract CSS', async () => {
      const { css, js } = await runStylex({
        debug: true,
        runtimeInjection: true,
      });

      expect(css).toBeUndefined();

      expect(js).toMatchInlineSnapshot(`
        "import _inject from '@stylexjs/stylex/lib/stylex-inject';
        import * as stylex from 'stylex';

        /**
         * Copyright (c) Meta Platforms, Inc. and affiliates.
         *
         * This source code is licensed under the MIT license found in the
         * LICENSE file in the root directory of this source tree.
         */

        var _inject2$2 = _inject;
        _inject2$2({
          ltr: ".x1lliihq{display:block}",
          priority: 3000
        });
        _inject2$2({
          ltr: ".xh8yej3{width:100%}",
          priority: 4000
        });
        var styles$2 = {
          bar: {
            "display-k1xSpc": "x1lliihq",
            "width-kzqmXN": "xh8yej3",
            $$css: "@stylexjs/rollup-plugin:__tests__/__fixtures__/otherStyles.js:14"
          }
        };

        /**
         * Copyright (c) Meta Platforms, Inc. and affiliates.
         *
         * This source code is licensed under the MIT license found in the
         * LICENSE file in the root directory of this source tree.
         */

        var _inject2$1 = _inject;
        _inject2$1({
          ltr: ".xt0psk2{display:inline}",
          priority: 3000
        });
        _inject2$1({
          ltr: ".x1egiwwb{height:500px}",
          priority: 4000
        });
        _inject2$1({
          ltr: ".x3hqpx7{width:50%}",
          priority: 4000
        });
        const styles$1 = {
          baz: {
            "display-k1xSpc": "xt0psk2",
            "height-kZKoxP": "x1egiwwb",
            "width-kzqmXN": "x3hqpx7",
            $$css: "@stylexjs/rollup-plugin:__tests__/__fixtures__/npmStyles.js:15"
          }
        };

        /**
         * Copyright (c) Meta Platforms, Inc. and affiliates.
         *
         * This source code is licensed under the MIT license found in the
         * LICENSE file in the root directory of this source tree.
         */

        var _inject2 = _inject;
        _inject2({
          ltr: "@keyframes xgnty7z-B{0%{opacity:.25;}100%{opacity:1;}}",
          priority: 0
        });
        _inject2({
          ltr: ".xeuoslp{animation-name:xgnty7z-B}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1gykpug:hover{background-color:red}",
          priority: 3130
        });
        _inject2({
          ltr: ".xu4yf9m{border-start-start-radius:7.5px}",
          priority: 3000
        });
        _inject2({
          ltr: ".x78zum5{display:flex}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1egiwwb{height:500px}",
          priority: 4000
        });
        _inject2({
          ltr: ".x1hm9lzh{margin-inline-start:10px}",
          priority: 3000
        });
        _inject2({
          ltr: ".xlrshdv{margin-top:99px}",
          priority: 4000
        });
        var styles = {
          foo: {
            "animationName-kKVMdj": "xeuoslp",
            "backgroundColor-kWkggS": "x1gykpug",
            "borderStartStartRadius-krdFHd": "xu4yf9m",
            "display-k1xSpc": "x78zum5",
            "height-kZKoxP": "x1egiwwb",
            "marginInlineStart-keTefX": "x1hm9lzh",
            "marginTop-keoZOQ": "xlrshdv",
            $$css: "@stylexjs/rollup-plugin:__tests__/__fixtures__/index.js:24"
          }
        };
        function App() {
          return stylex.props(styles$2.bar, styles.foo, styles$1.baz);
        }

        export { App as default };
        "
      `);
    });
  });
  it('output filename match pattern', async () => {
    const { output } = await runStylex({ fileName: 'stylex.[hash].css' });
    const css = output.find(
      (chunkOrAsset) =>
        chunkOrAsset.type === 'asset' &&
        /^stylex.[0-9a-f]{8}\.css$/.test(chunkOrAsset.fileName),
    );
    expect(css.source).toMatchInlineSnapshot(`
      "@layer priority1;

      @layer priority2 {
        .xeuoslp {
          animation-name: xgnty7z-B;
        }

        .xu4yf9m {
          border-start-start-radius: 7.5px;
        }

        .x1lliihq {
          display: block;
        }

        .x78zum5 {
          display: flex;
        }

        .xt0psk2 {
          display: inline;
        }

        .x1hm9lzh {
          margin-inline-start: 10px;
        }

        .x1gykpug:hover {
          background-color: red;
        }
      }

      @layer priority3 {
        .x1egiwwb {
          height: 500px;
        }

        .xlrshdv {
          margin-top: 99px;
        }

        .xh8yej3 {
          width: 100%;
        }

        .x3hqpx7 {
          width: 50%;
        }
      }

      @keyframes xgnty7z-B {
        0% {
          opacity: .25;
        }

        100% {
          opacity: 1;
        }
      }
      "
    `);
  });
});
