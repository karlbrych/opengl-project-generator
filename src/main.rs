use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Project Generator for OpenGL",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowBackend {
    Glfw,
    Sdl3,
}

impl WindowBackend {
    const ALL: [Self; 2] = [Self::Glfw, Self::Sdl3];

    fn as_str(self) -> &'static str {
        match self {
            Self::Glfw => "GLFW",
            Self::Sdl3 => "SDL3",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlLoader {
    Glad,
    Glad2,
    Glew,
}

impl GlLoader {
    const ALL: [Self; 3] = [Self::Glad, Self::Glad2, Self::Glew];

    fn as_str(self) -> &'static str {
        match self {
            Self::Glad => "glad",
            Self::Glad2 => "glad2",
            Self::Glew => "GLEW",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuildSystem {
    CMake,
    Meson,
}

impl BuildSystem {
    const ALL: [Self; 2] = [Self::CMake, Self::Meson];

    fn as_str(self) -> &'static str {
        match self {
            Self::CMake => "CMake",
            Self::Meson => "Meson",
        }
    }
}

#[derive(Clone, Debug)]
struct App {
    // Project metadata
    project_name: String,
    output_directory: String,
    cpp_standard: String,

    // OpenGL setup
    opengl_major: u8,
    opengl_minor: u8,
    core_profile: bool,
    debug_context: bool,
    srgb_framebuffer: bool,

    // Backend and loader choices
    window_backend: WindowBackend,
    gl_loader: GlLoader,
    build_system: BuildSystem,

    // Optional libraries/features to scaffold
    include_glm: bool,
    include_imgui: bool,
    include_stb_image: bool,
    include_assimp: bool,
    include_spdlog: bool,
    include_fmt: bool,

    // Project extras
    create_git_repo: bool,
    include_gitignore: bool,
    include_clang_format: bool,
    include_cmake_presets: bool,
    include_vscode_files: bool,

    // UX state
    status_message: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            project_name: "my_opengl_app".to_owned(),
            output_directory: "./generated".to_owned(),
            cpp_standard: "c++20".to_owned(),
            opengl_major: 4,
            opengl_minor: 6,
            core_profile: true,
            debug_context: true,
            srgb_framebuffer: true,
            window_backend: WindowBackend::Glfw,
            gl_loader: GlLoader::Glad,
            build_system: BuildSystem::CMake,
            include_glm: true,
            include_imgui: false,
            include_stb_image: true,
            include_assimp: false,
            include_spdlog: false,
            include_fmt: true,
            create_git_repo: true,
            include_gitignore: true,
            include_clang_format: true,
            include_cmake_presets: true,
            include_vscode_files: true,
            status_message: "Ready to generate project template".to_owned(),
        }
    }
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn generate_project(&mut self) {
        match self.generate_project_impl() {
            Ok(message) => self.status_message = message,
            Err(err) => self.status_message = format!("Generation failed: {err}"),
        }
    }

    fn generate_project_impl(&self) -> Result<String, String> {
        let project_name = sanitize_name(&self.project_name);
        if project_name.is_empty() {
            return Err("Project name is empty after sanitization".to_owned());
        }

        let root = self.project_root(&project_name);
        fs::create_dir_all(&root)
            .map_err(|e| format!("Could not create project root '{}': {e}", root.display()))?;

        self.create_layout(&root)?;
        self.write_project_files(&root, &project_name)?;
        self.setup_loader_dependencies(&root)?;

        if self.create_git_repo {
            self.try_git_init(&root);
        }

        Ok(format!(
            "Generated '{}' at '{}' with {} + {}",
            project_name,
            root.display(),
            self.window_backend.as_str(),
            self.gl_loader.as_str()
        ))
    }

    fn project_root(&self, project_name: &str) -> PathBuf {
        let output_dir = self.output_directory.trim();
        let base = if output_dir.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(output_dir)
        };
        base.join(project_name)
    }

    fn create_layout(&self, root: &Path) -> Result<(), String> {
        let mut dirs = vec![
            root.join("src"),
            root.join("include"),
            root.join("assets"),
            root.join("assets/shaders"),
            root.join("assets/textures"),
            root.join("cmake"),
            root.join("third_party"),
        ];

        if self.include_vscode_files {
            dirs.push(root.join(".vscode"));
        }

        for dir in dirs {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("Could not create directory '{}': {e}", dir.display()))?;
        }

