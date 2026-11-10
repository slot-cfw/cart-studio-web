import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fromDirectory, fromFiles, twins } from './card.js';

const notFound = () => Object.assign(new Error('not found'), { name: 'NotFoundError' });

const dirHandle = (tree) => ({
  kind: 'directory',
  async getDirectoryHandle(name) {
    if (typeof tree[name] !== 'object') throw notFound();
    return dirHandle(tree[name]);
  },
  async getFileHandle(name) {
    if (typeof tree[name] !== 'string') throw notFound();
    return fileHandle(tree[name]);
  },
  async *entries() {
    for (const [name, value] of Object.entries(tree)) {
      yield [name, typeof value === 'object' ? dirHandle(value) : fileHandle(value)];
    }
  },
});
const fileHandle = (text) => ({ kind: 'file', getFile: async () => new File([text], 'f') });

const listed = (tree, prefix = 'card') =>
  Object.entries(tree).flatMap(([name, value]) =>
    typeof value === 'object'
      ? listed(value, `${prefix}/${name}`)
      : [Object.assign(new File([value], name), { webkitRelativePath: `${prefix}/${name}` })],
  );

const GAMES = { Games: { GBA: { 'A.gba': '' } } };
const CONFIG = 'A = rounded 112233 solid\n';
const SYSTEM = 'A = notched 445566 clear\n';
const LABELS = 'A = auto\n';

const layers = async (source) => ({ system: await source.systemShells(), labels: await source.labelShells() });
const both = async (tree) => [await layers(await fromDirectory(dirHandle(tree))), await layers(fromFiles(listed(tree)))];

test('a card slot has moved reads its shells from Config', async () => {
  for (const got of await both({ ...GAMES, Config: { 'cart_shell.txt': CONFIG }, Labels: { 'cart_shell.txt': LABELS } })) {
    assert.deepEqual(got, { system: CONFIG, labels: LABELS });
  }
});

test('a card slot has not moved yet reads its shells from System', async () => {
  for (const got of await both({ ...GAMES, System: { 'cart_shell.ini': SYSTEM } })) {
    assert.deepEqual(got, { system: SYSTEM, labels: '' });
  }
});

test('Config wins over a System file left behind', async () => {
  for (const got of await both({ ...GAMES, Config: { 'cart_shell.txt': CONFIG }, System: { 'cart_shell.ini': SYSTEM } })) {
    assert.deepEqual(got, { system: CONFIG, labels: '' });
  }
});

test('an empty Config file is still the one slot reads', async () => {
  for (const got of await both({ ...GAMES, Config: { 'cart_shell.txt': '' }, System: { 'cart_shell.ini': SYSTEM } })) {
    assert.deepEqual(got, { system: '', labels: '' });
  }
});

test('a Config folder without the file falls back to System', async () => {
  for (const got of await both({ ...GAMES, Config: {}, System: { 'cart_shell.ini': SYSTEM } })) {
    assert.deepEqual(got, { system: SYSTEM, labels: '' });
  }
});

test('a card with no shell files reads as empty', async () => {
  for (const got of await both(GAMES)) {
    assert.deepEqual(got, { system: '', labels: '' });
  }
});

test('a Config file the browser refuses fails only its own layer', async () => {
  const refused = Object.assign(new Error('could not be read'), { name: 'NotReadableError' });
  const card = dirHandle({ ...GAMES, Config: { 'cart_shell.txt': CONFIG }, Labels: { 'cart_shell.txt': LABELS } });
  const config = await card.getDirectoryHandle('Config');
  const source = await fromDirectory({
    ...card,
    async getDirectoryHandle(name) {
      if (name !== 'Config') return card.getDirectoryHandle(name);
      return { ...config, getFileHandle: async () => ({ kind: 'file', getFile: async () => Promise.reject(refused) }) };
    },
  });
  await assert.rejects(source.systemShells(), { name: 'NotReadableError' });
  assert.equal(await source.labelShells(), LABELS);
});

test('a card slot has not started on since the rename reads its .ini shells', async () => {
  for (const got of await both({ ...GAMES, Config: { 'cart_shell.ini': CONFIG }, Labels: { 'cart_shell.ini': LABELS } })) {
    assert.deepEqual(got, { system: CONFIG, labels: LABELS });
  }
});

