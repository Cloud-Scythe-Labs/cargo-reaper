#![allow(deprecated)]
//! VST2 is deprecated, however, this is still the most up-to-date example
//! in the `reaper-rs` README. This template should be updated as soon as
//! possible to account for the change in standard.

use vst::plugin::{HostCallback, Info, Plugin};

reaper_low::reaper_vst_plugin!();

#[derive(Default)]
struct ReaperVstPlugin {
    host: HostCallback,
}

impl Plugin for ReaperVstPlugin {
    fn new(host: HostCallback) -> Self {
        Self { host }
    }

    fn get_info(&self) -> Info {
        Info {
            name: "REAPER VST Plugin".to_string(),
            unique_id: 6830,
            ..Default::default()
        }
    }

    fn init(&mut self) {
        if let Ok(context) = reaper_low::PluginContext::from_vst_plugin(
            &self.host,
            reaper_low::static_plugin_context(),
        ) {
            reaper_medium::ReaperSession::load(context)
                .reaper()
                .show_console_msg("Hello, world!");
        }
    }
}

vst::plugin_main!(ReaperVstPlugin);
