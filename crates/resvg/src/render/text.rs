// ...existing code...

fn render_text_path(
    canvas: &mut Canvas,
    tree: &usvg::Tree,
    text_path: &usvg::TextPath,
    chunks: &[TextChunk],
    span_transform: Transform,
    font_db: &fontdb::Database,
) -> Option<()> {
    // The path is already stored in the TextPath object
    let path = &text_path.path;

    // Render the text along the path using the path directly
    if path.is_empty() {
        return None;
    }

    // Calculate the path length to position text
    let path_len = path_length(path);
    if path_len <= 0.0 {
        return None;
    }

    // Position text at the start_offset along the path
    let mut offset = text_path.start_offset;
    if offset < 0.0 {
        offset = 0.0;
    } else if offset > path_len as f32 {
        offset = path_len as f32;
    }

    // Use the path directly for layout
    layout_text_on_path(
        canvas,
        tree,
        path,
        offset,
        chunks,
        span_transform,
        font_db,
    )
}

// ...existing code...

