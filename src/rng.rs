// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use rand_core::RngCore;

/// A [`rand::RngCore`] Rng implementation on top of Glib's internal RNG.
///
/// This enables the nice convenience APIs of [`rand`] without requiring any of
/// of the [`rand`] default features, and bringing in additional generator
/// dependencies.
#[derive(Copy, Clone)]
pub struct GlibRng;

impl RngCore for GlibRng {
    fn next_u32(&mut self) -> u32 {
        glib::random_int()
    }

    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_u32(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand_core::impls::fill_bytes_via_next(self, dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}
