'use strict';

// Fuzz worker. Loads Babel and one candidate, evaluates specs sent over IPC.
// A native crash kills only this process; the parent records it and respawns.

const { setup } = require('./compare.cjs');

const cfg = JSON.parse(process.env.STYLEX_FUZZ_CFG);
process.chdir(cfg.root);
const engine = setup(cfg);

process.on('message', (msg) => {
  if (msg.type !== 'eval') return;
  let result;
  try {
    result = engine.evalSpec(msg.spec, { full: msg.full });
  } catch (e) {
    result = { primary: 'harness-error', kinds: ['harness-error'], error: String(e && e.stack) };
  }
  process.send({ id: msg.id, result });
});
process.send({ ready: true });
