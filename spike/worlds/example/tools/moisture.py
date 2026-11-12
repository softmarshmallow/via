"""A generate step: this project's own idea of where the ground is wet.

It is handed the build of the land layer and hands back one field. Nothing here is the compiler's.
"""
import numpy as np
from scipy.ndimage import distance_transform_edt

from viasdk import step

s = step()
height, water = s.build.field("height"), s.build.field("water")
open_water = water > height                                           # the tarn, wherever it settled
beck = np.zeros(s.frame.shape, bool)
for x, y in s.build.features["beck"]["points"]:
    beck[s.frame.index(x, y)] = True
near = distance_transform_edt(~(open_water | beck)) * s.frame.resolution
s.write_field("wet", np.clip(1 - near / float(s.params["reach_m"]), 0, 1))
s.done()
