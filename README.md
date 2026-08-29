# OpenGL Project Generator

`opengl-project-generator` is a desktop app (built with Rust + egui/eframe) that scaffolds starter C++ OpenGL projects.

It lets you choose:
- window backend (GLFW or SDL3)
- OpenGL loader (glad, glad2, or GLEW)
- build system (CMake or Meson)
- optional project extras (Git repo, VS Code files, formatting config, and more)

When you click **Generate project template**, it creates a new project folder (default: `./generated/<project_name>`) with starter source files and build files.

## Prerequisites

To run the generator:

- Rust toolchain (stable) with Cargo installed
- Git (used to fetch glad into the generated project)

To build a generated project:

- CMake 3.21 or newer (or Meson) and a C++ compiler
- OpenGL development libraries
- The window backend you picked: GLFW or SDL3
- The loader you picked:
  - `glad` - Python 3 on PATH (standard library only)
  - `glad2` - Python 3 on PATH plus the Jinja2 module (`python -m pip install --user jinja2`)
  - `GLEW` - GLEW development libraries

Each generated project's CMakeLists.txt checks for these at configure time and
prints install instructions if something is missing. Optional libraries (GLM,
fmt, ImGui, ...) are linked when found and skipped when not, so they never break
the build.
## Run the project

From the repository root:

1. Build:
   - `cargo build`
2. Run:
   - `cargo run`
3. (Optional) Run tests:
   - `cargo test`

