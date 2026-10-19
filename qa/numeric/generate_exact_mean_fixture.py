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
    # Ordinary exponent ranges exercise the small accumulator.
    for count in [1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128]:
        for _ in range(16):
            center = generator.randrange(200, 1800)
            data = [from_bits((generator.randrange(2) << 63)
                              | ((center + generator.randrange(-32, 33)) << 52)
                              | generator.getrandbits(52)) for _ in range(count)]
            exact = sum((Fraction.from_float(value) for value in data), Fraction()) / count
            cases.append([to_bits(float(exact)), *map(to_bits, data)])
    # Quantized ordinary values have an exact floating sum within 53 bits.
    for count in [1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128]:
        for _ in range(8):
            data = [from_bits((generator.randrange(2) << 63)
                              | ((1023 + generator.randrange(-8, 9)) << 52)
                              | (generator.getrandbits(23) << 29)) for _ in range(count)]
            exact = sum((Fraction.from_float(value) for value in data), Fraction()) / count
            cases.append([to_bits(float(exact)), *map(to_bits, data)])
    # Test either side of the 128-bit certificate, including carries near capacity.
    for count in [2, 3, 8, 64, 128]:
        growth = (count - 1).bit_length()
        for delta in [-1, 0, 1]:
            spread = 128 - 53 - growth + delta
            high = from_bits(((1023 + spread) << 52) | ((1 << 52) - 1))
            low = from_bits((1023 << 52) | ((1 << 52) - 1))
            for pattern in range(3):
                data = [high if pattern == 0 or index % 2 == 0 else -high
                        for index in range(count - 1)] + [low if pattern != 2 else -low]
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
