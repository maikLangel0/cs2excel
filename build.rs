use {
    std::{env, io},
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    if let Some(_var_os) = env::var_os("CARGO_CFG_WINDOWS") {
        WindowsResource::new()
            .set_icon("assets\\images\\whatsapp_is_calling.ico")
            .compile()?;
    }
    Ok(())
}