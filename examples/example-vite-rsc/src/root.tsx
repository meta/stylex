/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import './index.css'; // keep minimal to ensure CSS asset exists in build
import viteLogo from '/vite.svg';
import { DevStyleXInject } from './DevStyleXInject';
import { getServerCounter, updateServerCounter } from './action.tsx';
import reactLogo from './assets/react.svg';
import { ClientCounter } from './client.tsx';
import * as stylex from '@stylexjs/stylex';

export function Root(props: { url: URL }) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <link href="/vite.svg" rel="icon" type="image/svg+xml" />
        <meta content="width=device-width, initial-scale=1.0" name="viewport" />
        <title>Vite + RSC + StyleX</title>
        <DevStyleXInject cssHref="/stylex.css" />
      </head>
      <body>
        <App {...props} />
      </body>
    </html>
  );
}

function App(props: { url: URL }) {
  return (
    <div id="root" {...stylex.props(styles.root)}>
      <div>
        <a
          href="https://vite.dev"
          target="_blank"
          {...stylex.props(styles.link, styles.linkHover)}
        >
          <img alt="Vite logo" src={viteLogo} {...stylex.props(styles.logo)} />
        </a>
        <a
          href="https://react.dev/reference/rsc/server-components"
          target="_blank"
          {...stylex.props(styles.link, styles.linkHover)}
        >
          <img
            alt="React logo"
            src={reactLogo}
            {...stylex.props(styles.logo, styles.reactLogo)}
          />
        </a>
      </div>
      <h1 {...stylex.props(styles.h1)}>Vite + RSC</h1>
      <div {...stylex.props(styles.card)}>
        <ClientCounter />
      </div>
      <div {...stylex.props(styles.card)}>
        <form action={updateServerCounter.bind(null, 1)}>
          <button {...stylex.props(styles.button)}>
            Server Counter: {getServerCounter()}
          </button>
        </form>
      </div>
      <div {...stylex.props(styles.card)}>Request URL: {props.url?.href}</div>
      <ul {...stylex.props(styles.readTheDocs)}>
        <li>
          Edit <code>src/client.tsx</code> to test client HMR.
        </li>
        <li>
          Edit <code>src/root.tsx</code> to test server HMR.
        </li>
        <li>
          Visit{' '}
          <a href="?__rsc" target="_blank">
            <code>?__rsc</code>
          </a>{' '}
          to view RSC stream payload.
        </li>
        <li>
          Visit{' '}
          <a href="?__nojs" target="_blank">
            <code>?__nojs</code>
          </a>{' '}
          to test server action without js enabled.
        </li>
      </ul>
    </div>
  );
}

const styles = stylex.create({
  root: {
    margin: 0,
    minHeight: '100vh',
    display: 'grid',
    placeItems: 'center',
    textAlign: 'center',
  },
  logo: {
    height: '6em',
    padding: '1.5em',
    willChange: 'filter',
    transitionProperty: 'filter',
    transitionDuration: '300ms',
    filter: {
      default: null,
      ':hover': 'drop-shadow(0 0 2em #646cffaa)',
    },
  },
  reactLogo: {
    filter: {
      default: null,
      ':hover': 'drop-shadow(0 0 2em #61dafbaa)',
    },
  },
  card: { padding: '2em' },
  readTheDocs: { color: '#888' },
  h1: { fontSize: '3.2em', lineHeight: 1.1 },
  link: { fontWeight: 500, color: '#646cff', textDecoration: 'none' },
  linkHover: { color: { ':hover': '#535bf2' } },
  button: {
    borderRadius: 8,
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: 'transparent',
    padding: '0.6em 1.2em',
    fontSize: '1em',
    fontWeight: 500,
    fontFamily: 'inherit',
    backgroundColor: 'green',
    color: 'white',
    cursor: 'pointer',
    transitionProperty: 'border-color',
    transitionDuration: '250ms',
  },
});
