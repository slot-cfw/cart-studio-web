import { test } from 'node:test';
import assert from 'node:assert/strict';
import { advanceExtensions, flatFromSettings, isAdvanceRoot, LABEL_EXTENSIONS, slotPath } from './advance.js';

test('either kernel file at the root marks a slot. advance card', () => {
  assert.equal(isAdvanceRoot(['EZKERNELNEW.BIN', 'Games']), true);
  assert.equal(isAdvanceRoot(['ezkernel.bin']), true);
  assert.equal(isAdvanceRoot(['Games', 'System']), false);
  assert.equal(isAdvanceRoot(['ezkernel.bin.bak']), false);
});

test('the kernel reads .agb and keeps .gb and .gbc apart', () => {
  assert.deepEqual(advanceExtensions, { GBA: ['gba', 'agb'], GB: ['gb'], GBC: ['gbc'] });
  assert.deepEqual(LABEL_EXTENSIONS, ['png', 'jpg', 'jpeg', 'webp', 'bmp']);
});

test('the device draws flat carts only when its settings say so', () => {
  assert.equal(flatFromSettings('SlotOmega=3\nSkeuomorphic=0\nEnd=1\n'), true);
  assert.equal(flatFromSettings('SlotOmega=3\r\nSkeuomorphic=0\r\n'), true);
  assert.equal(flatFromSettings('SlotOmega=2\nSkeuomorphic=0\n'), false);
  assert.equal(flatFromSettings('SlotOmega=3\nSkeuomorphic=1\n'), false);
  assert.equal(flatFromSettings(null), false);
  assert.equal(flatFromSettings(''), false);
});

test('generated files go under the card own System folder', () => {
  assert.equal(slotPath('System', 'CARTS', 'GBA', 'x.bmp'), 'System/SLOT/CARTS/GBA/x.bmp');
  assert.equal(slotPath('SYSTEM', 'conversion.json'), 'SYSTEM/SLOT/conversion.json');
});
