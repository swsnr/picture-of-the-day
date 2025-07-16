// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

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
            // We should not have nested elements inside text elements, but if
            // reality surprises us there, let's skip over the element to
            // put ourselves into a consistent state, and then continue.
            Event::Start(bytes_start) => {
                reader.read_to_end(bytes_start.to_end().name())?;
            }
            Event::End(_) => break,
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
