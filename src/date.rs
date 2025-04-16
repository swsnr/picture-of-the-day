// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use chrono::NaiveDate;

/// A boxed naive date.
///
/// Make [`chrono::NaiveDate`] available as property.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, glib::Boxed)]
#[boxed_type(name = "PotDNaiveDate", nullable)]
pub struct BoxedNaiveDate(NaiveDate);

impl From<NaiveDate> for BoxedNaiveDate {
    fn from(value: NaiveDate) -> Self {
        Self(value)
    }
}

impl From<BoxedNaiveDate> for NaiveDate {
    fn from(value: BoxedNaiveDate) -> Self {
        value.0
    }
}

/// Get the current date in the local timezone.
///
/// Take the current date from [`glib::Datetime::now_local`].
pub fn today_local() -> NaiveDate {
    let now = glib::DateTime::now_local().unwrap();
    NaiveDate::from_yo_opt(now.year(), u32::try_from(now.day_of_year()).unwrap()).unwrap()
}
