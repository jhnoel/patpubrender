use xmloxide::Document;
use xmloxide::parser::{self, ParseOptions};
use xmloxide::tree::{NodeId, NodeKind};

use crate::error::ParseError;
use crate::model::opaque::{XmlAttribute, XmlFragment, XmlName, XmlProlog};

pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    let options = ParseOptions::default().max_entity_expansions(100_000);
    parser::parse_str_with_options(input, &options)
        .map_err(|error| ParseError::MalformedXml(error.to_string()))
}

pub fn root_element(document: &Document) -> Result<NodeId, ParseError> {
    document
        .root_element()
        .ok_or_else(|| ParseError::MalformedXml("missing root element".to_string()))
}

pub fn child_elements(document: &Document, node: NodeId) -> Vec<NodeId> {
    document
        .children(node)
        .filter(|child| document.is_element(*child))
        .collect()
}

pub fn child_content_nodes(document: &Document, node: NodeId) -> Vec<NodeId> {
    document
        .children(node)
        .filter(|child| match &document.node(*child).kind {
            NodeKind::Text { content } | NodeKind::CData { content } => !content.trim().is_empty(),
            _ => true,
        })
        .collect()
}

pub fn attributes(document: &Document, node: NodeId) -> Vec<XmlAttribute> {
    document
        .attributes(node)
        .iter()
        .map(|attribute| XmlAttribute {
            prefix: attribute.prefix.clone(),
            local_name: attribute.name.clone(),
            value: attribute.value.clone(),
        })
        .collect()
}

pub fn text_content(document: &Document, node: NodeId) -> String {
    document.text_content(node)
}

pub fn prolog(input: &str, document: &Document) -> XmlProlog {
    let mut doctype_name = None;
    let mut doctype_public_id = None;
    let mut doctype_system_id = None;
    let mut internal_subset = None;

    for child in document.children(document.root()) {
        if let NodeKind::DocumentType {
            name,
            public_id,
            system_id,
            internal_subset: subset,
            ..
        } = &document.node(child).kind
        {
            doctype_name = Some(name.clone());
            doctype_public_id = public_id.clone();
            doctype_system_id = system_id.clone();
            internal_subset = subset.clone();
            break;
        }
    }

    let declaration = xml_declaration(input);

    XmlProlog {
        xml_version: declaration
            .and_then(|value| declaration_attribute(value, "version"))
            .or_else(|| document.version.clone()),
        xml_encoding: declaration.and_then(|value| declaration_attribute(value, "encoding")),
        xml_standalone: declaration.and_then(|value| declaration_attribute(value, "standalone")),
        doctype_name,
        doctype_public_id,
        doctype_system_id,
        internal_subset,
    }
}

fn xml_declaration(input: &str) -> Option<&str> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("<?xml") {
        return None;
    }

    let end = trimmed.find("?>")?;
    Some(&trimmed[..end + 2])
}

