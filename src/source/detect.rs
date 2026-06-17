use crate::error::DetectError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    UsptoApplicationV15,
    UsptoApplicationV16,
    UsptoApplicationV40,
    UsptoApplicationV41,
    UsptoApplicationV42,
    UsptoApplicationV43,
    UsptoApplicationV44,
    UsptoApplicationV45,
    UsptoApplicationV46,
    UsptoGrantV25,
    UsptoGrantV40,
    UsptoGrantV41,
    UsptoGrantV42,
    UsptoGrantV43,
    UsptoGrantV44,
    UsptoGrantV45,
    UsptoGrantV46,
    UsptoGrantV47,
    /// USPTO Green Book "APS" plain-text patent grants (product PTGRAPS,
    /// 1976-2001). Not XML: tagged fixed-column text, records separated by a
    /// bare `PATN` line.
    UsptoGrantApsGreenBook,
}

/// Recognize the Green Book "APS" plain-text format. These files carry no `<`
/// markup, so they must be detected before the XML root scan. A weekly file
/// begins with an `HHHHHT ... APS1 ... ISSUE` header line; a bare record begins
/// with a line that is exactly `PATN`.
pub fn is_aps_green_book(input: &str) -> bool {
    let mut lines = input.lines();
    // Skip any leading blank lines.
    let Some(first) = lines.find(|line| !line.trim().is_empty()) else {
        return false;
    };
    let first = first.trim_end();
    if first.starts_with("HHHHH") && first.contains("APS") {
        return true;
    }
    // A split record (or a header-less file) starts with a bare `PATN` line.
    first.trim() == "PATN"
}

pub fn detect_source_format(input: &str) -> Result<SourceFormat, DetectError> {
    if input.trim().is_empty() {
        return Err(DetectError::MalformedXml);
    }

    if is_aps_green_book(input) {
        return Ok(SourceFormat::UsptoGrantApsGreenBook);
    }

    let root_family = detect_root_family(input)?;
    let markers = detect_markers(input, root_family);

    for marker in &markers {
        if marker.family != root_family {
            return Err(DetectError::ConflictingVersionMarkers);
        }
    }

    let mut detected_format = None;
    for marker in markers {
        if let Some(format) = marker.format {
            if detected_format.is_some_and(|existing| existing != format) {
                return Err(DetectError::ConflictingVersionMarkers);
            }
            detected_format = Some(format);
        }
    }

    detected_format.ok_or(DetectError::UnknownFormat)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentFamily {
    Application,
    Grant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkerFamily {
    family: DocumentFamily,
    format: Option<SourceFormat>,
}

fn detect_root_family(input: &str) -> Result<DocumentFamily, DetectError> {
    let mut cursor = input.trim_start();

    while let Some(start) = cursor.find('<') {
        if !cursor[..start].trim().is_empty() {
            return Err(DetectError::MalformedXml);
        }

        let content = &cursor[start + 1..];

        if content.starts_with('?') {
            let end = content.find("?>").ok_or(DetectError::MalformedXml)?;
            cursor = &content[end + 2..];
            continue;
        }

        if content.starts_with("!DOCTYPE") {
            let end = content
                .find("]>")
                .map(|idx| idx + 2)
                .or_else(|| content.find('>').map(|idx| idx + 1))
                .ok_or(DetectError::MalformedXml)?;
            cursor = &content[end..];
            continue;
        }

        if content.starts_with("!--") {
            let end = content.find("-->").ok_or(DetectError::MalformedXml)?;
            cursor = &content[end + 3..];
            continue;
        }

        if content.starts_with('!') || content.starts_with('/') {
            return Err(DetectError::MalformedXml);
        }

        let name = root_name(content).ok_or(DetectError::MalformedXml)?;
        return match name {
            "patent-application-publication" | "us-patent-application" => {
                Ok(DocumentFamily::Application)
            }
            "us-patent-grant" | "PATDOC" => Ok(DocumentFamily::Grant),
            other => Err(DetectError::UnsupportedRoot(other.to_string())),
        };
    }

    Err(DetectError::MalformedXml)
}

fn detect_markers(input: &str, root_family: DocumentFamily) -> Vec<MarkerFamily> {
    let mut markers = Vec::new();

    if input.contains("pap-v15-2001-01-31.dtd") {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV15),
        });
    }
    if input.contains("pap-v16") {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV16),
        });
    }
    if input.contains("ST32-US-Grant-025xml.dtd") || input.contains("DTD=\"2.5\"") {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV25),
        });
    }

    if input.contains("us-patent-application-v40")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.0")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV40),
        });
    }
    if input.contains("us-patent-application-v41")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.1")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV41),
        });
    }
    if input.contains("us-patent-application-v42")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.2")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV42),
        });
    }
    if input.contains("us-patent-application-v43")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.3")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV43),
        });
    }
    if input.contains("us-patent-application-v44")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.4")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV44),
        });
    }
    if input.contains("us-patent-application-v45")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.5")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV45),
        });
    }
    if input.contains("us-patent-application-v46")
        || root_family == DocumentFamily::Application && input.contains("dtd-version=\"v4.6")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: Some(SourceFormat::UsptoApplicationV46),
        });
    }
    if input.contains("us-patent-grant-v40")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v40")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV40),
        });
    }
    if input.contains("us-patent-grant-v41")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.1")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV41),
        });
    }
    if input.contains("us-patent-grant-v42")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.2")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV42),
        });
    }
    if input.contains("us-patent-grant-v43")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.3")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV43),
        });
    }
    if input.contains("us-patent-grant-v44")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.4")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV44),
        });
    }
    if input.contains("us-patent-grant-v45")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.5")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV45),
        });
    }
    if input.contains("us-patent-grant-v46")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.6")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV46),
        });
    }
    if input.contains("us-patent-grant-v47")
        || root_family == DocumentFamily::Grant && input.contains("dtd-version=\"v4.7")
    {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: Some(SourceFormat::UsptoGrantV47),
        });
    }

    if input.contains("<!DOCTYPE us-patent-application") {
        markers.push(MarkerFamily {
            family: DocumentFamily::Application,
            format: None,
        });
    }
    if input.contains("<!DOCTYPE us-patent-grant") || input.contains("<!DOCTYPE PATDOC") {
        markers.push(MarkerFamily {
            family: DocumentFamily::Grant,
            format: None,
        });
    }

    markers
}

fn root_name(content: &str) -> Option<&str> {
    let end = content
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(content.len());
    (end > 0).then_some(&content[..end])
}
