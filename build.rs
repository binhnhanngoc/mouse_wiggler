fn main() {
    // Embed manifest (DPI awareness, Common Controls v6) and icon.
    embed_resource::compile("app.rc", embed_resource::NONE);
}
