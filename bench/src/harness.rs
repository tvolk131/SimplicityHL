//! Writing generated programs to disk and driving pipeline stages in
//! isolation.
//!
//! Stages after parsing need a [`CanonSourceFile`] whose path exists on disk
//! (the driver canonicalizes paths), so even single-file programs are
//! materialized into a temporary package. The directory is removed on drop.

use crate::corpus::GeneratedProgram;
use simplicityhl::resolution::{DependencyMap, DependencyMapBuilder};
use simplicityhl::source::{CanonPath, CanonSourceFile};
use simplicityhl::{UnstableFeature, UnstableFeatures};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// The features the corpus is compiled with.
///
/// `imports` and `enums` are unstable in the language, but the multi-file
/// shapes need imports and `examples/last_will.simf` needs enums; the
/// benchmark measures the compiler, not the feature gates.
pub fn unstable() -> UnstableFeatures {
    UnstableFeatures::new(UnstableFeature::ALL.iter().copied())
}

/// A temporary directory holding a materialized [`GeneratedProgram`].
pub struct TempPackage {
    dir: PathBuf,
}

impl TempPackage {
    /// Create an empty package directory under the system temp dir.
    pub fn new(tag: &str) -> Result<Self, String> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "simfonyhl-bench-{tag}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        Ok(Self { dir })
    }

    /// The package directory; it serves as the `crate::` root.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn write(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

impl Drop for TempPackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A generated program materialized on disk, ready to be fed to the compiler.
pub struct Materialized {
    package: TempPackage,
    entry_rel: String,
    /// The entry-file content; shared with the `CanonSourceFile` built by
    /// [`Materialized::source_file`], like `simc` does with the file it read.
    entry_content: Arc<str>,
}

impl Materialized {
    pub fn new(program: &GeneratedProgram, tag: &str) -> Result<Self, String> {
        let package = TempPackage::new(tag)?;
        let (entry_rel, entry_content) = match program {
            GeneratedProgram::Single { name, source } => {
                let file_name = format!("{}.simf", name.replace(':', "_"));
                package.write(&file_name, source)?;
                let content: Arc<str> = Arc::from(source.as_str());
                (file_name, content)
            }
            GeneratedProgram::Package { files, entry, .. } => {
                for (rel, content) in files {
                    package.write(rel, content)?;
                }
                let content: Arc<str> = Arc::from(
                    files
                        .iter()
                        .find(|(path, _)| path == entry)
                        .expect("entry in files")
                        .1
                        .as_str(),
                );
                (entry.clone(), content)
            }
        };
        Ok(Self {
            package,
            entry_rel,
            entry_content,
        })
    }

    /// The entry file as a [`CanonSourceFile`], canonicalized against the
    /// package root.
    pub fn source_file(&self) -> Result<CanonSourceFile, String> {
        let canon = CanonPath::canonicalize(&self.package.dir().join(&self.entry_rel))?;
        Ok(CanonSourceFile::new(canon, Arc::clone(&self.entry_content)))
    }

    /// The dependency map rooted at the package directory (no `--dep`
    /// remappings; generated packages only use `crate::` paths).
    pub fn dependency_map(&self) -> Result<DependencyMap, String> {
        let root = CanonPath::canonicalize(self.package.dir())?;
        DependencyMapBuilder::new()
            .build(root)
            .map_err(|e| e.to_string())
    }
}

/// Materialize a program and compile it end-to-end, returning the rendered
/// diagnostics on failure. Used by `corpus-gen --check` to keep the corpus
/// honest.
pub fn compile_end_to_end(program: &GeneratedProgram) -> Result<(), String> {
    let materialized = Materialized::new(program, "check")?;
    let dependencies = Arc::new(materialized.dependency_map()?);
    let template = simplicityhl::TemplateProgram::new_with_dep(
        materialized.source_file()?,
        &dependencies,
        &unstable(),
        Box::new(simplicityhl::ast::ElementsJetHinter::new()),
    )
    .map_err(|diags| diags.to_string())?;
    let arguments = arguments_for(&template, program.name())?;
    let compiled = template
        .instantiate(arguments, false)
        .map_err(|e| e.to_string())?;
    let bytes = compiled.commit().to_vec_without_witness();
    if bytes.is_empty() {
        return Err("serialized program is empty".to_string());
    }
    Ok(())
}

/// The program arguments for `name`: parsed from the example's `.args` file
/// when one exists, empty otherwise.
pub fn arguments_for(
    template: &simplicityhl::TemplateProgram,
    name: &str,
) -> Result<simplicityhl::Arguments, String> {
    match crate::corpus::example_args_text(name) {
        None => Ok(simplicityhl::Arguments::default()),
        Some(text) => {
            let unresolved: simplicityhl::UnresolvedValues = serde_json::from_str(&text)
                .map_err(|e| format!("cannot parse args for {name}: {e}"))?;
            unresolved
                .resolve(template.parameters())
                .map_err(|e| format!("cannot resolve args for {name}: {e}"))
        }
    }
}
