pub(crate) fn apply_paging(
    fetchxml: &str,
    page: i32,
    paging_cookie: Option<&str>,
) -> Result<String, String> {
    let mut updated = upsert_fetch_attr(fetchxml, "page", &page.to_string())?;
    if let Some(cookie) = paging_cookie {
        let escaped = escape_xml_attribute(cookie);
        updated = upsert_fetch_attr(&updated, "paging-cookie", &escaped)?;
    }
    Ok(updated)
}

pub(crate) fn ensure_aggregate_page_size(
    fetchxml: &str,
    aggregate_page_size: i32,
) -> Result<String, String> {
    if !fetchxml.contains("aggregate=\"true\"") {
        return Ok(fetchxml.to_string());
    }

    if fetch_tag_has_attr(fetchxml, "count")? {
        return Ok(fetchxml.to_string());
    }

    upsert_fetch_attr(fetchxml, "count", &aggregate_page_size.to_string())
}

pub(crate) fn fetch_tag_has_attr(fetchxml: &str, name: &str) -> Result<bool, String> {
    let fetch_start = fetchxml
        .find("<fetch")
        .ok_or_else(|| "FetchXML must start with a <fetch> element".to_string())?;
    let tag_end = fetchxml[fetch_start..]
        .find('>')
        .ok_or_else(|| "FetchXML <fetch> element is not closed".to_string())?
        + fetch_start;

    let tag = &fetchxml[fetch_start..=tag_end];
    Ok(tag.contains(&format!("{}=", name)))
}

fn upsert_fetch_attr(fetchxml: &str, name: &str, value: &str) -> Result<String, String> {
    let fetch_start = fetchxml
        .find("<fetch")
        .ok_or_else(|| "FetchXML must start with a <fetch> element".to_string())?;
    let tag_end = fetchxml[fetch_start..]
        .find('>')
        .ok_or_else(|| "FetchXML <fetch> element is not closed".to_string())?
        + fetch_start;

    let tag = &fetchxml[fetch_start..=tag_end];
    let attr_key = format!("{}=", name);
    if let Some(attr_index) = tag.find(&attr_key) {
        let quote_index = attr_index + attr_key.len();
        let quote = tag
            .as_bytes()
            .get(quote_index)
            .ok_or_else(|| format!("Invalid fetch attribute '{}'", name))?;
        if *quote != b'"' && *quote != b'\'' {
            return Err(format!("Invalid fetch attribute '{}'", name));
        }
        let quote_char = *quote as char;
        let value_start = quote_index + 1;
        let value_end = tag[value_start..]
            .find(quote_char)
            .ok_or_else(|| format!("Invalid fetch attribute '{}'", name))?
            + value_start;

        let mut replaced = String::new();
        replaced.push_str(&fetchxml[..fetch_start + value_start]);
        replaced.push_str(value);
        replaced.push_str(&fetchxml[fetch_start + value_end..]);
        return Ok(replaced);
    }

    let mut inserted = String::new();
    inserted.push_str(&fetchxml[..tag_end]);
    inserted.push(' ');
    inserted.push_str(name);
    inserted.push_str("=\"");
    inserted.push_str(value);
    inserted.push('"');
    inserted.push_str(&fetchxml[tag_end..]);
    Ok(inserted)
}

fn escape_xml_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