fn declaration_attribute(declaration: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=");
    let start = declaration.find(&pattern)? + pattern.len();
    let remainder = &declaration[start..];
    let quote = remainder.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let rest = &remainder[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

pub fn xml_fragment(document: &Document, node: NodeId) -> XmlFragment {
    match &document.node(node).kind {
        NodeKind::Element {
            name,
            prefix,
            attributes,
            ..
        } => XmlFragment::Element {
            name: XmlName {
                prefix: prefix.clone(),
                local_name: name.clone(),
            },
            attributes: attributes
                .iter()
                .map(|attribute| XmlAttribute {
                    prefix: attribute.prefix.clone(),
                    local_name: attribute.name.clone(),
                    value: attribute.value.clone(),
                })
                .collect(),
            children: document
                .children(node)
                .map(|child| xml_fragment(document, child))
                .collect(),
        },
        NodeKind::Text { content } | NodeKind::CData { content } => {
            XmlFragment::Text(content.clone())
        }
        NodeKind::EntityRef { name, value } => {
            XmlFragment::Text(value.clone().unwrap_or_else(|| format!("&{name};")))
        }
        NodeKind::Comment { content } => XmlFragment::Text(format!("<!--{content}-->")),
        NodeKind::ProcessingInstruction { target, data } => XmlFragment::Text(match data {
            Some(data) => format!("<?{target} {data}?>"),
            None => format!("<?{target}?>"),
        }),
        NodeKind::DocumentType {
            name,
            system_id,
            public_id,
            internal_subset,
        } => {
            let mut rendered = format!("<!DOCTYPE {name}");
            if let Some(public_id) = public_id {
                rendered.push_str(&format!(" PUBLIC \"{public_id}\""));
            }
            if let Some(system_id) = system_id {
                if public_id.is_none() {
                    rendered.push_str(" SYSTEM");
                }
                rendered.push_str(&format!(" \"{system_id}\""));
            }
            if let Some(internal_subset) = internal_subset {
                rendered.push_str(&format!(" [{internal_subset}]"));
            }
            rendered.push('>');
            XmlFragment::Text(rendered)
        }
        NodeKind::Document => XmlFragment::Text(String::new()),
    }
}

pub fn write_prolog(output: &mut String, prolog: &XmlProlog) {
    if let Some(version) = &prolog.xml_version {
        output.push_str("<?xml");
        output.push_str(&format!(" version=\"{version}\""));
        if let Some(encoding) = &prolog.xml_encoding {
            output.push_str(&format!(" encoding=\"{encoding}\""));
        }
        if let Some(standalone) = &prolog.xml_standalone {
            output.push_str(&format!(" standalone=\"{standalone}\""));
        }
        output.push_str("?>");
    }

    if let Some(doctype_name) = &prolog.doctype_name {
        output.push_str("<!DOCTYPE ");
        output.push_str(doctype_name);
        if let Some(public_id) = &prolog.doctype_public_id {
            output.push_str(&format!(" PUBLIC \"{public_id}\""));
        }
        if let Some(system_id) = &prolog.doctype_system_id {
            if prolog.doctype_public_id.is_none() {
                output.push_str(" SYSTEM");
            }
            output.push_str(&format!(" \"{system_id}\""));
        }
        if let Some(internal_subset) = &prolog.internal_subset {
            output.push_str(" [");
            output.push_str(internal_subset);
            output.push(']');
        }
        output.push('>');
    }
}

pub fn start_tag(output: &mut String, tag: &str, attributes: &[XmlAttribute]) {
    output.push('<');
    output.push_str(tag);
    write_attributes(output, attributes);
    output.push('>');
}

pub fn start_empty_tag(output: &mut String, tag: &str, attributes: &[XmlAttribute]) {
    output.push('<');
    output.push_str(tag);
    write_attributes(output, attributes);
    output.push_str("/>");
}

pub fn end_tag(output: &mut String, tag: &str) {
    output.push_str("</");
    output.push_str(tag);
    output.push('>');
}

pub fn write_text_element(output: &mut String, tag: &str, value: &str) {
    start_tag(output, tag, &[]);
    output.push_str(&escape_text(value));
    end_tag(output, tag);
}

pub fn write_xml_fragment(output: &mut String, fragment: &XmlFragment) {
    match fragment {
        XmlFragment::Text(value) => {
            if is_raw_markup_text(value) {
                output.push_str(value);
            } else {
                output.push_str(&escape_text(value));
            }
        }
        XmlFragment::Element {
            name,
            attributes,
            children,
        } => {
            let tag = qualified_name(name.prefix.as_deref(), &name.local_name);
            start_tag(output, &tag, attributes);
            for child in children {
                write_xml_fragment(output, child);
            }
            end_tag(output, &tag);
        }
    }
}

pub fn escape_text(value: &str) -> String {
    let mut escaped = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            '&' if looks_like_entity(&chars[index..]) => {
                escaped.push('&');
                index += 1;
            }
            '&' => {
                escaped.push_str("&amp;");
                index += 1;
            }
            '<' => {
                escaped.push_str("&lt;");
                index += 1;
            }
            '>' => {
                escaped.push_str("&gt;");
                index += 1;
            }
            character => {
                escaped.push(character);
                index += 1;
            }
        }
    }

    escaped
}

pub fn escape_attribute(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

pub fn qualified_name(prefix: Option<&str>, local_name: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}:{local_name}"),
        None => local_name.to_string(),
    }
}

pub fn write_attributes(output: &mut String, attributes: &[XmlAttribute]) {
    for attribute in attributes {
        output.push(' ');
        output.push_str(&qualified_name(
            attribute.prefix.as_deref(),
            &attribute.local_name,
        ));
        output.push_str("=\"");
        output.push_str(&escape_attribute(&attribute.value));
        output.push('"');
    }
}

fn looks_like_entity(chars: &[char]) -> bool {
    let Some(position) = chars.iter().position(|character| *character == ';') else {
        return false;
    };

    if position < 2 {
        return false;
    }

    chars[1..position]
        .iter()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '#' | '-' | '_'))
}

fn is_raw_markup_text(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with("<?") || trimmed.starts_with("<!")
}
