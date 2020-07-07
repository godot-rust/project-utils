use std::path::{Path, PathBuf};

/// Build mode of the crate
#[derive(Copy, Clone, Debug)]
pub enum BuildMode {
    Debug,
    Release,
}

/// A builder type that holds all necessary information about the project to
/// generate files in all the right places.
#[derive(Default)]
pub struct Builder {
    godot_project_dir: Option<PathBuf>,
    godot_resource_output_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    lib_name: Option<String>,
    build_mode: Option<BuildMode>,
}

impl Builder {
    /// Construct a new Builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// **REQUIRED** Set the path to the root of the Godot project.
    pub fn with_godot_project_dir(&mut self, dir: impl AsRef<Path>) {
        let dir = dir.as_ref().to_path_buf();

        self.godot_project_dir = Some(dir);
    }

    /// **REQUIRED** Set the path to the root of the Godot project.
    pub fn godot_project_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.with_godot_project_dir(dir);
        self
    }

    /// Set the path to the directory inside the Godot project to which the
    /// generates files should be saved.
    pub fn with_godot_resource_output_dir(&mut self, dir: impl AsRef<Path>) {
        let dir = dir.as_ref().to_path_buf();

        self.godot_resource_output_dir = Some(dir);
    }

    /// Set the path to the directory inside the Godot project to which the
    /// generates files should be saved.
    pub fn godot_resource_output_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.with_godot_resource_output_dir(dir);
        self
    }

    /// Set the path to the `target` directory in which cargo creates build
    /// artefacts.
    pub fn with_target_dir(&mut self, dir: impl AsRef<Path>) {
        let dir = dir.as_ref().to_path_buf();

        self.target_dir = Some(dir);
    }

    /// Set the path to the `target` directory in which cargo creates build
    /// artefacts.
    pub fn target_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.with_target_dir(dir);
        self
    }

    /// Set the name of the crate.
    pub fn with_lib_name(&mut self, name: impl AsRef<String>) {
        let name = name.as_ref().to_string();

        self.lib_name = Some(name);
    }

    /// Set the name of the crate.
    pub fn lib_name(mut self, name: impl AsRef<String>) -> Self {
        self.with_lib_name(name);
        self
    }

    /// Set the build mode of the crate.
    ///
    /// This will affect the path the `gdnlib` resource points to.
    pub fn with_build_mode(&mut self, mode: BuildMode) {
        self.build_mode = Some(mode);
    }

    /// Set the build mode of the crate.
    ///
    /// This will affect the path the `gdnlib` resource points to.
    pub fn build_mode(mut self, mode: BuildMode) -> Self {
        self.with_build_mode(mode);
        self
    }

    /// Build and generate files for the crate and all `classes`.
    ///
    /// # Panics
    ///
    /// This function panics if the `godot_project_dir` has not been set.
    pub fn build(self, classes: crate::scan::Classes) -> Result<(), std::io::Error> {
        let lib_name = self
            .lib_name
            .or_else(|| std::env::var("CARGO_PKG_NAME").ok())
            .expect("Package name not given and unable to find");
        let godot_project_dir = self
            .godot_project_dir
            .and_then(|path| path.canonicalize().ok())
            .expect("Godot project dir not given");
        let godot_resource_output_dir = self
            .godot_resource_output_dir
            .and_then(|path| path.canonicalize().ok())
            .unwrap_or_else(|| godot_project_dir.join("native"));
        let target_dir = self
            .target_dir
            .and_then(|path| path.canonicalize().ok())
            .or_else(|| {
                let dir = std::env::var("CARGO_TARGET_DIR").ok()?;
                PathBuf::from(dir).canonicalize().ok()
            })
            .or_else(|| {
                let dir = std::env::var("OUT_DIR").ok()?;
                let out_path = PathBuf::from(&dir);

                // target/{debug/release}/build/{crate}/out
                out_path.join("../../../../").canonicalize().ok()
            })
            .expect("Target dir not given and unable to find");
        let build_mode = self
            .build_mode
            .or_else(|| {
                let profile = std::env::var("PROFILE").ok()?;
                match profile.as_str() {
                    "release" => Some(BuildMode::Release),
                    "debug" => Some(BuildMode::Debug),
                    _ => None,
                }
            })
            .expect("Build mode not given and unable to find");

        std::fs::create_dir_all(&godot_resource_output_dir)?;
        rerun_if_changed(&godot_resource_output_dir);

        let gdnlib_path = godot_resource_output_dir.join(format!("{}.gdnlib", lib_name));

        {
            let output_base_path = match build_mode {
                BuildMode::Debug => target_dir.join("debug"),
                BuildMode::Release => target_dir.join("release"),
            };

            let rel_output_base_path = pathdiff::diff_paths(&output_base_path, &godot_project_dir)
                .expect("Unable to create relative path between Godot project and library output");

            let prefix;
            let output_path;

            if rel_output_base_path.starts_with("../") {
                // not in the project folder, use an absolute path
                prefix = "";
                output_path = output_base_path;
            } else {
                // output paths are inside the project folder, use a `res://` path
                prefix = "res://";
                output_path = rel_output_base_path;
            };

            let binaries = common_binary_outputs(&output_path, &lib_name);

            let gdnlib = generate_gdnlib(prefix, binaries);

            std::fs::write(&gdnlib_path, gdnlib)?;
            rerun_if_changed(&gdnlib_path);
        }

        for name in classes {
            let content = generate_gdns(&gdnlib_path, &name);
            let path = godot_resource_output_dir.join(format!("{}.gdns", &name));
            std::fs::write(&path, content)?;
            rerun_if_changed(&path);
        }

        Ok(())
    }
}

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}

struct Binaries {
    x11: PathBuf,
    osx: PathBuf,
    // TODO
    //
    // android: PathBuf,
    // ios: PathBuf,
    windows: PathBuf,
}

fn common_binary_outputs(base: &Path, name: &str) -> Binaries {
    Binaries {
        x11: base.join(format!("lib{}.so", name)),
        osx: base.join(format!("lib{}.dylib", name)),

        windows: base.join(format!("{}.dll", name)),
    }
}

fn generate_gdnlib(path_prefix: &str, binaries: Binaries) -> String {
    format!(
        r#"[entry]

X11.64="{prefix}{x11}"
OSX.64="{prefix}{osx}"
Windows.64="{prefix}{win}"

[dependencies]

X11.64=[  ]
OSX.64=[  ]

[general]

singleton=false
load_once=true
symbol_prefix="godot_"
reloadable=true"#,
        prefix = path_prefix,
        x11 = binaries.x11.display(),
        osx = binaries.osx.display(),
        win = binaries.windows.display(),
    )
}

fn generate_gdns(gdnlib_path: &Path, name: &str) -> String {
    format!(
        r#"[gd_resource type="NativeScript" load_steps=2 format=2]

[ext_resource path="{gdnlib}" type="GDNativeLibrary" id=1]

[resource]
class_name = "{name}"
script_class_name = "{name}"
library = ExtResource( 1 )
"#,
        gdnlib = gdnlib_path.display(),
        name = name,
    )
}
