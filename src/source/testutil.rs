// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use soup::prelude::SessionExt;

/// Create a session for testing
pub fn soup_session() -> soup::Session {
    let session = soup::Session::new();
    session.set_user_agent(crate::config::USER_AGENT);
    session
}
