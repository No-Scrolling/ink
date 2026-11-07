use std::ffi::CString;

use anyhow::{Result, bail};
use rquickjs::{Context, Runtime, qjs};

pub(super) const MAGIC: &[u8; 8] = b"INKQJS\x01\x00";

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
                (qjs::JS_EVAL_TYPE_GLOBAL
                    | qjs::JS_EVAL_FLAG_STRICT
                    | qjs::JS_EVAL_FLAG_COMPILE_ONLY) as i32,
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
