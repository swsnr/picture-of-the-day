// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use soup::prelude::SessionExt;

/// Create a session for testing
pub fn soup_session() -> soup::Session {
    let session = soup::Session::new();
    session.set_user_agent(crate::config::USER_AGENT);
    session
}

// TODO: Replace with glib::async_test once fixed
// See https://github.com/gtk-rs/gtk-rs-core/pull/1787 and
// https://github.com/gtk-rs/gtk-rs-core/pull/1789
/// Run a future on a new thread-default main context.
pub fn block_on_new_main_context<F>(f: F)
where
    F: Future<Output = ()>,
{
    let main_ctx = glib::MainContext::new();
    main_ctx
        .with_thread_default(|| main_ctx.block_on(f))
        .unwrap();
}
