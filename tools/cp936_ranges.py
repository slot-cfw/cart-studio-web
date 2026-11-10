import sys
from pathlib import Path

ranges = []
for cp in range(sys.maxunicode + 1):
    if 0xd800 <= cp <= 0xdfff:
        continue
    try:
        encoded = chr(cp).encode('cp936')
    except UnicodeEncodeError:
        continue
    assert len(encoded) == (1 if cp < 0x80 else 2), hex(cp)
    if ranges and ranges[-1][1] == cp - 1:
        ranges[-1][1] = cp
    else:
        ranges.append([cp, cp])

rows = '\n'.join(f'    (0x{a:04x}, 0x{b:04x}),' for a, b in ranges)
out = Path(sys.argv[1])
out.write_text(f'pub const RANGES: &[(u32, u32)] = &[\n{rows}\n];\n')
print(f'{len(ranges)} ranges from Python {sys.version.split()[0]}')
