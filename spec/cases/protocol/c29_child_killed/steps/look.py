import os
import subprocess
import time

def lint(step):
    child = subprocess.Popen(["sleep", "120"])
    with open("pids.txt", "w") as f:
        f.write("%d\n%d\n" % (os.getpid(), child.pid))
    while True:
        time.sleep(1)