test('a .txt wins over the .ini beside it', async () => {
  const tree = {
    ...GAMES,
    Config: { 'cart_shell.ini': SYSTEM, 'cart_shell.txt': CONFIG },
    Labels: { 'cart_shell.ini': SYSTEM, 'cart_shell.txt': LABELS },
  };
  for (const got of await both(tree)) {
    assert.deepEqual(got, { system: CONFIG, labels: LABELS });
  }
});

test('an .ini the browser will not open by name asks for slot to start once', async () => {
  const card = dirHandle({ ...GAMES, Labels: { 'cart_shell.ini': LABELS } });
  const labels = await card.getDirectoryHandle('Labels');
  const source = await fromDirectory({
    ...card,
    async getDirectoryHandle(name) {
      if (name !== 'Labels') return card.getDirectoryHandle(name);
      return {
        ...labels,
        async getFileHandle(file) {
          if (file.endsWith('.ini')) throw new TypeError('Name is not allowed.');
          return labels.getFileHandle(file);
        },
      };
    },
  });
  await assert.rejects(source.labelShells(), { name: 'OldShellFile' });
});

test('shells are written to Labels/cart_shell.txt', async () => {
  const written = {};
  const card = dirHandle({ ...GAMES, Labels: {} });
  const source = await fromDirectory({
    ...card,
    async getDirectoryHandle(name) {
      if (name !== 'Labels') return card.getDirectoryHandle(name);
      return {
        async getFileHandle(file) {
          return {
            async createWritable() {
              return { write: async (t) => (written[`${name}/${file}`] = t), close: async () => {} };
            },
          };
        },
      };
    },
  });
  await source.writeShells(LABELS);
  assert.deepEqual(written, { 'Labels/cart_shell.txt': LABELS });
});

test('a listed card carries every file in its Labels folder but dotfiles', async () => {
  const source = fromFiles(
    listed({
      ...GAMES,
      labels: {
        'cart_shell.txt': LABELS,
        '.DS_Store': '',
        GBA: { 'A.png': 'a', 'Gone.png': 'g', '._A.png': '' },
        Odd: { 'note.txt': 'n' },
      },
      Saves: { GBA: { 'A.sav': '' } },
    }),
  );
  const got = await Promise.all(source.labelFiles.map(async (f) => [f.path, await (await f.file()).text()]));
  assert.deepEqual(got, [
    ['Labels/cart_shell.txt', LABELS],
    ['Labels/GBA/A.png', 'a'],
    ['Labels/GBA/Gone.png', 'g'],
    ['Labels/Odd/note.txt', 'n'],
  ]);
});

test('a folder the browser listed as empty says it could not be read', () => {
  assert.throws(() => fromFiles([]), /couldn’t read that folder/);
});

test('a folder with files but no games is still not a slot card', () => {
  assert.throws(() => fromFiles(listed({ Photos: { 'a.jpg': '' } })), /doesn’t look like a slot SD card/);
});

test('roms loose in Games ask to be moved into platform folders by hand', async () => {
  const loose = { Games: { 'A.gba': '' } };
  const asks = { message: /Move them into the GBA, GB and GBC folders/ };
  await assert.rejects(fromDirectory(dirHandle(loose)), asks);
  assert.throws(() => fromFiles(listed(loose)), asks);
});

test('a card without shell files reads as empty where the browser refuses .ini names', async () => {
  const windows = (dir) => ({
    ...dir,
    async getDirectoryHandle(name) {
      return windows(await dir.getDirectoryHandle(name));
    },
    async getFileHandle(name) {
      if (name.endsWith('.ini')) throw new TypeError('Name is not allowed.');
      return dir.getFileHandle(name);
    },
  });
  const source = await fromDirectory(windows(dirHandle({ ...GAMES, Config: {}, Labels: {} })));
  assert.deepEqual(await layers(source), { system: '', labels: '' });
});

