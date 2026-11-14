import os
import time

def lint(step):
    with open("pids.txt", "w") as f:
        f.write("%d\n" % os.getpid())
    while True:
        time.sleep(1)  # never calls the engine, so it never sees $/cancelRequest
