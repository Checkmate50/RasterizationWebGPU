"""
Written by Dietrich
Usage: python time.py number_of_iterations [bin_commands ...]
"""

from encodings import utf_8
from msilib.schema import File
import sys
import random
import subprocess
import time
import asyncio
import typing
import datetime

def help():
  print("Usage: python time.py number_of_iterations")

# Get results of communicating with the program async
async def write_results(program : subprocess.Popen[bytes], file : typing.IO):
  output = program.communicate()
  output = [str(x) for x in output[0].split("\n")]
  for item in output:
    item = item.strip()
    if len(item) == 0:
      continue
    file.write("%s\n"%item)

# Get the result of a single file
async def process_bin(bin : str, file : typing.IO, sleepies : int):
  loop = asyncio.get_event_loop()
  program = subprocess.Popen(["cargo", "+nightly", "run", "--quiet", "--bin", bin, "resources/scenes/bunnyscene.glb"],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE, encoding="utf8")
  # Actually setup the async operation
  file.write("---" + bin + str(sleepies) + "---\n")
  loop.create_task(write_results(program, file))
  time.sleep(sleepies) # Some arbitrary wait time
  program.terminate()

def main():
  if len(sys.argv) < 2 or not sys.argv[1].isdigit():
    help()
    return
  bin_commands = []
  with open("bins.txt", "r") as f:
    for line in f:
      bin_commands.append(line.strip())
  bin_commands *= int(sys.argv[1])
  random.shuffle(bin_commands)
  filename = datetime.datetime.now().strftime(f"%Y-%m-%d-%H-%m")
  with open("results/" + filename + ".result", "w") as file:
    for bin in bin_commands:
      for sleepies in range(50, 501, 50):
        print("Processing " + bin + str(sleepies))
        asyncio.run(process_bin(bin, file, sleepies))

if __name__ == "__main__":
  main()