        Ok(())
    }

    fn write_project_files(&self, root: &Path, project_name: &str) -> Result<(), String> {
        write_text(
            &root.join("src/main.cpp"),
            &self.main_cpp_template(project_name),
        )?;
        write_text(
            &root.join("README.md"),
            &self.readme_template(project_name),
        )?;
        write_text(
            &root.join("third_party/README.md"),
            &self.third_party_readme(),
        )?;

        match self.build_system {
            BuildSystem::CMake => {
                write_text(&root.join("CMakeLists.txt"), &self.cmake_template(project_name))?;
                if self.include_cmake_presets {
                    write_text(
                        &root.join("CMakePresets.json"),
                        &self.cmake_presets_template(),
                    )?;
                }
            }
            BuildSystem::Meson => {
                write_text(&root.join("meson.build"), &self.meson_template(project_name))?;
            }
        }

        if self.include_gitignore {
            write_text(&root.join(".gitignore"), &gitignore_template())?;
        }

        if self.include_clang_format {
            write_text(&root.join(".clang-format"), &clang_format_template())?;
        }

        if self.include_vscode_files {
            write_text(&root.join(".vscode/tasks.json"), &self.vscode_tasks_template())?;
            write_text(
                &root.join(".vscode/launch.json"),
                &self.vscode_launch_template(project_name),
            )?;
        }

        Ok(())
    }

    fn try_git_init(&self, root: &Path) {
        let _ = Command::new("git")
            .arg("init")
            .current_dir(root)
            .status();
    }

    fn constrain_opengl_version(&mut self) {
        let max_minor = if self.opengl_major == 3 { 3 } else { 6 };
        self.opengl_minor = self.opengl_minor.min(max_minor);
    }

    fn setup_loader_dependencies(&self, root: &Path) -> Result<(), String> {
        match self.gl_loader {
            GlLoader::Glad | GlLoader::Glad2 => self.clone_glad_repo(root),
            GlLoader::Glew => Ok(()),
        }
    }

    fn clone_glad_repo(&self, root: &Path) -> Result<(), String> {
        let glad_dir = root.join("third_party/glad");
        if glad_dir.exists() {
            return Ok(());
        }

        // The repository's default branch is glad2, so glad 0.1 has to be asked
        // for by name -- otherwise the sources and the generated CMake disagree
        // about which header (glad/glad.h vs glad/gl.h) exists.
        let branch = match self.gl_loader {
            GlLoader::Glad2 => "glad2",
            _ => "master",
        };

        let status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                branch,
                "https://github.com/Dav1dde/glad.git",
                "third_party/glad",
            ])
            .current_dir(root)
            .status()
            .map_err(|e| format!("Could not clone glad repository: {e}"))?;

        if !status.success() {
            return Err("git clone for glad failed".to_owned());
        }

        // Vendor it: without dropping the nested .git, the surrounding project
        // sees third_party/glad as an unusable embedded repository.
        let _ = fs::remove_dir_all(glad_dir.join(".git"));

        Ok(())
    }

    fn main_cpp_template(&self, project_name: &str) -> String {
        let loader_include = match self.gl_loader {
            GlLoader::Glad => "#include <glad/glad.h>",
            GlLoader::Glad2 => "#include <glad/gl.h>",
            GlLoader::Glew => "#include <GL/glew.h>",
        };

        let profile_hint = if self.core_profile {
            "GLFW_OPENGL_CORE_PROFILE"
        } else {
            "GLFW_OPENGL_COMPAT_PROFILE"
        };

        match self.window_backend {
            WindowBackend::Glfw => format!(
                "#include <iostream>\n\n{loader_include}\n#include <GLFW/glfw3.h>\n\nint main() {{\n    if (!glfwInit()) {{\n        std::cerr << \"Failed to initialize GLFW\\n\";\n        return -1;\n    }}\n\n    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, {major});\n    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, {minor});\n    glfwWindowHint(GLFW_OPENGL_PROFILE, {profile_hint});\n    glfwWindowHint(GLFW_OPENGL_DEBUG_CONTEXT, {debug_hint});\n    glfwWindowHint(GLFW_SRGB_CAPABLE, {srgb_hint});\n\n    GLFWwindow* window = glfwCreateWindow(1280, 720, \"{project_name}\", nullptr, nullptr);\n    if (!window) {{\n        std::cerr << \"Failed to create GLFW window\\n\";\n        glfwTerminate();\n        return -1;\n    }}\n\n    glfwMakeContextCurrent(window);\n\n{loader_init}\n\n    while (!glfwWindowShouldClose(window)) {{\n        glClearColor(1.0f, 1.0f, 1.0f, 1.0f);\n        glClear(GL_COLOR_BUFFER_BIT);\n\n        glfwSwapBuffers(window);\n        glfwPollEvents();\n    }}\n\n    glfwDestroyWindow(window);\n    glfwTerminate();\n    return 0;\n}}\n",
                major = self.opengl_major,
                minor = self.opengl_minor,
                profile_hint = profile_hint,
                debug_hint = if self.debug_context { "GLFW_TRUE" } else { "GLFW_FALSE" },
                srgb_hint = if self.srgb_framebuffer { "GLFW_TRUE" } else { "GLFW_FALSE" },
                loader_init = self.glfw_loader_init(),
                project_name = project_name,
            ),
            WindowBackend::Sdl3 => format!(
                "#include <iostream>\n\n{loader_include}\n#include <SDL3/SDL.h>\n\nint main() {{\n    if (!SDL_Init(SDL_INIT_VIDEO)) {{\n        std::cerr << \"Failed to initialize SDL3: \" << SDL_GetError() << \"\\n\";\n        return -1;\n    }}\n\n    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, {major});\n    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, {minor});\n    SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, {profile_hint});\n    SDL_GL_SetAttribute(SDL_GL_FRAMEBUFFER_SRGB_CAPABLE, {srgb_hint});\n\n    SDL_Window* window = SDL_CreateWindow(\"{project_name}\", 1280, 720, SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE);\n    if (!window) {{\n        std::cerr << \"Failed to create SDL3 window: \" << SDL_GetError() << \"\\n\";\n        SDL_Quit();\n        return -1;\n    }}\n\n    SDL_GLContext gl_context = SDL_GL_CreateContext(window);\n    if (!gl_context) {{\n        std::cerr << \"Failed to create OpenGL context: \" << SDL_GetError() << \"\\n\";\n        SDL_DestroyWindow(window);\n        SDL_Quit();\n        return -1;\n    }}\n\n{loader_init}\n\n    bool running = true;\n    while (running) {{\n        SDL_Event event;\n        while (SDL_PollEvent(&event)) {{\n            if (event.type == SDL_EVENT_QUIT) {{\n                running = false;\n            }}\n        }}\n\n        glClearColor(1.0f, 1.0f, 1.0f, 1.0f);\n        glClear(GL_COLOR_BUFFER_BIT);\n        SDL_GL_SwapWindow(window);\n    }}\n\n    SDL_GL_DestroyContext(gl_context);\n    SDL_DestroyWindow(window);\n    SDL_Quit();\n    return 0;\n}}\n",
                major = self.opengl_major,
                minor = self.opengl_minor,
                profile_hint = if self.core_profile {
                    "SDL_GL_CONTEXT_PROFILE_CORE"
                } else {
                    "SDL_GL_CONTEXT_PROFILE_COMPATIBILITY"
                },
                srgb_hint = if self.srgb_framebuffer { 1 } else { 0 },
                loader_init = self.sdl_loader_init(),
                project_name = project_name,
            ),
        }
    }

    fn glfw_loader_init(&self) -> &'static str {
        match self.gl_loader {
            GlLoader::Glad => {
                "    if (!gladLoadGLLoader((GLADloadproc)glfwGetProcAddress)) {\n        std::cerr << \"Failed to initialize glad\\n\";\n        return -1;\n    }"
            }
            GlLoader::Glad2 => {
                "    if (!gladLoadGL((GLADloadfunc)glfwGetProcAddress)) {\n        std::cerr << \"Failed to initialize glad2\\n\";\n        return -1;\n    }"
            }
            GlLoader::Glew => {
                "    glewExperimental = GL_TRUE;\n    const GLenum glew_status = glewInit();\n#ifdef GLEW_ERROR_NO_GLX_DISPLAY\n    // GLEW reports this on Wayland even though the context is perfectly usable.\n    const bool glew_ok = glew_status == GLEW_OK || glew_status == GLEW_ERROR_NO_GLX_DISPLAY;\n#else\n    const bool glew_ok = glew_status == GLEW_OK;\n#endif\n    if (!glew_ok) {\n        std::cerr << \"Failed to initialize GLEW: \" << glewGetErrorString(glew_status) << \"\\n\";\n        return -1;\n    }"
            }
        }
    }

    fn sdl_loader_init(&self) -> &'static str {
        match self.gl_loader {
            GlLoader::Glad => {
                "    if (!gladLoadGLLoader((GLADloadproc)SDL_GL_GetProcAddress)) {\n        std::cerr << \"Failed to initialize glad\\n\";\n        return -1;\n    }"
            }
            GlLoader::Glad2 => {
                "    if (!gladLoadGL((GLADloadfunc)SDL_GL_GetProcAddress)) {\n        std::cerr << \"Failed to initialize glad2\\n\";\n        return -1;\n    }"
            }
            GlLoader::Glew => {
                "    glewExperimental = GL_TRUE;\n    const GLenum glew_status = glewInit();\n#ifdef GLEW_ERROR_NO_GLX_DISPLAY\n    // GLEW reports this on Wayland even though the context is perfectly usable.\n    const bool glew_ok = glew_status == GLEW_OK || glew_status == GLEW_ERROR_NO_GLX_DISPLAY;\n#else\n    const bool glew_ok = glew_status == GLEW_OK;\n#endif\n    if (!glew_ok) {\n        std::cerr << \"Failed to initialize GLEW: \" << glewGetErrorString(glew_status) << \"\\n\";\n        return -1;\n    }"
            }
        }
    }

    fn cmake_template(&self, project_name: &str) -> String {
        const TEMPLATE: &str = r##"cmake_minimum_required(VERSION 3.21)
project(@PROJECT_NAME@ LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD @CXX_STANDARD@)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Single-config generators (Makefiles, Ninja) start with no build type at all.
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE Debug CACHE STRING "Build type" FORCE)
endif()

