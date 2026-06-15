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

#[derive(Debug)]
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

        let mut args = vec![
            "clone",
            "--depth",
            "1",
            "https://github.com/Dav1dde/glad.git",
            "third_party/glad",
        ];

        if self.gl_loader == GlLoader::Glad2 {
            args.insert(1, "--branch");
            args.insert(2, "glad2");
        }

        let status = Command::new("git")
            .args(&args)
            .current_dir(root)
            .status()
            .map_err(|e| format!("Could not clone glad repository: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err("git clone for glad failed".to_owned())
        }
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
                "#include <iostream>\n\n{loader_include}\n#include <GLFW/glfw3.h>\n\nint main() {{\n    if (!glfwInit()) {{\n        std::cerr << \"Failed to initialize GLFW\\n\";\n        return -1;\n    }}\n\n    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, {major});\n    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, {minor});\n    glfwWindowHint(GLFW_OPENGL_PROFILE, {profile_hint});\n    glfwWindowHint(GLFW_OPENGL_DEBUG_CONTEXT, {debug_hint});\n\n    GLFWwindow* window = glfwCreateWindow(1280, 720, \"{project_name}\", nullptr, nullptr);\n    if (!window) {{\n        std::cerr << \"Failed to create GLFW window\\n\";\n        glfwTerminate();\n        return -1;\n    }}\n\n    glfwMakeContextCurrent(window);\n\n{loader_init}\n\n    while (!glfwWindowShouldClose(window)) {{\n        glClearColor(1.0f, 1.0f, 1.0f, 1.0f);\n        glClear(GL_COLOR_BUFFER_BIT);\n\n        glfwSwapBuffers(window);\n        glfwPollEvents();\n    }}\n\n    glfwDestroyWindow(window);\n    glfwTerminate();\n    return 0;\n}}\n",
                major = self.opengl_major,
                minor = self.opengl_minor,
                profile_hint = profile_hint,
                debug_hint = if self.debug_context { "GLFW_TRUE" } else { "GLFW_FALSE" },
                loader_init = self.glfw_loader_init(),
                project_name = project_name,
            ),
            WindowBackend::Sdl3 => format!(
                "#include <iostream>\n\n{loader_include}\n#include <SDL3/SDL.h>\n\nint main() {{\n    if (SDL_Init(SDL_INIT_VIDEO) < 0) {{\n        std::cerr << \"Failed to initialize SDL3: \" << SDL_GetError() << \"\\n\";\n        return -1;\n    }}\n\n    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, {major});\n    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, {minor});\n    SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, {profile_hint});\n    SDL_GL_SetAttribute(SDL_GL_FRAMEBUFFER_SRGB_CAPABLE, {srgb_hint});\n\n    SDL_Window* window = SDL_CreateWindow(\"{project_name}\", 1280, 720, SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE);\n    if (!window) {{\n        std::cerr << \"Failed to create SDL3 window: \" << SDL_GetError() << \"\\n\";\n        SDL_Quit();\n        return -1;\n    }}\n\n    SDL_GLContext gl_context = SDL_GL_CreateContext(window);\n    if (!gl_context) {{\n        std::cerr << \"Failed to create OpenGL context: \" << SDL_GetError() << \"\\n\";\n        SDL_DestroyWindow(window);\n        SDL_Quit();\n        return -1;\n    }}\n\n{loader_init}\n\n    bool running = true;\n    while (running) {{\n        SDL_Event event;\n        while (SDL_PollEvent(&event)) {{\n            if (event.type == SDL_EVENT_QUIT) {{\n                running = false;\n            }}\n        }}\n\n        glClearColor(1.0f, 1.0f, 1.0f, 1.0f);\n        glClear(GL_COLOR_BUFFER_BIT);\n        SDL_GL_SwapWindow(window);\n    }}\n\n    SDL_GL_DestroyContext(gl_context);\n    SDL_DestroyWindow(window);\n    SDL_Quit();\n    return 0;\n}}\n",
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
                "    glewExperimental = GL_TRUE;\n    if (glewInit() != GLEW_OK) {\n        std::cerr << \"Failed to initialize GLEW\\n\";\n        return -1;\n    }"
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
                "    glewExperimental = GL_TRUE;\n    if (glewInit() != GLEW_OK) {\n        std::cerr << \"Failed to initialize GLEW\\n\";\n        return -1;\n    }"
            }
        }
    }

    fn cmake_template(&self, project_name: &str) -> String {
        let cxx_std = parse_cpp_standard(&self.cpp_standard);
        let gl_profile = if self.core_profile { "core" } else { "compatibility" };
        let gl_api = format!("gl:{gl_profile}={}.{}", self.opengl_major, self.opengl_minor);

        let backend_block = match self.window_backend {
            WindowBackend::Glfw => "find_package(glfw3 CONFIG REQUIRED)\n",
            WindowBackend::Sdl3 => "find_package(SDL3 CONFIG REQUIRED)\n",
        };

        let backend_link = match self.window_backend {
            WindowBackend::Glfw => "glfw",
            WindowBackend::Sdl3 => "SDL3::SDL3",
        };

        let (loader_block, loader_link) = match self.gl_loader {
            GlLoader::Glad | GlLoader::Glad2 => (
                format!(
                    "set(GLAD_SOURCES_DIR \"${{CMAKE_CURRENT_SOURCE_DIR}}/third_party/glad\")\nadd_subdirectory(${{GLAD_SOURCES_DIR}}/cmake glad_cmake)\nglad_add_library(glad_loader REPRODUCIBLE LOADER API {gl_api})\n"
                ),
                "glad_loader",
            ),
            GlLoader::Glew => ("find_package(GLEW REQUIRED)\n".to_owned(), "GLEW::GLEW"),
        };

        let optional_packages = self.optional_package_notes();

        format!(
            "cmake_minimum_required(VERSION 3.24)\nproject({project_name} LANGUAGES C CXX)\n\nset(CMAKE_CXX_STANDARD {cxx_std})\nset(CMAKE_CXX_STANDARD_REQUIRED ON)\nset(CMAKE_CXX_EXTENSIONS OFF)\n\nfind_package(OpenGL REQUIRED)\n{backend_block}{loader_find}\n{optional_packages}\nadd_executable(${{PROJECT_NAME}} src/main.cpp)\n\ntarget_link_libraries(${{PROJECT_NAME}}\n    PRIVATE\n        OpenGL::GL\n        {backend_link}\n        {loader_link}\n)\n\nif(MSVC)\n    target_compile_options(${{PROJECT_NAME}} PRIVATE /W4 /permissive-)\nelse()\n    target_compile_options(${{PROJECT_NAME}} PRIVATE -Wall -Wextra -Wpedantic)\nendif()\n",
            project_name = project_name,
            cxx_std = cxx_std,
            backend_block = backend_block,
            loader_find = loader_block,
            optional_packages = optional_packages,
            backend_link = backend_link,
            loader_link = loader_link,
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
        "{\n  \"version\": 6,\n  \"configurePresets\": [\n    {\n      \"name\": \"default\",\n      \"displayName\": \"Default\",\n      \"generator\": \"Ninja\",\n      \"binaryDir\": \"${sourceDir}/build\",\n      \"cacheVariables\": {\n        \"CMAKE_BUILD_TYPE\": \"Debug\"\n      }\n    }\n  ],\n  \"buildPresets\": [\n    {\n      \"name\": \"default\",\n      \"configurePreset\": \"default\"\n    }\n  ]\n}\n"
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
        format!(
            "{{\n  \"version\": \"0.2.0\",\n  \"configurations\": [\n    {{\n      \"name\": \"Launch {project_name}\",\n      \"type\": \"cppvsdbg\",\n      \"request\": \"launch\",\n      \"program\": \"${{workspaceFolder}}/build/{project_name}.exe\",\n      \"args\": [],\n      \"cwd\": \"${{workspaceFolder}}\",\n      \"preLaunchTask\": \"{build_task}\"\n    }}\n  ]\n}}\n",
            project_name = project_name,
            build_task = match self.build_system {
                BuildSystem::CMake => "CMake: Build",
                BuildSystem::Meson => "Meson: Build",
            }
        )
    }

    fn readme_template(&self, project_name: &str) -> String {
        format!(
            "# {project_name}\n\nGenerated with OpenGL Project Generator.\n\n## Selected stack\n- Windowing: {window_backend}\n- GL loader: {gl_loader}\n- Build system: {build_system}\n- OpenGL: {major}.{minor}\n\n## Optional libraries toggled\n- GLM: {glm}\n- Dear ImGui: {imgui}\n- stb_image: {stb}\n- Assimp: {assimp}\n- fmt: {fmt}\n- spdlog: {spdlog}\n\n## Build\n### CMake\n1. cmake --preset default\n2. cmake --build --preset default\n\n### Meson\n1. meson setup build\n2. meson compile -C build\n\n## Notes\n- Install dependencies with vcpkg/conan/system package manager before building.\n- See third_party/README.md for dependency guidance.\n",
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
        )
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

    fn optional_package_notes(&self) -> String {
        let mut lines = Vec::new();
        if self.include_glm {
            lines.push("find_package(glm CONFIG QUIET)");
        }
        if self.include_imgui {
            lines.push("find_package(imgui CONFIG QUIET)");
        }
        if self.include_stb_image {
            lines.push("find_package(Stb CONFIG QUIET)");
        }
        if self.include_assimp {
            lines.push("find_package(assimp CONFIG QUIET)");
        }
        if self.include_fmt {
            lines.push("find_package(fmt CONFIG QUIET)");
        }
        if self.include_spdlog {
            lines.push("find_package(spdlog CONFIG QUIET)");
        }

        if lines.is_empty() {
            "".to_owned()
        } else {
            format!("{}\n", lines.join("\n"))
        }
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
    digits.parse::<u32>().unwrap_or(17)
}

fn gitignore_template() -> String {
    "# Build artifacts\n/build/\n/bin/\n\n# IDE\n.vscode/*\n!.vscode/tasks.json\n!.vscode/launch.json\n\n# OS\n.DS_Store\nThumbs.db\n\n# CMake\nCMakeUserPresets.json\n".to_owned()
}

fn clang_format_template() -> String {
    "BasedOnStyle: LLVM\nIndentWidth: 4\nColumnLimit: 100\nBreakBeforeBraces: Allman\nAllowShortFunctionsOnASingleLine: Empty\n".to_owned()
}
