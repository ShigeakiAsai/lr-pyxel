# math.py - lr-pyxel stub for Lakka/LibreELEC where math.so cannot be loaded
# Pure Python implementation - no external dependencies

pi  = 3.141592653589793
e   = 2.718281828459045
tau = 6.283185307179586
inf = float('inf')
nan = float('nan')

def _horner(coeffs, x):
    result = 0.0
    for c in coeffs:
        result = result * x + c
    return result

def sqrt(x):
    if x < 0: raise ValueError("math domain error")
    if x == 0: return 0.0
    y = float(x)
    # Newton-Raphson
    g = y / 2.0
    for _ in range(50):
        g_new = (g + y / g) / 2.0
        if abs(g_new - g) < 1e-15 * g:
            break
        g = g_new
    return g_new

def _reduce_angle(x):
    """Reduce x to [-pi/2, pi/2] range, return (reduced, sign)"""
    x = fmod(x, tau)
    if x < 0: x += tau
    if x > pi: x -= tau
    return x

def sin(x):
    x = fmod(x, tau)
    if x < 0: x += tau
    if x > pi: x -= tau
    # sin via Taylor series
    result = 0.0
    term = x
    x2 = x * x
    for i in range(1, 15):
        result += term
        term *= -x2 / ((2*i) * (2*i+1))
    return result

def cos(x):
    return sin(x + pi/2)

def tan(x):
    c = cos(x)
    if abs(c) < 1e-15: raise ValueError("math domain error")
    return sin(x) / c

def asin(x):
    if abs(x) > 1: raise ValueError("math domain error")
    if abs(x) == 1: return pi/2 * (1 if x > 0 else -1)
    return atan2(x, sqrt(1 - x*x))

def acos(x):
    if abs(x) > 1: raise ValueError("math domain error")
    return pi/2 - asin(x)

def atan(x):
    return atan2(x, 1.0)

def atan2(y, x):
    if x == 0:
        if y == 0: return 0.0
        return pi/2 if y > 0 else -pi/2
    if abs(x) > abs(y):
        t = y / x
        # atan via series for |t| <= 1
        result = 0.0
        term = t
        t2 = t * t
        for i in range(20):
            result += term / (2*i+1) * ((-1)**i)
            term *= t2
        return result if x > 0 else (result + pi if y >= 0 else result - pi)
    else:
        return pi/2 - atan2(x, y) if y > 0 else -pi/2 - atan2(x, y)

def exp(x):
    if x > 700: return inf
    if x < -700: return 0.0
    result = 1.0
    term = 1.0
    for i in range(1, 50):
        term *= x / i
        result += term
        if abs(term) < 1e-15: break
    return result

def log(x, base=e):
    if x <= 0: raise ValueError("math domain error")
    if x == 1: return 0.0
    # ln via atanh series: ln(x) = 2*atanh((x-1)/(x+1))
    y = (x - 1.0) / (x + 1.0)
    y2 = y * y
    result = 0.0
    term = y
    for i in range(1, 200, 2):
        result += term / i
        term *= y2
        if abs(term / i) < 1e-15: break
    result *= 2.0
    if base != e:
        result /= log(base)
    return result

def log2(x):  return log(x) / log(2.0)
def log10(x): return log(x) / log(10.0)

def pow(x, y):   return float(x) ** float(y)
def fabs(x):     return float(abs(x))
def floor(x):    return int(x) if x >= 0 else int(x) - (1 if x != int(x) else 0)
def ceil(x):     return int(x) if x <= 0 else int(x) + (1 if x != int(x) else 0)
def trunc(x):    return int(x)
def fmod(x, y):  return float(x) % float(y)
def remainder(x, y): return x - round(x/y) * y
def copysign(x, y): return abs(x) if y >= 0 else -abs(x)
def hypot(x, y): return sqrt(x*x + y*y)
def isfinite(x): return not (x == inf or x == -inf or x != x)
def isinf(x):    return x == inf or x == -inf
def isnan(x):    return x != x
def degrees(x):  return x * 180.0 / pi
def radians(x):  return x * pi / 180.0

def lgamma(x):
    if x < 0.5:
        return log(pi / sin(pi * x)) - lgamma(1.0 - x)
    x -= 1
    a = 0.99999999999980993
    coeffs = [676.5203681218851, -1259.1392167224028, 771.32342877765313,
              -176.61502916214059, 12.507343278686905, -0.13857109526572012,
              9.9843695780195716e-6, 1.5056327351493116e-7]
    for i, c in enumerate(coeffs):
        a += c / (x + i + 1)
    t = x + len(coeffs) - 0.5
    return 0.5 * log(2 * pi) + (x + 0.5) * log(t) - t + log(a)

def gcd(a, b):
    while b: a, b = b, a % b
    return abs(a)

def factorial(n):
    if n < 0: raise ValueError("factorial() not defined for negative values")
    result = 1
    for i in range(2, n + 1): result *= i
    return result

def comb(n, k):
    if k < 0 or k > n: return 0
    if k == 0 or k == n: return 1
    k = min(k, n - k)
    result = 1
    for i in range(k): result = result * (n - i) // (i + 1)
    return result

def prod(iterable, start=1):
    result = start
    for x in iterable: result *= x
    return result
