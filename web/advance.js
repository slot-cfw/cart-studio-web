const KERNELS = ['ezkernel.bin', 'ezkernelnew.bin'];

export const isAdvanceRoot = (names) => names.some((name) => KERNELS.includes(name.toLowerCase()));

export const advanceExtensions = { GBA: ['gba', 'agb'], GB: ['gb'], GBC: ['gbc'] };

export const LABEL_EXTENSIONS = ['png', 'jpg', 'jpeg', 'webp', 'bmp'];

export function flatFromSettings(text) {
  if (!text) return false;
  const lines = text.split('\n').map((line) => line.replace(/\r$/, ''));
  if (lines[0] !== 'SlotOmega=3') return false;
  return lines.includes('Skeuomorphic=0');
}

export const slotPath = (systemName, ...parts) => [systemName, 'SLOT', ...parts].join('/');

export const foldKey = (platform, stem) => `${platform}/${stem.normalize('NFC').toUpperCase().toLowerCase()}`;

const OTHER_LABELS = LABEL_EXTENSIONS.filter((e) => e !== 'png');

export function staleLabel(path, replaced) {
  const parts = path.split('/');
  if (parts.length !== 3 || parts[0].toLowerCase() !== 'labels') return false;
  const name = parts[2];
  const dot = name.lastIndexOf('.');
  if (dot < 0 || !OTHER_LABELS.includes(name.slice(dot + 1).toLowerCase())) return false;
  return replaced.has(foldKey(parts[1].toUpperCase(), name.slice(0, dot)));
}

export const conversionBlocked = (s) =>
  s.shellReadFailed || s.systemReadFailed ? 'slot. advance carts not written: the card’s shells couldn’t be read.' : null;