test('Game Boy Advance Video carts are the .gba files in Games/GBAV, shelved after GBA', async () => {
  const tree = {
    Games: {
      GBC: { 'C.gbc': '' },
      GBAV: { 'Shrek.gba': '', 'Tetris.gb': '' },
      GBA: { 'A.gba': '' },
    },
    Labels: { GBAV: { 'Shrek.png': 's' } },
  };
  for (const source of [await fromDirectory(dirHandle(tree)), fromFiles(listed(tree))]) {
    assert.deepEqual(
      source.carts.map((c) => `${c.platform}/${c.stem}`),
      ['GBA/A', 'GBAV/Shrek', 'GBC/C'],
    );
    assert.ok(source.labels.has('GBAV/Shrek'));
  }
});

test('twins are the carts a shelf would title alike', () => {
  const title = (c) => c.stem.replace(/ \(.*\)$/, '');
  const cart = (platform, stem) => ({ platform, stem });
  const hack = cart('GBC', 'Zelda (Hack)');
  const plain = cart('GBC', 'Zelda');
  const lone = cart('GBC', 'Tetris');
  const elsewhere = cart('GB', 'Zelda');
  assert.deepEqual(twins([hack, plain, lone, elsewhere], title), new Set([hack, plain]));
});

test('twins are found across Unicode forms', () => {
  const composed = { platform: 'GBA', stem: 'Pok\u00e9mon' };
  const decomposed = { platform: 'GBA', stem: 'Poke\u0301mon' };
  assert.deepEqual(twins([composed, decomposed], (c) => c.stem), new Set([composed, decomposed]));
});

const ADVANCE = {
  'ezkernel.bin': '',
  Games: { GBA: { 'a.agb': '', 'z.gba': '' }, GB: { 'b.gbc': '', 'g.gb': '' }, GBC: { 'c.gbc': '' }, GBAV: { 'v.gba': '' } },
  Labels: { GBA: { 'z.jpg': 'jpeg', 'Orphan.png': 'png' }, GB: { 'g.png': 'png', 'g.webp': 'webp' } },
  System: { SLOT: { 'SETTINGS.TXT': 'SlotOmega=3\n', CARTS: { GBA: { 'old.bmp': 'bmp' } }, 'conversion.json': '{}' } },
};

const cartsOf = (source) => source.carts.map((c) => `${c.platform}/${c.stem}${c.suffix ?? ''}`);
const sources = async (tree) => [await fromDirectory(dirHandle(tree)), fromFiles(listed(tree))];

test('a slot. advance card scans with the kernel rules', async () => {
  for (const source of await sources(ADVANCE)) {
    assert.equal(source.advance, true);
    assert.equal(source.systemName, 'System');
    assert.deepEqual(cartsOf(source), ['GBA/a.agb', 'GBA/z.gba', 'GBC/c.gbc', 'GB/g.gb']);
  }
});

test('a slot card keeps slot rules and has no slot. advance fields', async () => {
  const { 'ezkernel.bin': _, ...tree } = ADVANCE;
  for (const source of await sources(tree)) {
    assert.equal(source.advance, undefined);
    assert.equal(source.orphans, undefined);
    assert.ok(cartsOf(source).includes('GB/b'));
    assert.ok(!cartsOf(source).some((c) => c.startsWith('GBA/a')));
  }
});

test('a slot. advance card reads non-PNG labels and prefers a PNG', async () => {
  for (const source of await sources(ADVANCE)) {
    assert.equal(await (await source.labels.get('GBA/z')()).text(), 'jpeg');
    assert.equal(await (await source.labels.get('GB/g')()).text(), 'png');
  }
  const { 'ezkernel.bin': _, ...slot } = ADVANCE;
  for (const source of await sources(slot)) assert.equal(source.labels.get('GBA/z'), undefined);
});

test('labels with no rom are slot. advance orphans', async () => {
  for (const source of await sources(ADVANCE)) {
    const orphans = await Promise.all(
      source.orphans.map(async (o) => `${o.platform}/${o.stem}:${await (await o.file()).text()}`),
    );
    assert.deepEqual(orphans, ['GBA/Orphan:png']);
  }
});

test('the device settings are read from the card', async () => {
  for (const source of await sources(ADVANCE)) assert.equal(await source.settings(), 'SlotOmega=3\n');
  const { System: _, ...bare } = ADVANCE;
  for (const source of await sources(bare)) assert.equal(await source.settings(), null);
});

