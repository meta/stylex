/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();

const { parse } = require('@babel/parser');
const traverse = require('@babel/traverse').default;
const { evaluate } = require('../evaluate-path');
const { default: StateManager } = require('../state-manager');

function evaluateFirstStatement(code, functions) {
  const ast = parse(code);
  let result;
  traverse(ast, {
    Program(path, state) {
      const stateManager = new StateManager({
        ...state,
        file: { metadata: {} },
      });
      const statements = path.get('body');
      const statement = statements[0];
      if (!statement) {
        return;
      }
      if (statement.isVariableDeclaration()) {
        const valuePath = statement.get('declarations')[0].get('init');
        result = evaluate(valuePath, stateManager, functions);
      } else {
        result = evaluate(statement, stateManager, functions);
      }
    },
  });
  if (result === undefined || result.confident === false) {
    return { confident: false };
  } else {
    return result.value;
  }
}

function evaluateLastStatement(code, functions) {
  const ast = parse(code);
  let result;
  traverse(ast, {
    Program(path, state) {
      const stateManager = new StateManager({
        ...state,
        file: { metadata: {} },
      });
      const statements = path.get('body');
      const last = statements[statements.length - 1];
      if (last.isExpressionStatement()) {
        result = evaluate(last.get('expression'), stateManager, functions);
      }
    },
  });
  if (result === undefined || result.confident === false) {
    return { confident: false };
  } else {
    return result.value;
  }
}

