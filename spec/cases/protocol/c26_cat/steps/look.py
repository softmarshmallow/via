import subprocess

def lint(step):
    got = subprocess.run(["cat"], capture_output=True, timeout=20)
    step.info("cat", "step.look", "cat ran", "nothing to do", data={"bytes": len(got.stdout)},
              explain="cat read the null device, never a protocol frame.")