test('the zip carries the card own SLOT files but not what conversion makes', async () => {
  const source = fromFiles(listed(ADVANCE));
  assert.deepEqual(
    source.slotFiles.map((f) => f.path),
    ['System/SLOT/SETTINGS.TXT'],
  );
});

const writable = (tree) => ({
  kind: 'directory',
  async getDirectoryHandle(name, { create = false } = {}) {
    if (typeof tree[name] !== 'object') {
      if (!create) throw notFound();
      tree[name] = {};
    }
    return writable(tree[name]);
  },
  async getFileHandle(name, { create = false } = {}) {
    if (typeof tree[name] !== 'string') {
      if (!create) throw notFound();
      tree[name] = '';
    }
    return {
      kind: 'file',
      getFile: async () => new File([tree[name]], name),
      createWritable: async () => {
        let body = '';
        return {
          write: async (bytes) => (body += typeof bytes === 'string' ? bytes : new TextDecoder().decode(bytes)),
          close: async () => (tree[name] = body),
          abort: async () => {},
        };
      },
    };
  },
  async removeEntry(name) {
    if (!(name in tree)) throw notFound();
    delete tree[name];
  },
  async *entries() {
    for (const name of Object.keys(tree)) {
      yield [name, typeof tree[name] === 'object' ? writable(tree[name]) : await this.getFileHandle(name)];
    }
  },
});

test('generated files land in the card own System folder', async () => {
  const tree = structuredClone(ADVANCE);
  const source = await fromDirectory(writable(tree));
  await source.writeSlot(['CARTS', 'GBA', 'x.bmp'], new TextEncoder().encode('new'));
  await source.writeSlot(['conversion.json'], new TextEncoder().encode('{"a":1}'));
  assert.equal(tree.SYSTEM, undefined);
  assert.equal(tree.System.SLOT.CARTS.GBA['x.bmp'], 'new');
  assert.equal(tree.System.SLOT.CARTS.GBA['old.bmp'], 'bmp');
  assert.equal(tree.System.SLOT['conversion.json'], '{"a":1}');
});

test('a card with no System folder gets SYSTEM', async () => {
  const { System: _, ...bare } = structuredClone(ADVANCE);
  const source = await fromDirectory(writable(bare));
  await source.writeSlot(['LABELS', 'GB', 'y.bmp'], new TextEncoder().encode('y'));
  assert.equal(bare.SYSTEM.SLOT.LABELS.GB['y.bmp'], 'y');
});

test('a new PNG label replaces a non-PNG one on a slot. advance card', async () => {
  const tree = structuredClone(ADVANCE);
  const source = await fromDirectory(writable(tree));
  assert.equal(await source.write('GBA', 'z', new TextEncoder().encode('png'), true), 'written');
  assert.equal(tree.Labels.GBA['z.png'], 'png');
  assert.equal(tree.Labels.GBA['z.jpg'], undefined);
});

const CASED = {
  'ezkernelnew.bin': '',
  Games: { GBA: { 'Ruby.gba': '', 'STRASSE.gba': '' } },
  Labels: { GBA: { 'ruby.png': 'png', 'Straße.png': 'png' } },
};

test('a slot. advance card pairs labels with roms whatever their case', async () => {
  for (const source of await sources(CASED)) {
    assert.equal(await (await source.labels.get(source.keyOf('GBA', 'Ruby'))()).text(), 'png');
    assert.equal(await (await source.labels.get(source.keyOf('GBA', 'STRASSE'))()).text(), 'png');
    assert.deepEqual(source.orphans, []);
  }
});

test('a slot card still pairs labels by exact name', async () => {
  const { 'ezkernelnew.bin': _, ...slot } = CASED;
  for (const source of await sources(slot)) {
    assert.equal(source.labels.get(source.keyOf('GBA', 'Ruby')), undefined);
    assert.equal(source.keyOf('GBA', 'Ruby'), 'GBA/Ruby');
  }
});

test('a new PNG label removes a non-PNG one whose case differs', async () => {
  const tree = structuredClone({ ...CASED, Labels: { GBA: { 'ruby.jpg': 'jpeg' } } });
  const source = await fromDirectory(writable(tree));
  assert.equal(await source.write('GBA', 'Ruby', new TextEncoder().encode('png'), true), 'written');
  assert.equal(tree.Labels.GBA['ruby.jpg'], undefined);
});
