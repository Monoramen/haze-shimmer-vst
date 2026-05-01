use nih_plug::prelude::nih_export_standalone;
use shimmer_granular::PluginType;

fn main() {
    nih_export_standalone::<PluginType>();
}