describe('custom path evaluation works as expected', () => {
  test('Evaluates Primitive Value expressions', () => {
    expect(evaluateFirstStatement('1 + 2', {})).toBe(3);
    expect(evaluateFirstStatement('1 - 2', {})).toBe(-1);
    expect(evaluateFirstStatement('1 * 2', {})).toBe(2);
    expect(evaluateFirstStatement('1 / 2', {})).toBe(0.5);
    expect(evaluateFirstStatement('1 % 2', {})).toBe(1);
    expect(evaluateFirstStatement('1 ** 2', {})).toBe(1);
    expect(evaluateFirstStatement('1 << 2', {})).toBe(4);
    expect(evaluateFirstStatement('1 >> 2', {})).toBe(0);
    expect(evaluateFirstStatement('1 & 2', {})).toBe(0);
    expect(evaluateFirstStatement('1 | 2', {})).toBe(3);
    expect(evaluateFirstStatement('1 ^ 2', {})).toBe(3);
    expect(evaluateFirstStatement('1 && 2', {})).toBe(2);
    expect(evaluateFirstStatement('1 || 2', {})).toBe(1);

    expect(evaluateFirstStatement('null', {})).toBe(null);
    expect(evaluateFirstStatement('undefined', {})).toBe(undefined);
    expect(evaluateFirstStatement('true', {})).toBe(true);
    expect(evaluateFirstStatement('false', {})).toBe(false);
    expect(evaluateFirstStatement('let x = "hello";', {})).toBe('hello');
  });
  test('Evaluates Simple Arrays and Objects', () => {
    expect(evaluateFirstStatement('const x = {};', {})).toEqual({});
    expect(
      evaluateFirstStatement('const x = {name: "Name", age: 43};', {}),
    ).toEqual({ name: 'Name', age: 43 });

    expect(evaluateFirstStatement('const x = [];', {})).toEqual([]);
    expect(evaluateFirstStatement('const x = [1, 2, 3];', {})).toEqual([
      1, 2, 3,
    ]);
    expect(evaluateFirstStatement('const x = [1, 2, 3, 4, 5];', {})).toEqual([
      1, 2, 3, 4, 5,
    ]);
  });
  test('Evaluates Objects with spreads', () => {
    expect(
      evaluateFirstStatement(
        'const x = {name: "Name", ...({hero: true}), age: 43};',
        {},
      ),
    ).toEqual({ name: 'Name', hero: true, age: 43 });

    expect(
      evaluateFirstStatement(
        'const x = {name: "Name", ...({name: "StyleX", age: 1}), age: 43};',
        {},
      ),
    ).toEqual({ name: 'StyleX', age: 43 });
  });

  test('Evaluates built-in functions', () => {
    expect(evaluateFirstStatement('const x = Math.max(1, 2, 3);', {})).toBe(3);
    expect(evaluateFirstStatement('const x = Math.min(1, 2, 3);', {})).toBe(1);
  });

  test('Evaluates built-in functions without the Node global alias', () => {
    const globalDescriptor = Object.getOwnPropertyDescriptor(
      globalThis,
      'global',
    );
    Reflect.deleteProperty(globalThis, 'global');

    try {
      expect(
        evaluateFirstStatement('const x = Math.max(Number(1), Number(2));', {}),
      ).toBe(2);
    } finally {
      if (globalDescriptor) {
        Object.defineProperty(globalThis, 'global', globalDescriptor);
      }
    }
  });

  test('Evaluates custom functions', () => {
    function makeArray(...args) {
      return [...args].reverse();
    }

    expect(
      evaluateFirstStatement('const x = makeArray(1, 2, 3);', {
        identifiers: {
          makeArray: { fn: makeArray },
        },
      }),
    ).toEqual([3, 2, 1]);

    expect(
      evaluateFirstStatement('const x = stylex.makeArray(1, 2, 3);', {
        memberExpressions: { stylex: { makeArray: { fn: makeArray } } },
      }),
    ).toEqual([3, 2, 1]);
  });

  test('Evaluates custom functions that return non-static values', () => {
    class MyClass {
      constructor(value) {
        this.value = value;
      }
    }
    function makeClass(value) {
      return new MyClass(value);
    }

    expect(
      evaluateFirstStatement('const x = makeClass("Hello");', {
        identifiers: { makeClass: { fn: makeClass } },
      }),
    ).toEqual(new MyClass('Hello'));
  });

  test('Evaluates custom functions used as spread values', () => {
    function makeObj(value) {
      return { spreadValue: value };
    }

    expect(
      evaluateFirstStatement(
        'const x = {name: "Name", ...makeObj("Hello"), age: 30};',
        {
          identifiers: {
            makeObj: { fn: makeObj },
          },
        },
      ),
    ).toEqual({ name: 'Name', spreadValue: 'Hello', age: 30 });
  });

  test('Evaluates custom functions that take paths', () => {
    function getNode(path) {
      const { type, value } = path.node;
      return { type, value };
    }

    expect(
      evaluateFirstStatement('const x = getNode("Hello");', {
        identifiers: { getNode: { fn: getNode, takesPath: true } },
      }),
    ).toEqual({ type: 'StringLiteral', value: 'Hello' });
  });

  describe('evaluating function expressions', () => {
    test('function with a single params', () => {
      const fn = evaluateFirstStatement('const double = x => x * 2;', {});
      expect(typeof fn).toEqual('function');

      expect(fn(2)).toBe(4);
    });

    test('function with a two params', () => {
      const fn = evaluateFirstStatement('const add = (a, b) => a + b;', {});
      expect(typeof fn).toEqual('function');

      expect(fn(2, 7)).toBe(9);
    });

    test('Array map', () => {
      expect(
        evaluateFirstStatement('const x = [1, 2, 3].map(x => x * 2);', {}),
      ).toEqual([2, 4, 6]);
    });

    test('Array filter', () => {
      expect(
        evaluateFirstStatement(
          'const x = [1, 2, 3].filter(x => x % 2 === 0);',
          {},
        ),
      ).toEqual([2]);
    });

    test('Array map and filter', () => {
      expect(
        evaluateFirstStatement(
          'const x = [1, 2, 3].map(x => x * 2).filter(x => x % 2 === 0);',
          {},
        ),
      ).toEqual([2, 4, 6]);
    });

    test('Object entries', () => {
      expect(
        evaluateFirstStatement(
          'const x = Object.entries({a: 1, b: 2, c: 4}).filter((entry) => entry[1] % 2 === 0);',
          {},
        ),
      ).toEqual([
        ['b', 2],
        ['c', 4],
      ]);

      expect(
        evaluateFirstStatement(
          'const x = Object.fromEntries(Object.entries({a: 1, b: 2, c: 4}).filter((entry) => entry[1] % 2 === 0));',
          {},
        ),
      ).toEqual({
        b: 2,
        c: 4,
      });
    });

    test('Methods called by string should be bind', () => {
      expect(
        evaluateFirstStatement(
          'const x = "".concat("10px"," ").concat("10px");',
          {},
        ),
      ).toBe('10px 10px');
      expect(evaluateFirstStatement('const x = "abc".charCodeAt(0);', {})).toBe(
        97,
      );
    });
  });

  describe('evaluate-path prototype-chain escape prevention', () => {
    test('blocks reading .constructor off an object literal', () => {
      expect(evaluateFirstStatement('const x = ({}).constructor;', {})).toEqual(
        {
          confident: false,
        },
      );
    });

    test('blocks walking to Function via .constructor.constructor', () => {
      expect(
        evaluateFirstStatement('const x = ({}).constructor.constructor;', {}),
      ).toEqual({ confident: false });
    });

    test('blocks arbitrary code execution via Function constructor', () => {
      expect(
        evaluateLastStatement(
          '({}).constructor.constructor("return 1").call({});',
          {},
        ),
      ).toEqual({ confident: false });
    });

    test('blocks constructor calls on allowlisted globals', () => {
      expect(
        evaluateLastStatement('Object.constructor("return 1").call({});', {}),
      ).toEqual({ confident: false });
    });

    test('blocks computed-property access to constructor', () => {
      expect(
        evaluateLastStatement(
          '({})["constructor"]["constructor"]("return 1").call({});',
          {},
        ),
      ).toEqual({ confident: false });
    });

    test('blocks computed-property access after property-key coercion', () => {
      const code = `
        const key = Object('constructor');
        const FunctionConstructor = ({})[key][key];
        FunctionConstructor('return 1').call({});
      `;
      expect(
        evaluateLastStatement(code, {
          identifiers: {},
          memberExpressions: {},
        }),
      ).toEqual({ confident: false });
    });

    test('blocks __proto__ traversal', () => {
      expect(
        evaluateLastStatement(
          '({}).__proto__.constructor.constructor("return 1").call({});',
          {},
        ),
      ).toEqual({ confident: false });
    });

    test('blocks prototype traversal', () => {
      expect(evaluateFirstStatement('const x = Object.prototype;', {})).toEqual(
        {
          confident: false,
        },
      );
    });

    test('blocks .constructor off a string literal', () => {
      expect(
        evaluateLastStatement(
          '"abc".constructor.constructor("return 1").call({});',
          {},
        ),
      ).toEqual({ confident: false });
    });

    test('blocks .constructor reached through a bound variable', () => {
      const code = `
        const o = {};
        o.constructor.constructor("return 1").call({});
      `;
      expect(evaluateLastStatement(code, {})).toEqual({ confident: false });
    });

    test('blocks Function reached through reflection', () => {
      const code = `
        const objectPrototype = Object.getPrototypeOf({});
        const ObjectConstructor = Object.getOwnPropertyDescriptor(
          objectPrototype,
          'constructor',
        ).value;
        const functionPrototype = Object.getPrototypeOf(ObjectConstructor);
        const FunctionConstructor = Object.getOwnPropertyDescriptor(
          functionPrototype,
          'constructor',
        ).value;
        FunctionConstructor('return 1').call({});
      `;
      expect(
        evaluateLastStatement(code, {
          identifiers: {},
          memberExpressions: {},
        }),
      ).toEqual({ confident: false });
    });

    test('blocks Function reached through inherited config properties', () => {
      // `memberExpressions['constructor']` resolves to `Object` through the
      // prototype chain, and `Object.constructor` is `Function`.
      expect(
        evaluateLastStatement('constructor.constructor("return 1").call({});', {
          identifiers: {},
          memberExpressions: {},
        }),
      ).toEqual({ confident: false });
      expect(
        evaluateLastStatement('valueOf.constructor("return 1").call({});', {
          identifiers: {},
          memberExpressions: {},
        }),
      ).toEqual({ confident: false });
    });

    test('blocks reflective Object methods', () => {
      expect(
        evaluateFirstStatement('const x = Object.getPrototypeOf({});', {}),
      ).toEqual({ confident: false });
      expect(
        evaluateFirstStatement(
          "const x = Object.getOwnPropertyDescriptor({a: 1}, 'a');",
          {},
        ),
      ).toEqual({ confident: false });
      expect(
        evaluateFirstStatement('const x = Object.create({});', {}),
      ).toEqual({ confident: false });
      expect(
        evaluateFirstStatement('const x = Object.setPrototypeOf({}, {});', {}),
      ).toEqual({ confident: false });
    });

    // Only the methods that hand back prototype chain objects are rejected.
    // Ones that return plain data stay available, so the restriction does not
    // reach further than it has to.
    test('still allows built-in methods that return plain data', () => {
      expect(
        evaluateFirstStatement(
          'const x = Object.getOwnPropertyNames({a: 1});',
          {},
        ),
      ).toEqual(['a']);
      expect(evaluateFirstStatement('const x = Object.isFrozen({});', {})).toBe(
        false,
      );
      expect(
        evaluateFirstStatement('const x = Object.isExtensible({});', {}),
      ).toBe(true);
    });

    test('still deopts on non-deterministic and mutating methods', () => {
      expect(evaluateFirstStatement('const x = Math.random();', {})).toEqual({
        confident: false,
      });
      expect(
        evaluateFirstStatement('const x = Object.assign({}, {a: 1});', {}),
      ).toEqual({ confident: false });
      expect(
        evaluateFirstStatement('const x = Object.freeze({a: 1});', {}),
      ).toEqual({ confident: false });
    });

    test('still allows legitimate member access', () => {
      expect(evaluateFirstStatement('const x = "abc".length;', {})).toBe(3);
      expect(evaluateFirstStatement('const x = ({a: 1, b: 2}).b;', {})).toBe(2);
      expect(evaluateFirstStatement('const x = [10, 20][1];', {})).toBe(20);
    });

    // The checks above assert that each escape *deopts*. This one asserts the
    // property that actually matters: that a payload is never run, no matter
    // how the escape is spelled or whether the evaluator deopts or throws.
    test('a payload is never executed, however it is reached', () => {
      // Each vector is written out in full rather than interpolated from a
      // shared payload constant. Building these by interpolation reads to
      // static analysis as code construction from an unsanitized value, and
      // these are fixtures, never anything the evaluator should run.
      const vectors = [
        '({}).constructor.constructor("globalThis.__stylexPwned__ = 1")()',
        '({}).constructor.constructor("globalThis.__stylexPwned__ = 1").call({})',
        'Object.constructor("globalThis.__stylexPwned__ = 1").call({})',
        // `memberExpressions['constructor']` is `Object` via the prototype
        // chain, so `constructor.constructor` would be `Function`.
        'constructor.constructor("globalThis.__stylexPwned__ = 1").call({})',
        'valueOf.constructor("globalThis.__stylexPwned__ = 1").call({})',
        '({})["constructor"]["constructor"]("globalThis.__stylexPwned__ = 1").call({})',
        // Built at runtime, so an AST-level check on the key would miss it.
        '({})["const"+"ructor"]["con"+"structor"]("globalThis.__stylexPwned__ = 1").call({})',
        '[].constructor.constructor("globalThis.__stylexPwned__ = 1").call({})',
        '"a".constructor.constructor("globalThis.__stylexPwned__ = 1").call({})',
        '((x) => x).constructor("globalThis.__stylexPwned__ = 1").call({})',
        '[].map.constructor("globalThis.__stylexPwned__ = 1").call({})',
        'Object.getOwnPropertyDescriptor(Object.getPrototypeOf({}), "constructor").value.constructor("globalThis.__stylexPwned__ = 1").call({})',
        'Object.fromEntries([["a",1]]).constructor.constructor("globalThis.__stylexPwned__ = 1").call({})',
        'Array.from([1]).constructor.constructor("globalThis.__stylexPwned__ = 1").call({})',
        'new Function("globalThis.__stylexPwned__ = 1")()',
        'eval("globalThis.__stylexPwned__ = 1")',
        '[1].map((x) => x.constructor.constructor("globalThis.__stylexPwned__ = 1")())[0]',
      ];

      const executed = [];
      for (const vector of vectors) {
        delete globalThis.__stylexPwned__;
        try {
          evaluateLastStatement(vector + ';', {
            identifiers: {},
            memberExpressions: {},
          });
        } catch {
          // Refusing by throwing still means the payload did not run.
        }
        if (globalThis.__stylexPwned__ !== undefined) {
          executed.push(vector);
        }
      }
      delete globalThis.__stylexPwned__;

      expect(executed).toEqual([]);
    });
  });

  describe('evaluate-path mutation detection', () => {
    test('evaluates constant array correctly', () => {
      const code = `
        const a = [1, 2];
        a;
      `;
      expect(evaluateLastStatement(code, {})).toEqual([1, 2]);
    });

    test('should bail out when array is mutated via push', () => {
      const code = `
        const a = [1, 2];
        a.push(3);
        a;
      `;
      expect(evaluateLastStatement(code, {})).toEqual({ confident: false });
    });

    test('should bail out when array is mutated via assignment', () => {
      const code = `
        const a = [1, 2];
        a[0] = 3;
        a;
      `;
      expect(evaluateLastStatement(code, {})).toEqual({ confident: false });
    });

    test('should bail out when object is mutated via Object.assign', () => {
      const code = `
        const a = {bar: 'baz'};
        Object.assign(a, {foo: 1});
        a;
      `;
      expect(evaluateLastStatement(code, {})).toEqual({ confident: false });
    });
  });
});
