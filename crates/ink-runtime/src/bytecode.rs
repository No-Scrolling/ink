use anyhow::{Result, anyhow};
use rquickjs::{Ctx, Exception, FromJs, Value, qjs};

mod compiler;
use compiler::MAGIC;
pub use compiler::compile;

/// JavaScript to evaluate when a runtime starts.
pub enum Script {
    /// JavaScript source for development and host runtimes. Android releases reject it.
    Source(String),
    /// Output of [`compile`], produced by the same QuickJS build as this runtime.
    Bytecode(Vec<u8>),
    /// Read the script on the JavaScript thread, keeping file IO off the caller's thread.
    Load(Box<dyn FnOnce() -> Result<Script> + Send>),
}

impl Script {
    /// Interpret a bundle file: Ink bytecode when it carries the bytecode header, otherwise UTF-8 source.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.starts_with(MAGIC) {
            return Ok(Self::Bytecode(bytes));
        }
        Ok(Self::Source(
            String::from_utf8(bytes).map_err(|_| anyhow!("JavaScript bundle is not UTF-8"))?,
        ))
    }
}

impl From<String> for Script {
    fn from(source: String) -> Self {
        Self::Source(source)
    }
}

impl From<&str> for Script {
    fn from(source: &str) -> Self {
        Self::Source(source.to_owned())
    }
}

pub(crate) fn evaluate(ctx: &Ctx<'_>, script: Script, filename: &str) -> Result<()> {
    match script {
        Script::Source(source) => {
            anyhow::ensure!(
                !cfg!(all(target_os = "android", not(debug_assertions))),
                "{filename}: Android release runtimes require precompiled Ink bytecode; JavaScript source evaluation is unavailable"
            );
            ctx.eval_with_options::<(), _>(source, {
                let mut options = rquickjs::context::EvalOptions::default();
                options.filename = Some(filename.into());
                options
            })
            .map_err(|error| anyhow!("{}", rquickjs::CaughtError::from_error(ctx, error)))
        }
        Script::Bytecode(bytes) => evaluate_bytecode::<()>(ctx, &bytes).map_err(|error| {
            anyhow!(
                "{filename}: {}",
                rquickjs::CaughtError::from_error(ctx, error)
            )
        }),
        Script::Load(load) => evaluate(ctx, load()?, filename),
    }
}

pub(crate) fn evaluate_bytecode<'js, T: FromJs<'js>>(
    ctx: &Ctx<'js>,
    bytes: &[u8],
) -> rquickjs::Result<T> {
    let bytes = bytes
        .strip_prefix(MAGIC.as_slice())
        .ok_or_else(|| Exception::throw_type(ctx, "not Ink bytecode for this runtime"))?;
    let raw = ctx.as_raw().as_ptr();
    // Only load trusted bytecode built with this engine: embedded shims or signed APK assets.
    let result = unsafe {
        let function = qjs::JS_ReadObject(
            raw,
            bytes.as_ptr(),
            bytes.len() as _,
            qjs::JS_READ_OBJ_BYTECODE as i32,
        );
        if qjs::JS_IsException(function) {
            function
        } else {
            qjs::JS_EvalFunction(raw, function)
        }
    };
    if unsafe { qjs::JS_IsException(result) } {
        return Err(rquickjs::Error::Exception);
    }
    let value = unsafe { Value::from_raw(ctx.clone(), result) };
    T::from_js(ctx, value)
}
