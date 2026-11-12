/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

// declare class Var<+T> {
//   value: T;
// }
// This is the type for the variables object
export opaque type StyleXVar<out _Val extends unknown>: string = string;

export opaque type StyleXClassNameFor<out _K, out _V>: string = string;
