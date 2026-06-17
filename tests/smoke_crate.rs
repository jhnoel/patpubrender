use patpubrender::{
    SourceFormat, detect_source_format, parse_patent_aps, parse_patent_xml,
    parse_patent_xml_with_format, render_markdown, write_patent_xml,
};

#[test]
fn crate_exports_public_entry_points() {
    let _ = detect_source_format;
    let _ = parse_patent_xml;
    let _ = parse_patent_xml_with_format;
    let _ = parse_patent_aps;
    let _ = render_markdown;
    let _ = write_patent_xml;
    let _ = SourceFormat::UsptoApplicationV15;
}
