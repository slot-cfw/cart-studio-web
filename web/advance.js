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
