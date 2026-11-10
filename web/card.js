// A card, however the browser opened it. Chrome and Edge hand over a directory handle that can
// be written to. Everything else hands over a flat list of files with their paths, which can only
// be read, so the labels leave as a zip instead.

// slot files a cart under the folder its platform names, in Games/, Labels/, Saves/ and States/
// alike, and the folder is what decides the platform: the rom is never opened to ask.
// Listed newest first, which is the order the studio's shelves and tabs run in.
export const PLATFORMS = ['GBA', 'GBAV', 'GBC', 'GB'];

const HARDWARE = { GBAV: 'GBA' };
export const hardware = (platform) => HARDWARE[platform] ?? platform;

// slot's own rule, from slot-store's platform.rs. A .gba under GB/ is not a Game Boy cart and is
// passed over in silence, the way slot passes over it.
const EXTENSIONS = { GBA: ['gba'], GBAV: ['gba'], GB: ['gb', 'gbc'], GBC: ['gb', 'gbc'] };
const ANY_ROM = ['gba', 'gb', 'gbc'];

import { advanceExtensions, foldKey, isAdvanceRoot, LABEL_EXTENSIONS } from './advance.js';

const named = (dir) => PLATFORMS.find((p) => p === dir.toUpperCase());
const ext = (name) => name.slice(name.lastIndexOf('.') + 1).toLowerCase();
// Never a dotfile: macOS leaves `._` sidecars beside what it copies onto a card, extension and
// all, so the extension alone cannot tell them from the file they shadow.
const visible = (name) => !name.startsWith('.');
const isCart = (platform, name, advance = false) =>
  visible(name) && ((advance ? advanceExtensions : EXTENSIONS)[platform] ?? []).includes(ext(name));
const isRom = (name) => visible(name) && ANY_ROM.includes(ext(name));
const isLabel = (name, advance = false) => visible(name) && (advance ? LABEL_EXTENSIONS : ['png']).includes(ext(name));
const stemOf = (name) => name.slice(0, name.lastIndexOf('.'));
const suffixOf = (name) => name.slice(name.lastIndexOf('.'));
const GENERATED = ['carts', 'carts_flat', 'labels'];

function labelPicker(keyOf) {
  const ranks = new Map();
  const labels = new Map();
  const found = [];
  return {
    labels,
    found,
    add(platform, name, file) {
      const key = keyOf(platform, stemOf(name));
      const rank = LABEL_EXTENSIONS.indexOf(ext(name));
      if (ranks.has(key) && ranks.get(key) <= rank) return;
      if (!ranks.has(key)) found.push({ key, platform, stem: stemOf(name) });
      ranks.set(key, rank);
      labels.set(key, file);
    },
  };
}

function advanceFields(carts, picker) {
  const owned = new Set(carts.map((c) => foldKey(c.platform, c.stem)));
  const orphans = picker.found
    .filter((l) => advanceExtensions[l.platform] && !owned.has(l.key))
    .map((l) => ({ platform: l.platform, stem: l.stem, file: picker.labels.get(l.key) }));
  return { advance: true, orphans };
}

async function childNamed(dir, name) {
  for await (const [entry, handle] of dir.entries()) {
    if (handle.kind === 'directory' && entry.toLowerCase() === name.toLowerCase()) return handle;
  }
  return null;
}

// macOS lists exFAT names decomposed, whatever form was written, so names are compared composed.
// What gets written is still the listing's own spelling. The platform is part of the key because
// a .gb and a .gba can share a stem — two different games, two different carts.
const labelKey = (platform, stem) => `${platform}/${stem.normalize('NFC')}`;
export const twins = (carts, title) => {
  const seen = new Map();
  for (const c of carts) {
    const key = labelKey(c.platform, title(c));
    seen.set(key, [...(seen.get(key) ?? []), c]);
  }
  return new Set([...seen.values()].filter((group) => group.length > 1).flat());
};
const byCart = (a, b) =>
  a.platform !== b.platform
    ? PLATFORMS.indexOf(a.platform) - PLATFORMS.indexOf(b.platform)
    : a.stem < b.stem
      ? -1
      : a.stem > b.stem
        ? 1
        : 0;

const NO_GAMES = 'That doesn’t look like a slot SD card. Be sure to select the root of the card.';
const UNREADABLE =
  'Your browser couldn’t read that folder. In Firefox, the card’s hidden .Spotlight-V100 folder or a file with an accented name like é can cause this.';
const OLD_SHELL_FILE =
  'This browser can’t open cart_shell.ini. Start the latest slot once, then choose the card again.';
const OLD_LAYOUT =
  'This card keeps its roms loose in Games, where slot doesn’t look. Move them into the GBA, GB and GBC folders inside Games, then choose the card again.';

