//! Scanning of project sources.

use std::collections::HashSet;
use std::path::Path;

use proc_macro2::TokenTree;

/// Type-alias for a set of classes that were found from the scan.
pub type Classes = HashSet<String>;

/// Scan the directory at path `dir` for all `*.rs` files and find types which implement `NativeClass`.
pub fn scan_crate(dir: impl AsRef<Path>) -> Result<Classes, ScanError> {
    let rs_extension = std::ffi::OsString::from("rs");
    let mut paths = vec![];

    for file in ignore::Walk::new(dir.as_ref()) {
        let file = file.map_err(ScanError::WalkDir)?;

        let path = file.into_path();

        if path.extension() == Some(&rs_extension) {
            paths.push(path);
        }
    }

    let classes = paths
        .into_iter()
        .map(|path| -> Result<_, ScanError> {
            let contents = std::fs::read_to_string(&path).map_err(ScanError::ReadFile)?;

            let file = syn::parse_file(&contents).map_err(ScanError::Parse)?;

            find_classes(&file).map_err(ScanError::Parse)
        })
        .try_fold(
            HashSet::new(),
            |mut acc, classes_res| -> Result<_, ScanError> {
                let classes = classes_res?;
                acc.extend(classes.into_iter().map(|x| x.to_string()));
                Ok(acc)
            },
        )?;

    Ok(classes)
}

/// Error type for errors that can occur during scanning.
#[derive(Debug)]
pub enum ScanError {
    /// An error was encountered when exploring all the files.
    WalkDir(ignore::Error),
    /// An error was encountered when reading in a Rust source file.
    ReadFile(std::io::Error),
    /// An error was encountered when parsing a Rust source file.
    Parse(syn::Error),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::WalkDir(err) => {
                f.write_fmt(format_args!("Directory walking error: {}", err))
            }
            ScanError::ReadFile(err) => f.write_fmt(format_args!("File reading error: {}", err)),
            ScanError::Parse(err) => f.write_fmt(format_args!("Parsing error: {}", err)),
        }
    }
}

impl std::error::Error for ScanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScanError::WalkDir(err) => Some(err),
            ScanError::ReadFile(err) => Some(err),
            ScanError::Parse(err) => Some(err),
        }
    }
}

fn find_classes(file: &syn::File) -> Result<HashSet<syn::Ident>, syn::Error> {
    fn derives_nativeclass(attrs: &[syn::Attribute]) -> Result<bool, syn::Error> {
        let mut res = false;

        for attr in attrs {
            if !attr.path.is_ident("derive") {
                continue;
            }

            for t in attr.tokens.clone() {
                if let TokenTree::Group(g) = &t {
                    let s = g.stream();

                    for tt in s {
                        if let TokenTree::Ident(i) = tt {
                            if i == "NativeClass" {
                                res = true;
                                break;
                            }
                        }
                    }
                } else {
                    return Err(syn::Error::new(t.span(), "Unexpected #[derive attribute]"));
                }
            }
        }

        Ok(res)
    }

    struct Visitor {
        classes: HashSet<syn::Ident>,
        errors: Vec<syn::Error>,
    }

    impl<'ast> syn::visit::Visit<'ast> for Visitor {
        fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
            match derives_nativeclass(&s.attrs) {
                Err(err) => {
                    self.errors.push(err);
                }
                Ok(true) => {
                    self.classes.insert(s.ident.clone());
                }
                Ok(false) => {}
            }
            syn::visit::visit_item_struct(self, s)
        }

        fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
            match derives_nativeclass(&i.attrs) {
                Err(err) => {
                    self.errors.push(err);
                }
                Ok(true) => {
                    self.classes.insert(i.ident.clone());
                }
                Ok(false) => {}
            }
            syn::visit::visit_item_enum(self, i)
        }
    }

    let mut vis = Visitor {
        classes: HashSet::new(),
        errors: vec![],
    };

    syn::visit::visit_file(&mut vis, file);

    if vis.errors.is_empty() {
        Ok(vis.classes)
    } else {
        let mut err = vis.errors.pop().unwrap();

        for e in vis.errors {
            err.combine(e);
        }

        Err(err)
    }
}
