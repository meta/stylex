/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { TokenParser } from '../token-parser';

describe('TokenParser', () => {
  describe('oneOf', () => {
    it('parses the first parser', () => {
      const parser = TokenParser.oneOf(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
        TokenParser.tokens.Number.map((token) => token[4].value),
      );
      expect(parser.parseToEnd('foo')).toEqual('foo');
      expect(parser.parseToEnd('123')).toEqual(123);
    });

    it('fails to parse a different string', () => {
      const parser = TokenParser.oneOf(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
        TokenParser.tokens.Number.map((token) => token[4].value),
      );
      expect(parser.parse('baz') instanceof Error).toBe(true);
    });
  });

  describe('sequence', () => {
    it('parses a sequence', () => {
      const parser = TokenParser.sequence(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
        TokenParser.tokens.Whitespace,
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'baz' => value === 'baz',
        ),
      ).map(([foo, _whitespace, baz]) => [foo, baz]);
      expect(parser.parseToEnd('foo baz')).toEqual(['foo', 'baz']);
    });

    it('parses a sequence separated by whitespace', () => {
      const parser = TokenParser.sequence(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'bar' => value === 'bar',
        ),
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'baz' => value === 'baz',
        ),
      ).separatedBy(TokenParser.tokens.Whitespace);
      expect(parser.parseToEnd('foo bar baz')).toEqual(['foo', 'bar', 'baz']);
    });

    it('makes separators optional for optional parsers', () => {
      const parser = TokenParser.sequence(
        TokenParser.string('foo' as 'foo'),
        TokenParser.string('bar' as 'bar').optional,
        TokenParser.string('baz' as 'baz'),
      ).separatedBy(TokenParser.tokens.Whitespace);

      expect(parser.parseToEnd('foo bar baz')).toEqual(['foo', 'bar', 'baz']);
      expect(parser.parseToEnd('foo baz')).toEqual(['foo', undefined, 'baz']);
    });

    it('parses a sequence separated commas and optional whitespace', () => {
      const parser = TokenParser.sequence(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'bar' => value === 'bar',
        ),
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'baz' => value === 'baz',
        ),
      )
        .separatedBy(TokenParser.tokens.Comma)
        .separatedBy(TokenParser.tokens.Whitespace.optional);
      expect(parser.parseToEnd('foo, bar, baz')).toEqual(['foo', 'bar', 'baz']);
    });
  });

  describe('set', () => {
    it('parses a set', () => {
      const parser = TokenParser.setOf(
        TokenParser.string('foo' as 'foo'),
        TokenParser.string('baz' as 'baz'),
      ).separatedBy(TokenParser.tokens.Whitespace);

      expect(parser.parseToEnd('foo baz')).toEqual(['foo', 'baz']);

      expect(parser.parseToEnd('baz foo')).toEqual(['foo', 'baz']);
    });

    it('parses a set with double separators', () => {
      const parser = TokenParser.setOf(
        TokenParser.string('foo' as 'foo'),
        TokenParser.string('baz' as 'baz'),
      )
        .separatedBy(TokenParser.tokens.Comma)
        .separatedBy(TokenParser.tokens.Whitespace.optional);

      expect(parser.parseToEnd('foo,baz')).toEqual(['foo', 'baz']);
      expect(parser.parseToEnd('foo, baz')).toEqual(['foo', 'baz']);
      expect(parser.parseToEnd('foo   , baz')).toEqual(['foo', 'baz']);
      expect(parser.parseToEnd('foo   ,baz')).toEqual(['foo', 'baz']);

      expect(parser.parseToEnd('baz,foo')).toEqual(['foo', 'baz']);
      expect(parser.parseToEnd('baz, foo')).toEqual(['foo', 'baz']);
      expect(parser.parseToEnd('baz   , foo')).toEqual(['foo', 'baz']);
      expect(parser.parseToEnd('baz   ,foo')).toEqual(['foo', 'baz']);
    });

    it('makes separators optional for optional parsers', () => {
      const parser = TokenParser.setOf(
        TokenParser.string('foo' as 'foo'),
        TokenParser.string('bar' as 'bar').optional,
        TokenParser.string('baz' as 'baz'),
      ).separatedBy(TokenParser.tokens.Whitespace);

      expect(parser.parseToEnd('foo bar baz')).toEqual(['foo', 'bar', 'baz']);
      expect(parser.parseToEnd('foo baz bar')).toEqual(['foo', 'bar', 'baz']);
      expect(parser.parseToEnd('bar foo baz')).toEqual(['foo', 'bar', 'baz']);
      expect(parser.parseToEnd('bar baz foo')).toEqual(['foo', 'bar', 'baz']);
      expect(parser.parseToEnd('baz bar foo')).toEqual(['foo', 'bar', 'baz']);
      expect(parser.parseToEnd('baz foo bar')).toEqual(['foo', 'bar', 'baz']);

      expect(parser.parseToEnd('foo baz')).toEqual(['foo', undefined, 'baz']);
      expect(parser.parseToEnd('baz foo')).toEqual(['foo', undefined, 'baz']);
    });
  });

  describe('oneOrMore', () => {
    it('parses one or more', () => {
      const parser = TokenParser.oneOrMore(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
      ).separatedBy(TokenParser.tokens.Whitespace);
      expect(parser.parseToEnd('foo')).toEqual(['foo']);
      expect(parser.parseToEnd('foo foo')).toEqual(['foo', 'foo']);
      expect(parser.parseToEnd('foo foo foo')).toEqual(['foo', 'foo', 'foo']);
      expect(parser.parseToEnd('foo foo foo foo')).toEqual([
        'foo',
        'foo',
        'foo',
        'foo',
      ]);
      expect(parser.parseToEnd('foo foo foo foo foo')).toEqual([
        'foo',
        'foo',
        'foo',
        'foo',
        'foo',
      ]);
    });

    it('fails to parse a different string', () => {
      const parser = TokenParser.oneOrMore(
        TokenParser.tokens.Ident.map((token) => token[4].value).where(
          (value): implies value is 'foo' => value === 'foo',
        ),
      ).separatedBy(TokenParser.tokens.Whitespace);
      expect(parser.parse('bar') instanceof Error).toBe(true);
    });
  });

  describe('zeroOrMore', () => {
    it('parses zero or more', () => {
      const parser = TokenParser.zeroOrMore(
        TokenParser.string('foo' as 'foo'),
      ).separatedBy(TokenParser.tokens.Whitespace);

      expect(parser.parse('')).toEqual([]);
      expect(parser.parse('foo')).toEqual(['foo']);
      expect(parser.parse('foo foo')).toEqual(['foo', 'foo']);
      expect(parser.parse('foo foo foo')).toEqual(['foo', 'foo', 'foo']);
      expect(parser.parse('foo foo foo bar')).toEqual(['foo', 'foo', 'foo']);
      expect(parser.parse('foo foo foo for')).toEqual(['foo', 'foo', 'foo']);
    });
  });

  describe('failed alternatives', () => {
    const parser = TokenParser.oneOf(
      TokenParser.tokens.Ident.map((token) => token[4].value),
      TokenParser.tokens.Number.map((token) => token[4].value),
    );

    it('restores Error.stackTraceLimit', () => {
      const { stackTraceLimit } = Error;
      Error.stackTraceLimit = 37;
      try {
        expect(parser.parseToEnd('123')).toEqual(123);
        expect(Error.stackTraceLimit).toBe(37);
      } finally {
        Error.stackTraceLimit = stackTraceLimit;
      }
    });

    it('still parses when Error.stackTraceLimit is not writable', () => {
      const descriptor = Object.getOwnPropertyDescriptor<number>(
        Error,
        'stackTraceLimit',
      );
      Object.defineProperty(Error, 'stackTraceLimit', {
        value: Error.stackTraceLimit,
        writable: false,
        configurable: true,
      });
      try {
        expect(parser.parseToEnd('123')).toEqual(123);
        expect(parser.parse('"str"') instanceof Error).toBe(true);
      } finally {
        if (descriptor != null) {
          Object.defineProperty(Error, 'stackTraceLimit', descriptor);
        }
      }
    });
  });
});
