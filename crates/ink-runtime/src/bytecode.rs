use std::ffi::CString;

use anyhow::{Result, anyhow, bail};
use rquickjs::{Context, Ctx, Runtime, qjs};

/// Ink bytecode format and version.
const MAGIC: &[u8; 8] = b"INKQJS\x01\x00";

/// JavaScript to evaluate when a runtime starts.
pub enum Script {
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
        Ok(Self::Source(String::from_utf8(bytes).map_err(|_| anyhow!("JavaScript bundle is not UTF-8"))?))
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

/// Compile a global script to QuickJS bytecode.
///
/// Function source text is omitted. Line information remains so stack traces keep their positions.
pub fn compile(source: &str, filename: &str) -> Result<Vec<u8>> {
    let runtime = Runtime::new()?;
    let context = Context::full(&runtime)?;
    context.with(|ctx| {
        let source = CString::new(source)?;
        let filename = CString::new(filename)?;
        let raw = ctx.as_raw().as_ptr();
        unsafe {
            let function = qjs::JS_Eval(
                raw,
                source.as_ptr(),
                source.as_bytes().len() as _,
                filename.as_ptr(),
                (qjs::JS_EVAL_TYPE_GLOBAL | qjs::JS_EVAL_FLAG_STRICT | qjs::JS_EVAL_FLAG_COMPILE_ONLY) as i32,
            );
            if qjs::JS_IsException(function) {
                let error = rquickjs::CaughtError::from_error(&ctx, rquickjs::Error::Exception);
                bail!("could not compile {}: {error}", filename.to_string_lossy());
            }
            let mut length: qjs::size_t = 0;
            let bytes = qjs::JS_WriteObject(
                raw,
                &mut length,
                function,
                (qjs::JS_WRITE_OBJ_BYTECODE | qjs::JS_WRITE_OBJ_STRIP_SOURCE) as i32,
            );
            qjs::JS_FreeValue(raw, function);
            if bytes.is_null() {
                bail!("could not serialise {}", filename.to_string_lossy());
            }
            let mut output = Vec::with_capacity(MAGIC.len() + length as usize);
            output.extend_from_slice(MAGIC);
            output.extend_from_slice(std::slice::from_raw_parts(bytes, length as usize));
            qjs::js_free(raw, bytes.cast());
            Ok(output)
        }
    })
}

pub(crate) fn evaluate(ctx: &Ctx<'_>, script: Script, filename: &str) -> Result<()> {
    match script {
        Script::Source(source) => ctx
            .eval_with_options::<(), _>(source, {
                let mut options = rquickjs::context::EvalOptions::default();
                options.filename = Some(filename.into());
                options
            })
            .map_err(|error| anyhow!("{}", rquickjs::CaughtError::from_error(ctx, error))),
        Script::Bytecode(bytes) => {
            let bytes = bytes
                .strip_prefix(MAGIC.as_slice())
                .ok_or_else(|| anyhow!("{filename} is not Ink bytecode for this runtime"))?;
            let raw = ctx.as_raw().as_ptr();
            // Bytecode comes from the signed app package and was written by this QuickJS build.
            let result = unsafe {
                let function = qjs::JS_ReadObject(raw, bytes.as_ptr(), bytes.len() as _, qjs::JS_READ_OBJ_BYTECODE as i32);
                if qjs::JS_IsException(function) {
                    function
                } else {
                    qjs::JS_EvalFunction(raw, function)
                }
            };
            if unsafe { qjs::JS_IsException(result) } {
                return Err(anyhow!("{}", rquickjs::CaughtError::from_error(ctx, rquickjs::Error::Exception)));
            }
            unsafe { qjs::JS_FreeValue(raw, result) };
            Ok(())
        }
        Script::Load(load) => evaluate(ctx, load()?, filename),
    }
}
