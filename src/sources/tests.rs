#[cfg(test)]
mod tests {
    use crate::config::{PackageConfig, SpagoConfig, WorkspaceConfig};
    use crate::registry::{LocalPackage, Package, PackageName, PackageSet, PackageSetPackage};
    use crate::sources::{
        find_package_directory, generate_dependency_glob, generate_sources, BuildSources,
        DependencyGlob,
    };
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    /// Helper function to create a temporary directory structure for testing
    fn create_test_spago_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let spago_dir = temp_dir.path().join(".spago");
        fs::create_dir_all(&spago_dir).unwrap();
        (temp_dir, spago_dir)
    }

    /// Helper function to create a test package directory with src files
    fn create_test_package_dir(spago_dir: &Path, package_name: &str) -> PathBuf {
        let package_dir = spago_dir.join(package_name);
        let src_dir = package_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Create some test .purs files
        fs::write(src_dir.join("Main.purs"), "module Main where").unwrap();
        fs::write(src_dir.join("Utils.purs"), "module Utils where").unwrap();

        package_dir
    }

    /// Helper function to create a minimal test config
    fn create_test_config(spago_dir: &Path) -> SpagoConfig {
        let package_config = PackageConfig {
            name: PackageName::new("test-package"),
            dependencies: vec![PackageName::new("prelude"), PackageName::new("console")],
            test: None,
        };

        let workspace_config = WorkspaceConfig::default();

        SpagoConfig {
            package: package_config,
            workspace: workspace_config,
            workspace_root: spago_dir.parent().unwrap().to_path_buf(),
        }
    }

    /// Helper function to create a test package set
    fn create_test_package_set() -> PackageSet {
        let mut packages = HashMap::new();

        // Add some test packages
        packages.insert(
            PackageName::new("prelude"),
            Package::Remote(PackageSetPackage {
                name: PackageName::new("prelude"),
                repo: "https://github.com/purescript/purescript-prelude.git".to_string(),
                version: "v6.0.1".to_string(),
                dependencies: vec![],
            }),
        );

        packages.insert(
            PackageName::new("console"),
            Package::Remote(PackageSetPackage {
                name: PackageName::new("console"),
                repo: "https://github.com/purescript/purescript-console.git".to_string(),
                version: "v6.0.0".to_string(),
                dependencies: vec![PackageName::new("prelude")],
            }),
        );

        packages
    }

    #[test]
    fn test_dependency_glob_creation() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let package_name = PackageName::new("test-package");
        let package_dir = create_test_package_dir(&spago_dir, "test-package");
        let package_set = create_test_package_set();

        let result = generate_dependency_glob(&package_name, &spago_dir, &package_set, false);

        assert!(result.is_ok());
        let glob = result.unwrap();
        assert!(glob.is_some());

        let glob = glob.unwrap();
        assert_eq!(glob.package_name, "test-package");
        assert!(glob.glob_pattern.contains("test-package"));
        assert!(glob.glob_pattern.ends_with("/**/*.purs"));
        assert_eq!(glob.local_path, package_dir);
    }

    #[test]
    fn test_dependency_glob_without_src_directory() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let package_name = PackageName::new("test-package");
        let package_dir = spago_dir.join("test-package");
        fs::create_dir_all(&package_dir).unwrap();
        // Don't create src directory
        let package_set = create_test_package_set();

        let result = generate_dependency_glob(&package_name, &spago_dir, &package_set, false);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Package test-package not found"));
    }

    #[test]
    fn test_find_package_directory_local_package() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let package_name = PackageName::new("local-package");
        let local_path = spago_dir.join("local-package");
        fs::create_dir_all(&local_path).unwrap();

        let mut packages = HashMap::new();
        packages.insert(
            package_name.clone(),
            Package::Local(LocalPackage {
                name: package_name.clone(),
                path: local_path.clone(),
                test_dependencies: vec![],
                dependencies: vec![],
            }),
        );
        let package_set = packages;

        let result = find_package_directory(&package_name, &spago_dir, &package_set);

        assert!(result.is_ok());
        let found_path = result.unwrap();
        assert!(found_path.is_some());
        assert_eq!(found_path.unwrap(), local_path);
    }

    #[test]
    fn test_find_package_directory_in_spago_dir() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let package_name = PackageName::new("installed-package");
        let package_dir = spago_dir.join("installed-package");
        fs::create_dir_all(&package_dir).unwrap();
        let package_set = create_test_package_set();

        let result = find_package_directory(&package_name, &spago_dir, &package_set);

        assert!(result.is_ok());
        let found_path = result.unwrap();
        assert!(found_path.is_some());
        assert_eq!(found_path.unwrap(), package_dir);
    }

    #[test]
    fn test_find_package_directory_not_found() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let package_name = PackageName::new("nonexistent-package");
        let package_set = create_test_package_set();

        let result = find_package_directory(&package_name, &spago_dir, &package_set);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .to_string()
            .contains("Package nonexistent-package not found"));
    }

    #[test]
    fn test_generate_sources_without_spago_dir() {
        let temp_dir = TempDir::new().unwrap();
        let spago_dir = temp_dir.path().join(".spago");
        // Don't create .spago directory
        let config = create_test_config(&spago_dir);
        let package_set = create_test_package_set();

        let result = generate_sources(&config, Some(package_set), false, false);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("No .spago directory found"));
    }

    #[test]
    fn test_generate_sources_basic() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let config = create_test_config(&spago_dir);
        let package_set = create_test_package_set();

        // Create test packages
        create_test_package_dir(&spago_dir, "prelude");
        create_test_package_dir(&spago_dir, "console");

        let result = generate_sources(&config, Some(package_set), false, false);

        assert!(result.is_ok());
        let sources = result.unwrap();

        assert_eq!(sources.main_sources, "./src/**/*.purs");
        assert!(!sources.dependency_globs.is_empty());

        // Check that we have globs for our dependencies
        let glob_names: Vec<String> = sources
            .dependency_globs
            .iter()
            .map(|g| g.package_name.clone())
            .collect();
        assert!(glob_names.contains(&"prelude".to_string()));
        assert!(glob_names.contains(&"console".to_string()));
    }

    #[test]
    fn test_generate_sources_filters_main_sources() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let config = create_test_config(&spago_dir);
        let package_set = create_test_package_set();

        // Create a package that would generate the same pattern as main sources
        let _package_dir = create_test_package_dir(&spago_dir, "prelude");
        create_test_package_dir(&spago_dir, "console");
        // Create a src directory that would match main sources pattern
        let main_src_dir = spago_dir.parent().unwrap().join("src");
        fs::create_dir_all(&main_src_dir).unwrap();
        fs::write(main_src_dir.join("Main.purs"), "module Main where").unwrap();

        let result = generate_sources(&config, Some(package_set), false, false);

        assert!(result.is_ok());
        let sources = result.unwrap();

        // Should not include any globs that match the main sources pattern
        for glob in &sources.dependency_globs {
            assert_ne!(glob.glob_pattern, "./src/**/*.purs");
        }
    }

    #[test]
    fn test_build_sources_struct() {
        let dependency_glob = DependencyGlob {
            package_name: "test-package".to_string(),
            glob_pattern: "/path/to/test-package/src/**/*.purs".to_string(),
            local_path: PathBuf::from("/path/to/test-package"),
        };

        let sources = BuildSources {
            dependency_globs: vec![dependency_glob.clone()],
            main_sources: "./src/**/*.purs".to_string(),
        };

        assert_eq!(sources.dependency_globs.len(), 1);
        assert_eq!(sources.dependency_globs[0].package_name, "test-package");
        assert_eq!(sources.main_sources, "./src/**/*.purs");
    }

    #[test]
    fn test_dependency_glob_clone() {
        let original = DependencyGlob {
            package_name: "test-package".to_string(),
            glob_pattern: "/path/to/test-package/src/**/*.purs".to_string(),
            local_path: PathBuf::from("/path/to/test-package"),
        };

        let cloned = original.clone();

        assert_eq!(original.package_name, cloned.package_name);
        assert_eq!(original.glob_pattern, cloned.glob_pattern);
        assert_eq!(original.local_path, cloned.local_path);
    }

    #[test]
    fn test_generate_sources_verbose_output() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let config = create_test_config(&spago_dir);
        let package_set = create_test_package_set();

        // Create test packages
        create_test_package_dir(&spago_dir, "prelude");
        create_test_package_dir(&spago_dir, "console");

        // Capture stdout to test verbose output
        let result = generate_sources(&config, Some(package_set), false, true);

        assert!(result.is_ok());
        let sources = result.unwrap();

        // Should have generated at least one dependency glob
        assert!(!sources.dependency_globs.is_empty());
    }

    #[test]
    fn test_generate_sources_with_empty_dependencies() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();

        // Create config with no dependencies
        let mut config = create_test_config(&spago_dir);
        config.package.dependencies = vec![];

        let package_set = create_test_package_set();

        let result = generate_sources(&config, Some(package_set), false, false);

        assert!(result.is_ok());
        let sources = result.unwrap();

        assert_eq!(sources.main_sources, "./src/**/*.purs");
        assert!(sources.dependency_globs.is_empty());
    }

    #[test]
    fn test_generate_sources_with_missing_package() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();
        let config = create_test_config(&spago_dir);
        let package_set = create_test_package_set();

        // Don't create the package directories

        let result = generate_sources(&config, Some(package_set), false, false);

        // Should fail because packages are missing
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = error.to_string();
        // The error could be about any missing package (prelude, console, etc.)
        // and could come from either find_package_directory or generate_dependency_glob
        assert!(
            error_msg.contains("not found in .spago")
                || error_msg.contains("not found. Couldn't generate a glob for it.")
        );
    }

    #[test]
    fn test_generate_sources_circular_dependency_does_not_include_current_package() {
        let (_temp_dir, spago_dir) = create_test_spago_dir();

        // Create a config where the current package depends on itself (circular dependency)
        let mut config = create_test_config(&spago_dir);
        config.package.dependencies = vec![
            PackageName::new("prelude"),
            PackageName::new("console"),
            config.package.name.clone(), // Add self as dependency (circular)
        ];

        let mut package_set = create_test_package_set();

        // Add the current package to the package set for circular dependency test
        package_set.insert(
            config.package.name.clone(),
            Package::Local(LocalPackage {
                name: config.package.name.clone(),
                path: PathBuf::from("./test"),
                test_dependencies: vec![],
                dependencies: vec![], // The circular dependency is in the config, not the package set
            }),
        );

        // Create package directories
        create_test_package_dir(&spago_dir, "prelude");
        create_test_package_dir(&spago_dir, "console");

        // Don't create directory for the current package (test-package) since it's the main package
        let result = generate_sources(&config, Some(package_set), false, false);

        assert!(result.is_ok());
        let sources = result.unwrap();

        // Check that the current package is NOT included in dependency globs
        let glob_names: Vec<String> = sources
            .dependency_globs
            .iter()
            .map(|g| g.package_name.clone())
            .collect();

        // Should not include the current package name in dependency globs
        assert!(!glob_names.contains(&config.package.name.0));
        assert_eq!(sources.main_sources, "./src/**/*.purs");
    }
}
