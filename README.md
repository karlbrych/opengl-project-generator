# OpenGL Project Generator

`opengl-project-generator` is a desktop app (built with Rust + egui/eframe) that scaffolds starter C++ OpenGL projects.

It lets you choose:
- window backend (GLFW or SDL3)
- OpenGL loader (glad, glad2, or GLEW)
- build system (CMake or Meson)
- optional project extras (Git repo, VS Code files, formatting config, and more)

When you click **Generate project template**, it creates a new project folder (default: `./generated/<project_name>`) with starter source files and build files.

## Prerequisites

- Rust toolchain (stable) with Cargo installed
- Git
- Cmake
- OpenGL development libraries  
- GLFW, GLEW packages 
## Run the project

From the repository root:

1. Build:
   - `cargo build`
2. Run:
   - `cargo run`
3. (Optional) Run tests:
   - `cargo test`

