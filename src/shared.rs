use std::io::{stderr, stdout};

use anyhow::Result;
use gix::features::progress;

pub const DEFAULT_FRAME_RATE: f32 = 6.0;

pub type ProgressRange = std::ops::RangeInclusive<prodash::progress::key::Level>;
pub const STANDARD_RANGE: ProgressRange = 2..=2;

/// If verbose is true, the env logger will be forcibly set to 'info' logging level. Otherwise env logging facilities
/// will just be initialized.
#[allow(unused)] // Squelch warning because it's used in porcelain as well and we can't know that at compile time
pub fn init_env_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_module_path(false)
        .init();
}

pub fn prepare_and_run<T: Send + 'static>(
    name: &str,
    verbose: bool,
    progress: bool,
    #[cfg_attr(not(feature = "prodash-render-tui"), allow(unused_variables))]
    progress_keep_open: bool,
    range: impl Into<Option<ProgressRange>>,
    run: impl FnOnce(
            progress::DoOrDiscard<prodash::tree::Item>,
            &mut dyn std::io::Write,
            &mut dyn std::io::Write,
        ) -> Result<T>
        + Send
        + 'static,
) -> Result<T> {
    crate::shared::init_env_logger();

    match (verbose, progress) {
        (false, false) => run(
            progress::DoOrDiscard::from(None),
            &mut stdout(),
            &mut stderr(),
        ),
        (true, false) => {
            let progress = progress_tree();
            let sub_progress = progress.add_child(name);

            let handle =
                setup_line_renderer_range(&progress, range.into().unwrap_or(STANDARD_RANGE));

            let mut out = Vec::<u8>::new();
            let mut err = Vec::<u8>::new();
            let res = run(
                progress::DoOrDiscard::from(Some(sub_progress)),
                &mut out,
                &mut err,
            );
            handle.shutdown_and_wait();
            std::io::Write::write_all(&mut stdout(), &out)?;
            std::io::Write::write_all(&mut stderr(), &err)?;
            res
        }
        (true, true) | (false, true) => {
            use std::io::Write;

            use crate::shared;

            enum Event<T> {
                UiDone,
                ComputationDone(Result<T>, Vec<u8>),
            }
            let progress = prodash::tree::Root::new();
            let sub_progress = progress.add_child(name);
            let render_tui = prodash::render::tui(
                stdout(),
                std::sync::Arc::downgrade(&progress),
                prodash::render::tui::Options {
                    title: "gitoxide".into(),
                    frames_per_second: shared::DEFAULT_FRAME_RATE,
                    stop_if_progress_missing: !progress_keep_open,
                    throughput: true,
                    ..Default::default()
                },
            )
            .expect("tui to come up without io error");
            let (tx, rx) = std::sync::mpsc::sync_channel::<Event<T>>(1);
            let ui_handle = std::thread::spawn({
                let tx = tx.clone();
                move || {
                    futures_lite::future::block_on(render_tui);
                    tx.send(Event::UiDone).ok();
                }
            });
            let thread = std::thread::spawn(move || {
                // We might have something interesting to show, which would be hidden by the alternate screen if there is a progress TUI
                // We know that the printing happens at the end, so this is fine.
                let mut out = Vec::new();
                let res = run(
                    progress::DoOrDiscard::from(Some(sub_progress)),
                    &mut out,
                    &mut stderr(),
                );
                tx.send(Event::ComputationDone(res, out)).ok();
            });
            loop {
                match rx.recv() {
                    Ok(Event::UiDone) => {
                        // We don't know why the UI is done, usually it's the user aborting.
                        // We need the computation to stop as well so let's wait for that to happen
                        gix::interrupt::trigger();
                        continue;
                    }
                    Ok(Event::ComputationDone(res, out)) => {
                        ui_handle.join().ok();
                        stdout().write_all(&out)?;
                        break res;
                    }
                    Err(_err) => match thread.join() {
                        Ok(()) => unreachable!(
                            "BUG: We shouldn't fail to receive unless the thread has panicked"
                        ),
                        Err(panic) => std::panic::resume_unwind(panic),
                    },
                }
            }
        }
    }
}

pub fn progress_tree() -> std::sync::Arc<prodash::tree::Root> {
    prodash::tree::root::Options {
        message_buffer_capacity: 200,
        ..Default::default()
    }
    .into()
}

pub fn setup_line_renderer_range(
    progress: &std::sync::Arc<prodash::tree::Root>,
    levels: std::ops::RangeInclusive<prodash::progress::key::Level>,
) -> gix::progress::prodash::render::line::JoinHandle {
    prodash::render::line(
        std::io::stderr(),
        std::sync::Arc::downgrade(progress),
        prodash::render::line::Options {
            level_filter: Some(levels),
            frames_per_second: DEFAULT_FRAME_RATE,
            initial_delay: Some(std::time::Duration::from_millis(1000)),
            timestamp: true,
            throughput: true,
            hide_cursor: true,
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    )
}
