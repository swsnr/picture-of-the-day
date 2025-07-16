// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Parse items from RSS channel documents.

use std::fmt::Display;

use quick_xml::{
    NsReader,
    name::{Namespace, ResolveResult},
};

use crate::xml::{read_text, read_to_start};

#[derive(Debug)]
pub enum RssError {
    XmlError(quick_xml::Error),
    NoRssDocument,
    MissingChannel,
    InvalidDateTime(jiff::Error),
}

impl Display for RssError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::XmlError(error) => write!(f, "Invalid XML: {error}"),
            Self::NoRssDocument => write!(f, "Missing top-level rss element, not an RSS document"),
            Self::MissingChannel => write!(f, "Missing top-level RSS channel"),
            Self::InvalidDateTime(error) => write!(f, "Invalid date time: {error}"),
        }
    }
}

impl std::error::Error for RssError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::XmlError(error) => Some(error),
            Self::InvalidDateTime(error) => Some(error),
            Self::MissingChannel | Self::NoRssDocument => None,
        }
    }
}

impl From<quick_xml::Error> for RssError {
    fn from(error: quick_xml::Error) -> Self {
        Self::XmlError(error)
    }
}

impl From<jiff::Error> for RssError {
    fn from(error: jiff::Error) -> Self {
        Self::InvalidDateTime(error)
    }
}

impl From<quick_xml::events::attributes::AttrError> for RssError {
    fn from(error: quick_xml::events::attributes::AttrError) -> Self {
        quick_xml::Error::InvalidAttr(error).into()
    }
}

type Result<T> = std::result::Result<T, RssError>;

#[derive(Debug, Clone, Default)]
pub struct RssItem {
    pub title: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
    pub thumbnail: Option<String>,
    pub pubdate: Option<jiff::Zoned>,
}

fn read_item(reader: &mut NsReader<&[u8]>) -> Result<RssItem> {
    let mut item = RssItem::default();

    while let Some(start) = read_to_start(reader)? {
        let (ns, local_name) = reader.resolve_element(start.name());
        match (ns, local_name.as_ref()) {
            (ResolveResult::Unbound, b"title") => {
                item.title = Some(read_text(reader)?);
            }
            (ResolveResult::Unbound, b"description") => {
                item.description = Some(read_text(reader)?);
            }
            (ResolveResult::Unbound, b"link") => {
                item.link = Some(read_text(reader)?.trim().to_owned());
            }
            (ResolveResult::Unbound, b"pubDate") => {
                let text = read_text(reader)?;
                let date = jiff::fmt::rfc2822::parse(text.trim())?;
                item.pubdate = Some(date);
            }
            (ResolveResult::Bound(Namespace(b"http://search.yahoo.com/mrss/")), b"thumbnail") => {
                if let Some(url) = start.try_get_attribute(b"url")? {
                    item.thumbnail = Some(
                        url.decode_and_unescape_value(reader.decoder())?
                            .into_owned(),
                    );
                }
                // Skip over the (empty) content of the thumbnail
                reader.read_to_end(start.name())?;
            }
            // Skip over all elements we're not interested in
            _ => {
                reader.read_to_end(start.name())?;
            }
        }
    }

    Ok(item)
}

fn read_to_next_item(reader: &mut NsReader<&[u8]>) -> Result<Option<()>> {
    while let Some(start) = read_to_start(reader)? {
        let (ns, local_name) = reader.resolve_element(start.name());
        if ns == ResolveResult::Unbound && local_name.as_ref() == b"item" {
            return Ok(Some(()));
        }
        // Consume the unknown element
        reader.read_to_end(start.to_end().name())?;
    }
    Ok(None)
}

/// A simple RSS reader.
pub struct RssItemIterator<R>(NsReader<R>);

impl Iterator for RssItemIterator<&[u8]> {
    type Item = Result<RssItem>;

