use {
    std::{env, io},
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
    
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_icon("cat_glorp.ico")
            .compile()?;
    }
    Ok(())
}
