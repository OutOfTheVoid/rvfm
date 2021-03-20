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
mod mtimer;
mod math_accel;
mod cart_loader;
mod sound_out;

use application_gui::ApplicationGUI;

fn main() {
	ApplicationGUI::run();
}
