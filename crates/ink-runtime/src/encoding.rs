use rquickjs::{Array, Ctx, Exception, Function, Result, TypedArray};

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals()
        .set("__inkEncodeUtf8", Function::new(ctx.clone(), encode)?)?;
    ctx.globals()
        .set("__inkDecodeUtf8", Function::new(ctx.clone(), decode)?)?;
    Ok(())
}

fn encode(ctx: Ctx<'_>, source: String) -> Result<TypedArray<'_, u8>> {
    TypedArray::new(ctx, source.into_bytes())
}

fn decode<'js>(
    ctx: Ctx<'js>,
    input: TypedArray<'js, u8>,
    fatal: bool,
    stream: bool,
) -> Result<Array<'js>> {
    let bytes = input
        .as_bytes()
        .ok_or_else(|| Exception::throw_type(&ctx, "Detached input buffer"))?;
    let mut consumed = 0;
    while consumed < bytes.len() {
        match std::str::from_utf8(&bytes[consumed..]) {
            Ok(_) => {
                consumed = bytes.len();
                break;
            }
            Err(error) => {
                consumed += error.valid_up_to();
                match error.error_len() {
                    None if stream => break,
                    _ if fatal => return Err(Exception::throw_type(&ctx, "Invalid UTF-8 data")),
                    Some(length) => consumed += length,
                    None => consumed = bytes.len(),
                }
            }
        }
    }
    let result = Array::new(ctx)?;
    result.set(0, String::from_utf8_lossy(&bytes[..consumed]).as_ref())?;
    result.set(1, consumed)?;
    Ok(result)
}