    fn next(&mut self) -> Option<Self::Item> {
        match read_to_next_item(&mut self.0) {
            Ok(Some(())) => Some(read_item(&mut self.0)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

/// Read RSS items from an XML reader.
///
/// Read the RSS channel from the given XML `reader`.
///
/// # Errors
///
/// If reading the RSS channel failed.
pub fn read_rss_channel(mut reader: NsReader<&[u8]>) -> Result<RssItemIterator<&[u8]>> {
    // Expand empty items to avoid having to handle empty elements separately
    reader.config_mut().expand_empty_elements = true;

    // Consume the top-level rss element
    read_to_start(&mut reader)?.ok_or(RssError::NoRssDocument)?;

    // Read until we find the channel
    while let Some(start) = read_to_start(&mut reader)? {
        let (ns, name) = reader.resolve_element(start.name());
        if ns == ResolveResult::Unbound && name.as_ref() == b"channel" {
            break;
        }
        reader.read_to_end(start.name())?;
    }

    Ok(RssItemIterator(reader))
}

#[cfg(test)]
mod tests {
    use quick_xml::NsReader;

    use crate::xml::read_to_start;

    const XML: &str = r#"\
    <rss version="2.0"
         xmlns:atom="http://www.w3.org/2005/Atom"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:content="http://purl.org/rss/1.0/modules/content/"
         xmlns:media="http://search.yahoo.com/mrss/"
         xmlns:georss="http://www.georss.org/georss">
          <channel>
                    <item>
                    <title><![CDATA[Istanbul: A Turkish Delight]]></title>
                    <link>https://earthobservatory.nasa.gov/images/154195/istanbul-a-turkish-delight</link>
                                        <media:thumbnail url="https://eoimages.gsfc.nasa.gov/images/imagerecords/154000/154195/iss072e034369_th.jpg"></media:thumbnail>
                                    <description><![CDATA[The large metropolis, fringed by scenic coastline, stretches across two continents.]]></description>
                                    <content:encoded>
                        <![CDATA[
                        <p>
                            <a href="https://earthobservatory.nasa.gov/images/154195/istanbul-a-turkish-delight"><img
                                        src="https://eoimages.gsfc.nasa.gov/images/imagerecords/154000/154195/iss072e034369_th.jpg"
                                        border="0" alt="Istanbul: A Turkish Delight"/></a><br/>
                            The large metropolis, fringed by scenic coastline, stretches across two continents.</p>
                        <p><a href="https://earthobservatory.nasa.gov/images/154195/istanbul-a-turkish-delight">Read More...</a></p>
                        ]]>
                    </content:encoded>
                                    <dc:creator><![CDATA[ NASA Earth Observatory ]]></dc:creator>
                    <pubDate>Sun, 20 Apr 2025 00:00:00 -0400</pubDate>
                                        <categories>
                                                        <category>Land</category>
                                                </categories>
                                    <guid>https://earthobservatory.nasa.gov/images/154195/istanbul-a-turkish-delight</guid>
                                        <georss:point>29.02 41.06</georss:point>
                                </item>
        </channel>
    </rss>"#;

    #[test]
    fn read_item() {
        let mut reader = NsReader::from_str(XML);

        let start = read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"rss");

        let start = read_to_start(&mut reader).unwrap().unwrap();
        assert_eq!(start.name().as_ref(), b"channel");

        super::read_to_next_item(&mut reader).unwrap();
        let item = super::read_item(&mut reader).unwrap();

        assert_eq!(item.title.unwrap(), "Istanbul: A Turkish Delight");
        assert_eq!(
            item.description.unwrap(),
            "The large metropolis, fringed by scenic coastline, stretches across two continents."
        );
        assert_eq!(
            item.link.unwrap(),
            "https://earthobservatory.nasa.gov/images/154195/istanbul-a-turkish-delight"
        );
        assert_eq!(
            item.thumbnail.unwrap(),
            "https://eoimages.gsfc.nasa.gov/images/imagerecords/154000/154195/iss072e034369_th.jpg"
        );
        assert_eq!(
            item.pubdate.unwrap().timestamp().to_string(),
            "2025-04-20T04:00:00Z"
        );
    }

    #[test]
    fn read_channel() {
        let mut items = super::read_rss_channel(NsReader::from_str(XML))
            .unwrap()
            .collect::<super::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(items.len(), 1);
        let item = items.pop().unwrap();

        assert_eq!(item.title.unwrap(), "Istanbul: A Turkish Delight");
        assert_eq!(
            item.description.unwrap(),
            "The large metropolis, fringed by scenic coastline, stretches across two continents."
        );
        assert_eq!(
            item.link.unwrap(),
            "https://earthobservatory.nasa.gov/images/154195/istanbul-a-turkish-delight"
        );
        assert_eq!(
            item.thumbnail.unwrap(),
            "https://eoimages.gsfc.nasa.gov/images/imagerecords/154000/154195/iss072e034369_th.jpg"
        );
        assert_eq!(
            item.pubdate.unwrap().timestamp().to_string(),
            "2025-04-20T04:00:00Z"
        );
    }
}
