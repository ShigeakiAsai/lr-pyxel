# random.py - lr-pyxel stub for Lakka where _random.so cannot be loaded
# Pure Python implementation using os.urandom() as entropy source.
# Does NOT import pyxel to avoid circular dependency issues.

import os as _os
import struct as _struct

class _MT:
    """Minimal Mersenne Twister implementation."""
    N = 624
    M = 397
    MATRIX_A = 0x9908b0df
    UPPER_MASK = 0x80000000
    LOWER_MASK = 0x7fffffff

    def __init__(self):
        self.mt = [0] * self.N
        self.index = self.N + 1
        self.seed(int.from_bytes(_os.urandom(4), 'big'))

    def seed(self, s):
        self.mt[0] = s & 0xffffffff
        for i in range(1, self.N):
            self.mt[i] = (1812433253 * (self.mt[i-1] ^ (self.mt[i-1] >> 30)) + i) & 0xffffffff
        self.index = self.N

    def _generate(self):
        for i in range(self.N):
            y = (self.mt[i] & self.UPPER_MASK) | (self.mt[(i+1) % self.N] & self.LOWER_MASK)
            self.mt[i] = self.mt[(i + self.M) % self.N] ^ (y >> 1)
            if y & 1:
                self.mt[i] ^= self.MATRIX_A
        self.index = 0

    def randint32(self):
        if self.index >= self.N:
            self._generate()
        y = self.mt[self.index]
        self.index += 1
        y ^= y >> 11
        y ^= (y << 7) & 0x9d2c5680
        y ^= (y << 15) & 0xefc60000
        y ^= y >> 18
        return y

    def random(self):
        return self.randint32() / 4294967296.0

_rng = _MT()

def seed(a=None):
    if a is None:
        _rng.seed(int.from_bytes(_os.urandom(4), 'big'))
    else:
        _rng.seed(int(a) & 0xffffffff)

def random():
    return _rng.random()

def uniform(a, b):
    return a + (b - a) * _rng.random()

def randint(a, b):
    return a + int(_rng.random() * (b - a + 1))

def randrange(start, stop=None, step=1):
    if stop is None:
        start, stop = 0, start
    width = stop - start
    if step == 1:
        return start + int(_rng.random() * width)
    n = (width + step - 1) // step
    return start + step * int(_rng.random() * n)

def choice(seq):
    if not seq:
        raise IndexError("Cannot choose from an empty sequence")
    return seq[int(_rng.random() * len(seq))]

def choices(population, weights=None, k=1):
    if weights is None:
        return [choice(population) for _ in range(k)]
    total = sum(weights)
    result = []
    for _ in range(k):
        r = _rng.random() * total
        cumulative = 0.0
        for item, w in zip(population, weights):
            cumulative += w
            if r <= cumulative:
                result.append(item)
                break
    return result

def shuffle(lst):
    n = len(lst)
    for i in range(n - 1, 0, -1):
        j = int(_rng.random() * (i + 1))
        lst[i], lst[j] = lst[j], lst[i]

def sample(population, k):
    n = len(population)
    if k > n:
        raise ValueError("Sample larger than population")
    result = list(population)
    shuffle(result)
    return result[:k]

def getrandbits(k):
    result = 0
    for _ in range(k):
        result = (result << 1) | (_rng.randint32() & 1)
    return result

def triangular(low=0.0, high=1.0, mode=None):
    if mode is None:
        mode = (low + high) / 2.0
    u = _rng.random()
    c = (mode - low) / (high - low)
    if u < c:
        return low + (high - low) * (u * c) ** 0.5
    return high - (high - low) * ((1 - u) * (1 - c)) ** 0.5

def gauss(mu=0.0, sigma=1.0):
    # Box-Muller without math module
    import math as _m
    u1 = max(_rng.random(), 1e-10)
    u2 = _rng.random()
    z = _m.sqrt(-2.0 * _m.log(u1)) * _m.cos(2.0 * _m.pi * u2)
    return mu + sigma * z

def expovariate(lambd):
    import math as _m
    return -_m.log(1.0 - max(_rng.random(), 1e-10)) / lambd

class Random:
    def seed(self, a=None): seed(a)
    def random(self): return random()
    def uniform(self, a, b): return uniform(a, b)
    def randint(self, a, b): return randint(a, b)
    def randrange(self, *args): return randrange(*args)
    def choice(self, seq): return choice(seq)
    def choices(self, population, weights=None, k=1): return choices(population, weights, k)
    def shuffle(self, lst): shuffle(lst)
    def sample(self, population, k): return sample(population, k)
    def gauss(self, mu=0.0, sigma=1.0): return gauss(mu, sigma)
    def getrandbits(self, k): return getrandbits(k)

_inst = Random()
