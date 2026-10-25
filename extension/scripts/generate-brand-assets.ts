import { Resvg } from '@resvg/resvg-js';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const source = await readFile(join(root, 'assets/gamepad-desktop.svg'));
const outputs = new Map<string, Buffer>();
const check = process.argv.includes('--check');

for (const path of [
  'desktop/assets/gamepad-desktop.svg',
  'site/assets/gamepad-desktop-icon.svg',
  'extension/public/gamepad-desktop-icon.svg',
]) {
  outputs.set(path, source);
}

function png(size: number, background?: string): Buffer {
  return Buffer.from(
    new Resvg(source, { fitTo: { mode: 'width', value: size }, background }).render().asPng(),
  );
}

const logo = png(1024);
outputs.set('desktop/assets/gamepad-desktop.png', logo);
outputs.set('extension/public/gamepad-desktop-logo.png', logo);
outputs.set('site/assets/favicon-32.png', png(32));
outputs.set('site/assets/apple-touch-icon.png', png(180, '#0A111F'));

for (const size of [16, 32, 48, 128]) {
  outputs.set(`extension/public/icons/icon-${size}.png`, png(size));
}

// ICO stores one PNG frame per Windows display size; 0 encodes a 256px dimension.
const sizes = [16, 20, 24, 32, 48, 64, 128, 256];
const frames = sizes.map((size) => png(size));
const directory = Buffer.alloc(6 + sizes.length * 16);
directory.writeUInt16LE(1, 2);
directory.writeUInt16LE(sizes.length, 4);
let offset = directory.length;
for (const [index, size] of sizes.entries()) {
  const frame = frames[index]!;
  const entry = 6 + index * 16;
  directory[entry] = directory[entry + 1] = size === 256 ? 0 : size;
  directory.writeUInt16LE(1, entry + 4);
  directory.writeUInt16LE(32, entry + 6);
  directory.writeUInt32LE(frame.length, entry + 8);
  directory.writeUInt32LE(offset, entry + 12);
  offset += frame.length;
}
const ico = Buffer.concat([directory, ...frames]);
outputs.set('desktop/assets/gamepad-desktop.ico', ico);
outputs.set('site/assets/favicon.ico', ico);

for (const [path, expected] of outputs) {
  const destination = join(root, path);
  if (check) {
    if (!existsSync(destination) || !(await readFile(destination)).equals(expected)) {
      console.error(`Brand asset is out of date: ${path}`);
      process.exitCode = 1;
    }
  } else {
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, expected);
  }
}
if (!process.exitCode) {
  console.log(
    `${outputs.size} brand assets ${check ? 'verified' : 'generated'} from assets/gamepad-desktop.svg`,
  );
}
