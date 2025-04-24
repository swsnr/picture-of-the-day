// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use jiff::civil::Date;

/// A boxed civil date.
///
/// Make [`jiff::civil::Date`] available as property.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, glib::Boxed)]
#[boxed_type(name = "PotDCivilDate", nullable)]
pub struct BoxedCivilDate(Date);

impl From<Date> for BoxedCivilDate {
    fn from(value: Date) -> Self {
        Self(value)
    }
}

impl From<BoxedCivilDate> for Date {
    fn from(value: BoxedCivilDate) -> Self {
        value.0
    }
}

/// Get the current date in the local timezone.
///
/// Take the current date from [`glib::Datetime::now_local`].
pub fn today_local() -> Date {
    let now = glib::DateTime::now_local().unwrap();
    Date::new(
        i16::try_from(now.year()).unwrap(),
        i8::try_from(now.month()).unwrap(),
        i8::try_from(now.day_of_month()).unwrap(),
    )
    .unwrap()
}
