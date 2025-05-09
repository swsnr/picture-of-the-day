// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! XML utilities.

use quick_xml::{
    NsReader, Result,
    events::{BytesStart, Event},
};

pub mod rss;

/// Read to start of next element.
///
/// Return the element start, or `None` when encountering [`Event::Eof`] or
/// [`Event::End`].  Callers should fully read the element before calling this
/// method again, to maintain the current hierarchy level.
pub fn read_to_start<'a>(reader: &mut NsReader<&'a [u8]>) -> Result<Option<BytesStart<'a>>> {
    std::iter::from_fn(|| Some(reader.read_event()))
        .take_while(|e| !matches!(e, Ok(Event::Eof | Event::End(_))))
        .find_map(|e| match e {
            Ok(Event::Start(start)) => Some(Ok(start)),
            Ok(_) => None,
            Err(err) => Some(Err(err)),
        })
        .transpose()
}

/// Read inner text of the current element.
///
/// Call after [`Event::Start`] to read the inner text of the element just started.
pub fn read_text(reader: &mut NsReader<&[u8]>) -> Result<String> {
    let mut buffer = String::new();
    loop {
        let event = reader.read_event()?;
        match event {
            // We should not have nested elements inside text elements
            Event::Start(bytes_start) => {
                // Skip over the element to make sure we're in a consistent state
                reader.read_to_end(bytes_start.to_end().name())?;
                return Err(quick_xml::Error::TextNotFound);
            }
            Event::End(_) => break,
            Event::Eof => {
                return Err(quick_xml::Error::UnexpectedEof(
                    "Unexpected EoF while reading text".into(),
                ));
            }
            Event::Text(text) => {
                let text = text.unescape()?;
                if !text.trim().is_empty() {
                    buffer.push_str(&text);
                }
            }
            Event::CData(cdata) => {
                buffer.push_str(&reader.decoder().decode(&cdata)?);
            }
            _ => {}
        }
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use quick_xml::NsReader;

    #[test]
    fn read_to_start() {
        let xml = r"\
<foo>
     <bar></bar>
  </foo>";
        let mut reader = NsReader::from_str(xml);
        let start = super::read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"foo");
        let start = super::read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"bar");

        let xml = r"\
<foo>
     <bar></bar>
  </different_thing_here>";
        let mut reader = NsReader::from_str(xml);
        let start = super::read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"foo");
        let start = super::read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"bar");

        let xml = r"\
        <foo>
             <bar>
          </different_thing_here>";
        let mut reader = NsReader::from_str(xml);
        let start = super::read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"foo");
        let start = super::read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"bar");
    }
}
