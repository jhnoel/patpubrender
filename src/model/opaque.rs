#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XmlProlog {
    pub xml_version: Option<String>,
    pub xml_encoding: Option<String>,
    pub xml_standalone: Option<String>,
    pub doctype_name: Option<String>,
    pub doctype_public_id: Option<String>,
    pub doctype_system_id: Option<String>,
    pub internal_subset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpaqueBlock {
    pub xml: XmlFragment,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpaqueInline {
    pub xml: XmlFragment,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XmlAttribute {
    pub prefix: Option<String>,
    pub local_name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XmlName {
    pub prefix: Option<String>,
    pub local_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlFragment {
    Element {
        name: XmlName,
        attributes: Vec<XmlAttribute>,
        children: Vec<XmlFragment>,
    },
    Text(String),
}

impl Default for XmlFragment {
    fn default() -> Self {
        Self::Text(String::new())
    }
}
