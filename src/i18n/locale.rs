// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use glib::GString;

/// Find all language and territory codes in the current locale environment.
///
/// Like [`glib::language_names`] but filters all items with codeset and modifier,
/// and also C and POSIX locales, and thus only returns actual language and
/// language/territory codes.
pub fn language_and_territory_codes() -> impl Iterator<Item = GString> {
    glib::language_names()
        .into_iter()
        // Filter items with codesets (separated by .) and modifiers (separated by @)
        // See setlocale(3).
        //
        // What remains are `language` and `language_territory` items; these always
        // exists even if the actual locale configuration uses `language_territory.codeset``,
        // because glib explodes all locale name variants into the returned array.
        .filter(|c| !c.contains(['.', '@']))
        // Filter out portable locales which do not correspond to any particular
        // language or territory, see setlocale(3)
        .filter(|c| c != "C" && c != "POSIX")
}

/// Find all language codes from the current locale environment.
///
/// Like [`language_and_territory_codes`] but also filter `language_territory`
/// items to only leave plain ASCII-only language codes.
pub fn language_codes() -> impl Iterator<Item = GString> {
    // _ separates language from territory, see setlocale(3)
    language_and_territory_codes().filter(|c| !c.contains('_'))
}
