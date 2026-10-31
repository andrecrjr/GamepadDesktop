// @ts-check
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

/** @param {string} version @param {string} cwd */
export function setCargoVersion(version, cwd = process.cwd()) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error('A stable release version such as 1.2.3 is required');
  }
  const files = [
    { file: 'desktop/Cargo.toml', marker: /^\[package\]\r?$/m },
    { file: 'desktop/Cargo.lock', marker: /^name = "gamepad-desktop"\r?$/m },
  ];
  const updates = files.map(({ file, marker }) => {
    const path = join(cwd, file);
    const sections = readFileSync(path, 'utf8').split(/(?=^\[)/m);
    const matches = sections.filter((section) => marker.test(section));
    const pattern = /^(version\s*=\s*")[^"\r\n]+(")/m;
    if (matches.length !== 1 || !pattern.test(matches[0])) {
      throw new Error(`Expected one application version in ${file}`);
    }
    const content = sections
      .map((section) =>
        marker.test(section) ? section.replace(pattern, `$1${version}$2`) : section,
      )
      .join('');
    return { path, content };
  });
  for (const { path, content } of updates) writeFileSync(path, content);
}

/**
 * @typedef {{cwd: string, env: Record<string, string | undefined>, nextRelease: {version: string}, options: {dryRun?: boolean}}} ReleaseContext
 */

/** @param {unknown} _config @param {ReleaseContext} context */
export function verifyRelease(_config, { env, nextRelease, options }) {
  if (!options.dryRun && env.GAMEPAD_DESKTOP_RELEASE_VERSION !== nextRelease.version) {
    throw new Error(`Release ${nextRelease.version} does not match the planned build version`);
  }
}

/** @param {unknown} config @param {ReleaseContext} context */
export function prepare(config, context) {
  verifyRelease(config, context);
  if (!context.options.dryRun) setCargoVersion(context.nextRelease.version, context.cwd);
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  setCargoVersion(process.env.GAMEPAD_DESKTOP_RELEASE_VERSION ?? '');
}
