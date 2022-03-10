"""
Written by Dietrich
Usage: python time.py number_of_iterations [bin_commands ...]
"""

from encodings import utf_8
import sys
import random
import subprocess
import time
import asyncio

def help():
  print("Usage: python time.py number_of_iterations [bin_commands ...]")

# Get results of communicating with the program async
async def write_results(program : subprocess.Popen[bytes], filename : str):
  output = program.communicate()
  output = [str(x) for x in output[0].split("\n")]
  with open(filename, 'w') as f:
    for item in output:
      if ":" not in item:
        continue
      f.write("%s\n"%item.split(":")[1].strip())

# Get the result of a single file
async def process_bin(bin : str, iteration: int):
  loop = asyncio.get_event_loop()
  program = subprocess.Popen(["cargo", "+nightly", "run", "--quiet", "--bin", bin, "resources/scenes/bunnyscene.glb"],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE, encoding="utf8")
  # Actually setup the async operation
  loop.create_task(write_results(program, "results/" + bin + str(iteration) + ".results"))
  time.sleep(5) # Some arbitrary wait time
  program.terminate()

def main():
  if len(sys.argv) < 3 or not sys.argv[1].isdigit():
    help()
    return
  bin_commands = sys.argv[2:]
  bin_commands *= int(sys.argv[1])
  random.shuffle(bin_commands)
  counts = dict()
  for bin in bin_commands:
    if bin not in counts:
      counts[bin] = 0
    else:
      counts[bin] += 1
    print("Processing " + bin)
    asyncio.run(process_bin(bin, counts[bin]))

if __name__ == "__main__":
  main()