# Keep the executable in one predictable place across generators, so the
# debugger configs and the asset copy below always agree on the path.
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
foreach(output_config IN ITEMS DEBUG RELEASE RELWITHDEBINFO MINSIZEREL)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_${output_config} "${CMAKE_BINARY_DIR}/bin")
endforeach()

# Used as a fallback everywhere a package ships only a .pc file.
find_package(PkgConfig QUIET)

# ---------------------------------------------------------------- OpenGL
set(OpenGL_GL_PREFERENCE GLVND)
find_package(OpenGL REQUIRED)

# ---------------------------------------------------------------- Window backend
@BACKEND_BLOCK@
# ---------------------------------------------------------------- OpenGL loader
@LOADER_BLOCK@
# ---------------------------------------------------------------- Application
file(GLOB_RECURSE APP_SOURCES CONFIGURE_DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp"
    "${CMAKE_CURRENT_SOURCE_DIR}/src/*.c"
)

add_executable(${PROJECT_NAME} ${APP_SOURCES})

target_include_directories(${PROJECT_NAME} PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/include")

target_link_libraries(${PROJECT_NAME}
    PRIVATE
        OpenGL::GL
        ${BACKEND_LIB}
        ${LOADER_LIB}
)

if(MSVC)
    target_compile_options(${PROJECT_NAME} PRIVATE /W4 /permissive-)
else()
    target_compile_options(${PROJECT_NAME} PRIVATE -Wall -Wextra -Wpedantic)
endif()
@OPTIONAL_BLOCK@
# ---------------------------------------------------------------- Assets
# Shaders and textures are loaded relative to the executable at runtime.
add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            "${CMAKE_CURRENT_SOURCE_DIR}/assets"
            "$<TARGET_FILE_DIR:${PROJECT_NAME}>/assets"
    COMMENT "Copying assets next to the executable"
)
"##;

        TEMPLATE
            .replace("@PROJECT_NAME@", project_name)
            .replace("@CXX_STANDARD@", &parse_cpp_standard(&self.cpp_standard).to_string())
            .replace("@BACKEND_BLOCK@", &self.cmake_backend_block())
            .replace("@LOADER_BLOCK@", &self.cmake_loader_block())
            .replace("@OPTIONAL_BLOCK@", &self.cmake_optional_block())
    }

    /// Resolves the window backend through its CMake config package first and
    /// pkg-config second, so a system package manager install works as well as
    /// vcpkg/conan. Sets `BACKEND_LIB` to whichever target was found.
    fn cmake_backend_block(&self) -> String {
        match self.window_backend {
            WindowBackend::Glfw => r##"find_package(glfw3 CONFIG QUIET)
if(TARGET glfw)
    set(BACKEND_LIB glfw)
elseif(PkgConfig_FOUND)
    pkg_check_modules(GLFW3_PC QUIET IMPORTED_TARGET glfw3)
    if(TARGET PkgConfig::GLFW3_PC)
        set(BACKEND_LIB PkgConfig::GLFW3_PC)
    endif()
endif()

if(NOT BACKEND_LIB)
    message(FATAL_ERROR
        "GLFW was not found. Install it with one of:\n"
        "  Debian/Ubuntu : sudo apt install libglfw3-dev\n"
        "  Fedora        : sudo dnf install glfw-devel\n"
        "  Arch          : sudo pacman -S glfw\n"
        "  macOS         : brew install glfw\n"
        "  vcpkg         : vcpkg install glfw3 (configure with the vcpkg toolchain file)")
endif()
"##
            .to_owned(),
            WindowBackend::Sdl3 => r##"find_package(SDL3 CONFIG QUIET)
if(TARGET SDL3::SDL3)
    set(BACKEND_LIB SDL3::SDL3)
elseif(PkgConfig_FOUND)
    pkg_check_modules(SDL3_PC QUIET IMPORTED_TARGET sdl3)
    if(TARGET PkgConfig::SDL3_PC)
        set(BACKEND_LIB PkgConfig::SDL3_PC)
    endif()
endif()

if(NOT BACKEND_LIB)
    message(FATAL_ERROR
        "SDL3 was not found. Install it with one of:\n"
        "  Fedora : sudo dnf install SDL3-devel\n"
        "  Arch   : sudo pacman -S sdl3\n"
        "  macOS  : brew install sdl3\n"
        "  vcpkg  : vcpkg install sdl3 (configure with the vcpkg toolchain file)\n"
        "  source : https://github.com/libsdl-org/SDL")
endif()
"##
            .to_owned(),
        }
    }

    /// Builds the loader. glad is compiled from the checkout in `third_party/`,
    /// which the generator clones; GLEW comes from the system. Sets `LOADER_LIB`.
    fn cmake_loader_block(&self) -> String {
        let api = format!(
            "{}.{}",
            self.opengl_major, self.opengl_minor
        );
        let profile = if self.core_profile { "core" } else { "compatibility" };

        match self.gl_loader {
            GlLoader::Glad => format!(
                r##"set(GLAD_DIR "${{CMAKE_CURRENT_SOURCE_DIR}}/third_party/glad")
if(NOT EXISTS "${{GLAD_DIR}}/CMakeLists.txt")
    message(FATAL_ERROR
        "third_party/glad is missing. Fetch it with:\n"
        "  git clone --depth 1 --branch master https://github.com/Dav1dde/glad.git third_party/glad")
endif()

# glad 0.1 generates its sources at build time with a Python script. It needs
# only the standard library, but the interpreter has to exist -- fail here with
# a readable message rather than midway through the build.
find_package(Python COMPONENTS Interpreter REQUIRED)

set(GLAD_API "gl={api}" CACHE STRING "" FORCE)
set(GLAD_PROFILE "{profile}" CACHE STRING "" FORCE)
set(GLAD_GENERATOR "c" CACHE STRING "" FORCE)
set(GLAD_REPRODUCIBLE ON CACHE BOOL "" FORCE)

# glad 0.1 declares cmake_minimum_required(VERSION 3.0), which CMake 4 rejects.
if(CMAKE_VERSION VERSION_GREATER_EQUAL 4.0)
    set(CMAKE_POLICY_VERSION_MINIMUM 3.5)
endif()

add_subdirectory("${{GLAD_DIR}}" glad_build)
set(LOADER_LIB glad)
"##,
                api = api,
                profile = profile,
            ),
            GlLoader::Glad2 => format!(
                r##"set(GLAD_SOURCES_DIR "${{CMAKE_CURRENT_SOURCE_DIR}}/third_party/glad")
if(NOT EXISTS "${{GLAD_SOURCES_DIR}}/cmake/CMakeLists.txt")
    message(FATAL_ERROR
        "third_party/glad is missing. Fetch it with:\n"
        "  git clone --depth 1 --branch glad2 https://github.com/Dav1dde/glad.git third_party/glad")
endif()

find_package(Python COMPONENTS Interpreter REQUIRED)

# Unlike glad 0.1, the glad2 generator depends on Jinja2. Check for it now so
# the failure is one actionable line instead of a traceback during the build.
execute_process(
    COMMAND "${{Python_EXECUTABLE}}" -c "import jinja2"
    RESULT_VARIABLE GLAD2_JINJA_RESULT
    OUTPUT_QUIET
    ERROR_QUIET
)
if(NOT GLAD2_JINJA_RESULT EQUAL 0)
    message(FATAL_ERROR
        "The glad2 generator needs the Jinja2 Python module:\n"
        "  \"${{Python_EXECUTABLE}}\" -m pip install --user jinja2\n"
        "Or regenerate this project with the 'glad' loader, which has no such dependency.")
endif()

add_subdirectory("${{GLAD_SOURCES_DIR}}/cmake" glad_build)
glad_add_library(glad_loader REPRODUCIBLE LOADER API gl:{profile}={api})
set(LOADER_LIB glad_loader)
"##,
                api = api,
                profile = profile,
            ),
            GlLoader::Glew => r##"find_package(GLEW QUIET)
if(TARGET GLEW::GLEW)
    set(LOADER_LIB GLEW::GLEW)
elseif(PkgConfig_FOUND)
    pkg_check_modules(GLEW_PC QUIET IMPORTED_TARGET glew)
    if(TARGET PkgConfig::GLEW_PC)
        set(LOADER_LIB PkgConfig::GLEW_PC)
    endif()
endif()

if(NOT LOADER_LIB)
    message(FATAL_ERROR
        "GLEW was not found. Install it with one of:\n"
        "  Debian/Ubuntu : sudo apt install libglew-dev\n"
        "  Fedora        : sudo dnf install glew-devel\n"
        "  Arch          : sudo pacman -S glew\n"
        "  macOS         : brew install glew\n"
        "  vcpkg         : vcpkg install glew (configure with the vcpkg toolchain file)")
endif()
"##
            .to_owned(),
        }
    }

    /// Optional libraries are linked when present and skipped when not, so a
    /// missing one never breaks the configure step. Each defines a HAVE_* macro
    /// the application code can test.
    fn cmake_optional_block(&self) -> String {
        let mut uses: Vec<String> = Vec::new();
        if self.include_glm {
            uses.push("app_use_optional(glm HAVE_GLM glm::glm glm)".to_owned());
        }
        if self.include_imgui {
            uses.push("app_use_optional(imgui HAVE_IMGUI imgui::imgui)".to_owned());
        }
        if self.include_assimp {
            uses.push("app_use_optional(assimp HAVE_ASSIMP assimp::assimp)".to_owned());
        }
        if self.include_fmt {
            uses.push("app_use_optional(fmt HAVE_FMT fmt::fmt)".to_owned());
        }
        if self.include_spdlog {
            uses.push("app_use_optional(spdlog HAVE_SPDLOG spdlog::spdlog)".to_owned());
        }

        let stb_block = if self.include_stb_image {
            r##"
# stb_image is a single header, so it has no link target -- just an include dir.
find_path(STB_IMAGE_INCLUDE_DIR NAMES stb_image.h PATH_SUFFIXES stb)
if(STB_IMAGE_INCLUDE_DIR)
    target_include_directories(${PROJECT_NAME} PRIVATE "${STB_IMAGE_INCLUDE_DIR}")
    target_compile_definitions(${PROJECT_NAME} PRIVATE HAVE_STB_IMAGE)
    message(STATUS "Optional dependency stb_image: using ${STB_IMAGE_INCLUDE_DIR}")
else()
    message(STATUS "Optional dependency stb_image: not found, skipped")
endif()
"##
        } else {
            ""
        };

        if uses.is_empty() && stb_block.is_empty() {
            return String::new();
        }

        let helper = if uses.is_empty() {
            String::new()
        } else {
            format!(
                r##"
# Links PKG to the executable if it can be found, and defines DEFINE when it is.
# Extra arguments are the imported target names to try, in order.
function(app_use_optional PKG DEFINE)
    find_package(${{PKG}} CONFIG QUIET)
    foreach(candidate IN LISTS ARGN)
        if(TARGET ${{candidate}})
            target_link_libraries(${{PROJECT_NAME}} PRIVATE ${{candidate}})
            target_compile_definitions(${{PROJECT_NAME}} PRIVATE ${{DEFINE}})
            message(STATUS "Optional dependency ${{PKG}}: using ${{candidate}}")
            return()
        endif()
    endforeach()

    if(PkgConfig_FOUND)
        string(TOLOWER "${{PKG}}" pc_module)
        pkg_check_modules(OPT_${{PKG}} QUIET IMPORTED_TARGET ${{pc_module}})
        if(TARGET PkgConfig::OPT_${{PKG}})
            target_link_libraries(${{PROJECT_NAME}} PRIVATE PkgConfig::OPT_${{PKG}})
            target_compile_definitions(${{PROJECT_NAME}} PRIVATE ${{DEFINE}})
            message(STATUS "Optional dependency ${{PKG}}: using pkg-config module ${{pc_module}}")
            return()
        endif()
    endif()

    message(STATUS "Optional dependency ${{PKG}}: not found, skipped")
endfunction()

{uses}
"##,
                uses = uses.join("\n"),
            )
        };

        format!(
            "\n# ---------------------------------------------------------------- Optional libraries{helper}{stb_block}"
        )
    }

    fn meson_template(&self, project_name: &str) -> String {
        let cxx_std = self.cpp_standard.clone();
        let backend_dep = match self.window_backend {
            WindowBackend::Glfw => "dependency('glfw3')",
            WindowBackend::Sdl3 => "dependency('sdl3')",
        };
        let loader_dep = match self.gl_loader {
            GlLoader::Glew => "dependency('glew')",
            _ => "dependency('glad')",
        };

        format!(
            "project('{project_name}', 'cpp', default_options: ['cpp_std={cxx_std}', 'warning_level=3'])\n\nopengl_dep = dependency('opengl')\nwindow_dep = {backend_dep}\nloader_dep = {loader_dep}\n\nexecutable('{project_name}',\n  'src/main.cpp',\n  dependencies: [opengl_dep, window_dep, loader_dep],\n  install: true\n)\n",
            project_name = project_name,
            cxx_std = cxx_std,
            backend_dep = backend_dep,
            loader_dep = loader_dep,
        )
    }

    fn cmake_presets_template(&self) -> String {
        // Preset version 3 is what CMake 3.21 understands, and no generator is
        // pinned so the platform default is used (Ninja is not always present).
        r##"{
  "version": 3,
  "cmakeMinimumRequired": {
    "major": 3,
    "minor": 21,
    "patch": 0
  },
  "configurePresets": [
    {
      "name": "default",
      "displayName": "Default (Debug)",
      "binaryDir": "${sourceDir}/build",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug",
        "CMAKE_EXPORT_COMPILE_COMMANDS": "ON"
      }
    },
    {
      "name": "release",
      "displayName": "Release",
      "inherits": "default",
      "binaryDir": "${sourceDir}/build-release",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Release"
      }
    }
  ],
  "buildPresets": [
    {
      "name": "default",
      "configurePreset": "default",
      "configuration": "Debug"
    },
    {
      "name": "release",
      "configurePreset": "release",
      "configuration": "Release"
    }
  ]
}
"##
        .to_owned()
    }

    fn vscode_tasks_template(&self) -> String {
        match self.build_system {
            BuildSystem::CMake => "{\n  \"version\": \"2.0.0\",\n  \"tasks\": [\n    {\n      \"label\": \"CMake: Configure\",\n      \"type\": \"shell\",\n      \"command\": \"cmake --preset default\"\n    },\n    {\n      \"label\": \"CMake: Build\",\n      \"type\": \"shell\",\n      \"command\": \"cmake --build --preset default\",\n      \"group\": \"build\",\n      \"dependsOn\": [\"CMake: Configure\"]\n    }\n  ]\n}\n"
                .to_owned(),
            BuildSystem::Meson => "{\n  \"version\": \"2.0.0\",\n  \"tasks\": [\n    {\n      \"label\": \"Meson: Setup\",\n      \"type\": \"shell\",\n      \"command\": \"meson setup build\"\n    },\n    {\n      \"label\": \"Meson: Build\",\n      \"type\": \"shell\",\n      \"command\": \"meson compile -C build\",\n      \"group\": \"build\",\n      \"dependsOn\": [\"Meson: Setup\"]\n    }\n  ]\n}\n"
                .to_owned(),
        }
    }

    fn vscode_launch_template(&self, project_name: &str) -> String {
        // CMake drops the executable in build/bin (see CMakeLists.txt); Meson
        // leaves it at the top of the build directory. cwd matches the binary so
        // the copied assets/ folder resolves at runtime.
        let (binary_dir, build_task) = match self.build_system {
            BuildSystem::CMake => ("${workspaceFolder}/build/bin", "CMake: Build"),
            BuildSystem::Meson => ("${workspaceFolder}/build", "Meson: Build"),
        };

        const TEMPLATE: &str = r##"{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Launch @PROJECT_NAME@",
      "type": "cppdbg",
      "request": "launch",
      "program": "@BINARY_DIR@/@PROJECT_NAME@",
      "args": [],
      "cwd": "@BINARY_DIR@",
      "stopAtEntry": false,
      "externalConsole": false,
      "MIMode": "gdb",
      "preLaunchTask": "@BUILD_TASK@",
      "osx": {
        "MIMode": "lldb"
      },
      "windows": {
        "type": "cppvsdbg",
        "program": "@BINARY_DIR@/@PROJECT_NAME@.exe"
      }
    }
  ]
}
"##;

        TEMPLATE
            .replace("@PROJECT_NAME@", project_name)
            .replace("@BINARY_DIR@", binary_dir)
            .replace("@BUILD_TASK@", build_task)
    }

    fn readme_template(&self, project_name: &str) -> String {
        format!(
            "# {project_name}\n\nGenerated with OpenGL Project Generator.\n\n## Selected stack\n- Windowing: {window_backend}\n- GL loader: {gl_loader}\n- Build system: {build_system}\n- OpenGL: {major}.{minor}\n\n## Optional libraries toggled\n- GLM: {glm}\n- Dear ImGui: {imgui}\n- stb_image: {stb}\n- Assimp: {assimp}\n- fmt: {fmt}\n- spdlog: {spdlog}\n\n## Build\n### CMake\n1. cmake --preset default\n2. cmake --build --preset default\n\nThe executable and a copy of assets/ end up in build/bin.\nUse the `release` preset instead of `default` for an optimised build.\n\n### Meson\n1. meson setup build\n2. meson compile -C build\n\n## Requirements\n- A C++ compiler and CMake 3.21 or newer.\n- {window_backend} development files (see third_party/README.md for install commands).\n{loader_requirement}\n\nOptional libraries are linked only if they are found; each one that is defines a\nHAVE_* macro (HAVE_GLM, HAVE_FMT, ...) you can test in your code. A missing one\nnever breaks the build.\n",
            project_name = project_name,
            window_backend = self.window_backend.as_str(),
            gl_loader = self.gl_loader.as_str(),
            build_system = self.build_system.as_str(),
            major = self.opengl_major,
            minor = self.opengl_minor,
            glm = yes_no(self.include_glm),
            imgui = yes_no(self.include_imgui),
            stb = yes_no(self.include_stb_image),
            assimp = yes_no(self.include_assimp),
            fmt = yes_no(self.include_fmt),
            spdlog = yes_no(self.include_spdlog),
            loader_requirement = self.loader_requirement_note(),
        )
    }

    /// What the chosen loader needs at build time, so the generated README says
    /// the same thing the CMake preflight checks enforce.
    fn loader_requirement_note(&self) -> &'static str {
        match self.gl_loader {
            GlLoader::Glad => {
                "- Python 3 on PATH: glad generates its sources during the build (standard library only)."
            }
            GlLoader::Glad2 => {
                "- Python 3 on PATH plus the Jinja2 module (`python -m pip install --user jinja2`):\n  the glad2 generator runs during the build."
            }
            GlLoader::Glew => "- GLEW development files.",
        }
    }

    fn third_party_readme(&self) -> String {
        format!(
            "# Third-party dependencies\n\nThis project expects the following base dependencies:\n- Window backend: {}\n- OpenGL loader: {}\n\nRecommended ways to install:\n- vcpkg (Windows): vcpkg install glfw3 sdl3 glad glew\n- conan: declare packages in conanfile\n- system package manager on Linux/macOS\n\nOptional libraries selected by generator:\n- GLM: {}\n- Dear ImGui: {}\n- stb_image: {}\n- Assimp: {}\n- fmt: {}\n- spdlog: {}\n",
            self.window_backend.as_str(),
            self.gl_loader.as_str(),
            yes_no(self.include_glm),
            yes_no(self.include_imgui),
            yes_no(self.include_stb_image),
            yes_no(self.include_assimp),
            yes_no(self.include_fmt),
            yes_no(self.include_spdlog),
        )
    }

}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Paint a white background for the entire UI
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, egui::CornerRadius::same(0), egui::Color32::WHITE);
        ui.heading("OpenGL C++ Project Generator");
        ui.label("Outline UI for project scaffolding options");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Project name:");
            ui.text_edit_singleline(&mut self.project_name);
        });
        ui.horizontal(|ui| {
            ui.label("Output dir:");
            ui.text_edit_singleline(&mut self.output_directory);
        });
        ui.horizontal(|ui| {
            ui.label("C++ standard:");
            ui.text_edit_singleline(&mut self.cpp_standard);
        });
        egui::ComboBox::from_label("Window backend")
            .selected_text(self.window_backend.as_str())
            .show_ui(ui, |ui| {
                for backend in WindowBackend::ALL {
                    ui.selectable_value(&mut self.window_backend, backend, backend.as_str());
                }
            });

        egui::ComboBox::from_label("OpenGL loader")
            .selected_text(self.gl_loader.as_str())
            .show_ui(ui, |ui| {
                for loader in GlLoader::ALL {
                    ui.selectable_value(&mut self.gl_loader, loader, loader.as_str());
                }
            });

        egui::ComboBox::from_label("Build system")
            .selected_text(self.build_system.as_str())
            .show_ui(ui, |ui| {
                for build in BuildSystem::ALL {
                    ui.selectable_value(&mut self.build_system, build, build.as_str());
                }
            });

        ui.horizontal(|ui| {
            ui.label("OpenGL version:");
            let prev_major = self.opengl_major;
            ui.add(egui::DragValue::new(&mut self.opengl_major).range(3..=4));
            if self.opengl_major != prev_major {
                self.constrain_opengl_version();
            }
            ui.label(".");
            let max_minor = if self.opengl_major == 3 { 3 } else { 6 };
            ui.add(egui::DragValue::new(&mut self.opengl_minor).range(0..=max_minor));
        });

        ui.separator();
        ui.label("Optional libraries");
        ui.checkbox(&mut self.include_glm, "GLM");
        ui.checkbox(&mut self.include_imgui, "Dear ImGui");
        ui.checkbox(&mut self.include_stb_image, "stb_image");
        ui.checkbox(&mut self.include_assimp, "Assimp");
        ui.checkbox(&mut self.include_fmt, "fmt");
        ui.checkbox(&mut self.include_spdlog, "spdlog");

        ui.separator();
        ui.label("Project options");
        ui.checkbox(&mut self.core_profile, "Core profile");
        ui.checkbox(&mut self.debug_context, "Debug context");
        ui.checkbox(&mut self.srgb_framebuffer, "sRGB framebuffer");
        ui.checkbox(&mut self.create_git_repo, "Initialize git repository");
        ui.checkbox(&mut self.include_gitignore, "Add .gitignore");
        ui.checkbox(&mut self.include_clang_format, "Add .clang-format");
        ui.checkbox(&mut self.include_cmake_presets, "Add CMakePresets.json");
        ui.checkbox(&mut self.include_vscode_files, "Add .vscode launch/tasks");

        ui.separator();
        if ui.button("Generate Project").clicked() {
            self.generate_project();
        }

        ui.label(&self.status_message);
    }
}

