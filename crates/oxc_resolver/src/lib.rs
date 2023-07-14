//! # Oxc Resolver
//!
//! Tests ported from [enhanced-resolve](https://github.com/webpack/enhanced-resolve).
//!
//! Algorithm from <https://nodejs.org/api/modules.html#all-together>.

mod error;
mod package_json;
mod path;
mod request;

use std::{
    fs,
    path::{Path, PathBuf},
};

use package_json::PackageJson;

pub use crate::error::{JSONError, ResolveError};
use crate::{path::ResolvePath, request::Request};

pub type ResolveResult = Result<PathBuf, ResolveError>;
type ResolveState = Result<Option<PathBuf>, ResolveError>;

pub struct Resolver;

impl Resolver {
    pub fn new() -> Self {
        Self
    }

    /// Resolve `request` at `path`
    ///
    /// # Errors
    ///
    /// * Will return `Err` for [ResolveError]
    pub fn resolve<P: AsRef<Path>>(&self, path: P, request: &str) -> ResolveResult {
        self.resolve_impl(path.as_ref(), request)
    }

    fn resolve_impl(&self, path: &Path, request: &str) -> ResolveResult {
        let request = Request::try_from(request).map_err(ResolveError::RequestError)?;
        let path = ResolvePath::from(path);

        match request {
            Request::Relative(_) => {
                let path = path.join(&request);
                if let Some(path) = self.load_as_file(&path)? {
                    return Ok(path);
                }
                if let Some(path) = self.load_as_directory(&path)? {
                    return Ok(path);
                }
                Err(ResolveError::NotFound)
            }
            Request::Absolute(_) => {
                unreachable!()
            }
        }
    }

    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn load_as_file(&self, path: &Path) -> ResolveState {
        // 1. If X is a file, load X as its file extension format. STOP
        if path.is_file() {
            return Ok(Some(path.to_path_buf()));
        }
        // 2. If X.js is a file, load X.js as JavaScript text. STOP
        let path_js = path.with_extension("js");
        if path_js.is_file() {
            return Ok(Some(path_js));
        }
        // 3. If X.json is a file, parse X.json to a JavaScript Object. STOP
        // 4. If X.node is a file, load X.node as binary addon. STOP
        Ok(None)
    }

    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn load_index(&self, path: &Path) -> ResolveState {
        // 1. If X/index.js is a file, load X/index.js as JavaScript text. STOP
        if path.with_file_name("index.js").is_file() {
            return Ok(Some(path.with_file_name("index.js")));
        }
        // 2. If X/index.json is a file, parse X/index.json to a JavaScript object. STOP
        // 3. If X/index.node is a file, load X/index.node as binary addon. STOP
        Ok(None)
    }

    fn load_as_directory(&self, path: &Path) -> ResolveState {
        // 1. If X/package.json is a file,
        let package_json_path = path.join("package.json");
        if package_json_path.is_file() {
            // a. Parse X/package.json, and look for "main" field.
            let package_json_string = fs::read_to_string(&package_json_path).unwrap();
            let package_json = PackageJson::try_from(package_json_string.as_str())
                .map_err(|error| ResolveError::from_serde_json_error(package_json_path, &error))?;
            // b. If "main" is a falsy value, GOTO 2.
            if let Some(main_field) = &package_json.main {
                // c. let M = X + (json main field)
                let main_field_path = path.join(main_field);
                // d. LOAD_AS_FILE(M)
                if let Some(path) = self.load_as_file(&main_field_path)? {
                    return Ok(Some(path));
                }
                // e. LOAD_INDEX(M)
                if let Some(path) = self.load_as_file(&main_field_path)? {
                    return Ok(Some(path));
                }
                // f. LOAD_INDEX(X) DEPRECATED
            }
            // g. THROW "not found"
            return Err(ResolveError::NotFound);
        }
        // 2. LOAD_INDEX(X)
        self.load_index(path)
    }
}