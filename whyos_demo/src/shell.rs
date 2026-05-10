use crate::{board::Uart, uart::{SHELL_RX_QUEUE, hw_print, init_uart}};
use whyos_shell::{Program, Shell};

pub fn init_shell(uart: Uart) {
    init_uart(uart);

    whyos::TaskBuilder::new(crate::shell::shell_task)
        .priority(7)
        .stack_size(whyos::StackSize::LARGE)
        .name("shell")
        .spawn()
        .unwrap();
}

static MY_PROGRAMS: &[Program] = &[];

#[unsafe(no_mangle)]
pub extern "C" fn shell_task() {
    let mut shell = Shell::new(&SHELL_RX_QUEUE, hw_print, MY_PROGRAMS);
    shell.run();
}