fn write_text(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create '{}': {e}", parent.display()))?;
    }
    fs::write(path, content).map_err(|e| format!("Could not write '{}': {e}", path.display()))
}

fn sanitize_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_lowercase()
}

fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

fn parse_cpp_standard(value: &str) -> u32 {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    let parsed = digits.parse::<u32>().unwrap_or(17);
    // CMAKE_CXX_STANDARD only accepts these; anything else fails at configure time.
    if [11, 14, 17, 20, 23, 26].contains(&parsed) {
        parsed
    } else {
        17
    }
}

fn gitignore_template() -> String {
    "# Build artifacts\n/build*/\n/bin/\ncompile_commands.json\n\n# IDE\n.vscode/*\n!.vscode/tasks.json\n!.vscode/launch.json\n\n# OS\n.DS_Store\nThumbs.db\n\n# CMake\nCMakeUserPresets.json\n".to_owned()
}

fn clang_format_template() -> String {
    "BasedOnStyle: LLVM\nIndentWidth: 4\nColumnLimit: 100\nBreakBeforeBraces: Allman\nAllowShortFunctionsOnASingleLine: Empty\n".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scaffolds into a scratch directory. Set OPENGL_GENERATOR_TEST_OUT to keep
    /// the output somewhere a build can be run against it.
    fn generate_into(app: &App, label: &str) -> PathBuf {
        let base = std::env::var("OPENGL_GENERATOR_TEST_OUT")
            .unwrap_or_else(|_| std::env::temp_dir().join("opengl-generator-tests").display().to_string());
        let out = PathBuf::from(base).join(label);
        let _ = fs::remove_dir_all(&out);

        let mut app = App { output_directory: out.display().to_string(), ..app.clone() };
        app.project_name = "my_opengl_app".to_owned();
        app.generate_project_impl().expect("generation should succeed");
        out.join("my_opengl_app")
    }

    fn matrix() -> Vec<(String, App)> {
        let mut cases = Vec::new();
        for backend in WindowBackend::ALL {
            for loader in GlLoader::ALL {
                let app = App {
                    window_backend: backend,
                    gl_loader: loader,
                    create_git_repo: false,
                    ..App::default()
                };
                let label = format!(
                    "{}-{}",
                    backend.as_str().to_lowercase(),
                    loader.as_str().to_lowercase()
                );
                cases.push((label, app));
            }
        }
        cases
    }

    #[test]
    fn generates_every_backend_and_loader_combination() {
        for (label, app) in matrix() {
            let root = generate_into(&app, &label);
            let cmake = fs::read_to_string(root.join("CMakeLists.txt")).unwrap();

            assert!(cmake.contains("cmake_minimum_required(VERSION 3.21)"), "{label}");
            assert!(cmake.contains("add_executable(${PROJECT_NAME} ${APP_SOURCES})"), "{label}");
            assert!(cmake.contains("${BACKEND_LIB}"), "{label}");
            assert!(cmake.contains("${LOADER_LIB}"), "{label}");
            // Every path that sets these must also fail loudly when unset.
            assert!(
                cmake.contains("set(BACKEND_LIB") || cmake.contains("set(LOADER_LIB"),
                "{label}"
            );
            assert!(root.join("CMakePresets.json").exists(), "{label}");
        }
    }

    #[test]
    fn glad_headers_match_the_cloned_branch() {
        for (label, app) in matrix() {
            let root = generate_into(&app, &label);
            let main_cpp = fs::read_to_string(root.join("src/main.cpp")).unwrap();
            if app.gl_loader != GlLoader::Glew {
                assert!(!root.join("third_party/glad/.git").exists(), "{label}: nested repo left behind");
            }
            match app.gl_loader {
                // glad 0.1 lays its header out as glad/glad.h, glad2 as glad/gl.h.
                GlLoader::Glad => {
                    assert!(main_cpp.contains("#include <glad/glad.h>"), "{label}");
                    // glad 0.1 exposes its CMake support at the checkout root;
                    // glad2 only has it under cmake/.
                    assert!(root.join("third_party/glad/CMakeLists.txt").exists(), "{label}");
                    assert!(!root.join("third_party/glad/cmake/GladConfig.cmake").exists(), "{label}");
                }
                GlLoader::Glad2 => {
                    assert!(main_cpp.contains("#include <glad/gl.h>"), "{label}");
                    assert!(root.join("third_party/glad/cmake/GladConfig.cmake").exists(), "{label}");
                }
                GlLoader::Glew => {
                    assert!(main_cpp.contains("#include <GL/glew.h>"), "{label}");
                }
            }
        }
    }

    #[test]
    fn rejects_cpp_standards_cmake_would_not_accept() {
        assert_eq!(parse_cpp_standard("c++20"), 20);
        assert_eq!(parse_cpp_standard("c++23"), 23);
        assert_eq!(parse_cpp_standard("gnu++17"), 17);
        assert_eq!(parse_cpp_standard("c++21"), 17);
        assert_eq!(parse_cpp_standard("nonsense"), 17);
    }
}
