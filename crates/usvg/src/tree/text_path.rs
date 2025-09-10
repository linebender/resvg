// ...existing code...

impl TextPath {
    pub fn from_xml_node(node: &roxmltree::Node) -> Option<Self> {
        // ...existing code...

        // Fix: Support both xlink:href and href
        let href = node.attribute((XLINK_NAMESPACE, "href"))
            .or_else(|| node.attribute("href"))?;

        // ...existing code...
        Some(TextPath {
            href: href.to_string(),
            // ...existing fields...
        })
    }
}

// ...existing code...

