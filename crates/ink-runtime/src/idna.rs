use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};
use rquickjs::{Ctx, Function, IntoJs, Result, Value};

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals()
        .set("__inkDomainToASCII", Function::new(ctx.clone(), convert)?)
}

fn convert<'js>(ctx: Ctx<'js>, domain: String) -> Result<Value<'js>> {
    // Match whatwg-url's non-strict, nontransitional tr46 options. Uts46 always
    // checks bidi and joiners; whatwg-url checks forbidden domain code points.
    match Uts46::new().to_ascii(
        domain.as_bytes(),
        AsciiDenyList::EMPTY,
        Hyphens::Allow,
        DnsLength::Ignore,
    ) {
        Ok(value) => value.as_ref().into_js(&ctx),
        Err(_) => Ok(Value::new_null(ctx)),
    }
}
