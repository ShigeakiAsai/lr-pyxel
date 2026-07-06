# struct.py - lr-pyxel stub for Lakka where _struct.so cannot be loaded
# Implements the subset used by base64.py (and urllib.request via base64)

import sys

class error(Exception):
    pass

class Struct:
    def __init__(self, fmt):
        self.format = fmt
        self.size = self._calc_size(fmt)

    def _calc_size(self, fmt):
        # Parse format string to calculate size
        size = 0
        i = 0
        count = 0
        # Skip byte order prefix
        if fmt and fmt[0] in '@=<>!':
            i = 1
        while i < len(fmt):
            c = fmt[i]
            if c.isdigit():
                count = count * 10 + int(c)
            else:
                n = max(count, 1)
                count = 0
                sizes = {
                    'x': 1, 'c': 1, 'b': 1, 'B': 1, '?': 1,
                    'h': 2, 'H': 2, 'i': 4, 'I': 4, 'l': 4, 'L': 4,
                    'q': 8, 'Q': 8, 'f': 4, 'd': 8, 'n': 8, 'N': 8,
                    'P': 8, 's': 1, 'p': 1,
                }
                size += sizes.get(c, 0) * n
            i += 1
        return size

    def pack(self, *args):
        return pack(self.format, *args)

    def unpack(self, buffer):
        return unpack(self.format, buffer)

    def unpack_from(self, buffer, offset=0):
        return unpack_from(self.format, buffer, offset)

def _parse_fmt(fmt):
    """Parse format string into list of (count, code) tuples."""
    result = []
    i = 0
    big_endian = True
    if fmt and fmt[0] in '@=<>!':
        big_endian = fmt[0] in '>!'
        i = 1
    count = 0
    while i < len(fmt):
        c = fmt[i]
        if c.isdigit():
            count = count * 10 + int(c)
        else:
            result.append((max(count, 1), c, big_endian))
            count = 0
        i += 1
    return result

def pack(fmt, *args):
    result = b''
    items = _parse_fmt(fmt)
    arg_idx = 0
    for count, code, big in items:
        for _ in range(count):
            v = args[arg_idx] if arg_idx < len(args) else 0
            arg_idx += 1
            if code == 'I' or code == 'L':
                result += v.to_bytes(4, 'big' if big else 'little')
            elif code == 'i' or code == 'l':
                result += v.to_bytes(4, 'big' if big else 'little', signed=True)
            elif code == 'H':
                result += v.to_bytes(2, 'big' if big else 'little')
            elif code == 'h':
                result += v.to_bytes(2, 'big' if big else 'little', signed=True)
            elif code == 'B':
                result += bytes([v & 0xFF])
            elif code == 'b':
                result += bytes([v & 0xFF])
            elif code == 'Q':
                result += v.to_bytes(8, 'big' if big else 'little')
            elif code == 'q':
                result += v.to_bytes(8, 'big' if big else 'little', signed=True)
            elif code == 'c':
                result += v if isinstance(v, bytes) else bytes([v])
            elif code == 's':
                result += v[:count].ljust(count, b'\x00')
                break
            else:
                result += b'\x00' * 4
    return result

def unpack(fmt, buffer):
    items = _parse_fmt(fmt)
    result = []
    offset = 0
    for count, code, big in items:
        for _ in range(count):
            if code == 'I' or code == 'L':
                v = int.from_bytes(buffer[offset:offset+4], 'big' if big else 'little')
                offset += 4
            elif code == 'i' or code == 'l':
                v = int.from_bytes(buffer[offset:offset+4], 'big' if big else 'little', signed=True)
                offset += 4
            elif code == 'H':
                v = int.from_bytes(buffer[offset:offset+2], 'big' if big else 'little')
                offset += 2
            elif code == 'h':
                v = int.from_bytes(buffer[offset:offset+2], 'big' if big else 'little', signed=True)
                offset += 2
            elif code == 'B':
                v = buffer[offset]
                offset += 1
            elif code == 'b':
                v = buffer[offset] if buffer[offset] < 128 else buffer[offset] - 256
                offset += 1
            elif code == 'Q':
                v = int.from_bytes(buffer[offset:offset+8], 'big' if big else 'little')
                offset += 8
            elif code == 'q':
                v = int.from_bytes(buffer[offset:offset+8], 'big' if big else 'little', signed=True)
                offset += 8
            else:
                v = 0
                offset += 4
            result.append(v)
    return tuple(result)

def unpack_from(fmt, buffer, offset=0):
    return unpack(fmt, buffer[offset:])

def calcsize(fmt):
    return Struct(fmt).size
