# random.py - lr-pyxel stub for Lakka where _random.so cannot be loaded
# Implements the subset of random functions used by Pyxel games.
# Uses pyxel's built-in random functions as the underlying generator.

import pyxel as _pyxel

def seed(a=None):
    if a is not None:
        _pyxel.rseed(int(a) & 0xFFFFFFFF)

def random():
    return _pyxel.rndf(0.0, 1.0)

def uniform(a, b):
    return _pyxel.rndf(float(a), float(b))

def randint(a, b):
    return _pyxel.rndi(int(a), int(b))

def randrange(start, stop=None, step=1):
    if stop is None:
        start, stop = 0, start
    width = stop - start
    if step == 1:
        return start + _pyxel.rndi(0, width - 1)
    n = (width + step - 1) // step
    return start + step * _pyxel.rndi(0, n - 1)

def choice(seq):
    if not seq:
        raise IndexError("Cannot choose from an empty sequence")
    return seq[_pyxel.rndi(0, len(seq) - 1)]

def choices(population, weights=None, k=1):
    if weights is None:
        return [choice(population) for _ in range(k)]
    # Weighted selection
    total = sum(weights)
    result = []
    for _ in range(k):
        r = _pyxel.rndf(0.0, total)
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
        j = _pyxel.rndi(0, i)
        lst[i], lst[j] = lst[j], lst[i]

def sample(population, k):
    n = len(population)
    if k > n:
        raise ValueError("Sample larger than population")
    result = list(population)
    shuffle(result)
    return result[:k]

def gauss(mu=0.0, sigma=1.0):
    # Box-Muller transform using pyxel functions only (no math import)
    _pi = 3.141592653589793
    u1 = _pyxel.rndf(1e-10, 1.0)
    u2 = _pyxel.rndf(0.0, 1.0)
    # pyxel.sin/cos use degrees, pyxel.sqrt available
    r = _pyxel.sqrt(-2.0 * _log(u1))
    theta_deg = 2.0 * _pi * u2 * 180.0 / _pi  # convert to degrees
    z = r * _pyxel.cos(theta_deg)
    return mu + sigma * z

def _log(x):
    # Natural log without math module - using pyxel's atan2/sqrt
    if x <= 0: raise ValueError("math domain error")
    # ln(x) = 2 * sum((x-1)/(x+1))^(2k+1) / (2k+1)
    y = (x - 1.0) / (x + 1.0)
    y2 = y * y
    result = 0.0
    term = y
    for i in range(1, 100, 2):
        result += term / i
        term *= y2
        if abs(term / i) < 1e-12:
            break
    return 2.0 * result

def triangular(low=0.0, high=1.0, mode=None):
    if mode is None:
        mode = (low + high) / 2.0
    u = random()
    c = (mode - low) / (high - low)
    if u < c:
        return low + (high - low) * (u * c) ** 0.5
    return high - (high - low) * ((1 - u) * (1 - c)) ** 0.5

def expovariate(lambd):
    u = _pyxel.rndf(0.0, 1.0 - 1e-10)
    return -_log(1.0 - u) / lambd

def getrandbits(k):
    result = 0
    for _ in range(k):
        result = (result << 1) | _pyxel.rndi(0, 1)
    return result

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
