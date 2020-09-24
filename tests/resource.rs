use gdnative_project_utils::*;

#[test]
fn gdnlib_target_in_project() {
    let godot_proj_dir = tempfile::tempdir().unwrap();
    let asset_dir = godot_proj_dir.path().join("native");
    let target_dir = godot_proj_dir.path().join("target");

    std::fs::create_dir_all(&asset_dir).unwrap();
    std::fs::create_dir_all(&target_dir).unwrap();

    Generator::new()
        .lib_name("generator_test")
        .target_dir(&target_dir)
        .godot_project_dir(&godot_proj_dir)
        .godot_resource_output_dir(&asset_dir)
        .build_mode(BuildMode::Debug)
        .build(Classes::new())
        .expect("Should generate resources");

    let gdnlib_path = asset_dir.join("generator_test.gdnlib");
    assert!(gdnlib_path.exists());
    assert!(gdnlib_path.is_file());

    let content = std::fs::read_to_string(&gdnlib_path).unwrap();

    // check linux
    assert!(content.contains("=\"res://target/debug/libgenerator_test.so\""));
    // check android aarch64
    assert!(content.contains("=\"res://target/aarch64-linux-android/debug/libgenerator_test.so\""));
    // check windows, the path should still be with forward slashes
    assert!(content.contains("=\"res://target/debug/generator_test.dll\""));
}

#[test]
fn gdnlib_target_outside_of_project() {
    let godot_proj_dir = tempfile::tempdir().unwrap();
    let target_dir = tempfile::tempdir().unwrap();
    let asset_dir = godot_proj_dir.path().join("native");

    std::fs::create_dir_all(&asset_dir).unwrap();

    Generator::new()
        .lib_name("generator_test")
        .target_dir(&target_dir)
        .godot_project_dir(&godot_proj_dir)
        .godot_resource_output_dir(&asset_dir)
        .build_mode(BuildMode::Debug)
        .build(Classes::new())
        .expect("Should generate resources");

    let gdnlib_path = asset_dir.join("generator_test.gdnlib");
    assert!(gdnlib_path.exists());
    assert!(gdnlib_path.is_file());

    let content = std::fs::read_to_string(&gdnlib_path).unwrap();

    // make sure it doesn't use `res://`

    // check linux
    assert!(content.contains(&format!(
        "=\"{}/debug/libgenerator_test.so\"",
        target_dir.path().display()
    )));
    // check android aarch64
    assert!(content.contains(&format!(
        "=\"{}/aarch64-linux-android/debug/libgenerator_test.so\"",
        target_dir.path().display()
    )));
    // check windows, the path should still be with forward slashes
    assert!(content.contains(&format!(
        "=\"{}/debug/generator_test.dll\"",
        target_dir.path().display()
    )));
}

#[test]
fn gdns() {
    let c: Classes = vec!["Test".to_string(), "AnotherTest".to_string()]
        .into_iter()
        .collect();

    let godot_proj_dir = tempfile::tempdir().unwrap();
    let asset_dir = godot_proj_dir.path().join("native");
    let target_dir = godot_proj_dir.path().join("target");

    std::fs::create_dir_all(&asset_dir).unwrap();
    std::fs::create_dir_all(&target_dir).unwrap();

    Generator::new()
        .lib_name("gdns_test")
        .build_mode(BuildMode::Debug)
        .target_dir(&target_dir)
        .godot_project_dir(&godot_proj_dir)
        .godot_resource_output_dir(&asset_dir)
        .build(c.clone())
        .expect("Should generate resources");

    for class in c {
        let path = asset_dir.join(format!("{}.gdns", class));
        assert!(path.exists());
        assert!(path.is_file());

        let content = std::fs::read_to_string(&path).unwrap();

        assert!(content.contains(&format!("class_name = \"{}\"", class)));
        assert!(content.contains(&format!("script_class_name = \"{}\"", class)));
    }
}
