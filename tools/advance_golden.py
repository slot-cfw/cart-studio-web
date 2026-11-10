import io
import json
import shutil
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageOps

fork = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
sys.path.insert(0, str(fork / 'tools'))
import convert_labels as c
from simple_cartridge import shell_face

card = out / 'card'
expected = out / 'expected'


def rom(platform, name, code='', flag=0, title=b'', licensee=0):
    p = card / 'Games' / platform / name
    p.parent.mkdir(parents=True, exist_ok=True)
    data = bytearray(0x150)
    if platform == 'GBA':
        data[0xac:0xb0] = code.ljust(4).encode()
    else:
        data[0x134:0x134 + len(title)] = title
        data[0x13f:0x143] = code.ljust(4).encode()
        data[0x143] = flag
        data[0x14a] = licensee
    p.write_bytes(data)


def pattern(w, h, seed, alpha=False):
    im = Image.new('RGBA', (w, h))
    px = im.load()
    for y in range(h):
        for x in range(w):
            r = (x * 255 // max(1, w - 1) + seed * 37) % 256
            g = (y * 255 // max(1, h - 1) + seed * 91) % 256
            b = ((x ^ y) * 7 + seed * 13) % 256
            a = 255
            if alpha:
                dx, dy = x - w / 2, y - h / 2
                a = 255 if dx * dx / (w * w / 6) + dy * dy / (h * h / 6) < 1 else 0
            px[x, y] = (r, g, b, a)
    return im


def label(platform, name, im, fmt='PNG', **kw):
    p = card / 'Labels' / platform / name
    p.parent.mkdir(parents=True, exist_ok=True)
    im.save(p, fmt, **kw)


def make_card():
    if card.exists():
        shutil.rmtree(card)
    card.mkdir(parents=True)
    (card / 'ezkernelnew.bin').write_bytes(b'')
    rom('GBA', 'Ruby.gba', 'AXVE')
    label('GBA', 'Ruby.png', pattern(240, 124, 1))
    rom('GBA', 'Silver.gba', 'MXXE')
    label('GBA', 'Silver.png', pattern(300, 160, 2))
    rom('GBA', 'Plain.gba', 'ABCE')
    label('GBA', 'Plain.png', pattern(264, 136, 3))
    rom('GBA', "Spider-Man - Mysterio's Menace (USA).agb", 'AMSE')
    rom('GBA', 'Logo.gba', 'ALGE')
    label('GBA', 'Logo.png', pattern(220, 113, 4, alpha=True))
    rom('GBA', 'Photo.gba', 'APHE')
    exif = Image.Exif()
    exif[0x0112] = 6
    label('GBA', 'Photo.jpg', pattern(124, 240, 5).convert('RGB'), 'JPEG', quality=90, exif=exif.tobytes())
    rom('GBA', 'Cafe Ole [T+Eng] (Japan).gba', 'ACBJ')
    rom('GBA', 'Picked.gba', 'APKE')
    label('GBA', 'Picked.png', pattern(200, 100, 6))
    label('GBA', 'Orphan.png', pattern(180, 90, 7))
    rom('GB', 'Grey.gb')
    label('GB', 'Grey.png', pattern(223, 199, 8))
    rom('GB', 'Black.gb', flag=0x80)
    rom('GB', 'POKEMON RED.gb', title=b'POKEMON RED', licensee=1)
    label('GB', 'POKEMON RED.png', pattern(150, 140, 9))
    rom('GB', 'Crystal.gb', 'BYTE')
    rom('GB', 'Shared.gb')
    rom('GBC', 'Shared.gbc', flag=0xc0)
    rom('GBC', 'Prism.gbc', flag=0xc0)
    label('GBC', 'Prism.png', pattern(223, 199, 10))
    rom('GBC', 'Picked.gbc', flag=0xc0)
    label('GBC', 'Picked.png', pattern(100, 120, 11))
    rom('GBC', 'Cleared.gbc', flag=0x80)
    label('GBC', 'Cleared.png', pattern(160, 160, 12))
    (card / 'System').mkdir()
    (card / 'System' / 'cart_shell.txt').write_text(
        'Cleared = rounded abcdef clear\nSilver = auto 112233 glitter\n', encoding='utf-8')
    (card / 'Labels' / 'cart_shell.txt').write_text(
        'Cleared = auto\nPicked = rounded 123456 glitter\nGrey = notched aabbcc clear\n'
        'Ruby = auto 8844CC solid\nCafé = auto 223344 solid\n', encoding='utf-8')


def dump(name, im):
    d = expected / 'pillow'
    d.mkdir(parents=True, exist_ok=True)
    im.convert('RGBA').save(d / f'{name}.png')


def primitives():
    for shape in ('GBA', 'GB', 'GBC'):
        panel = c.SPECS[shape][1]
        for colour, finish in (('35353a', 'solid'), ('c2332e', 'clear'), ('86b9bf', 'glitter')):
            clear = finish != 'solid'
            name = f'{shape.lower()}_{"clear" if clear and shape != "GB" else "neutral"}.png'
            dump(f'tinted-{shape}-{colour}-{finish}', c.tinted(name, colour, None, clear, finish == 'glitter'))
            dump(f'shell_face-{shape}-{colour}-{finish}', shell_face(c.ASSETS, shape, colour, finish, panel))
    dump('resize-gba', c.tinted('gba_neutral.png', '35353a').resize((120, 71), Image.Resampling.LANCZOS))
    back = c.tinted('gba_clear.png', 'c2332e', None, True, False).copy()
    back.alpha_composite(Image.open(c.ASSETS / 'board.png').convert('RGBA'))
    dump('composite', back)
    logo = Image.open(card / 'Labels' / 'GBA' / 'Logo.png').convert('RGBA')
    for size in ((88, 45), (74, 66)):
        dump(f'fit-{size[0]}x{size[1]}', ImageOps.fit(logo, size, method=Image.Resampling.LANCZOS, centering=(.5, .5)))
    titles = ["Spider-Man - Mysterio's Menace (USA)", 'Crème Brûlée [T+Eng] (Japan)',
              'Super Mario Advance 4 - Super Mario Bros. 3 (USA, Europe) (Rev 1)',
              'Supercalifragilisticexpialidocious-Extraordinarily', '(Only Tags)']
    for n, title in enumerate(titles):
        for size in ((88, 45), (74, 66)):
            dump(f'label-{n}-{size[0]}x{size[1]}', c.generated_label(title, size))
    (expected / 'pillow' / 'titles.json').write_text(json.dumps(titles, ensure_ascii=False))


def convert():
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp) / 'card'
        shutil.copytree(card, work)
        c.convert(work, work, work, allow_card_output=True)
        target = expected / 'SYSTEM' / 'SLOT'
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(work / 'SYSTEM' / 'SLOT', target)


make_card()
if expected.exists():
    shutil.rmtree(expected)
primitives()
convert()
import PIL
(expected / 'pillow' / 'version.txt').write_text(PIL.__version__ + '\n')
