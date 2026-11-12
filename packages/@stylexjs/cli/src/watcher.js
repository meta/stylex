/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { ModuleType, TransformConfig } from './config';

import {
  clearInputModuleDir,
  fetchModule,
  findModuleDir,
  COMPILED_MODULES_DIR_NAME,
} from './modules';
import { compileDirectory } from './transform';

import ansis from 'ansis';
import watchman from 'fb-watchman';

type Subscription = {
  since?: string,
  fields: Array<string>,
  relative_root?: string,
};
type Watcher = {};

type Response = {
  watch: Watcher,
  relative_path: string,
  warning: string,
  subscribe: string,
  clock: string,
};

type OnEvent = {
  subscription: string,
  files: Array<$ReadOnly<Object>>,
};

declare class WatchmanClient {
  capabilityCheck(config: Object, callback: (error: any) => void): void;
  command(
    config:
      | ['watch-project', string]
      | ['clock', Watcher]
      | ['subscribe', Watcher, 'jsFileChanged', Subscription],
    callback: (error: any, response: Response) => void,
  ): void;

  on(eventName: string, callback: (event: OnEvent) => void): void;
  end(): void;
}

export function startWatcher(config: TransformConfig) {
  const watchmanClient: WatchmanClient = new watchman.Client();

  watchmanClient.capabilityCheck(
    { optional: [], required: ['relative_root'] },
    function (error, _resp) {
      if (error) {
        console.log(error);
        watchmanClient.end();
        return;
      }

      // Initiate the watch
      watchmanClient.command(
        ['watch-project', config.input],
        function (error, resp) {
          if (error) {
            console.error('[stylex] error initiating watch:', error);
            return;
          }
          if ('warning' in resp) {
            console.log('warning: ', resp.warning);
          }
          subscribeInput(
            watchmanClient,
            resp.watch,
            resp.relative_path,
            config,
          );
          console.log(
            'Watching for style changes in',
            ansis.green(resp.relative_path),
          );
        },
      );

      watchModuleDirectories(watchmanClient, config);
    },
  );
}

function watchModuleDirectories(
  client: WatchmanClient,
  config: TransformConfig,
) {
  if (
    config.modules_EXPERIMENTAL == null ||
    config.modules_EXPERIMENTAL.length === 0
  ) {
    return;
  }

  for (const module of config.modules_EXPERIMENTAL) {
    const moduleName = Array.isArray(module) ? module[0] : module;
    const moduleDir = findModuleDir(moduleName, config);
    if (moduleDir == null) {
      console.error(
        `[stylex] could not find module directory for: ${moduleName}`,
      );
      continue;
    }

    client.command(['watch-project', moduleDir], function (error, resp) {
      if (error) {
        console.error(`[stylex] error watching module ${moduleName}:`, error);
        return;
      }
      if ('warning' in resp) {
        console.log('warning: ', resp.warning);
      }

      client.command(['clock', resp.watch], function (error, clockResp) {
        if (error) {
          console.error(
            `[stylex] error getting clock for module ${moduleName}:`,
            error,
          );
          return;
        }
        subscribeModule(
          client,
          resp.watch,
          resp.relative_path,
          config,
          module,
          moduleName,
          clockResp.clock,
        );
        console.log(
          'Watching for style changes in module',
          ansis.green(moduleName),
        );
      });
    });
  }
}

function registerSubscription(
  client: WatchmanClient,
  watcher: Watcher,
  relative_path: string,
  subName: string,
  sinceClock?: string,
) {
  const subscription: Subscription = {
    fields: ['name', 'size', 'mtime_ms', 'exists', 'type'],
  };
  if (sinceClock != null) {
    subscription.since = sinceClock;
  }
  if (relative_path) {
    subscription.relative_root = relative_path;
  }

  client.command(
    ['subscribe', watcher, subName, subscription],
    function (error: string, _resp: Response) {
      if (error) {
        console.error(`[stylex] failed to subscribe to ${subName}: `, error);
        return;
      }
    },
  );
}

function compileAndCleanup(
  config: TransformConfig,
  filesToCompile: Array<string>,
  filesToDelete: Array<string>,
) {
  compileDirectory(config, filesToCompile, filesToDelete)
    .catch((transformError) => {
      console.error(transformError);
    })
    .finally(() => {
      clearInputModuleDir(config);
    });
}

function subscribeInput(
  client: WatchmanClient,
  watcher: Watcher,
  relative_path: string,
  config: TransformConfig,
) {
  registerSubscription(client, watcher, relative_path, 'jsFileChanged');

  const isNotCompiledModule = (file: $ReadOnly<Object>) =>
    !file.name.startsWith(COMPILED_MODULES_DIR_NAME);

  client.on('subscription', function (resp: OnEvent) {
    if (resp.subscription !== 'jsFileChanged') return;

    compileAndCleanup(
      config,
      resp.files
        .filter((file) => file.exists && isNotCompiledModule(file))
        .map((file) => file.name),
      resp.files
        .filter((file) => !file.exists && isNotCompiledModule(file))
        .map((file) => file.name),
    );
  });
}

function subscribeModule(
  client: WatchmanClient,
  watcher: Watcher,
  relative_path: string,
  config: TransformConfig,
  moduleConfig: ModuleType,
  moduleName: string,
  sinceClock: string,
) {
  const subName = 'moduleChanged:' + moduleName;
  registerSubscription(client, watcher, relative_path, subName, sinceClock);

  client.on('subscription', function (resp: OnEvent) {
    if (resp.subscription !== subName) return;

    fetchModule(moduleConfig, config);

    const prefix = COMPILED_MODULES_DIR_NAME + '/' + moduleName + '/';

    compileAndCleanup(
      config,
      resp.files
        .filter((file) => file.exists)
        .map((file) => prefix + file.name),
      resp.files
        .filter((file) => !file.exists)
        .map((file) => prefix + file.name),
    );
  });
}