export async function fromDirectory(root) {
  const rootNames = [];
  for await (const [name] of root.entries()) rootNames.push(name);
  const advance = isAdvanceRoot(rootNames);
  const keyOf = advance ? foldKey : labelKey;
  const systemName = rootNames.find((n) => n.toLowerCase() === 'system') ?? 'SYSTEM';
  let games;
  try {
    games = await root.getDirectoryHandle('Games');
  } catch {
    throw new Error(NO_GAMES);
  }
  const carts = [];
  let sawPlatform = false;
  let loose = false;
  for await (const [name, handle] of games.entries()) {
    const platform = handle.kind === 'directory' ? named(name) : null;
    if (!platform) {
      if (handle.kind === 'file' && isRom(name)) loose = true;
      continue;
    }
    sawPlatform = true;
    for await (const [file, fh] of handle.entries()) {
      if (fh.kind === 'file' && isCart(platform, file, advance)) {
        const cart = { platform, stem: stemOf(file), file: () => fh.getFile() };
        if (advance) cart.suffix = suffixOf(file);
        carts.push(cart);
      }
    }
  }
  if (!sawPlatform && loose) throw new Error(OLD_LAYOUT);

  const picker = labelPicker(keyOf);
  const labels = picker.labels;
  try {
    const dir = await root.getDirectoryHandle('Labels');
    for await (const [name, handle] of dir.entries()) {
      const platform = handle.kind === 'directory' ? named(name) : null;
      if (!platform) continue;
      for await (const [file, fh] of handle.entries()) {
        if (fh.kind === 'file' && isLabel(file, advance)) picker.add(platform, file, () => fh.getFile());
      }
    }
  } catch {
    // No Labels folder yet: every cart needs a label.
  }

  const shellFile = async (folder) => {
    let dir;
    try {
      dir = await root.getDirectoryHandle(folder);
    } catch (e) {
      if (e.name === 'NotFoundError') return null;
      throw e;
    }
    const read = async (name) => {
      try {
        return await (await (await dir.getFileHandle(name)).getFile()).text();
      } catch (e) {
        if (e.name === 'NotFoundError') return null;
        throw e;
      }
    };
    const txt = await read('cart_shell.txt');
    if (txt !== null) return txt;
    let listed = false;
    for await (const [name] of dir.entries()) listed ||= name.toLowerCase() === 'cart_shell.ini';
    if (!listed) return null;
    try {
      return await read('cart_shell.ini');
    } catch (e) {
      if (e.name !== 'TypeError') throw e;
      throw Object.assign(new Error(OLD_SHELL_FILE, { cause: e }), { name: 'OldShellFile' });
    }
  };
  const systemShells = async () => (await shellFile('Config')) ?? (await shellFile('System')) ?? '';
  const labelShells = async () => (await shellFile('Labels')) ?? '';

  const slotDir = async (create) => {
    const dir = (await childNamed(root, systemName)) ?? (create ? await root.getDirectoryHandle(systemName, { create }) : null);
    if (!dir) return null;
    return (await childNamed(dir, 'SLOT')) ?? (create ? await dir.getDirectoryHandle('SLOT', { create }) : null);
  };
  const extra = advance
    ? {
        ...advanceFields(carts, picker),
        systemName,
        name: root.name ?? '',
        async settings() {
          const dir = await slotDir(false);
          if (!dir) return null;
          for await (const [name, handle] of dir.entries()) {
            if (handle.kind === 'file' && name.toLowerCase() === 'settings.txt') return (await handle.getFile()).text();
          }
          return null;
        },
        async writeSlot(parts, bytes) {
          let dir = await slotDir(true);
          for (const name of parts.slice(0, -1)) {
            dir = (await childNamed(dir, name)) ?? (await dir.getDirectoryHandle(name, { create: true }));
          }
          const out = await (await dir.getFileHandle(parts.at(-1), { create: true })).createWritable();
          try {
            await out.write(bytes);
            await out.close();
          } catch (e) {
            await out.abort().catch(() => {});
            throw e;
          }
        },
      }
    : {};

  return {
    ...extra,
    carts: carts.sort(byCart),
    labels,
    keyOf,
    direct: true,
    systemShells,
    labelShells,
    async writeShells(text) {
      const dir = await root.getDirectoryHandle('Labels', { create: true });
      const file = await dir.getFileHandle('cart_shell.txt', { create: true });
      const out = await file.createWritable();
      try {
        await out.write(text);
        await out.close();
      } catch (e) {
        await out.abort().catch(() => {});
        throw e;
      }
    },
    // A label already on the card is skipped unless `replace` says to write over it.
    async write(platform, stem, bytes, replace = false) {
      const root_dir = await root.getDirectoryHandle('Labels', { create: true });
      const dir = await root_dir.getDirectoryHandle(platform, { create: true });
      const name = `${stem}.png`;
      let existed = false;
      try {
        await dir.getFileHandle(name);
        existed = true;
        if (!replace) return 'skipped';
      } catch (e) {
        if (e.name !== 'NotFoundError') throw e;
      }
      // The file exists from here on, empty until the write closes. A write that fails (a full
      // card, permission withdrawn, the card pulled) must take it away again: an empty file left
      // behind reads as a label the card already has, so the studio would never make this one.
      const file = await dir.getFileHandle(name, { create: true });
      let out = null;
      try {
        out = await file.createWritable();
        await out.write(bytes);
        await out.close();
      } catch (e) {
        await out?.abort().catch(() => {});
        // A label being replaced is left as it was: the write only lands when it closes.
        if (!existed) await dir.removeEntry(name).catch(() => {});
        throw e;
      }
      if (advance) {
        const stale = [];
        for await (const [other, handle] of dir.entries()) {
          const same = foldKey(platform, stemOf(other)) === foldKey(platform, stem);
          if (handle.kind === 'file' && same && ext(other) !== 'png' && LABEL_EXTENSIONS.includes(ext(other))) {
            stale.push(other);
          }
        }
        for (const other of stale) await dir.removeEntry(other).catch(() => {});
      }
      return 'written';
    },
  };
}

