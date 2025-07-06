use godot::prelude::*;

mod display;
mod debug_line_3d;

struct Rogue3dRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for Rogue3dRustExtension {}