# math.py - lr-pyxel stub for Lakka/LibreELEC where math.so cannot be loaded
# Provides the math functions required by Python's random module and common games.
# Functions not yet implemented raise NotImplementedError.

import pyxel as _pyxel

# Constants
pi  = 3.141592653589793
e   = 2.718281828459045
tau = 6.283185307179586
inf = float('inf')
nan = float('nan')

def _deg(rad):
    return rad * 180.0 / pi

def _rad(deg):
    return deg * pi / 180.0

# Trigonometry (pyxel uses degrees internally)
def sin(x):   return _pyxel.sin(_deg(x))
def cos(x):   return _pyxel.cos(_deg(x))
def acos(x):
    # acos via atan2: acos(x) = atan2(sqrt(1-x*x), x)
    return atan2(_pyxel.sqrt(max(0.0, 1.0 - x * x)), x)
def atan(x):  return _rad(_pyxel.atan2(x, 1.0))
def atan2(y, x): return _rad(_pyxel.atan2(y, x))

# Rounding
def floor(x): return _pyxel.floor(x)
def ceil(x):  return _pyxel.ceil(x)
def fabs(x):  return float(abs(x))

# Power / exponential
def sqrt(x):  return _pyxel.sqrt(x)

def exp(x):
    # e^x via Taylor series (sufficient for random module usage)
    if x > 700: return inf
    if x < -700: return 0.0
    result = 1.0
    term = 1.0
    for i in range(1, 50):
        term *= x / i
        result += term
        if abs(term) < 1e-15:
            break
    return result

def log(x, base=e):
    if x <= 0:
        raise ValueError("math domain error")
    # Natural log via series (slow but functional)
    # Use identity: ln(x) = 2 * atanh((x-1)/(x+1))
    # atanh(y) = y + y^3/3 + y^5/5 + ...
    y = (x - 1.0) / (x + 1.0)
    y2 = y * y
    result = 0.0
    term = y
    for i in range(1, 200, 2):
        result += term / i
        term *= y2
        if abs(term / i) < 1e-15:
            break
    result *= 2.0
    if base != e:
        result /= log(base)
    return result

def log2(x):  return log(x) / log(2.0)
def log10(x): return log(x) / log(10.0)

def lgamma(x):
    # Stirling approximation (sufficient for random module)
    if x < 0.5:
        return log(pi / sin(pi * x)) - lgamma(1.0 - x)
    x -= 1
    a = 0.99999999999980993
    coeffs = [
        676.5203681218851, -1259.1392167224028, 771.32342877765313,
        -176.61502916214059, 12.507343278686905, -0.13857109526572012,
        9.9843695780195716e-6, 1.5056327351493116e-7
    ]
    for i, c in enumerate(coeffs):
        a += c / (x + i + 1)
    t = x + len(coeffs) - 0.5
    return 0.5 * log(2 * pi) + (x + 0.5) * log(t) - t + log(a)

def isfinite(x): return not (x == inf or x == -inf or x != x)
def isinf(x):    return x == inf or x == -inf
def isnan(x):    return x != x

def pow(x, y):   return float(x) ** float(y)
def fmod(x, y):  return float(x) % float(y)
def trunc(x):    return int(x)
def copysign(x, y): return abs(x) if y >= 0 else -abs(x)
def hypot(x, y): return sqrt(x*x + y*y)

def gcd(a, b):
    while b:
        a, b = b, a % b
    return abs(a)

def factorial(n):
    if n < 0: raise ValueError("factorial() not defined for negative values")
    result = 1
    for i in range(2, n + 1):
        result *= i
    return result

def comb(n, k):
    if k < 0 or k > n: return 0
    if k == 0 or k == n: return 1
    k = min(k, n - k)
    result = 1
    for i in range(k):
        result = result * (n - i) // (i + 1)
    return result

def perm(n, k=None):
    if k is None: k = n
    if k < 0 or k > n: return 0
    result = 1
    for i in range(n, n - k, -1):
        result *= i
    return result

def degrees(x): return _deg(x)
def radians(x): return _rad(x)

def remainder(x, y): return x - round(x/y) * y

def prod(iterable, start=1):
    result = start
    for x in iterable:
        result *= x
    return result

def sum(iterable, start=0):
    result = start
    for x in iterable:
        result += x
    return result
