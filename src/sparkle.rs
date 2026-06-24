use std::io::Cursor;

use anyhow::{Context, Result};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc2822;

/// Static metadata for a Sparkle RSS appcast feed.
#[derive(Clone, Debug)]
pub struct Feed {
    /// RSS channel title.
    pub title: String,
    /// Product or website URL.
    pub link: String,
    /// RSS channel description.
    pub description: String,
    /// RSS language code.
    pub language: String,
}

/// One Sparkle release item.
#[derive(Clone, Debug)]
pub struct Item {
    /// Display title, for example `ArcTap 1.2.0`.
    pub title: String,
    /// Release notes URL.
    pub release_notes_url: String,
    /// CFBundleVersion build number.
    pub build_number: String,
    /// CFBundleShortVersionString display version.
    pub display_version: String,
    /// Sparkle channel. `stable` is omitted from the XML.
    pub channel: String,
    /// Minimum supported macOS version.
    pub minimum_system_version: String,
    /// DMG download URL.
    pub enclosure_url: String,
    /// DMG file length in bytes.
    pub enclosure_length: String,
    /// Sparkle EdDSA signature.
    pub ed_signature: String,
    /// RFC 2822 publication date.
    pub publication_date: String,
}

impl Item {
    /// Fill `publication_date` with the current UTC time.
    pub fn with_current_publication_date(mut self) -> Result<Self> {
        self.publication_date = OffsetDateTime::now_utc()
            .format(&Rfc2822)
            .context("format Sparkle pubDate")?;
        Ok(self)
    }
}

/// Create a new appcast XML document containing `item`.
pub fn new_appcast(feed: &Feed, item: &Item) -> Result<String> {
    let mut writer = xml_writer();
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;

    let mut rss = BytesStart::new("rss");
    rss.push_attribute(("version", "2.0"));
    rss.push_attribute((
        "xmlns:sparkle",
        "http://www.andymatuschak.org/xml-namespaces/sparkle",
    ));
    writer.write_event(Event::Start(rss))?;

    writer.write_event(Event::Start(BytesStart::new("channel")))?;
    write_text_element(&mut writer, "title", &feed.title)?;
    write_text_element(&mut writer, "link", &feed.link)?;
    write_text_element(&mut writer, "description", &feed.description)?;
    write_text_element(&mut writer, "language", &feed.language)?;
    write_item(&mut writer, item)?;
    writer.write_event(Event::End(BytesEnd::new("channel")))?;
    writer.write_event(Event::End(BytesEnd::new("rss")))?;

    writer_to_string(writer)
}

/// Merge `item` into an existing appcast, removing any item for the same display version.
pub fn merge_appcast(existing: &str, item: &Item) -> Result<String> {
    let mut reader = Reader::from_str(existing);
    reader.config_mut().trim_text(false);
    let mut writer = xml_writer();
    let mut inserted = false;

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Start(start) if start.name().as_ref() == b"item" => {
                let events = collect_item_events(&mut reader, start.into_owned())?;
                if !item_events_match_version(&events, &item.display_version)? {
                    write_events(&mut writer, &events)?;
                }
            }
            Event::End(end) if end.name().as_ref() == b"channel" => {
                if !inserted {
                    write_item(&mut writer, item)?;
                    inserted = true;
                }
                writer.write_event(Event::End(end.into_owned()))?;
            }
            event => writer.write_event(event.into_owned())?,
        }
    }

    if !inserted {
        write_item(&mut writer, item)?;
    }

    writer_to_string(writer)
}

fn collect_item_events(
    reader: &mut Reader<&[u8]>,
    start: BytesStart<'static>,
) -> Result<Vec<Event<'static>>> {
    let mut events = vec![Event::Start(start)];
    let mut depth = 1usize;
    while depth > 0 {
        let event = reader.read_event()?.into_owned();
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
        events.push(event);
    }
    Ok(events)
}

fn item_events_match_version(events: &[Event<'static>], display_version: &str) -> Result<bool> {
    let mut current: Option<Vec<u8>> = None;
    for event in events {
        match event {
            Event::Start(start)
                if start.name().as_ref() == b"sparkle:shortVersionString"
                    || start.name().as_ref() == b"sparkle:version" =>
            {
                current = Some(start.name().as_ref().to_vec());
            }
            Event::Text(text)
                if current.is_some() && text.decode()?.as_ref() == display_version =>
            {
                return Ok(true);
            }
            Event::End(end)
                if current
                    .as_deref()
                    .is_some_and(|name| name == end.name().as_ref()) =>
            {
                current = None;
            }
            _ => {}
        }
    }
    Ok(false)
}

fn write_events(writer: &mut Writer<Cursor<Vec<u8>>>, events: &[Event<'static>]) -> Result<()> {
    let mut index = 0;
    while index < events.len() {
        if is_stable_channel_triplet(events, index)? {
            index += 3;
            continue;
        }
        writer.write_event(events[index].clone())?;
        index += 1;
    }
    Ok(())
}

fn is_stable_channel_triplet(events: &[Event<'static>], index: usize) -> Result<bool> {
    let [Event::Start(start), Event::Text(text), Event::End(end)] =
        events.get(index..index + 3).unwrap_or(&[])
    else {
        return Ok(false);
    };
    Ok(start.name().as_ref() == b"sparkle:channel"
        && end.name().as_ref() == b"sparkle:channel"
        && text.decode()?.as_ref() == "stable")
}

fn write_item(writer: &mut Writer<Cursor<Vec<u8>>>, item: &Item) -> Result<()> {
    writer.write_event(Event::Start(BytesStart::new("item")))?;
    write_text_element(writer, "title", &item.title)?;
    write_text_element(writer, "sparkle:releaseNotesLink", &item.release_notes_url)?;
    write_text_element(writer, "pubDate", &item.publication_date)?;
    write_text_element(writer, "sparkle:version", &item.build_number)?;
    write_text_element(writer, "sparkle:shortVersionString", &item.display_version)?;
    if item.channel != "stable" {
        write_text_element(writer, "sparkle:channel", &item.channel)?;
    }
    write_text_element(
        writer,
        "sparkle:minimumSystemVersion",
        &item.minimum_system_version,
    )?;

    let mut enclosure = BytesStart::new("enclosure");
    enclosure.push_attribute(("url", item.enclosure_url.as_str()));
    enclosure.push_attribute(("length", item.enclosure_length.as_str()));
    enclosure.push_attribute(("type", "application/octet-stream"));
    enclosure.push_attribute(("sparkle:edSignature", item.ed_signature.as_str()));
    writer.write_event(Event::Empty(enclosure))?;
    writer.write_event(Event::End(BytesEnd::new("item")))?;
    Ok(())
}

fn write_text_element(writer: &mut Writer<Cursor<Vec<u8>>>, name: &str, text: &str) -> Result<()> {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}

fn xml_writer() -> Writer<Cursor<Vec<u8>>> {
    Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2)
}

fn writer_to_string(writer: Writer<Cursor<Vec<u8>>>) -> Result<String> {
    String::from_utf8(writer.into_inner().into_inner()).context("appcast XML is not UTF-8")
}
