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
