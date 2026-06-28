# SPDX-License-Identifier: MIT
# test_bridge.py
# Verifies pyxel_bridge init, load_game, set_input, run_frame, get_framebuffer.

import sys
import pyxel_bridge

print("[test] init...", end=" ")
assert pyxel_bridge.init(), "init() failed"
print("OK")

print("[test] load_game...", end=" ")
assert pyxel_bridge.load_game("test_game.py"), "load_game() failed"
print("OK")

print("[test] set_input + run_frame x3...", end=" ")
for i in range(3):
    pyxel_bridge.set_input(0)   # no buttons pressed
    pyxel_bridge.run_frame()
print("OK")

print("[test] get_framebuffer...", end=" ")
fb = pyxel_bridge.get_framebuffer()
expected_size = pyxel_bridge.SCREEN_W * pyxel_bridge.SCREEN_H * 2
assert len(fb) == expected_size, f"expected {expected_size} bytes, got {len(fb)}"
print(f"OK ({len(fb)} bytes)")

# Spot-check: pixel at (0,0) should be color 11 (light blue) in RGB565
px_low  = fb[0]
px_high = fb[1]
rgb565  = px_low | (px_high << 8)
expected = pyxel_bridge._palette_rgb565[11]
print(f"[test] pixel (0,0) RGB565: {hex(rgb565)} (expected {hex(expected)})", end=" ")
assert rgb565 == expected, "pixel color mismatch"
print("OK")

print("\nAll tests passed!")