export function fromFiles(files) {
  if (files.length === 0) throw new Error(UNREADABLE);
  const paths = files.map((file) => file.webkitRelativePath.split('/'));
  const advance = isAdvanceRoot(paths.filter((parts) => parts.length === 2).map((parts) => parts[1]));
  const keyOf = advance ? foldKey : labelKey;
  const systemName = paths.find((parts) => parts.length >= 3 && parts[1].toLowerCase() === 'system')?.[1] ?? 'SYSTEM';
  const carts = [];
  const picker = labelPicker(keyOf);
  const labels = picker.labels;
  const labelFiles = [];
  const slotFiles = [];
  let settingsFile = null;
  let sawPlatform = false;
  let loose = false;
  const shellFiles = {};
  for (const file of files) {
    const parts = file.webkitRelativePath.split('/');
    if (advance && parts.length >= 4 && parts[1].toLowerCase() === 'system' && parts[2].toLowerCase() === 'slot') {
      const rest = parts.slice(3);
      const made =
        (rest.length > 1 && GENERATED.includes(rest[0].toLowerCase()) && ext(rest.at(-1)) === 'bmp') ||
        (rest.length === 1 && rest[0].toLowerCase() === 'conversion.json');
      if (rest.length === 1 && rest[0].toLowerCase() === 'settings.txt') settingsFile = file;
      if (!made && rest.every(visible)) slotFiles.push({ path: [systemName, 'SLOT', ...rest].join('/'), file: async () => file });
    }
    if (parts.length >= 3 && parts[1].toLowerCase() === 'labels' && parts.slice(2).every(visible)) {
      labelFiles.push({ path: ['Labels', ...parts.slice(2)].join('/'), file: async () => file });
    }
    // The picked folder, then Games or Labels, then the platform, then the file. Anything
    // shallower is the layout slot swept away; anything deeper is not slot's.
    if (parts.length === 3) {
      const [, dir, name] = parts;
      if (dir.toLowerCase() === 'games' && isRom(name)) loose = true;
      const ext = ['cart_shell.txt', 'cart_shell.ini'].indexOf(name.toLowerCase());
      if (['config', 'system', 'labels'].includes(dir.toLowerCase()) && ext !== -1) {
        (shellFiles[dir.toLowerCase()] ??= [])[ext] = file;
      }
      continue;
    }
    if (parts.length !== 4) continue;
    const [, dir, folder, name] = parts;
    const platform = named(folder);
    if (!platform) continue;
    if (dir.toLowerCase() === 'games') {
      sawPlatform = true;
      if (isCart(platform, name, advance)) {
        const cart = { platform, stem: stemOf(name), file: async () => file };
        if (advance) cart.suffix = suffixOf(name);
        carts.push(cart);
      }
    } else if (dir.toLowerCase() === 'labels' && isLabel(name, advance)) {
      picker.add(platform, name, async () => file);
    }
  }
  if (!sawPlatform) throw new Error(loose ? OLD_LAYOUT : NO_GAMES);
  const text = async (files) => {
    const file = files?.[0] ?? files?.[1];
    return file ? file.text() : '';
  };
  const extra = advance
    ? {
        ...advanceFields(carts, picker),
        systemName,
        name: paths[0][0],
        slotFiles,
        settings: async () => settingsFile?.text() ?? null,
      }
    : {};
  return {
    ...extra,
    carts: carts.sort(byCart),
    labels,
    keyOf,
    labelFiles,
    direct: false,
    write: null,
    systemShells: () => text(shellFiles.config ?? shellFiles.system),
    labelShells: () => text(shellFiles.labels),
  };
}

export { labelKey };
