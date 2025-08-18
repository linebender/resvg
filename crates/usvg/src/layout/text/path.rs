    if segments.is_empty() {
        return None;
    }

    // Find a total offset along the path.
    let path_len = path_length(path.as_ref()) as f32;
    if path_len <= 0.0 {
        return None; // Can't layout text on a zero-length path
    }

    let mut offset = text_path.start_offset;
    // Ensure offset is within valid range
    if offset < 0.0 {
        offset = 0.0;
    } else if offset > path_len {
        offset = path_len;
    }

    let mut spans = Vec::new();
