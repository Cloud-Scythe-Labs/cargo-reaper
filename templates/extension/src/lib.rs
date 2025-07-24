#[reaper_macros::reaper_extension_plugin]
fn plugin_main(context: reaper_low::PluginContext) -> Result<(), Box<dyn std::error::Error>> {
    reaper_medium::ReaperSession::load(context)
        .reaper()
        .show_console_msg("Hello, world!");

    Ok(())
}
