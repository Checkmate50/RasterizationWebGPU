# CS5626 Rasterization with WebGPU

I've elected to do this project with WebGPU, for some reason. It's been pretty fun so far. This actually seems like it's going better so far than the ray tracing in Rust project. WebGPU is more low level than OpenGL, but I think it's useful to play around with a more modern standard.

## Building
To run this, literally all you need to do is be in the correct directory and do `cargo +nightly run --bin 'name_of_bin' resources/scenes/'name_of_scene'.gdb` (as this is on nightly Rust).

## New Test
Create a fresh folder and add its name (after src_) to `Cargo.toml` and `bins.txt`

## Timing and Graphing
`time.py` is the timing script, and should be run with a number of iterations.  `bins.txt` is the list of binaries to be timed and graphed.  `graph.py` is the script to actually graph the data -- provide the same count as an argument to time.py as to graph.py, and you should be good to go
