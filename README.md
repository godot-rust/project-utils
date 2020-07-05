# GDNative Project Utilities

## Automatically creating `.gdnlib` and `.gdns` files

This crate autogenerates a `.gdnlib` file for a crate and `.gdns` files for all
types that derive `NativeClass` from a cargo build script.

### Example

The following code in the `build.rs` (or any cargo build script) will
automatically generate the Godot resources when the Rust code changes.

```rust
use gdnative_project_utils::*;

fn main() -> Result<(), Box<dyn std::error::Error>>{
    /// directory to scan for Rust files
    let classes = scan_crate("src")?;

    /// generate files inside the Godot project directory
    Generator::new()
        .godot_project_dir("../")
        .build(classes)?;

    Ok(())
}
```

## License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be licensed under the [MIT license](LICENSE.md), without any additional terms or conditions.