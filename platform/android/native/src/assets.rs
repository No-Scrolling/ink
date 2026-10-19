use std::{ffi::{CStr, CString}, io::Read, ptr::NonNull, sync::Arc};

use anyhow::{Context, Result};
use ink_runtime::Script;
use jni::{Env, objects::JObject, refs::Global};
use ndk::asset::AssetManager;

struct Manager {
    pointer: NonNull<ndk_sys::AAssetManager>,
    // The native manager is valid while its Java owner is reachable.
    _owner: Global<JObject<'static>>,
}

// AAssetManager is documented as safe for concurrent use; the global reference keeps it alive.
unsafe impl Send for Manager {}
unsafe impl Sync for Manager {}

/// The APK's assets, readable from any thread.
#[derive(Clone)]
pub(super) struct BundleAssets(Arc<Manager>);

impl BundleAssets {
    pub(super) fn new(env: &mut Env<'_>, manager: &JObject<'_>) -> jni::errors::Result<Self> {
        let owner = env.new_global_ref(manager)?;
        let pointer = unsafe { ndk_sys::AAssetManager_fromJava(env.get_raw().cast(), manager.as_raw().cast()) };
        let pointer = NonNull::new(pointer).ok_or(jni::errors::Error::NullPtr("AssetManager"))?;
        Ok(Self(Arc::new(Manager { pointer, _owner: owner })))
    }

    fn open(&self, name: &CStr) -> Option<ndk::asset::Asset> {
        unsafe { AssetManager::from_ptr(self.0.pointer) }.open(name)
    }

    #[cfg(feature = "store")]
    pub(super) fn java_manager(&self) -> &JObject<'static> {
        self.0._owner.as_ref()
    }

    pub(super) fn read(&self, name: &CStr) -> Result<Vec<u8>> {
        contents(self.open(name).with_context(|| format!("missing asset {}", name.to_string_lossy()))?)
    }

    /// Load `<stem>.jsbc` bytecode, falling back to `<stem>.js` source.
    pub(super) fn script(&self, stem: &str) -> Result<Script> {
        if let Some(asset) = self.open(&CString::new(format!("{stem}.jsbc"))?) {
            return Ok(Script::Bytecode(contents(asset)?));
        }
        let source = self.read(&CString::new(format!("{stem}.js"))?)?;
        Ok(Script::Source(String::from_utf8(source).context("JavaScript asset is not UTF-8")?))
    }
}

fn contents(mut asset: ndk::asset::Asset) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(asset.length());
    asset.read_to_end(&mut bytes)?;
    Ok(bytes)
}
