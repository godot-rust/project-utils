use path_slash::PathExt;
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
    pub fn with_lib_name(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref().to_string();

        self.lib_name = Some(name);
    }

    /// Set the name of the crate.
    pub fn lib_name(mut self, name: impl AsRef<str>) -> Self {
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

        let gdnlib_path = godot_resource_output_dir.join(format!("{}.gdnlib", lib_name));

        {
            let target_base_path = target_dir;

            let target_rel_path = pathdiff::diff_paths(&target_base_path, &godot_project_dir)
                .expect("Unable to create relative path between Godot project and library output");

            let prefix;
            let output_path;

            if target_rel_path.starts_with("../") {
                // not in the project folder, use an absolute path
                prefix = "";
                output_path = target_base_path;
            } else {
                // output paths are inside the project folder, use a `res://` path
                prefix = "res://";
                output_path = target_rel_path;
            };

            let binaries = common_binary_outputs(&output_path, build_mode, &lib_name);

            let file_exists = gdnlib_path.exists() && gdnlib_path.is_file();

            if !file_exists {
                let gdnlib = generate_gdnlib(prefix, binaries);
                std::fs::write(&gdnlib_path, gdnlib)?;
            }
        }

        let rel_gdnlib_path = pathdiff::diff_paths(&gdnlib_path, &godot_project_dir)
            .expect("Unable to create relative path between Godot project and library output");

        let prefix;
        let output_path;

        if rel_gdnlib_path.starts_with("../") {
            // not in the project folder, use an absolute path
            prefix = "";
            output_path = &gdnlib_path;
        } else {
            // output paths are inside the project folder, use a `res://` path
            prefix = "res://";
            output_path = &rel_gdnlib_path;
        };

        for name in classes {
            let path = godot_resource_output_dir.join(format!("{}.gdns", &name));

            let file_exists = path.exists() && path.is_file();

            if !file_exists {
                let content = generate_gdns(&prefix, &output_path, &name);
                std::fs::write(&path, content)?;
            }
        }

        Ok(())
    }
}

struct Binaries {
    x11: PathBuf,
    osx: PathBuf,
    // TODO
    // ios: PathBuf,
    windows: PathBuf,
    android_aarch64: PathBuf,
    android_armv7: PathBuf,
    android_x86: PathBuf,
    android_x86_64: PathBuf,
}

fn common_binary_outputs(target: &Path, mode: BuildMode, name: &str) -> Binaries {
    let mode_path = match mode {
        BuildMode::Debug => "debug",
        BuildMode::Release => "release",
    };

    // NOTE: If a crate has a hyphen in the name, at least on Linux the resulting library
    // will have it replaced with an underscore. I assume other platforms do the same?
    let name = name.replace("-", "_");

    Binaries {
        x11: target.join(mode_path).join(format!("lib{}.so", name)),
        osx: target.join(mode_path).join(format!("lib{}.dylib", name)),

        windows: target.join(mode_path).join(format!("{}.dll", name)),
        android_armv7: target
            .join("armv7-linux-androideabi")
            .join(mode_path)
            .join(format!("lib{}.so", name)),
        android_aarch64: target
            .join("aarch64-linux-android")
            .join(mode_path)
            .join(format!("lib{}.so", name)),
        android_x86: target
            .join("i686-linux-android")
            .join(mode_path)
            .join(format!("lib{}.so", name)),
        android_x86_64: target
            .join("x86_64-linux-android")
            .join(mode_path)
            .join(format!("lib{}.so", name)),
    }
}

fn generate_gdnlib(path_prefix: &str, binaries: Binaries) -> String {
    format!(
        r#"[entry]
Android.armeabi-v7a="{prefix}{android_armv7}"
Android.arm64-v8a="{prefix}{android_aarch64}"
Android.x86="{prefix}{android_x86}"
Android.x86_64="{prefix}{android_x86_64}"
X11.64="{prefix}{x11}"
OSX.64="{prefix}{osx}"
Windows.64="{prefix}{win}"

[dependencies]

Android.armeabi-v7a=[  ]
Android.arm64-v8a=[  ]
Android.x86=[  ]
Android.x86_64=[  ]
X11.64=[  ]
OSX.64=[  ]

[general]

singleton=false
load_once=true
symbol_prefix="godot_"
reloadable=true"#,
        prefix = path_prefix,
        android_armv7 = binaries.android_armv7.to_slash_lossy(),
        android_aarch64 = binaries.android_aarch64.to_slash_lossy(),
        android_x86 = binaries.android_x86.to_slash_lossy(),
        android_x86_64 = binaries.android_x86_64.to_slash_lossy(),
        x11 = binaries.x11.to_slash_lossy(),
        osx = binaries.osx.to_slash_lossy(),
        win = binaries.windows.to_slash_lossy(),
    )
}

fn generate_gdns(path_prefix: &str, gdnlib_path: &Path, name: &str) -> String {
    format!(
        r#"[gd_resource type="NativeScript" load_steps=2 format=2]

[ext_resource path="{prefix}{gdnlib}" type="GDNativeLibrary" id=1]

[resource]
class_name = "{name}"
script_class_name = "{name}"
library = ExtResource( 1 )
"#,
        prefix = path_prefix,
        gdnlib = gdnlib_path.to_slash_lossy(),
        name = name,
    )
}
