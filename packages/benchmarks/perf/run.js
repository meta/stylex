/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('fs');
const yargs = require('yargs/yargs');
const { hideBin } = require('yargs/helpers');

const createTests = require('./tests/transform-create-tests');
const createThemeTests = require('./tests/transform-create-theme-tests');
const runtimePropsTests = require('./tests/runtime-props-tests');

// run.js --outfile filename.js
const argv = yargs(hideBin(process.argv)).option('outfile', {
  alias: 'o',
  type: 'string',
  description: 'Output file',
  demandOption: false,
}).argv;
const outfile = argv.outfile;

const aggregatedResults = {};
const options = {
  callback(data, suiteName) {
    const testResults = data.reduce((acc, test) => {
      const { name, ops } = test;
      acc[name] = ops;
      return acc;
    }, {});

    aggregatedResults[suiteName] = testResults;
  },
};

console.log('Running "perf" benchmark, please wait...');

// Run tests
createTests(options);
createThemeTests(options);
runtimePropsTests(options);

const aggregatedResultsString = JSON.stringify(aggregatedResults, null, 2);

// Print / Write results
const now = new Date();
const year = now.getFullYear();
const month = String(now.getMonth() + 1).padStart(2, '0'); // Month is 0-indexed
const day = String(now.getDate()).padStart(2, '0');
const hours = String(now.getHours()).padStart(2, '0');
const minutes = String(now.getMinutes()).padStart(2, '0');
const timestamp = `${year}${month}${day}-${hours}${minutes}`;

const dirpath = `${process.cwd()}/.logs`;
const filepath = `${dirpath}/perf-${timestamp}.json`;
if (!fs.existsSync(dirpath)) {
  fs.mkdirSync(dirpath);
}
const outpath = outfile || filepath;
fs.writeFileSync(outpath, `${aggregatedResultsString}\n`);

console.log(aggregatedResultsString);
console.log('Results written to', outpath);
