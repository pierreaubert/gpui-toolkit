"""Generate exact weighted mean bits from independent Python Fraction arithmetic."""

from fractions import Fraction
from pathlib import Path
import random
import struct


def bits(value):
    return struct.unpack(">Q", struct.pack(">d", value))[0]


def floating(word):
    return struct.unpack(">d", struct.pack(">Q", word))[0]


def main():
    randomizer = random.Random(20261020)
    cases = []

    def emit(pairs):
        total = sum((Fraction.from_float(v) * Fraction.from_float(w) for v, w in pairs), Fraction())
        weight = sum((Fraction.from_float(w) for _, w in pairs), Fraction())
        cases.append([bits(float(total / weight)), *(bits(value) for pair in pairs for value in pair)])

    tiny = floating(1)
    largest = floating(0x7fefffffffffffff)
    for count in [1, 2, 3, 4, 8, 15, 32, 64]:
        for mode in range(72):
            pairs = []
            for index in range(count):
                if mode < 24:
                    value_exponent, weight_exponent = randomizer.randrange(2047), randomizer.randrange(2047)
                else:
                    value_exponent = 1023 + randomizer.randrange(-8, 9)
                    weight_exponent = 1023 + randomizer.randrange(-8, 9)
                value = floating((randomizer.randrange(2) << 63) | (value_exponent << 52) | randomizer.getrandbits(52))
                weight = floating((weight_exponent << 52) | randomizer.getrandbits(52))
                if mode % 6 == 0:
                    weight = 1.0
                elif mode % 6 == 1:
                    value = [tiny, -tiny, 0.0, -0.0][index % 4]
                elif mode % 6 == 2:
                    weight = [tiny, largest, 1.0][index % 3]
                elif mode % 6 == 3:
                    value = largest
                elif mode % 6 == 4 and index % 2 == 1:
                    value, weight = -pairs[-1][0], pairs[-1][1]
                pairs.append((value, weight))
            emit(pairs)
    for pairs in [
        [(1e16, 1.0), (3.0, 1.0), (-1e16, 1.0)],
        [(largest, largest), (-largest, largest), (1e-300, largest)],
        [(largest, tiny), (-largest, tiny), (tiny, 1.0)],
        [(tiny, 1.0), (0.0, 1.0)],
        [(-tiny, 1.0), (0.0, 1.0)],
        [(tiny, 1.0), (tiny, 2.0)],
        [(-0.0, largest)],
        [(0.0, tiny)],
    ]:
        emit(pairs)
    # Saturated u128 denominators exercise remainder-bit division without room
    # to shift the numerator. Extreme products exercise wide signed underflow.
    low = floating((1023 << 52) | ((1 << 52) - 1))
    high = floating(((1023 + 74) << 52) | ((1 << 52) - 1))
    for pairs in [
        [(1.0, high), (-1.0, high), (1.0, low)],
        [(1.0, high), (-1.0, high), (-1.0, low)],
        [(largest, largest), (tiny, tiny), (-largest, largest)],
        [(largest, largest), (-tiny, tiny), (-largest, largest)],
        [(largest, largest), (-largest, largest), (-tiny, largest)],
        [(largest, 1.0), (1e-300, 2.0), (-largest, 1.0)],
    ]:
        emit(pairs)
    root = Path(__file__).resolve().parents[2]
    target = root / "crates/gpui-dataset-transformation/tests/fixtures/weighted_mean.tsv"
    target.write_text("# Python Fraction weighted mean oracle, seed 20261020; expected bits, then value/weight pairs.\n" + "\n".join(" ".join(f"{word:016x}" for word in row) for row in cases) + "\n")
    print(f"Generated {len(cases)} independent weighted expectations.")


if __name__ == "__main__":
    main()
