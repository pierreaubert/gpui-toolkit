"""Regenerate exact-mean expectations using independent rational arithmetic."""

from fractions import Fraction
from pathlib import Path
import random
import struct


def to_bits(value):
    return struct.unpack(">Q", struct.pack(">d", value))[0]


def from_bits(word):
    return struct.unpack(">d", struct.pack(">Q", word))[0]


def main():
    generator = random.Random(20261019)
    cases = []
    for count in [1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128]:
        for mode in range(48):
            data = [
                from_bits(
                    (generator.randrange(2) << 63)
                    | (generator.randrange(2047) << 52)
                    | generator.getrandbits(52)
                )
                for _ in range(count)
            ]
            if mode % 4 == 0:
                for index in range(0, len(data) - 1, 2):
                    data[index + 1] = -data[index]
            elif mode % 4 == 1:
                data = [data[0]] * count
            elif mode % 4 == 2:
                data = [
                    from_bits(generator.randrange(32) | (generator.randrange(2) << 63))
                    for _ in data
                ]
            exact = sum((Fraction.from_float(value) for value in data), Fraction()) / count
            cases.append([to_bits(float(exact)), *map(to_bits, data)])
    root = Path(__file__).resolve().parents[2]
    fixture = root / "crates/gpui-dataset-transformation/tests/fixtures/exact_mean.tsv"
    fixture.parent.mkdir(exist_ok=True)
    fixture.write_text(
        "# Python Fraction oracle, seed 20261019; expected mean bits followed by input bits.\n"
        + "\n".join(" ".join(f"{word:016x}" for word in case) for case in cases)
        + "\n"
    )
    print(f"Generated {len(cases)} independent rational expectations.")


if __name__ == "__main__":
    main()
