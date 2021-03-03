mod application_gui;
mod application_core;
mod fm_mio;
mod debug_device;
mod elf_loader;
mod gpu;
mod raw_fb_renderer;
mod fm_interrupt_bus;
mod fb_present_renderer;
mod dsp_dma;
mod cpu1_controller;
mod sound_device;

use application_gui::ApplicationGUI;

fn main() {
	ApplicationGUI::run();
}
