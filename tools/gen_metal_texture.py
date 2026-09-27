"""Genera assets/textures/metal_ornamental.png — la textura del material Metal.

Herramienta de autoría, no parte del programa: se corre una vez y el PNG resultante se
commitea como asset. El raytracer solo lo lee (con la crate `image`), igual que el curso
carga las texturas del laberinto. Solo librería estándar (zlib + struct) para escribir
el PNG.

Diseño: latón cepillado (vetas horizontales), bisel claro arriba-izquierda y oscuro
abajo-derecha, remaches en las esquinas y un rombo grabado al centro. Está en tonos
casi neutros: el color dorado real lo pone `Material.diffuse`.
"""

import math
import random
import struct
import zlib
from pathlib import Path

SIZE = 32
OUT = Path(__file__).resolve().parent.parent / "assets" / "textures" / "metal_ornamental.png"


def clamp(v):
    return max(0, min(255, int(v)))


def pixel(x, y, rng, row_streak):
    v = 205 + row_streak[y] + rng.uniform(-6, 6)

    # Bisel: borde de 2 texels, claro arriba/izquierda, oscuro abajo/derecha.
    if x < 2 or y < 2:
        v += 35
    elif x >= SIZE - 2 or y >= SIZE - 2:
        v -= 60

    # Remaches en las cuatro esquinas.
    for cx, cy in ((5, 5), (SIZE - 6, 5), (5, SIZE - 6), (SIZE - 6, SIZE - 6)):
        d = math.hypot(x - cx, y - cy)
        if d < 2.2:
            v = 245 - d * 30

    # Rombo grabado al centro: solo el contorno, hundido (más oscuro).
    diamond = abs(x - SIZE / 2 + 0.5) + abs(y - SIZE / 2 + 0.5)
    if 8.5 < diamond < 10.0:
        v -= 70
    elif diamond < 3.0:
        v += 25

    return clamp(v)


def main():
    rng = random.Random(5005)
    row_streak = [rng.uniform(-18, 18) for _ in range(SIZE)]

    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)  # filtro PNG "None" por fila
        for x in range(SIZE):
            v = pixel(x, y, rng, row_streak)
            raw += bytes((v, v, clamp(v * 0.97)))

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(png)
    print(f"escrito {OUT} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
