use std::{collections::HashSet, fs::read_dir, path::{Path, PathBuf}};

use crate::parser::error::{ParserError, ParserResult};


#[derive(Clone, Debug)]
pub struct LayoutHints {
    pub venv: Option<PathBuf>,
    pub module_root: Option<PathBuf>,
    pub module: Option<String>,
}

impl Default for LayoutHints {
    fn default() -> Self {
        Self { venv: None, module_root: None, module: None }
    }
}

#[derive(Clone, Debug)]
pub struct LayoutResult {
    pub venv_dir: PathBuf,
    pub site_packages_dir: PathBuf,
    pub module_root: PathBuf,
    pub module_name: String,
}

pub struct LayoutResolver;

impl LayoutResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(
        &self,
        project_dir: &Path,
        files: &[PathBuf],
        hints: &LayoutHints,
    ) -> ParserResult<LayoutResult> {
        // import_root: hint -> project/src -> project
        let import_root = hints
            .module_root
            .as_ref()
            .map(|p| if p.is_absolute() { p.clone() } else { project_dir.join(p) })
            .unwrap_or_else(|| {
                let src = project_dir.join("src");
                if src.is_dir() { src } else { project_dir.to_path_buf() }
            });

        // venv_dir: hint -> heuristic
        let venv_dir = hints
            .venv
            .as_ref()
            .map(|p| if p.is_absolute() { p.clone() } else { project_dir.join(p) })
            .or_else(|| self.find_venv(project_dir))
            .ok_or(ParserError::MissingVirtualEnv)?;

        // site-packages
        let site_packages_dir = self.find_site_packages(&venv_dir)
            .ok_or(ParserError::MissingSitePackages)?;

        // module override -> discovery
        if let Some(name_raw) = hints.module.as_ref() {
            let name = name_raw.trim_end_matches(".py").to_string();
            let file_path = import_root.join(format!("{}.py", &name));
            let module_root = import_root.join(&name);

            if file_path.is_file() {
                return Ok(LayoutResult {
                    venv_dir,
                    site_packages_dir,
                    module_root: import_root,
                    module_name: name,
                });
            }

            if module_root.is_dir()
                && module_root.join("__init__.py").is_file()
            {
                return Ok(LayoutResult {
                    venv_dir,
                    site_packages_dir,
                    module_root,
                    module_name: name,
                });
            }

            return Err(ParserError::MissingModule);
        }

        // Discovery: find top-level directories under import_root that contain __init__.py.
        let mut candidates: HashSet<String> = HashSet::new();

        for p in files.iter().filter(|p| p.starts_with(&import_root)) {
            if p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s == "__init__.py")
                .unwrap_or(false)
            {
                if let Ok(rel) = p.strip_prefix(&import_root) {
                    if let Some(first) = rel.components().next() {
                        if let Some(s) = first.as_os_str().to_str() {
                            candidates.insert(s.to_string());
                        }
                    }
                }
            }
        }

        if candidates.len() != 1 {
            return Err(ParserError::MissingModule);
        }

        let module_name = candidates.into_iter().next().unwrap();
        let module_root = import_root.join(&module_name);

        if !module_root.is_dir() || !module_root.join("__init__.py").is_file() {
            return Err(ParserError::MissingModule);
        }

        Ok(LayoutResult {
            venv_dir,
            site_packages_dir,
            module_root,
            module_name,
        })
    }

    fn find_venv(&self, project_dir: &Path) -> Option<PathBuf> {
        for dir in ["venv", ".venv", "env", ".env"].iter() {
            let p = project_dir.join(dir);
            if p.is_dir() {
                return Some(p);
            }
        }
        None
    }

    fn find_site_packages(&self, venv_path: &Path) -> Option<PathBuf> {
        let lib_path = venv_path.join("lib");
        if !lib_path.is_dir() {
            return None;
        }

        for entry in read_dir(&lib_path).ok()? {
            let entry = entry.ok()?;
            if entry.file_name().to_str()?.starts_with("python") {
                let sp = entry.path().join("site-packages");
                if sp.is_dir() {
                    return Some(sp);
                }
            }
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::{
        fs::{self, File},
        io::Write,
    };

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(path).unwrap();
        writeln!(f, "{content}").unwrap();
    }

    fn make_venv(dir: &Path) {
        let lib = dir.join("lib/python3.11/site-packages");
        fs::create_dir_all(&lib).unwrap();
    }

    fn collect_files(root: &Path) -> Vec<PathBuf> {
        let mut out = vec![];

        for entry in root.read_dir().unwrap().flatten() {
            let path = entry.path();

            if path.is_file() {
                out.push(path);
            } else if path.is_dir() {
                out.extend(collect_files(&path));
            }
        }

        out
    }

    #[test]
    fn resolves_flat_layout() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        // project/my_package/__init__.py
        write(&root.join("my_package/__init__.py"), "");
        write(&root.join("my_package/utils.py"), "");

        // venv
        make_venv(&root.join("venv"));

        let files = collect_files(root);
        let hints = LayoutHints::default();

        let res = LayoutResolver::new()
            .resolve(root, &files, &hints)
            .unwrap();

        assert_eq!(res.module_name, "my_package");
        assert_eq!(res.module_root, root.join("my_package"));
    }

    #[test]
    fn resolves_src_layout() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        // project/src/my_package/__init__.py
        write(&root.join("src/my_package/__init__.py"), "");
        write(&root.join("src/my_package/mod.py"), "");

        make_venv(&root.join("venv"));

        let files = collect_files(root);
        let hints = LayoutHints::default();

        let res = LayoutResolver::new()
            .resolve(root, &files, &hints)
            .unwrap();

        assert_eq!(res.module_name, "my_package");
        assert_eq!(res.module_root, root.join("src/my_package"));
    }

    #[test]
    fn resolves_single_file_module() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        // project/my_module.py
        write(&root.join("my_module.py"), "");

        make_venv(&root.join("venv"));

        let files = collect_files(root);

        let hints = LayoutHints {
            module: Some("my_module".into()),
            ..Default::default()
        };

        let res = LayoutResolver::new()
            .resolve(root, &files, &hints)
            .unwrap();

        assert_eq!(res.module_name, "my_module");
        assert_eq!(res.module_root, root.join(""));
    }

    #[test]
    fn module_override_directory() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("src/custom_pkg/__init__.py"), "");
        write(&root.join("src/custom_pkg/a.py"), "");

        make_venv(&root.join("venv"));

        let files = collect_files(root);

        let hints = LayoutHints {
            module: Some("custom_pkg".into()),
            ..Default::default()
        };

        let res = LayoutResolver::new()
            .resolve(root, &files, &hints)
            .unwrap();

        assert_eq!(res.module_name, "custom_pkg");
        assert_eq!(res.module_root, root.join("src/custom_pkg"));
    }

    #[test]
    fn module_root_override() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("code/my_pkg/__init__.py"), "");

        make_venv(&root.join("venv"));

        let files = collect_files(root);

        let hints = LayoutHints {
            module_root: Some("code".into()),
            ..Default::default()
        };

        let res = LayoutResolver::new()
            .resolve(root, &files, &hints)
            .unwrap();

        println!("{:#?}", res.module_root);

        assert_eq!(res.module_root, root.join("code").join("my_pkg"));
        assert_eq!(res.module_name, "my_pkg");
    }

    #[test]
    fn venv_override() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("my_package/__init__.py"), "");

        let custom_venv = root.join("custom_env");
        make_venv(&custom_venv);

        let files = collect_files(root);

        let hints = LayoutHints {
            venv: Some("custom_env".into()),
            ..Default::default()
        };

        let res = LayoutResolver::new()
            .resolve(root, &files, &hints)
            .unwrap();

        assert_eq!(res.venv_dir, custom_venv);
    }

    #[test]
    fn missing_venv_errors() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("my_package/__init__.py"), "");

        let files = collect_files(root);

        let err = LayoutResolver::new()
            .resolve(root, &files, &LayoutHints::default())
            .unwrap_err();

        matches!(err, ParserError::MissingVirtualEnv);
    }

    #[test]
    fn missing_site_packages_errors() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("my_package/__init__.py"), "");

        fs::create_dir_all(root.join("venv/lib")).unwrap(); // but no pythonX.Y/site-packages

        let files = collect_files(root);

        let err = LayoutResolver::new()
            .resolve(root, &files, &LayoutHints::default())
            .unwrap_err();

        matches!(err, ParserError::MissingSitePackages);
    }

    #[test]
    fn multiple_package_candidates_errors() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("pkg_a/__init__.py"), "");
        write(&root.join("pkg_b/__init__.py"), "");

        make_venv(&root.join("venv"));

        let files = collect_files(root);

        let err = LayoutResolver::new()
            .resolve(root, &files, &LayoutHints::default())
            .unwrap_err();

        matches!(err, ParserError::MissingModule);
    }

    #[test]
    fn no_package_candidate_errors() {
        let td = TempDir::new().unwrap();
        let root = td.path();

        write(&root.join("misc/file.py"), "");

        make_venv(&root.join("venv"));

        let files = collect_files(root);

        let err = LayoutResolver::new()
            .resolve(root, &files, &LayoutHints::default())
            .unwrap_err();

        matches!(err, ParserError::MissingModule);
    }
}
