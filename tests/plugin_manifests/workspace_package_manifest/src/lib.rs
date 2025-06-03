use reaper_macros::reaper_extension_plugin;
use std::error::Error;
use std::sync::OnceLock;
use fragile::Fragile;
use reaper_low::PluginContext;
use reaper_medium::{ProjectRef, Reaper, ReaperSession};

/// REAPER will call this extension entry-point function once when it's starting.
#[reaper_extension_plugin]
fn plugin_main(context: PluginContext) -> Result<(), Box<dyn Error>> {
    // Create a medium-level API session
    let mut session = ReaperSession::load(context);
    // Get access to most REAPER functions
    let reaper = session.reaper();
    // Print message to the ReaScript console
    reaper.show_console_msg("Hello from reaper-rs!\n");
    // Print the file path of the currently loaded project. Extensions are loaded very early
    // in the REAPER starting process. At that time, REAPER's last project has probably not
    // been loaded yet, so this will likely give `None`.
    print_project_file_path(reaper);
    // Now let's register a timer to get a place in REAPER's main event loop and defer stuff
    // for later execution.
    session.plugin_register_add_timer(timer_callback)?;
    // But wait ... we can't just stop here, otherwise `session` would be dropped (because it
    // goes out of scope), and the timer would be unregistered without ever firing.
    // We need to make sure that `session` lives longer than the execution of the `plugin_main` function!
    // We do that by saving it in a static variable.
    let _ = REAPER_SESSION.set(Fragile::new(session));
    Ok(())
}

extern "C" fn timer_callback() {
    let reaper_session = reaper_session();
    print_project_file_path(reaper_session.reaper());
}

fn print_project_file_path(reaper: &Reaper) {
    let project_path = reaper
        .enum_projects(ProjectRef::Current, 256)
        .map(|project| project.file_path);
    reaper.show_console_msg(format!("Project path: {project_path:?}\n"));
}

fn reaper_session() -> &'static ReaperSession {
    REAPER_SESSION.get().expect("ReaperSession hasn't been set as static variable yet").get()
}

/// The static variable that keeps the reaper-medium [`ReaperSession`] in memory while REAPER
/// is running.
///
/// - `OnceLock` is the standard Rust way to set static variables that will be initialized once and never again.
/// - `Fragile` comes from a third-party crate and makes sure that the session is only ever accessed from the main
///   thread. This is important because [`ReaperSession`] is not thread-safe. Without [`Fragile`], it wouldn't even
///   compile, thanks to Rust's safety rules :)
static REAPER_SESSION: OnceLock<Fragile<ReaperSession>> = OnceLock::new();
